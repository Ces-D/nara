use crate::{
    channels::ChannelRegistry,
    database,
    error::CadenceError,
    registry::{JobContext, JobOutcome, TaskRegistry},
};
use chrono::Utc;
use std::{sync::Arc, time::Duration};

pub async fn worker_tick(
    db: &database::CadenceDBPool,
    registry: &TaskRegistry,
    channels: &ChannelRegistry,
) -> Result<(), CadenceError> {
    let Some(job) = database::pop_pending_job(db).await? else {
        return Ok(());
    };

    let Some(handler) = registry.get(&job.task_type) else {
        database::mark_job_failed(db, job.id).await?;
        log::warn!("no handler for task_type={}", job.task_type);
        return Ok(());
    };

    let ctx = JobContext {
        job_id: job.id,
        payload: job.payload,
        artifact_ref: job.artifact_ref,
        channels,
        db,
    };
    let outcome = handler.run(&ctx).await.unwrap_or_else(JobOutcome::Failed);
    match outcome {
        JobOutcome::Done => database::mark_job_completed(db, job.id).await?,
        JobOutcome::Spawn {
            task_type,
            payload,
            artifact_ref,
            delay,
        } => {
            let child = database::CreateChildJob {
                parent_id: job.id,
                task_type,
                payload,
                artifact_ref,
                delay,
            };
            database::insert_child_job(db, child).await?;
            database::mark_job_completed(db, job.id).await?;
        }
        JobOutcome::Retry { after } => database::requeue_job(db, job.id, after).await?,
        JobOutcome::Failed(cadence_error) => {
            log::error!("Job failed: {:?}", cadence_error);
            database::mark_job_failed(db, job.id).await?
        }
    };
    Ok(())
}

pub async fn scheduler_tick(db: &database::CadenceDBPool) -> Result<(), CadenceError> {
    let due = database::get_due_schedules(db).await?;
    for s in due {
        let job = database::CreateJob {
            schedule_id: Some(s.id),
            task_type: s.task_type,
            payload: s.payload,
            artifact_ref: None,
            due_unix: Utc::now(),
        };
        database::insert_job(db, job).await?;
        database::advance_schedule(db, s.id).await?;
    }
    Ok(())
}

pub async fn run(
    db: database::CadenceDBPool,
    tasks: Arc<TaskRegistry>,
    channels: Arc<ChannelRegistry>,
) {
    let scheduler_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = scheduler_tick(&scheduler_db).await {
                log::error!("scheduler_tick: {e}");
            }
        }
    });
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        if let Err(e) = worker_tick(&db, &tasks, &channels).await {
            log::error!("worker_tick: {e}");
        }
    }
}
