use clap::{Parser, Subcommand};

mod client;
mod error;
mod konan;

use client::TitansTowerClient;

#[derive(Debug, Subcommand)]
pub enum Command {
    #[clap(about = "Konan: physical printer and scheduling commands")]
    Konan(konan::KonanArgs),
}

#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    subcommand_required = true,
    about = "Command-line client for the titans_tower server",
    long_about = "Command-line client for the titans_tower server. Reads the server base URL from the NARA_SERVER_URL environment variable (e.g. http://127.0.0.1:3000)."
)]
pub struct App {
    #[clap(subcommand)]
    pub command: Command,
}

fn main() {
    env_logger::init();
    let app = App::parse();

    let client = match TitansTowerClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nara: {e}");
            std::process::exit(1);
        }
    };

    let result = match app.command {
        Command::Konan(konan_args) => konan::handle_konan_command(konan_args, &client),
    };

    if let Err(e) = result {
        eprintln!("nara: {e}");
        std::process::exit(1);
    }
}
