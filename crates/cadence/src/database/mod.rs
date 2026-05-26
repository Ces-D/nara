use crate::error::CadenceError;
use chrono::{DateTime, Utc};
use rrule::{RRule, Unvalidated};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod connection;
pub use connection::{CadenceDBPool, CadenceDBPoolConnection, pool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub rrule: Option<RRule<Unvalidated>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub at_unix: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub next_run_unix: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub start_unix: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(i64)]
pub enum JobStatus {
    Pending = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
}

impl JobStatus {
    pub fn from_i64(v: i64) -> Option<Self> {
        Some(match v {
            0 => Self::Pending,
            1 => Self::Running,
            2 => Self::Completed,
            3 => Self::Failed,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub schedule_id: Option<i64>,
    pub parent_job_id: Option<i64>,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub artifact_ref: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub due_unix: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub finished_at: Option<DateTime<Utc>>,
}

// ~~~~~~~~~~~~ ops

pub async fn pop_pending_job(
    pool: &connection::CadenceDBPool,
) -> Result<Option<Job>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let conn = pool.get()?;
        let job = conn
            .query_row(
                "UPDATE job
                 SET status = ?1
                 WHERE id = (
                     SELECT id FROM job
                     WHERE status = ?2 AND due_unix <= ?3
                     ORDER BY due_unix ASC
                     LIMIT 1
                 )
                 RETURNING id, schedule_id, parent_job_id, task_type, payload, status,
                           artifact_ref, due_unix, created_at, finished_at",
                rusqlite::params![JobStatus::Running as i64, JobStatus::Pending as i64, now,],
                row_to_job,
            )
            .optional()?;
        Ok::<_, CadenceError>(job)
    })
    .await?
}

pub async fn mark_job_failed(
    pool: &connection::CadenceDBPool,
    job_id: i64,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE job
             SET status = ?1, finished_at = ?2
             WHERE id = ?3",
            rusqlite::params![JobStatus::Failed as i64, Utc::now().timestamp(), job_id,],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

pub async fn mark_job_completed(
    pool: &connection::CadenceDBPool,
    job_id: i64,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE job
             SET status = ?1, finished_at = ?2
             WHERE id = ?3",
            rusqlite::params![JobStatus::Completed as i64, Utc::now().timestamp(), job_id,],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

pub struct CreateChildJob {
    pub parent_id: i64,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub artifact_ref: Option<String>,
    pub delay: Duration,
}

pub async fn insert_child_job(
    pool: &connection::CadenceDBPool,
    job: CreateChildJob,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    let task_type = job.task_type.clone();
    let payload = job.payload.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let due_unix = now + job.delay.as_secs() as i64;
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO job
                 (schedule_id, parent_job_id, task_type, payload, status,
                  artifact_ref, due_unix, created_at, finished_at)
             VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            rusqlite::params![
                job.parent_id,
                task_type,
                payload,
                JobStatus::Pending as i64,
                job.artifact_ref,
                due_unix,
                now,
            ],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

pub async fn requeue_job(
    pool: &connection::CadenceDBPool,
    job_id: i64,
    after: Duration,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let due_unix = Utc::now().timestamp() + after.as_secs() as i64;
        let conn = pool.get()?;
        conn.execute(
            "UPDATE job
             SET status = ?1, due_unix = ?2, finished_at = NULL
             WHERE id = ?3",
            rusqlite::params![JobStatus::Pending as i64, due_unix, job_id],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

pub struct CreateJob {
    pub schedule_id: Option<i64>,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub artifact_ref: Option<String>,
    pub due_unix: DateTime<Utc>,
}

pub async fn insert_job(
    pool: &connection::CadenceDBPool,
    job: CreateJob,
) -> Result<i64, CadenceError> {
    let pool = pool.clone();
    let task_type = job.task_type;
    let payload = job.payload.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO job
                 (schedule_id, parent_job_id, task_type, payload, status,
                  artifact_ref, due_unix, created_at, finished_at)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            rusqlite::params![
                job.schedule_id,
                task_type,
                payload,
                JobStatus::Pending as i64,
                job.artifact_ref,
                job.due_unix.timestamp(),
                now,
            ],
        )?;
        Ok::<_, CadenceError>(conn.last_insert_rowid())
    })
    .await?
}

pub async fn get_job(
    pool: &connection::CadenceDBPool,
    id: i64,
) -> Result<Option<Job>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let job = conn
            .query_row(
                "SELECT id, schedule_id, parent_job_id, task_type, payload, status,
                        artifact_ref, due_unix, created_at, finished_at
                 FROM job WHERE id = ?1",
                rusqlite::params![id],
                row_to_job,
            )
            .optional()?;
        Ok::<_, CadenceError>(job)
    })
    .await?
}

