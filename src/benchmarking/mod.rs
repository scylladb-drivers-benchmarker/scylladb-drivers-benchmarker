mod executor;

use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use crate::benchmarking::executor::{Callback, CompileError, execute_all};
use crate::command::{Command, CommandParsingError};
use crate::commit_hash::CommitHash;
use crate::database::utilities::BenchmarkFilters;
use crate::flame_graph::BenchMeasure;
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
    NoStoreDir { required_by: MeasurementMethod },
    Measurement(#[from] Box<dyn Error + 'static>),
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

struct ExecutorCallback<'a, PointsType: Iterator<Item = BenchmarkPoint>> {
    points: PointsType,
    timeout: Option<Duration>,
    database: &'a Database,
    param_generator: BenchmarkParamsBuilder,
}

fn helper(err: impl Error + 'static) -> Box<dyn Error> {
    Box::new(err)
}

impl<'a, PointsType: Iterator<Item = BenchmarkPoint>> Callback
    for ExecutorCallback<'a, PointsType>
{
    type ReturnType = Result<(), BenchmarkingError>;

    fn call(self, exec: impl executor::MeasuringEquipment) -> Self::ReturnType {
        let execute = |point| {
            if let Some(timeout) = self.timeout {
                exec.execute_with_timeout(point, timeout)
            } else {
                exec.execute(point)
            }
        };

        for point in self.points {
            let record = execute(point).map_err(helper)?;
            self.database
                .insert_data(self.param_generator.finalize(point), record)?;
        }
        Ok(())
    }
}

pub fn benchmark(
    database: &Database,
    commit_hash: CommitHash,
    benchmark_config: BenchmarkConfig,
    backend_config: BackendConfig,
    bench_measure: BenchMeasure,
    benchmark_mode: BenchmarkMode,
) -> Result<(), BenchmarkingError> {
    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = benchmark_config;

    let measurement_method: MeasurementMethod = bench_measure.clone().into();
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
        bench_measure,
        Command::from_str(&backend_config.run_command)?,
        ExecutorCallback {
            points: points.into_iter(),
            timeout: benchmark_data.timeout,
            database,
            param_generator,
        },
    )?
}
