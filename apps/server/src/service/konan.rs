use crate::error::{ServiceError, ServiceResult};
use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use cadence_core::database::{CreateSchedule, Schedule};
use konan_core::{
    KonanScheduler,
    print_ops::{self, PrintFileTask},
    template::{BoxOutline, HabitTracker},
};

pub const MAX_UPLOAD_BYTES: usize = 1024 * 1024;

pub async fn print_outline(
    State(scheduler): State<KonanScheduler>,
    Json(payload): Json<BoxOutline>,
) -> ServiceResult<StatusCode> {
    scheduler.print_outline(payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn print_tracker(
    State(scheduler): State<KonanScheduler>,
    Json(payload): Json<HabitTracker>,
) -> ServiceResult<StatusCode> {
    scheduler.print_tracker(payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn print_file(
    State(scheduler): State<KonanScheduler>,
    Json(payload): Json<PrintFileTask>,
) -> ServiceResult<StatusCode> {
    scheduler.print_file(payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn upload_file(mut multipart: Multipart) -> ServiceResult<StatusCode> {
    let mut uploaded = 0;
    while let Some(field) = multipart.next_field().await? {
        let raw_name = field.file_name().map(str::to_string).ok_or_else(|| {
            ServiceError::BadRequest("missing file name in multipart field".into())
        })?;
        let file_name = sanitize_upload_filename(&raw_name).map_err(ServiceError::BadRequest)?;

        if !file_name.ends_with(".md") {
            return Err(ServiceError::BadRequest(format!(
                "file must be a markdown file (.md): {file_name}"
            )));
        }

        let data = field.bytes().await?;

        if data.len() > MAX_UPLOAD_BYTES {
            return Err(ServiceError::PayloadTooLarge(format!(
                "file exceeds 1MB limit: {} bytes",
                data.len()
            )));
        }

        if std::str::from_utf8(&data).is_err() {
            return Err(ServiceError::BadRequest(
                "file content must be valid UTF-8".into(),
            ));
        }

        tokio::task::spawn_blocking(move || print_ops::upload_print_file(&file_name, &data))
            .await??;
        uploaded += 1;
    }

    if uploaded == 0 {
        return Err(ServiceError::BadRequest("no files uploaded".into()));
    }

    Ok(StatusCode::CREATED)
}

/// Rejects any filename that contains path separators, NUL bytes, or `..`
/// components, and additionally collapses to `Path::file_name` as a belt-and-
/// suspenders check. Returns the validated basename, or a message suitable for
/// surfacing as a BadRequest.
fn sanitize_upload_filename(raw: &str) -> Result<String, String> {
    let bad = raw.is_empty()
        || raw.contains('/')
        || raw.contains('\\')
        || raw.contains('\0')
        || raw.contains("..");
    if bad {
        return Err(format!("invalid file name: {raw}"));
    }
    let basename = std::path::Path::new(raw)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("invalid file name: {raw}"))?;
    if basename != raw {
        return Err(format!("invalid file name: {raw}"));
    }
    Ok(basename.to_string())
}

pub async fn create_print_schedule(
    State(scheduler): State<KonanScheduler>,
    Json(payload): Json<CreateSchedule>,
) -> ServiceResult<Json<i64>> {
    let id = scheduler.create_schedule_raw(payload).await?;
    Ok(Json(id))
}

pub async fn list_scheduled_print_tasks(
    State(scheduler): State<KonanScheduler>,
) -> ServiceResult<Json<Vec<Schedule>>> {
    let schedules = scheduler.list_schedules().await?;
    Ok(Json(schedules))
}

pub async fn delete_scheduled_print_task(
    State(scheduler): State<KonanScheduler>,
    Path(id): Path<i64>,
) -> ServiceResult<StatusCode> {
    scheduler.delete_schedule(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
