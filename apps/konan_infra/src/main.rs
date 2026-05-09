use clap::{Parser, Subcommand};

mod scheduler;
mod worker;

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scheduler,
    Worker,
}

#[derive(Debug, Parser)]
#[clap(author, version, subcommand_required = true)]
pub struct App {
    #[clap(subcommand)]
    pub command: Commands,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let app = App::parse();
    match app.command {
        Commands::Scheduler => scheduler::scheduler_loop().await,
        Commands::Worker => worker::worker_loop().await,
    }
}
