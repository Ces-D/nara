use super::connection::{KonanDbError, KonanDbPoolConnection};
use crate::print_file_directory;
use crate::print_ops::{CreatePrintJob, CreateSchedule, PrintJob, PrintJobStatus, Schedule};
use chrono::{DateTime, Utc};
use rrule::{RRule, Unvalidated, Validated};
use rusqlite::{OptionalExtension, named_params};
use std::io;

fn task_serialize_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(error.into())
}
fn rrule_parse_error(error: rrule::RRuleError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
fn invalid_start_unix(unix: i64) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Integer,
        format!("invalid start_unix: {unix}").into(),
    )
}

/////////////// Print Job

/// Returns the oldest pending print job, or None if the queue is empty.
pub fn get_pending_print_job(
    conn: &KonanDbPoolConnection,
) -> Result<Option<PrintJob>, KonanDbError> {
    Ok(conn
        .query_row(
            "SELECT id, schedule_id, task, created_at_unix, status \
             FROM print_job WHERE status = :status ORDER BY created_at_unix LIMIT 1",
            named_params! { ":status": PrintJobStatus::Pending.as_int() },
            |row| {
                Ok(PrintJob {
                    id: row.get(0)?,
                    schedule_id: row.get(1)?,
                    task: row.get(2)?,
                    created_at_unix: row.get(3)?,
                    status: row.get(4)?,
                })
            },
        )
        .optional()?)
}

/// Updates the status of a print job by id.
pub fn update_print_job_status(
    conn: &KonanDbPoolConnection,
    id: i64,
    status: PrintJobStatus,
) -> Result<usize, KonanDbError> {
    Ok(conn.execute(
        "UPDATE print_job SET status = :status WHERE id = :id",
        named_params! {
            ":status": status.as_int(),
            ":id": id,
        },
    )?)
}

/// Inserts a new print job with status Pending and created_at_unix set to now.
pub fn create_print_job(
    conn: &KonanDbPoolConnection,
    job: CreatePrintJob,
) -> Result<usize, KonanDbError> {
    let task = serde_json::to_string(&job.task).map_err(task_serialize_error)?;
    let created_at_unix = Utc::now().timestamp();
    let status = PrintJobStatus::Pending.as_int();
    Ok(conn.execute(
        "INSERT INTO print_job (schedule_id, task, created_at_unix, status) \
         VALUES (:schedule_id, :task, :created_at_unix, :status)",
        named_params! {
            ":schedule_id": job.schedule_id,
            ":task": task,
            ":created_at_unix": created_at_unix,
            ":status": status,
        },
    )?)
}

/////////////// Schedule

fn next_run_unix(start: DateTime<Utc>, r_rule: RRule<Validated>) -> Option<i64> {
    rrule::RRuleSet::new(start.with_timezone(&rrule::Tz::America__New_York))
        .rrule(r_rule)
        .after(Utc::now().with_timezone(&rrule::Tz::America__New_York))
        .all(1)
        .dates
        .first()
        .map(|v| v.timestamp())
}

/// Returns all schedules whose `next_run_unix` is <= now (i.e. past-due).
pub fn get_due_schedules(conn: &KonanDbPoolConnection) -> Result<Vec<Schedule>, KonanDbError> {
    let now = Utc::now().timestamp();
    let mut stmt = conn.prepare(
        "SELECT id, name, task, r_rule, start_unix, next_run_unix \
         FROM schedule WHERE next_run_unix <= :now",
    )?;
    let schedules = stmt
        .query_map(named_params! { ":now": now }, |row| {
            Ok(Schedule {
                id: row.get(0)?,
                name: row.get(1)?,
                task: row.get(2)?,
                r_rule: row.get(3)?,
                start_unix: row.get(4)?,
                next_run_unix: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(schedules)
}

/// Returns every schedule row, ordered by id.
pub fn list_schedules(conn: &KonanDbPoolConnection) -> Result<Vec<Schedule>, KonanDbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, task, r_rule, start_unix, next_run_unix \
         FROM schedule ORDER BY id",
    )?;
    let schedules = stmt
        .query_map([], |row| {
            Ok(Schedule {
                id: row.get(0)?,
                name: row.get(1)?,
                task: row.get(2)?,
                r_rule: row.get(3)?,
                start_unix: row.get(4)?,
                next_run_unix: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(schedules)
}

/// Deletes a schedule by id.
pub fn delete_schedule(conn: &KonanDbPoolConnection, id: i64) -> Result<usize, KonanDbError> {
    Ok(conn.execute(
        "DELETE FROM schedule WHERE id = :id",
        named_params! { ":id": id },
    )?)
}

/// Inserts a new schedule row, computing the initial `next_run_unix` from the rrule.
pub fn create_schedule(
    conn: &KonanDbPoolConnection,
    schedule: CreateSchedule,
) -> Result<usize, KonanDbError> {
    let task = serde_json::to_string(&schedule.task).map_err(task_serialize_error)?;
    let start = schedule.start.timestamp();
    let r_rule = schedule
        .r_rule
        .validate(schedule.start.with_timezone(&rrule::Tz::America__New_York))
        .map_err(rrule_parse_error)?;
    let next_run = next_run_unix(schedule.start, r_rule.clone());
    let out = conn.execute(
        "INSERT INTO schedule (name, task, r_rule, start_unix, next_run_unix) \
         VALUES (:name, :task, :r_rule, :start_unix, :next_run_unix)",
        named_params! {
            ":name": schedule.name,
            ":task": task,
            ":r_rule": r_rule.to_string(),
            ":start_unix": start,
            ":next_run_unix": next_run,
        },
    )?;
    Ok(out)
}

/// Recalculates and updates `next_run_unix` for each schedule based on its rrule and current time.
pub fn advance_schedules(
    conn: &KonanDbPoolConnection,
    schedules: Vec<Schedule>,
) -> Result<(), KonanDbError> {
    for schedule in schedules {
        let start = DateTime::<Utc>::from_timestamp(schedule.start_unix, 0)
            .ok_or_else(|| invalid_start_unix(schedule.start_unix))?;
        let r_rule: RRule<Unvalidated> = schedule.r_rule.parse().map_err(rrule_parse_error)?;
        let r_rule = r_rule
            .validate(start.with_timezone(&rrule::Tz::America__New_York))
            .map_err(rrule_parse_error)?;
        let next_run = next_run_unix(start, r_rule);
        conn.execute(
            "UPDATE schedule SET next_run_unix = :next_run_unix WHERE id = :id",
            named_params! {
                ":next_run_unix": next_run,
                ":id": schedule.id,
            },
        )?;
    }
    Ok(())
}

/////////////// Print File

/// Reads a file from the print file directory by name.
pub fn read_print_file(file_name: &str) -> io::Result<Vec<u8>> {
    let path = print_file_directory().join(file_name);
    std::fs::read(&path)
}

/// Writes a markdown file to the print file directory.
/// Returns an error if `file_name` does not end with `.md` or contains any
/// path-component characters. Callers are still expected to validate input,
/// but this check stops path-traversal even if a caller forgets.
pub fn upload_print_file(file_name: &str, content: &[u8]) -> io::Result<()> {
    if !file_name.ends_with(".md") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("file must be a markdown file (.md): {file_name}"),
        ));
    }
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
        || file_name.contains("..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid file name: {file_name}"),
        ));
    }
    let dir = print_file_directory();
    std::fs::write(dir.join(file_name), content)
}
