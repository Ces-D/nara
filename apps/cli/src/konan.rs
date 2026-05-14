use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use konan_core::{
    print_ops::{CreateSchedule, PrintFileTask, PrintTask},
    template::{BoxOutline, HabitTracker},
};
use rrule::{RRule, Unvalidated};
use std::io::Read;
use std::path::PathBuf;

use crate::client::TitansTowerClient;
use crate::error::CliError;

#[derive(Debug, Subcommand)]
pub enum KonanCommand {
    #[clap(
        about = "Create a recurring scheduled print task on the server",
        long_about = "Create a recurring scheduled print task on the server. The task argument is a JSON-encoded PrintTask ({\"Outline\":..}, {\"Tracker\":..}, or {\"File\":..}). Pass `-` to read the JSON from stdin so it can be piped from another konan subcommand invoked with `--task true`."
    )]
    CreateSchedule {
        #[clap(help = "Human-readable name for the schedule")]
        name: String,
        #[clap(
            short,
            long,
            help = "iCalendar recurrence rule, e.g. FREQ=DAILY;COUNT=5"
        )]
        r_rule: String,
        #[clap(short, long, help = "First run time (RFC 3339 / ISO 8601, UTC)")]
        start: DateTime<Utc>,
        #[clap(short, long, help = "PrintTask JSON, or `-` to read from stdin")]
        task: String,
    },
    #[clap(about = "Delete a scheduled print task by id")]
    DeleteSchedule {
        #[clap(help = "Server-side schedule id (see `list-schedules`)")]
        id: i64,
    },
    #[clap(about = "List every scheduled print task on the server")]
    ListSchedules,
    #[clap(about = "Print (or emit) a habit tracker for a date range")]
    Habit {
        #[clap(help = "Habit name displayed on the tracker")]
        habit: String,
        #[clap(
            short,
            long,
            help = "Start date of the tracking window (RFC 3339, UTC)"
        )]
        start_date: DateTime<Utc>,
        #[clap(short, long, help = "End date of the tracking window (RFC 3339, UTC)")]
        end_date: DateTime<Utc>,
        #[clap(
            short,
            long,
            help = "When true, emit the PrintTask JSON to stdout instead of sending it to the server"
        )]
        task: Option<bool>,
    },
    #[clap(about = "Print (or emit) a box outline page")]
    Outline {
        #[clap(short, long, help = "Optional date banner at the top of the page")]
        date: Option<DateTime<Utc>>,
        #[clap(short, long, help = "Optional text banner at the top of the page")]
        banner: Option<String>,
        #[clap(short, long, help = "Number of rows in the box body")]
        rows: Option<u32>,
        #[clap(short, long, help = "Render rows as lined instead of blank")]
        lined: bool,
        #[clap(
            short,
            long,
            help = "When true, emit the PrintTask JSON to stdout instead of sending it to the server"
        )]
        task: Option<bool>,
    },
    #[clap(
        about = "Print (or emit) a markdown file",
        long_about = "Print a markdown file. In immediate mode the local file is first uploaded to the server, then queued for printing. With `--task true`, only emits the PrintTask JSON (referencing the file's basename) and skips the upload."
    )]
    File {
        #[clap(help = "Local path to the markdown file to upload and print")]
        loc: PathBuf,
        #[clap(short, long, help = "Optional max rows to print")]
        rows: Option<u32>,
        #[clap(
            short,
            long,
            help = "When true, emit the PrintTask JSON (referencing the file's basename) to stdout without uploading or printing"
        )]
        task: Option<bool>,
    },
}

#[derive(Debug, Parser)]
#[clap(about = "Konan subcommands talk to the /konan/* routes on the titans_tower server")]
pub struct KonanArgs {
    #[clap(subcommand)]
    pub command: KonanCommand,
}

pub fn handle_konan_command(args: KonanArgs, client: &TitansTowerClient) -> Result<(), CliError> {
    match args.command {
        KonanCommand::Outline {
            date,
            banner,
            rows,
            lined,
            task,
        } => {
            let mut bo = BoxOutline::default();
            bo.set_rows(rows.unwrap_or(30))
                .set_lined(lined)
                .set_banner(banner)
                .set_date_banner(date);
            if task == Some(true) {
                println!("{}", serde_json::to_string(&PrintTask::Outline(bo))?);
            } else {
                client.print_outline(&bo)?;
            }
        }
        KonanCommand::Habit {
            habit,
            start_date,
            end_date,
            task,
        } => {
            let ht = HabitTracker::new(habit, start_date, end_date);
            if task == Some(true) {
                println!("{}", serde_json::to_string(&PrintTask::Tracker(ht))?);
            } else {
                client.print_tracker(&ht)?;
            }
        }
        KonanCommand::File { loc, rows, task } => {
            let basename = loc
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| CliError::InvalidPath(loc.display().to_string()))?
                .to_string();
            let pft = PrintFileTask {
                file_name: basename,
                rows,
            };
            if task == Some(true) {
                println!("{}", serde_json::to_string(&PrintTask::File(pft))?);
            } else {
                client.upload_file(&loc)?;
                client.print_file(&pft)?;
            }
        }
        KonanCommand::CreateSchedule {
            name,
            r_rule,
            start,
            task,
        } => {
            let task_json = if task.trim() == "-" {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                task
            };
            let parsed_task: PrintTask = serde_json::from_str(task_json.trim())?;
            let parsed_rrule: RRule<Unvalidated> = r_rule
                .parse()
                .map_err(|e: rrule::RRuleError| CliError::RRule(e.to_string()))?;
            let payload = CreateSchedule {
                name,
                task: parsed_task,
                r_rule: parsed_rrule,
                start,
            };
            let id = client.create_schedule(&payload)?;
            println!("{id}");
        }
        KonanCommand::ListSchedules => {
            let schedules = client.list_schedules()?;
            for s in schedules {
                let next = s
                    .next_run_unix
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string());
                println!("{}\t{}\t{}\t{}", s.id, s.name, s.start_unix, next);
            }
        }
        KonanCommand::DeleteSchedule { id } => {
            if client.delete_schedule(id)? {
                println!("deleted {id}");
            } else {
                eprintln!("schedule {id} not found");
            }
        }
    }
    Ok(())
}
