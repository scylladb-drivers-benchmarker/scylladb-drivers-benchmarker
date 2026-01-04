mod execution;

use crate::benchmarking::execution::{CompileError, MeasurementError};
use crate::command::CommandParsingError;
use crate::commit_hash::CommitHash;
use crate::database::utilities::BenchmarkFilters;
use crate::utilities::{BenchmarkMode, BenchmarkParamsBuilder, BenchmarkPoint};
use execution::{Executor, build_source};

use crate::config::{backend::BackendConfig, benchmark::BenchmarkConfig};

use super::database::*;

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
    param_generator: &BenchmarkParamsBuilder,
    benchmark_mode: BenchmarkMode,
) -> Result<Vec<BenchmarkPoint>, DatabaseError> {
    match benchmark_mode {
        BenchmarkMode::UseCached => config_points
            .filter_map(
                |point| match database.result_exists(param_generator.finalize(point)) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(point)),
                    Err(e) => Some(Err(e)),
                },
            )
            .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>(),
        BenchmarkMode::ForceRerun => config_points
            .map(|point| {
                database.drop_data(&BenchmarkFilters::filter_exact_param(
                    &param_generator.finalize(point),
                ))?;
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
    benchmark_mode: BenchmarkMode,
) -> Result<(), BenchmarkingError> {
    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = benchmark_config;

    let param_generator = BenchmarkParamsBuilder::new(
        commit_hash.clone(),
        benchmark_name.clone(),
        measurement_method.clone(),
    );

    let points = filter_points(
        database,
        benchmark_data.benchmark_points(),
        &param_generator,
        benchmark_mode,
    )?;

    if points.is_empty() {
        return Ok(());
    }

    let built_source = build_source(&backend_config.build_command)?;

    let executor = Executor::new(built_source, &backend_config.run_command)
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
        let benchmark_result = execute(point)?;
        database.insert_data(param_generator.finalize(point), benchmark_result)?;
    }

    Ok(())
}
