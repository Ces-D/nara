use konan_core::interpreter::MarkdownInterpreter;
use konan_core::print_ops::{
    KonanDbPoolConnection, PrintJobStatus, get_pending_print_job, pool, read_print_file,
    update_print_job_status,
};
use konan_core::printer::RongtaPrinter;
use std::time::Duration;

pub async fn worker_loop() {
    // Failure to open the database at startup is non-recoverable; let systemd restart us.
    let pool = pool().expect("failed to open database connection");
    loop {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("worker: failed to check out database connection: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let pending = match get_pending_print_job(&conn) {
            Ok(p) => p,
            Err(e) => {
                log::error!("worker: failed to query pending print jobs: {e}");
                drop(conn);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let sleep_for = match pending {
            Some(pending) => {
                let job_id = pending.id;
                let result = match pending.task {
                    konan_core::print_ops::PrintTask::Outline(box_outline) => {
                        let mut printer = RongtaPrinter::new(true);
                        let driver = konan_core::printer::configured_printer();
                        box_outline.print(&mut printer, driver).map(|_| ())
                    }
                    konan_core::print_ops::PrintTask::Tracker(habit_tracker) => {
                        let mut printer = RongtaPrinter::new(true);
                        let driver = konan_core::printer::configured_printer();
                        habit_tracker.print(&mut printer, driver).map(|_| ())
                    }
                    konan_core::print_ops::PrintTask::File(file_task) => {
                        match read_print_file(&file_task.file_name) {
                            Ok(bytes) => {
                                let content = String::from_utf8_lossy(&bytes);
                                let printer = RongtaPrinter::new(true);
                                let mut interpreter = MarkdownInterpreter::new(printer);
                                interpreter.render_content(&content);
                                let driver = konan_core::printer::configured_printer();
                                interpreter.print(file_task.rows, driver).map(|_| ())
                            }
                            Err(e) => Err(e.into()),
                        }
                    }
                };

                match result {
                    Ok(_) => {
                        log::info!("print job {job_id} completed successfully");
                        flush_status(&conn, job_id, PrintJobStatus::Completed).await;
                        Duration::from_secs(15)
                    }
                    Err(e) => {
                        log::error!("print job {job_id} failed: {e}");
                        flush_status(&conn, job_id, PrintJobStatus::Failed).await;
                        Duration::from_secs(30)
                    }
                }
            }
            None => Duration::from_secs(60),
        };
        drop(conn);
        tokio::time::sleep(sleep_for).await;
    }
}

/// Persist a job's terminal status, retrying transient failures. Panics if all
/// retries fail — letting the job remain in `Pending` while the worker continues
/// would cause the same job to be picked up and reprinted on the next iteration.
async fn flush_status(conn: &KonanDbPoolConnection, job_id: i64, status: PrintJobStatus) {
    const MAX_ATTEMPTS: u32 = 3;
    for attempt in 1..=MAX_ATTEMPTS {
        match update_print_job_status(conn, job_id, status) {
            Ok(_) => return,
            Err(e) if attempt < MAX_ATTEMPTS => {
                log::warn!(
                    "worker: failed to update status for job {job_id} (attempt {attempt}/{MAX_ATTEMPTS}): {e}"
                );
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(e) => {
                log::error!(
                    "worker: failed to update status for job {job_id} after {MAX_ATTEMPTS} attempts: {e}"
                );
                panic!("worker: unable to persist status for job {job_id}; aborting");
            }
        }
    }
}
