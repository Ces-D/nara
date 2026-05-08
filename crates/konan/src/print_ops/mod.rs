use crate::template::{BoxOutline, HabitTracker};
use chrono::{DateTime, Utc};
use rrule::{RRule, Validated};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};

mod connection;
mod database;

pub use connection::{KonanDbError, KonanDbPool, KonanDbPoolConnection, pool};
pub use database::{
    advance_schedules, create_print_job, create_schedule, delete_schedule, get_due_schedules,
    get_pending_print_job, list_schedules, read_print_file, update_print_job_status,
    upload_print_file,
};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PrintFileTask {
    pub file_name: String,
    pub rows: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum PrintTask {
    Outline(BoxOutline),
    Tracker(HabitTracker),
    File(PrintFileTask),
}

impl FromSql for PrintTask {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        serde_json::from_str(&s).map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

pub enum PrintJobStatus {
    Pending,
    Completed,
    Failed,
}

impl PrintJobStatus {
    fn as_int(&self) -> i64 {
        match self {
            Self::Pending => 0,
            Self::Completed => 1,
            Self::Failed => 2,
        }
    }
}

impl FromSql for PrintJobStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match i64::column_result(value)? {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Completed),
            2 => Ok(Self::Failed),
            n => Err(FromSqlError::OutOfRange(n)),
        }
    }
}

pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub task: PrintTask,
    pub r_rule: String,
    pub start_unix: i64,
    pub next_run_unix: Option<i64>,
}

pub struct CreateSchedule {
    pub name: String,
    pub task: PrintTask,
    pub r_rule: RRule<Validated>,
    pub start: DateTime<Utc>,
}

pub struct PrintJob {
    pub id: i64,
    schedule_id: Option<i64>,
    pub task: PrintTask,
    created_at_unix: i64,
    status: PrintJobStatus,
}

pub struct CreatePrintJob {
    pub task: PrintTask,
    pub schedule_id: Option<i64>,
}
