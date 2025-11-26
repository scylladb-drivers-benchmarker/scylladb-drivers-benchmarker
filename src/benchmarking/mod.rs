mod execution;

use std::{error::Error, iter::zip, path::PathBuf};

use crate::utilities::BenchmarkParams;
use crate::{benchmarking::execution::ErrBenchmarkResult, commit_hash::CommitHash};
use execution::{Executor, SourceCode, compile};

use crate::config::{backend::BackendConfig, benchmark::BenchmarkConfig, find_config};

use super::database::*;

use clap::Parser;

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

fn benchmark(
    database: &Database,
    executor: &Executor,
    benchmark_points: impl Iterator<Item = u32>,
    commit_hash: CommitHash,
    benchmark_name: String,
) -> Result<(), Box<dyn Error>> {
    let measurement_method: String = executor.command().program().into();

    let benchmark_params = |param: u32| {
        BenchmarkParams::new(
            commit_hash.clone(),
            benchmark_name.clone(),
            param.into(),
            measurement_method.clone(),
        )
    };

    for param in benchmark_points {
        if database.data_exists(benchmark_params(param))? {
            continue;
        }
        let benchmark_result = executor.run(param)?;
        database.insert_data(benchmark_params(param), benchmark_result)?;
    }

    Ok(())
}

fn main(args: &BenchmarkingArguments, database: &Database) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig =
        find_config(&args.benchmark_name, &args.driver_config_path)?;
    let backend_config: BackendConfig = find_config(&args.driver_name, &args.driver_config_path)?;

    let git_hash: CommitHash = CommitHash::from_repository()?;

    let compiled_source = compile(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let mut executor = Executor::new(compiled_source, backend_config.run_command.as_str())?;
    executor.measure(args.instrumentation.as_ref())?;

    benchmark(
        database,
        &executor,
        benchmark_config.iter_points(),
        git_hash,
        benchmark_config.name.clone(),
    )
}
