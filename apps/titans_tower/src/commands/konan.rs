use crate::{
    AppError, Context,
    ui::{parse_date, parse_rrule},
};
use chrono::{DateTime, Duration, Utc};
use konan_core::{
    print_ops::{
        self, CreatePrintJob, CreateSchedule, KonanDbError, KonanDbPool, KonanDbPoolConnection,
        PrintTask, Schedule,
    },
    template::{BoxOutline, HabitTracker},
};

async fn run_db_blocking<F, T>(pool: KonanDbPool, f: F) -> Result<T, AppError>
where
    F: FnOnce(&mut KonanDbPoolConnection) -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || -> Result<T, AppError> {
        let mut conn = pool.get().map_err(KonanDbError::from)?;
        f(&mut conn)
    })
    .await?
}

#[poise::command(slash_command)]
pub async fn template(
    ctx: Context<'_>,
    #[description = "Number of rows (default 30)"] rows: Option<u32>,
    #[description = "Lined background (default true)"] lined: Option<bool>,
    #[description = "Print-out Banner"] banner: Option<String>,
    #[description = "Print-out Date (format: MM-DD-YYYY)"] date: Option<String>,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let mut draft = BoxOutline::default();
    draft
        .set_rows(rows.unwrap_or(30))
        .set_lined(lined.unwrap_or(true))
        .set_banner(banner);
    if let Some(d) = date {
        let parsed = parse_date("date", &d)?;
        draft.set_date_banner(Some(parsed));
    }
    let job = CreatePrintJob {
        task: PrintTask::Outline(draft),
        schedule_id: None,
    };
    let pool = ctx.data().konan_pool.clone();
    run_db_blocking(pool, move |conn| {
        print_ops::create_print_job(conn, job)?;
        Ok(())
    })
    .await?;
    ctx.say("Created template").await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn tracker(
    ctx: Context<'_>,
    #[description = "Habit name"] habit: String,
    #[description = "Start date MM-DD-YYYY (default today)"] start_date: Option<String>,
    #[description = "End date MM-DD-YYYY (default 2 weeks after start)"] end_date: Option<String>,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let start = match start_date {
        Some(date) => parse_date("start_date", &date)?,
        None => Utc::now(),
    };
    let end = match end_date {
        Some(date) => parse_date("end_date", &date)?,
        None => start + Duration::weeks(2),
    };
    let draft = HabitTracker::new(habit, start, end);
    let job = CreatePrintJob {
        task: PrintTask::Tracker(draft),
        schedule_id: None,
    };
    let pool = ctx.data().konan_pool.clone();
    run_db_blocking(pool, move |conn| {
        print_ops::create_print_job(conn, job)?;
        Ok(())
    })
    .await?;
    ctx.say(format!(
        "Created tracker from {} to {}",
        start.format("%m-%d-%Y"),
        end.format("%m-%d-%Y")
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn schedule_template(
    ctx: Context<'_>,
    #[description = "Schedule name"] name: String,
    #[description = "RRULE string (e.g. FREQ=WEEKLY;BYDAY=MO)"] rrule: String,
    #[description = "Schedule start MM-DD-YYYY (default today)"] schedule_start: Option<String>,
    #[description = "Number of rows (default 30)"] rows: Option<u32>,
    #[description = "Lined background (default true)"] lined: Option<bool>,
    #[description = "Print-out Banner"] banner: Option<String>,
    #[description = "Print-out Date (format: MM-DD-YYYY)"] date: Option<String>,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let mut draft = BoxOutline::default();
    draft
        .set_rows(rows.unwrap_or(30))
        .set_lined(lined.unwrap_or(true))
        .set_banner(banner);
    if let Some(d) = date {
        let parsed = parse_date("date", &d)?;
        draft.set_date_banner(Some(parsed));
    }
    let start = match schedule_start {
        Some(s) => parse_date("schedule_start", &s)?,
        None => Utc::now(),
    };
    let r_rule = parse_rrule("rrule", &rrule, start)?;
    let schedule = CreateSchedule {
        name: name.clone(),
        task: PrintTask::Outline(draft),
        r_rule,
        start,
    };
    let pool = ctx.data().konan_pool.clone();
    run_db_blocking(pool, move |conn| {
        print_ops::create_schedule(conn, schedule)?;
        Ok(())
    })
    .await?;
    ctx.say(format!("Created template schedule '{name}'"))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn schedule_tracker(
    ctx: Context<'_>,
    #[description = "Schedule name"] name: String,
    #[description = "RRULE string (e.g. FREQ=WEEKLY;BYDAY=MO)"] rrule: String,
    #[description = "Habit name"] habit: String,
    #[description = "Schedule start MM-DD-YYYY (default today)"] schedule_start: Option<String>,
    #[description = "Tracker start date MM-DD-YYYY (default today)"] start_date: Option<String>,
    #[description = "Tracker end date MM-DD-YYYY (default 2 weeks after start)"] end_date: Option<
        String,
    >,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let tracker_start = match start_date {
        Some(date) => parse_date("start_date", &date)?,
        None => Utc::now(),
    };
    let tracker_end = match end_date {
        Some(date) => parse_date("end_date", &date)?,
        None => tracker_start + Duration::weeks(2),
    };
    let draft = HabitTracker::new(habit, tracker_start, tracker_end);
    let start = match schedule_start {
        Some(s) => parse_date("schedule_start", &s)?,
        None => Utc::now(),
    };
    let r_rule = parse_rrule("rrule", &rrule, start)?;
    let schedule = CreateSchedule {
        name: name.clone(),
        task: PrintTask::Tracker(draft),
        r_rule,
        start,
    };
    let pool = ctx.data().konan_pool.clone();
    run_db_blocking(pool, move |conn| {
        print_ops::create_schedule(conn, schedule)?;
        Ok(())
    })
    .await?;
    ctx.say(format!("Created tracker schedule '{name}'"))
        .await?;
    Ok(())
}

fn task_kind(task: &PrintTask) -> &'static str {
    match task {
        PrintTask::Outline(_) => "template",
        PrintTask::Tracker(_) => "tracker",
        PrintTask::File(_) => "file",
    }
}

fn format_unix(unix: i64) -> String {
    DateTime::<Utc>::from_timestamp(unix, 0)
        .map(|d| d.format("%m-%d-%Y").to_string())
        .unwrap_or_else(|| format!("invalid({unix})"))
}

fn format_schedule(s: &Schedule) -> String {
    let next_run = s
        .next_run_unix
        .map(format_unix)
        .unwrap_or_else(|| "—".to_string());
    format!(
        "**{}** (id: {}) [{}]\n  rrule: `{}`\n  start: {}\n  next run: {}",
        s.name,
        s.id,
        task_kind(&s.task),
        s.r_rule,
        format_unix(s.start_unix),
        next_run,
    )
}

#[poise::command(slash_command)]
pub async fn list_schedules(ctx: Context<'_>) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().konan_pool.clone();
    let schedules = run_db_blocking(pool, |conn| Ok(print_ops::list_schedules(conn)?)).await?;
    if schedules.is_empty() {
        ctx.say("No schedules created.").await?;
    } else {
        for schedule in &schedules {
            ctx.say(format_schedule(schedule)).await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn delete_schedule(
    ctx: Context<'_>,
    #[description = "Id of the schedule"] id: i64,
) -> Result<(), AppError> {
    ctx.defer().await?;
    let pool = ctx.data().konan_pool.clone();
    let changed =
        run_db_blocking(pool, move |conn| Ok(print_ops::delete_schedule(conn, id)?)).await?;
    let msg = if changed == 0 {
        format!("No schedule found with id {id}.")
    } else {
        format!("Schedule {id} deleted.")
    };
    ctx.say(msg).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands(
        "template",
        "tracker",
        "schedule_template",
        "schedule_tracker",
        "list_schedules",
        "delete_schedule",
    ),
    subcommand_required
)]
pub async fn konan(_ctx: Context<'_>) -> Result<(), AppError> {
    Ok(())
}
