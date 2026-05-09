use konan_core::print_ops::{
    CreatePrintJob, advance_schedules, create_print_job, get_due_schedules, pool,
};
use std::time::Duration;

pub async fn scheduler_loop() {
    // Failure to open the database at startup is non-recoverable; let systemd restart us.
    let pool = pool().expect("failed to open database connection");
    loop {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("scheduler: failed to check out database connection: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        let due_schedules = match get_due_schedules(&conn) {
            Ok(s) => s,
            Err(e) => {
                log::error!("scheduler: failed to query due schedules: {e}");
                drop(conn);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let mut tick_failed = false;
        for row in due_schedules.iter() {
            let job = CreatePrintJob {
                task: row.task.clone(),
                schedule_id: Some(row.id),
            };
            if let Err(e) = create_print_job(&conn, job) {
                log::error!(
                    "scheduler: failed to create print job for schedule {}: {e}",
                    row.id
                );
                tick_failed = true;
                break;
            }
            log::info!("Scheduled task {}: {}", row.id, row.name);
        }

        if !tick_failed {
            if let Err(e) = advance_schedules(&conn, due_schedules) {
                log::error!("scheduler: failed to advance schedules: {e}");
                tick_failed = true;
            }
        }

        drop(conn);
        let sleep_for = if tick_failed { 5 } else { 60 };
        tokio::time::sleep(Duration::from_secs(sleep_for)).await;
    }
}
