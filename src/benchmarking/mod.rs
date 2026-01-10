mod executor;

use std::str::FromStr;

use crate::benchmarking::executor::{CompileError, MeasurementError, execute_all};
use crate::command::{Command, CommandParsingError};
use crate::commit_hash::CommitHash;
use crate::database::utilities::BenchmarkFilters;
use crate::measurement::MeasurementMethod;
use crate::utilities::{BenchmarkMode, BenchmarkParamsBuilder, BenchmarkPoint};
use executor::build_source;

use crate::config::{backend::BackendConfig, benchmark::BenchmarkConfig};

use super::database::*;

#[justerror::Error]
pub enum BenchmarkingError {
    Compile(#[from] CompileError),
    Database(#[from] DatabaseError),
    ParsingRun(#[from] CommandParsingError),
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
    measurement_method: MeasurementMethod,
    benchmark_mode: BenchmarkMode,
) -> Result<(), BenchmarkingError> {
    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = benchmark_config;

    let param_generator =
        BenchmarkParamsBuilder::new(commit_hash, benchmark_name, measurement_method.to_string());

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

    execute_all(
        built_source,
        measurement_method,
        Command::from_str(&backend_config.run_command)?,
        points.into_iter(),
        |point, record| database.insert_data(param_generator.finalize(point), record),
        benchmark_data.timeout,
    )
}
