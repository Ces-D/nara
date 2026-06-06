use crate::error::ServiceError;
use async_trait::async_trait;
use bean::database::BeanDBPool;
use cadence_core::{
    error::CadenceError,
    registry::{Handler, JobContext, JobOutcome, Task, TaskRegistry},
};
use chrono::{DateTime, TimeZone, Utc};
use collect::PersonDay;
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Http};
use std::sync::Arc;

mod collect;
mod schedule;
mod store;
mod summarize;

pub use schedule::ensure_schedule;

/// The channel this feature reads from and posts summaries back to.
pub const CHANNEL: &str = "daily-dilly";

/// Name of the recurring cadence schedule that drives the nightly summary.
pub const SCHEDULE_NAME: &str = "daily-dilly-summary";

/// Hour (America/New_York) at which the daily summary runs.
pub const SUMMARY_HOUR: u32 = 22;

const DISCORD_MAX_MESSAGE_LEN: usize = 1900;

/// Detailed help text pinned to #daily-dilly on startup.
pub fn help_message() -> String {
    "**Daily Dilly**\n\
     Drop your thoughts here through the day. Every night at 10pm (US Eastern) \
     each person's messages from that day are summarized and saved, and the \
     summary is posted back here."
        .to_string()
}

/// Empty payload — the handler discovers everything it needs at run time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyDillySummarize {}

impl Task for DailyDillySummarize {
    const TASK_TYPE: &'static str = "daily_dilly.summarize";
}

pub struct DailyDillyHandler {
    http: Arc<Http>,
    bean: BeanDBPool,
    channel_id: ChannelId,
    summarizer: Arc<dyn summarize::Summarizer>,
}

impl DailyDillyHandler {
    /// Construct a handler using the default (placeholder) summarizer.
    pub fn new(http: Arc<Http>, bean: BeanDBPool, channel_id: ChannelId) -> Self {
        Self::with_summarizer(http, bean, channel_id, Arc::new(summarize::JoinSummarizer))
    }

    /// Construct a handler with an explicit summarizer — the seam for an
    /// LLM-backed implementation (and for tests with a fake).
    pub fn with_summarizer(
        http: Arc<Http>,
        bean: BeanDBPool,
        channel_id: ChannelId,
        summarizer: Arc<dyn summarize::Summarizer>,
    ) -> Self {
        Self {
            http,
            bean,
            channel_id,
            summarizer,
        }
    }

    async fn summarize_and_post(
        &self,
        person: &PersonDay,
        entry_date: DateTime<Utc>,
    ) -> Result<(), CadenceError> {
        let (title, summary) = self.summarizer.summarize(person).await?;
        store::store_summary(&self.bean, person, &title, &summary, entry_date).await?;

        let mut post = format!("**{}** — {title}\n{summary}", person.display);
        if post.len() > DISCORD_MAX_MESSAGE_LEN {
            post.truncate(DISCORD_MAX_MESSAGE_LEN);
            post.push('…');
        }
        self.channel_id
            .say(&self.http, post)
            .await
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl Handler<DailyDillySummarize> for DailyDillyHandler {
    async fn run(
        &self,
        _payload: DailyDillySummarize,
        _ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        let day_start = et_today_at(0, 0);
        let entry_date = et_today_at(SUMMARY_HOUR, 0);

        // Gather the day's messages from the configured #daily-dilly channel,
        // grouped by author.
        let by_author =
            match collect::collect_day_messages(&self.http, self.channel_id, day_start.timestamp())
                .await
            {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("daily-dilly: failed to read #{CHANNEL}: {e}");
                    return Ok(JobOutcome::Done);
                }
            };

        for person in by_author.values() {
            if let Err(e) = self.summarize_and_post(person, entry_date).await {
                log::error!(
                    "daily-dilly: failed to summarize for {}: {e}",
                    person.username
                );
            }
        }

        Ok(JobOutcome::Done)
    }
}

/// Register the Daily Dilly summary handler with the cadence task registry,
/// resolving the feature's channel from the owner-provisioned config.
pub fn register(
    tasks: &mut TaskRegistry,
    http: Arc<Http>,
    bean: BeanDBPool,
) -> Result<(), ServiceError> {
    let channel_id = titans_tower::configured_channel_id(CHANNEL)?;
    tasks.register::<DailyDillySummarize, _>(DailyDillyHandler::new(http, bean, channel_id));
    Ok(())
}

/// `hour:min` today in America/New_York, as a UTC instant. Falls back to "now"
/// if the local time is somehow ambiguous/invalid (e.g. a DST transition).
fn et_today_at(hour: u32, min: u32) -> DateTime<Utc> {
    let tz = rrule::Tz::America__New_York;
    let now_et = Utc::now().with_timezone(&tz);
    let Some(naive) = now_et.date_naive().and_hms_opt(hour, min, 0) else {
        return Utc::now();
    };
    tz.from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}
