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

pub fn main(args: &BenchmarkingArguments, database: &Database) -> Result<(), Box<dyn Error>> {
    let backend_config: BackendConfig = find_config(&args.driver_name, &args.driver_config_path)?;

    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = find_config(&args.benchmark_name, &args.driver_config_path)?;

    let commit_hash: CommitHash = CommitHash::from_repository()?;

    let benchmark_params = |param: u32| {
        BenchmarkParams::new(
            commit_hash.clone(),
            benchmark_name.clone(),
            param.into(),
            args.instrumentation.clone(),
        )
    };

    let points = benchmark_data
        .benchmark_points()
        .filter_map(|point| {
            match database.data_exists(benchmark_params(point)) {
                Ok(true) => Some(Ok(point)), 
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<u32>, sqlite::Error>>()?;

    if points.is_empty() {
        return Ok(());
    }

    let compiled_source = compile(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let mut executor = Executor::new(compiled_source, backend_config.run_command.as_str())?;
    executor.measure(args.instrumentation.as_ref())?;

    for point in points {
        let benchmark_result = executor.run(point)?;
        database.insert_data(benchmark_params(point), benchmark_result)?;
    }

    Ok(())
}
