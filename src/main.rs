mod benchmarking;
mod database;
mod plotting;

use crate::database::Database;

use clap::Parser;
use std::path::PathBuf;

fn default_database_location() -> PathBuf {
    dirs::home_dir().unwrap().join("benchmarker.db")
}

use clap::Subcommand;

#[derive(Debug, Subcommand)]
enum Command {
    Benchmark(benchmarking::BackendArguments),
    Plot(plotting::FrontendArguments),
}

#[derive(Debug, Parser)]
#[clap(name = "my-app", version)]
pub struct App {
    #[clap(subcommand)]
    command: Command,
}
fn main() {
    let _db = Database::new(default_database_location()); // TODO use default database location or provided in argument (config?)
    let args = App::parse();
    match args.command {
        Command::Benchmark(back_args) => {
            println!("{:?}", back_args);
        }
        Command::Plot(plot_args) => {
            println!("{:?}", plot_args);
        }
    }
}
