use crate::print_ops::{
    CHANNEL_PRINT, KonanDeliverPayload, MIME_OUTLINE, MIME_TRACKER, PrintFileTask,
    print_file_directory,
};
use crate::template::{BoxOutline, HabitTracker};
use async_trait::async_trait;
use cadence_core::{
    channels::Artifact,
    error::CadenceError,
    registry::{Handler, JobContext, JobOutcome},
};

pub struct OutlineBuildHandler;

#[async_trait]
impl Handler<BoxOutline> for OutlineBuildHandler {
    async fn run(
        &self,
        outline: BoxOutline,
        _ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        JobOutcome::spawn(KonanDeliverPayload::Outline { outline })
    }
}

pub struct TrackerBuildHandler;

#[async_trait]
impl Handler<HabitTracker> for TrackerBuildHandler {
    async fn run(
        &self,
        tracker: HabitTracker,
        _ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        JobOutcome::spawn(KonanDeliverPayload::Tracker { tracker })
    }
}

pub struct FileBuildHandler;

#[async_trait]
impl Handler<PrintFileTask> for FileBuildHandler {
    async fn run(
        &self,
        task: PrintFileTask,
        _ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        let file_path = print_file_directory().join(&task.file_name);
        if !file_path.exists() {
            return Err(CadenceError::Channel(format!(
                "file not found: {}",
                task.file_name
            )));
        }

        JobOutcome::spawn_with(
            KonanDeliverPayload::File {
                file_name: task.file_name.clone(),
                rows: task.rows,
            },
            Some(task.file_name),
            std::time::Duration::ZERO,
        )
    }
}

pub struct KonanPrintDeliverHandler;

#[async_trait]
impl Handler<KonanDeliverPayload> for KonanPrintDeliverHandler {
    async fn run(
        &self,
        payload: KonanDeliverPayload,
        ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        let artifact = match payload {
            KonanDeliverPayload::Outline { outline } => Artifact::Bytes {
                mime: MIME_OUTLINE.into(),
                data: serde_json::to_vec(&outline)
                    .map_err(|e| CadenceError::Channel(e.to_string()))?,
            },
            KonanDeliverPayload::Tracker { tracker } => Artifact::Bytes {
                mime: MIME_TRACKER.into(),
                data: serde_json::to_vec(&tracker)
                    .map_err(|e| CadenceError::Channel(e.to_string()))?,
            },
            KonanDeliverPayload::File { file_name, rows: _ } => {
                Artifact::MarkdownFile(print_file_directory().join(file_name))
            }
        };
        ctx.channels.deliver(CHANNEL_PRINT, artifact).await?;
        Ok(JobOutcome::Done)
    }
}