#[derive(Debug, Default, Clone)]
pub struct ListJobsFilter {
    pub status: Option<JobStatus>,
    pub schedule_id: Option<i64>,
    pub parent_job_id: Option<i64>,
}

pub async fn list_jobs(
    pool: &connection::CadenceDBPool,
    filter: ListJobsFilter,
) -> Result<Vec<Job>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, schedule_id, parent_job_id, task_type, payload, status,
                    artifact_ref, due_unix, created_at, finished_at
             FROM job
             WHERE (:status IS NULL OR status = :status)
               AND (:schedule_id IS NULL OR schedule_id = :schedule_id)
               AND (:parent_job_id IS NULL OR parent_job_id = :parent_job_id)
             ORDER BY due_unix DESC",
        )?;
        let jobs = stmt
            .query_map(
                rusqlite::named_params! {
                    ":status": filter.status.map(|s| s as i64),
                    ":schedule_id": filter.schedule_id,
                    ":parent_job_id": filter.parent_job_id,
                },
                row_to_job,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, CadenceError>(jobs)
    })
    .await?
}

pub async fn get_children_of(
    pool: &connection::CadenceDBPool,
    parent_job_id: i64,
) -> Result<Vec<Job>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, schedule_id, parent_job_id, task_type, payload, status,
                    artifact_ref, due_unix, created_at, finished_at
             FROM job WHERE parent_job_id = ?1 ORDER BY created_at",
        )?;
        let jobs = stmt
            .query_map(rusqlite::params![parent_job_id], row_to_job)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, CadenceError>(jobs)
    })
    .await?
}

// ~~~~~~~~~~~~ schedule ops

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSchedule {
    pub name: String,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub rrule: Option<RRule<Unvalidated>>,
    #[serde(default, with = "chrono::serde::ts_seconds_option")]
    pub at_unix: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub start_unix: DateTime<Utc>,
}

pub async fn create_schedule(
    pool: &connection::CadenceDBPool,
    schedule: CreateSchedule,
) -> Result<i64, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let next_run = compute_initial_next_run(
            schedule.start_unix,
            schedule.rrule.as_ref(),
            schedule.at_unix,
        )?;
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO schedule
                 (name, task_type, payload, rrule, at_unix, next_run_unix, start_unix)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                schedule.name,
                schedule.task_type,
                schedule.payload,
                schedule.rrule.as_ref().map(|r| r.to_string()),
                schedule.at_unix.map(|d| d.timestamp()),
                next_run,
                schedule.start_unix.timestamp(),
            ],
        )?;
        Ok::<_, CadenceError>(conn.last_insert_rowid())
    })
    .await?
}

pub async fn get_schedule(
    pool: &connection::CadenceDBPool,
    id: i64,
) -> Result<Option<Schedule>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let schedule = conn
            .query_row(
                "SELECT id, name, task_type, payload, rrule, at_unix, next_run_unix, start_unix
                 FROM schedule WHERE id = ?1",
                rusqlite::params![id],
                row_to_schedule,
            )
            .optional()?;
        Ok::<_, CadenceError>(schedule)
    })
    .await?
}

pub async fn list_schedules(
    pool: &connection::CadenceDBPool,
) -> Result<Vec<Schedule>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, task_type, payload, rrule, at_unix, next_run_unix, start_unix
             FROM schedule ORDER BY id",
        )?;
        let schedules = stmt
            .query_map([], row_to_schedule)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, CadenceError>(schedules)
    })
    .await?
}

pub async fn get_due_schedules(
    pool: &connection::CadenceDBPool,
) -> Result<Vec<Schedule>, CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let now = Utc::now().timestamp();
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, task_type, payload, rrule, at_unix, next_run_unix, start_unix
             FROM schedule
             WHERE next_run_unix IS NOT NULL AND next_run_unix <= ?1",
        )?;
        let schedules = stmt
            .query_map(rusqlite::params![now], row_to_schedule)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, CadenceError>(schedules)
    })
    .await?
}

