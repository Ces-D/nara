use crate::{
    database::{self, CadenceDBPool, CreateJob, CreateSchedule},
    error::CadenceError,
    registry::Task,
};
use chrono::{DateTime, Utc};
use rrule::{RRule, Unvalidated};

/// Enqueue a job for `T`. The task_type is derived from `T::TASK_TYPE`, and the
/// payload is serialized exactly once at this boundary.
pub async fn enqueue<T: Task>(pool: &CadenceDBPool, payload: T) -> Result<i64, CadenceError> {
    let payload =
        serde_json::to_value(payload).map_err(|e| CadenceError::Channel(e.to_string()))?;
    database::insert_job(
        pool,
        CreateJob {
            schedule_id: None,
            task_type: T::TASK_TYPE.to_string(),
            payload,
            artifact_ref: None,
            due_unix: Utc::now(),
        },
    )
    .await
}

pub async fn schedule<T: Task>(
    pool: &CadenceDBPool,
    name: String,
    payload: T,
    rrule: Option<RRule<Unvalidated>>,
    at_unix: Option<DateTime<Utc>>,
    start_unix: DateTime<Utc>,
) -> Result<i64, CadenceError> {
    let payload =
        serde_json::to_value(payload).map_err(|e| CadenceError::Channel(e.to_string()))?;
    database::create_schedule(
        pool,
        CreateSchedule {
            name,
            task_type: T::TASK_TYPE.to_string(),
            payload,
            rrule,
            at_unix,
            start_unix,
        },
    )
    .await
}
