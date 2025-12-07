mod execution;

use std::path::Path;

use crate::benchmarking::execution::{CompileError, MeasurementError};
use crate::command::CommandParsingError;
use crate::commit_hash::CommitHash;
use crate::config::{ConfigError, find_config};
use crate::utilities::{BenchmarkParams, BenchmarkPoint};
use execution::{Executor, SourceCode, build_source};

use crate::config::{backend::BackendConfig, benchmark::BenchmarkConfig};

use super::database::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct BenchmarkingArguments {
    #[arg(short, long)]
    pub driver_name: String,

    #[arg(short, long)]
    pub measurement_method: String,
}

#[justerror::Error]
pub enum ExecutorBuildingError {
    RunParsingError(CommandParsingError),
    MeasureParsingError(CommandParsingError),
}

#[justerror::Error]
pub enum BenchmarkingError {
    CompileError(
        #[from]
        #[source]
        CompileError,
    ),
    DbError(
        #[from]
        #[source]
        DatabaseError,
    ),
    ConfigError(
        #[from]
        #[source]
        ConfigError,
    ),
    ExecutorBuildingError(
        #[from]
        #[source]
        ExecutorBuildingError,
    ),
    MeasurementError(
        #[from]
        #[source]
        MeasurementError,
    ),
}

pub fn benchmark(
    database: &Database,
    commit_hash: CommitHash,
    benchmark_config: BenchmarkConfig,
    backend_config_path: &Path,
    measurement_method: String,
) -> Result<(), BenchmarkingError> {
    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = benchmark_config;

    let benchmark_params = |param: BenchmarkPoint| {
        BenchmarkParams::new(
            commit_hash.clone(),
            benchmark_name.clone(),
            param,
            measurement_method.clone(),
        )
    };

    let points = benchmark_data
        .benchmark_points()
        .filter_map(
            |point| match database.data_exists(benchmark_params(point)) {
                Ok(true) => None,
                Ok(false) => Some(Ok(point)),
                Err(e) => Some(Err(e)),
            },
        )
        .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>()?;

    if points.is_empty() {
        return Ok(());
    }

    let backend_config: BackendConfig = find_config(&benchmark_name, backend_config_path)?;

    let built_source = build_source(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let executor = Executor::new(built_source, backend_config.run_command)
        .map_err(ExecutorBuildingError::RunParsingError)?
        .with_measure(&measurement_method)
        .map_err(ExecutorBuildingError::MeasureParsingError)?;

    for point in points.iter().cloned() {
        let benchmark_record = executor.execute(point)?;
        database.insert_data(benchmark_params(point), benchmark_record)?;
    }

    Ok(())
}
