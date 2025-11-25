mod benchmarking;
mod config;
mod database;
mod plotting;
mod commit_hash;
mod command;

use crate::database::Database;

use clap::Parser;
use std::path::PathBuf;

fn default_database_location() -> PathBuf {
    dirs::home_dir().unwrap().join("benchmarker.db")
}

use clap::Subcommand;

#[derive(Debug, Subcommand)]
enum SubCommand {
    Benchmark(benchmarking::BenchmarkingArguments),
    Plot(plotting::FrontendArguments),
}

#[derive(Debug, Parser)]
#[clap(name = "my-app", version)]
pub struct App {
    #[clap(subcommand)]
    command: SubCommand,
}
fn main() {
    let _db = Database::new(default_database_location()); // TODO use default database location or provided in argument (config?)
    let args = App::parse();
    match args.command {
        SubCommand::Benchmark(back_args) => {
            println!("{:?}", back_args);
        }
        SubCommand::Plot(plot_args) => {
            println!("{:?}", plot_args);
        }
    }
}
