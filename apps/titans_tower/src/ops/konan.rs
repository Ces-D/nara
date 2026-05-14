use crate::db::run_konan_blocking;
use crate::error::ServiceError;
use konan_core::{
    print_ops::{
        self, CreatePrintJob, CreateSchedule, KonanDbPool, PrintFileTask, PrintTask, Schedule,
    },
    template::{BoxOutline, HabitTracker},
};

pub async fn create_outline(pool: KonanDbPool, outline: BoxOutline) -> Result<(), ServiceError> {
    let job = CreatePrintJob {
        task: PrintTask::Outline(outline),
        schedule_id: None,
    };
    run_konan_blocking(pool, move |conn| {
        print_ops::create_print_job(conn, job)?;
        Ok(())
    })
    .await
}

pub async fn create_tracker(pool: KonanDbPool, tracker: HabitTracker) -> Result<(), ServiceError> {
    let job = CreatePrintJob {
        task: PrintTask::Tracker(tracker),
        schedule_id: None,
    };
    run_konan_blocking(pool, move |conn| {
        print_ops::create_print_job(conn, job)?;
        Ok(())
    })
    .await
}

pub async fn create_file_job(pool: KonanDbPool, file: PrintFileTask) -> Result<(), ServiceError> {
    let job = CreatePrintJob {
        task: PrintTask::File(file),
        schedule_id: None,
    };
    run_konan_blocking(pool, move |conn| {
        print_ops::create_print_job(conn, job)?;
        Ok(())
    })
    .await
}

pub async fn create_schedule(
    pool: KonanDbPool,
    schedule: CreateSchedule,
) -> Result<usize, ServiceError> {
    run_konan_blocking(pool, move |conn| {
        Ok(print_ops::create_schedule(conn, schedule)?)
    })
    .await
}

pub async fn list_schedules(pool: KonanDbPool) -> Result<Vec<Schedule>, ServiceError> {
    run_konan_blocking(pool, |conn| Ok(print_ops::list_schedules(conn)?)).await
}

pub async fn delete_schedule(pool: KonanDbPool, id: i64) -> Result<usize, ServiceError> {
    run_konan_blocking(pool, move |conn| Ok(print_ops::delete_schedule(conn, id)?)).await
}

pub async fn upload_file(
    file_name: String,
    content: axum::body::Bytes,
) -> Result<(), ServiceError> {
    tokio::task::spawn_blocking(move || print_ops::upload_print_file(&file_name, &content))
        .await??;
    Ok(())
}
