#[allow(dead_code)]
mod benchmarking;
mod command;
mod commit_hash;
mod config;
mod database;
mod plotting;
mod utilities;

use crate::database::Database;

use clap::Parser;
use std::{error::Error, path::PathBuf};

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
fn main() -> Result<(), Box<dyn Error>> {
    let _db = Database::new(default_database_location())?; // TODO use default database location or provided in argument (config?)
    let args = App::parse();
    match args.command {
        SubCommand::Benchmark(benchmarking_args) => {
            benchmarking::main(&benchmarking_args, &_db)?;
        }
        SubCommand::Plot(plot_args) => {
            println!("{:?}", plot_args);
        }
    }
    Ok(())
}
