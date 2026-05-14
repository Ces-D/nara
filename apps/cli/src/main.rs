use clap::{Parser, Subcommand};

mod konan;

#[derive(Debug, Subcommand)]
pub enum Command {
    Konan(konan::KonanArgs),
}

#[derive(Debug, Parser)]
#[clap(author, version, subcommand_required = true)]
pub struct App {
    #[clap(subcommand)]
    pub command: Command,
}

fn main() {
    env_logger::init();
    let app = App::parse();
    match app.command {
        Command::Konan(konan_args) => todo!(),
    }
}
