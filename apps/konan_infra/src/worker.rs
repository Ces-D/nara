use konan_core::interpreter::MarkdownInterpreter;
use konan_core::print_ops::{
    PrintJobStatus, get_pending_print_job, pool, read_print_file, update_print_job_status,
};
use konan_core::printer::RongtaPrinter;
use std::time::Duration;

pub async fn worker_loop() {
    let pool = pool().expect("failed to open database connection");
    loop {
        let conn = pool.get().expect("failed to check out database connection");
        let sleep_for = match get_pending_print_job(&conn)
            .expect("failed to query pending print jobs")
        {
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
                        let _ = update_print_job_status(&conn, job_id, PrintJobStatus::Completed);
                        Duration::from_secs(15)
                    }
                    Err(e) => {
                        log::error!("print job {job_id} failed: {e}");
                        let _ = update_print_job_status(&conn, job_id, PrintJobStatus::Failed);
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
