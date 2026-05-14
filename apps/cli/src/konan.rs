use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum KonanCommand {
    CreateSchedule {
        name: String,
        r_rule: String,
        start: DateTime<Utc>,
        task: String,
    },
    DeleteSchedule {
        id: i64,
    },
    ListSchedules,
    Habit {
        habit: String,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        task: Option<bool>,
    },
    Outline {
        date: Option<DateTime<Utc>>,
        banner: Option<String>,
        rows: u32,
        lined: bool,
        task: Option<bool>,
    },
    File {
        loc: PathBuf,
        rows: Option<u32>,
        task: Option<bool>,
    },
}

#[derive(Debug, Parser)]
pub struct KonanArgs {
    #[clap(subcommand)]
    pub command: KonanCommand,
}

pub fn handle_konan_command(args: KonanArgs) {
    match args.command {
        KonanCommand::CreateSchedule {
            name,
            r_rule,
            start,
            task,
        } => todo!(),
        KonanCommand::DeleteSchedule { id } => todo!(),
        KonanCommand::ListSchedules => todo!(),
        KonanCommand::Habit {
            habit,
            start_date,
            end_date,
            task,
        } => todo!(),
        KonanCommand::Outline {
            date,
            banner,
            rows,
            lined,
            task,
        } => todo!(),
        KonanCommand::File { loc, rows, task } => todo!(),
    }
}
