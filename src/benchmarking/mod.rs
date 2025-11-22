mod command;

use super::database::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct BackendArguments {
    #[arg(short, long)]
    pub benchmark_name: String,

    #[arg(short, long)]
    pub driver_name: String,

    #[arg(short, long)]
    pub measuring_command: String,
}
