mod execution;

use std::{error::Error, iter::zip, path::PathBuf};

use crate::{commit_hash::CommitHash, database};
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

type ErrBenchmarkResult = Result<database::BenchmarkResult, Box<dyn Error>>;

fn benchmark_all(
    args: &BenchmarkingArguments,
    benchmark_config: &BenchmarkConfig,
    backend_config: &BackendConfig,
) -> Result<impl Iterator<Item = ErrBenchmarkResult>, Box<dyn Error>> {
    let compiled_source = compile(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let mut executor = Executor::new(compiled_source, backend_config.run_command.as_str())?;
    executor.measure(args.instrumentation.as_ref())?;

    let benchmark_one = move |benchmark_point: u32| -> ErrBenchmarkResult {
        let output = executor.execute(benchmark_point)?;
        let output = String::from_utf8(output.stdout)?;
        Ok(database::BenchmarkResult::new(Some(output)))
    };

    Ok(benchmark_config.iter_points().map(benchmark_one))
}

fn save_benchmark_results(
    args: &BenchmarkingArguments,
    git_hash: CommitHash,
    results: impl Iterator<Item = ErrBenchmarkResult>,
    benchmark_config: &BenchmarkConfig,
    database: &Database,
) -> Result<(), Box<dyn Error>> {
    for (param, result) in zip(benchmark_config.iter_points(), results) {
        match result {
            Err(error) => {
                return Err(error);
            }
            Ok(correct) => database.insert_data(
                BenchmarkParams::new(
                    git_hash.clone(),
                    args.benchmark_name.clone(),
                    param.into(),
                    args.instrumentation.clone(),
                ),
                correct,
            )?,
        }
    }

    Ok(())
}

fn main(args: &BenchmarkingArguments, database: &Database) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig =
        find_config(&args.benchmark_name, &args.driver_config_path)?;
    let backend_config: BackendConfig = find_config(&args.driver_name, &args.driver_config_path)?;

    let git_hash: CommitHash = CommitHash::from_repository()?;
    let results = benchmark_all(args, &benchmark_config, &backend_config)?;
    save_benchmark_results(args, git_hash, results, &benchmark_config, database)
}