pub async fn update_schedule(
    pool: &connection::CadenceDBPool,
    id: i64,
    update: CreateSchedule,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let next_run =
            compute_initial_next_run(update.start_unix, update.rrule.as_ref(), update.at_unix)?;
        let conn = pool.get()?;
        conn.execute(
            "UPDATE schedule
             SET name = ?1, task_type = ?2, payload = ?3, rrule = ?4,
                 at_unix = ?5, next_run_unix = ?6, start_unix = ?7
             WHERE id = ?8",
            rusqlite::params![
                update.name,
                update.task_type,
                update.payload,
                update.rrule.as_ref().map(|r| r.to_string()),
                update.at_unix.map(|d| d.timestamp()),
                next_run,
                update.start_unix.timestamp(),
                id,
            ],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

pub async fn delete_schedule(
    pool: &connection::CadenceDBPool,
    id: i64,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        conn.execute("DELETE FROM schedule WHERE id = ?1", rusqlite::params![id])?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

/// Recompute `next_run_unix` after the schedule has fired.
/// For recurring schedules, advances to the next rrule occurrence.
/// For one-shot schedules, clears `next_run_unix` so it won't fire again.
pub async fn advance_schedule(
    pool: &connection::CadenceDBPool,
    id: i64,
) -> Result<(), CadenceError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let schedule = conn.query_row(
            "SELECT id, name, task_type, payload, rrule, at_unix, next_run_unix, start_unix
             FROM schedule WHERE id = ?1",
            rusqlite::params![id],
            row_to_schedule,
        )?;
        let next_run = match schedule.rrule.as_ref() {
            Some(rrule) => next_run_from_rrule(schedule.start_unix, rrule)?,
            None => None,
        };
        conn.execute(
            "UPDATE schedule SET next_run_unix = ?1 WHERE id = ?2",
            rusqlite::params![next_run, id],
        )?;
        Ok::<_, CadenceError>(())
    })
    .await?
}

// ~~~~~~~~~~~~ Helpers
fn ts(row: &rusqlite::Row<'_>, col: &str) -> rusqlite::Result<DateTime<Utc>> {
    let v: i64 = row.get(col)?;
    DateTime::<Utc>::from_timestamp(v, 0).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            format!("invalid timestamp in {col}: {v}").into(),
        )
    })
}

fn ts_opt(row: &rusqlite::Row<'_>, col: &str) -> rusqlite::Result<Option<DateTime<Utc>>> {
    row.get::<_, Option<i64>>(col)?
        .map(|v| {
            DateTime::<Utc>::from_timestamp(v, 0).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    format!("invalid timestamp in {col}: {v}").into(),
                )
            })
        })
        .transpose()
}

fn rrule_to_db_error(error: rrule::RRuleError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn next_run_from_rrule(
    start: DateTime<Utc>,
    rrule: &RRule<Unvalidated>,
) -> rusqlite::Result<Option<i64>> {
    let tz = rrule::Tz::America__New_York;
    let validated = rrule
        .clone()
        .validate(start.with_timezone(&tz))
        .map_err(rrule_to_db_error)?;
    Ok(rrule::RRuleSet::new(start.with_timezone(&tz))
        .rrule(validated)
        .after(Utc::now().with_timezone(&tz))
        .all(1)
        .dates
        .first()
        .map(|d| d.timestamp()))
}

fn compute_initial_next_run(
    start: DateTime<Utc>,
    rrule: Option<&RRule<Unvalidated>>,
    at_unix: Option<DateTime<Utc>>,
) -> rusqlite::Result<Option<i64>> {
    match rrule {
        Some(r) => next_run_from_rrule(start, r),
        None => Ok(at_unix.map(|d| d.timestamp())),
    }
}

fn row_to_schedule(row: &rusqlite::Row<'_>) -> rusqlite::Result<Schedule> {
    let rrule_str: Option<String> = row.get("rrule")?;
    let rrule = rrule_str
        .map(|s| s.parse::<RRule<Unvalidated>>())
        .transpose()
        .map_err(rrule_to_db_error)?;
    Ok(Schedule {
        id: row.get("id")?,
        name: row.get("name")?,
        task_type: row.get("task_type")?,
        payload: row.get("payload")?,
        rrule,
        at_unix: ts_opt(row, "at_unix")?,
        next_run_unix: ts_opt(row, "next_run_unix")?,
        start_unix: ts(row, "start_unix")?,
    })
}

fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let status_int: i64 = row.get("status")?;
    let status = JobStatus::from_i64(status_int).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Integer,
            format!("unknown job status: {status_int}").into(),
        )
    })?;

    Ok(Job {
        id: row.get("id")?,
        schedule_id: row.get("schedule_id")?,
        parent_job_id: row.get("parent_job_id")?,
        task_type: row.get("task_type")?,
        payload: row.get("payload")?,
        status,
        artifact_ref: row.get("artifact_ref")?,
        due_unix: ts(row, "due_unix")?,
        created_at: ts(row, "created_at")?,
        finished_at: ts_opt(row, "finished_at")?,
    })
}
