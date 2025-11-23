mod command;
mod execution;

use std::{error::Error, path::PathBuf, process::Output};

use command::Command;
use execution::{CompiledSource, Executor, SourceCode, compile};

use crate::config::{
    backend::BackendConfig,
    benchmark::{self, BenchmarkConfig, BenchmarkConfigList},
    find_config,
};

use super::database::*;

use clap::{Parser, builder::Str};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct BenchmarkingArguments {
    #[arg(short, long)]
    pub benchmark_name: String,

    #[arg(long, default_value = "./config.yml")]
    pub benchmark_config_path: PathBuf,

    #[arg(short, long)]
    pub driver_name: String,

    #[arg(long, default_value = "./config.yml")]
    pub driver_config_path: PathBuf,

    #[arg(short, long)]
    pub instrumentation: String,
}

fn benchmark(args: BenchmarkingArguments) -> Result<Vec<String>, Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig =
        find_config(&args.benchmark_name, &args.driver_config_path)?;
    let backend_config: BackendConfig = find_config(&args.driver_name, &args.driver_config_path)?;

    let compiled_source = compile(backend_config.build_command, SourceCode { path: None })?;

    let mut executor = Executor::new(compiled_source, backend_config.run_command)?;
    executor.measure(args.instrumentation)?;

    let mut outputs: Vec<String> = Vec::new();
    for benchmark_point in benchmark_config.iter_points() {
        let output = executor.execute(benchmark_point)?;
        let output = String::from_utf8(output.stdout)?;
        outputs.push(output);
    }

    Ok(outputs)
}
