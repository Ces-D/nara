use crate::discord::{
    Context,
    ui::{parse_date, parse_rrule},
};
use crate::error::ServiceError;
use cadence_core::database::Schedule;
use cadence_core::registry::Task;
use chrono::{Duration, Utc};
use konan_core::{
    print_ops::PrintFileTask,
    template::{BoxOutline, HabitTracker},
};

#[poise::command(slash_command)]
pub async fn template(
    ctx: Context<'_>,
    #[description = "Number of rows (default 30)"] rows: Option<u32>,
    #[description = "Lined background (default true)"] lined: Option<bool>,
    #[description = "Print-out Banner"] banner: Option<String>,
    #[description = "Print-out Date (format: MM-DD-YYYY)"] date: Option<String>,
) -> Result<(), ServiceError> {
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

    ctx.data().konan.print_outline(draft).await?;
    ctx.say("Created template").await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn tracker(
    ctx: Context<'_>,
    #[description = "Habit name"] habit: String,
    #[description = "Start date MM-DD-YYYY (default today)"] start_date: Option<String>,
    #[description = "End date MM-DD-YYYY (default 2 weeks after start)"] end_date: Option<String>,
) -> Result<(), ServiceError> {
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
    ctx.data().konan.print_tracker(draft).await?;
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
) -> Result<(), ServiceError> {
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
    ctx.data()
        .konan
        .schedule_outline(name.clone(), draft, Some(r_rule), start)
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
) -> Result<(), ServiceError> {
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
    ctx.data()
        .konan
        .schedule_tracker(name.clone(), draft, Some(r_rule), start)
        .await?;
    ctx.say(format!("Created tracker schedule '{name}'"))
        .await?;
    Ok(())
}

fn task_kind(task_type: &str) -> &'static str {
    const OUTLINE: &str = <BoxOutline as Task>::TASK_TYPE;
    const TRACKER: &str = <HabitTracker as Task>::TASK_TYPE;
    const FILE: &str = <PrintFileTask as Task>::TASK_TYPE;
    match task_type {
        OUTLINE => "template",
        TRACKER => "tracker",
        FILE => "file",
        _ => "unknown",
    }
}

fn format_schedule(s: &Schedule) -> String {
    let next_run = s
        .next_run_unix
        .map(|d| d.format("%m-%d-%Y").to_string())
        .unwrap_or_else(|| "—".to_string());
    let rrule = s
        .rrule
        .as_ref()
        .map(|r| r.to_string())
        .unwrap_or_else(|| "—".to_string());
    format!(
        "**{}** (id: {}) [{}]\n  rrule: `{}`\n  start: {}\n  next run: {}",
        s.name,
        s.id,
        task_kind(&s.task_type),
        rrule,
        s.start_unix.format("%m-%d-%Y"),
        next_run,
    )
}

#[poise::command(slash_command)]
pub async fn list_schedules(ctx: Context<'_>) -> Result<(), ServiceError> {
    ctx.defer().await?;
    let schedules = ctx.data().konan.list_schedules().await?;
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
) -> Result<(), ServiceError> {
    ctx.defer().await?;
    ctx.data().konan.delete_schedule(id).await?;
    ctx.say(format!("Schedule {id} deleted.")).await?;
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
pub async fn konan(_ctx: Context<'_>) -> Result<(), ServiceError> {
    Ok(())
}
