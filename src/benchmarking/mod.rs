mod execution;

use crate::benchmarking::execution::{CompileError, MeasurementError};
use crate::command::CommandParsingError;
use crate::commit_hash::CommitHash;
use crate::utilities::{BenchmarkFilters, BenchmarkMode, BenchmarkParams, BenchmarkPoint};
use execution::{Executor, build_source};

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
    RunParsing(CommandParsingError),
    MeasureParsing(CommandParsingError),
}

#[justerror::Error]
pub enum BenchmarkingError {
    Compile(#[from] CompileError),
    Database(#[from] DatabaseError),
    ExecutorBuilding(#[from] ExecutorBuildingError),
    Measurement(#[from] MeasurementError),
}

fn filter_points(
    database: &Database,
    config_points: impl Iterator<Item = BenchmarkPoint>,
    params_generator: impl Fn(BenchmarkPoint) -> BenchmarkParams,
    benchmark_mode: BenchmarkMode,
) -> Result<Vec<BenchmarkPoint>, DatabaseError> {
    match benchmark_mode {
        BenchmarkMode::UseCached => config_points
            .filter_map(
                |point| match database.result_exists(params_generator(point)) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(point)),
                    Err(e) => Some(Err(e)),
                },
            )
            .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>(),
        BenchmarkMode::ForceRerun => config_points
            .map(|point| {
                database.drop_data(&BenchmarkFilters::filter_exact_param(&params_generator(
                    point,
                )))?;
                Ok(point)
            })
            .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>(),
    }
}

pub fn benchmark(
    database: &Database,
    commit_hash: CommitHash,
    benchmark_config: BenchmarkConfig,
    backend_config: BackendConfig,
    measurement_method: String,
    bechmark_mode: BenchmarkMode,
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

    let points = filter_points(
        database,
        benchmark_data.benchmark_points(),
        benchmark_params,
        bechmark_mode,
    )?;

    if points.is_empty() {
        return Ok(());
    }

    build_source(&backend_config.build_command)?;

    let executor = Executor::new(&backend_config.run_command)
        .map_err(ExecutorBuildingError::RunParsing)?
        .with_measure(&measurement_method)
        .map_err(ExecutorBuildingError::MeasureParsing)?;

    let execute = |point| {
        if let Some(timeout) = benchmark_data.timeout {
            executor.execute_with_timeout(point, timeout)
        } else {
            executor.execute(point)
        }
    };

    for point in points.into_iter() {
        let benchmark_record = execute(point)?;
        database.insert_data(benchmark_params(point), benchmark_record)?;
    }

    Ok(())
}
