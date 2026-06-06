use super::{DailyDillySummarize, SCHEDULE_NAME, SUMMARY_HOUR};
use crate::error::ServiceError;
use cadence_core::database::CadenceDBPool;

/// Idempotently ensure the recurring 10pm (America/New_York) Daily Dilly summary
/// schedule exists, so it survives restarts without piling up duplicates.
pub async fn ensure_schedule(pool: &CadenceDBPool) -> Result<(), ServiceError> {
    let existing = cadence_core::database::list_schedules(pool).await?;
    if existing.iter().any(|s| s.name == SCHEDULE_NAME) {
        log::debug!("daily-dilly: schedule already present");
        return Ok(());
    }
    let start = chrono::Utc::now();
    let rrule = titans_tower::parse_rrule(
        "rrule",
        &format!("FREQ=DAILY;BYHOUR={SUMMARY_HOUR};BYMINUTE=0;BYSECOND=0"),
        start,
    )?;
    cadence_core::schedule::<DailyDillySummarize>(
        pool,
        SCHEDULE_NAME.to_string(),
        DailyDillySummarize::default(),
        Some(rrule),
        None,
        start,
    )
    .await?;
    log::info!("daily-dilly: seeded summary schedule (10pm America/New_York)");
    Ok(())
}
