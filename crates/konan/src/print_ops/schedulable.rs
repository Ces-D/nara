use crate::print_ops::{
    CHANNEL_PRINT, KonanDeliverPayload, MIME_OUTLINE, MIME_TRACKER, PrintFileTask, TASK_FILE_BUILD,
    TASK_OUTLINE_BUILD, TASK_PRINT_DELIVER, TASK_TRACKER_BUILD, print_file_directory,
};
use crate::template::{BoxOutline, HabitTracker};
use async_trait::async_trait;
use cadence_core::{
    channels::Artifact,
    error::CadenceError,
    registry::{JobContext, JobOutcome, Schedulable},
};
use std::time::Duration;

pub struct OutlineBuildHandler;

#[async_trait]
impl Schedulable for OutlineBuildHandler {
    fn task_type(&self) -> &'static str {
        TASK_OUTLINE_BUILD
    }

    async fn run(&self, ctx: &JobContext) -> Result<JobOutcome, CadenceError> {
        let outline: BoxOutline = serde_json::from_value(ctx.payload.clone())
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        let payload = serde_json::to_value(KonanDeliverPayload::Outline { outline })
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        Ok(JobOutcome::Spawn {
            task_type: TASK_PRINT_DELIVER.into(),
            payload,
            artifact_ref: None,
            delay: Duration::ZERO,
        })
    }
}

pub struct TrackerBuildHandler;

#[async_trait]
impl Schedulable for TrackerBuildHandler {
    fn task_type(&self) -> &'static str {
        TASK_TRACKER_BUILD
    }

    async fn run(&self, ctx: &JobContext) -> Result<JobOutcome, CadenceError> {
        let tracker: HabitTracker = serde_json::from_value(ctx.payload.clone())
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        let payload = serde_json::to_value(KonanDeliverPayload::Tracker { tracker })
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        Ok(JobOutcome::Spawn {
            task_type: TASK_PRINT_DELIVER.into(),
            payload,
            artifact_ref: None,
            delay: Duration::ZERO,
        })
    }
}

pub struct FileBuildHandler;

#[async_trait]
impl Schedulable for FileBuildHandler {
    fn task_type(&self) -> &'static str {
        TASK_FILE_BUILD
    }

    async fn run(&self, ctx: &JobContext) -> Result<JobOutcome, CadenceError> {
        let task: PrintFileTask = serde_json::from_value(ctx.payload.clone())
            .map_err(|e| CadenceError::Channel(e.to_string()))?;

        let file_path = print_file_directory().join(&task.file_name);
        if !file_path.exists() {
            return Err(CadenceError::Channel(format!(
                "file not found: {}",
                task.file_name
            )));
        }

        let payload = serde_json::to_value(KonanDeliverPayload::File {
            file_name: task.file_name.clone(),
            rows: task.rows,
        })
        .map_err(|e| CadenceError::Channel(e.to_string()))?;

        Ok(JobOutcome::Spawn {
            task_type: TASK_PRINT_DELIVER.into(),
            payload,
            artifact_ref: Some(task.file_name),
            delay: Duration::ZERO,
        })
    }
}

pub struct KonanPrintDeliverHandler;

#[async_trait]
impl Schedulable for KonanPrintDeliverHandler {
    fn task_type(&self) -> &'static str {
        TASK_PRINT_DELIVER
    }

    async fn run(&self, ctx: &JobContext) -> Result<JobOutcome, CadenceError> {
        let payload: KonanDeliverPayload = serde_json::from_value(ctx.payload.clone())
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
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
