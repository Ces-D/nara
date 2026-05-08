use konan_core::print_ops::{
    CreatePrintJob, advance_schedules, create_print_job, get_due_schedules, pool,
};
use std::time::Duration;

pub async fn scheduler_loop() {
    let pool = pool().expect("failed to open database connection");
    loop {
        let conn = pool.get().expect("failed to check out database connection");
        let due_schedules = get_due_schedules(&conn).expect("failed to query due schedules");
        for row in due_schedules.iter() {
            let job = CreatePrintJob {
                task: row.task.clone(),
                schedule_id: Some(row.id),
            };
            create_print_job(&conn, job).expect("failed to create print job");
            log::info!("Scheduled task {}: {}", row.id, row.name);
        }
        advance_schedules(&conn, due_schedules).expect("failed to advance schedules");
        drop(conn);
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
