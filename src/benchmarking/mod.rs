mod executor;

use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use executor::build_source;
use log::{debug, info};

use super::database::{Database, DatabaseError};
use crate::benchmarking::executor::command_executor::CommandExecutor;
use crate::benchmarking::executor::flame_executor::FlameExecutor;
use crate::benchmarking::executor::output_executor::OutputExecutor;
use crate::benchmarking::executor::{CompileError, MeasuringEquipment};
use crate::command::{Command, CommandParsingError};
use crate::commit_hash::CommitHash;
use crate::config::backend::BackendConfig;
use crate::config::benchmark::BenchmarkData;
use crate::database::utilities::BenchmarkFilters;
use crate::flame_graph::FlameFrequency;
use crate::measurement::MeasurementMethod;
use crate::utilities::{BenchmarkParamsBuilder, BenchmarkPoint};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BenchMeasure {
    Time,
    PerfStat,
    FlameGraph {
        flame_repo: PathBuf,
        frequency: FlameFrequency,
        store_dir: PathBuf,
    },
    Command(Command),
}

impl From<BenchMeasure> for MeasurementMethod {
    fn from(value: BenchMeasure) -> Self {
        match value {
            BenchMeasure::Time => MeasurementMethod::Time,
            BenchMeasure::PerfStat => MeasurementMethod::Perf,
            BenchMeasure::FlameGraph { .. } => MeasurementMethod::FlameGraph,
            BenchMeasure::Command(command) => MeasurementMethod::Command(command),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BenchmarkMode {
    UseCached,
    ForceRerun,
}

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
            .filter_map(|point| {
                debug!("Searching in database point: {point}...");
                match database.result_exists(param_generator.finalize(point)) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(point)),
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>(),
        BenchmarkMode::ForceRerun => config_points
            .map(|point| {
                debug!("Trying to remove from database point: {point}...");
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
    benchmark_config: BenchmarkData,
    backend_config: BackendConfig,
    bench_measure: BenchMeasure,
    benchmark_mode: BenchmarkMode,
) -> Result<(), BenchmarkingError> {
    let BenchmarkData {
        name: benchmark_name,
        points,
        timeout,
    } = benchmark_config;

    let measurement_method: MeasurementMethod = bench_measure.clone().into();
    let param_generator =
        BenchmarkParamsBuilder::new(commit_hash, benchmark_name, measurement_method.to_string());

    info!("Filtering points...");
    let points = filter_points(
        database,
        points.into_iter(),
        &param_generator,
        benchmark_mode,
    )?;

    if points.is_empty() {
        info!("All points already in database");
        return Ok(());
    }

    info!("Building...");
    build_source(&backend_config.build_command)?;

    let run_command = Command::from_str(&backend_config.run_command)?;

    let exec: &dyn MeasuringEquipment = match bench_measure {
        BenchMeasure::Time => &OutputExecutor::new_time(run_command),
        BenchMeasure::PerfStat => &OutputExecutor::new_perf_stat(run_command),
        BenchMeasure::FlameGraph {
            flame_repo,
            frequency,
            store_dir,
        } => &FlameExecutor::new(flame_repo, store_dir, frequency, run_command),
        BenchMeasure::Command(command) => &CommandExecutor::new(command, run_command),
    };

    let execute = |point| {
        if let Some(timeout) = timeout {
            exec.execute_with_timeout(point, timeout)
        } else {
            exec.execute(point)
        }
    };

    let no_points = points.len();
    info!("Measuring...");
    for (idx, point) in (1..).zip(points.into_iter()) {
        info!("Measuring [{idx}/{no_points}] in {point}...");
        let record = execute(point)?;
        database.insert_data(param_generator.finalize(point), record)?;
    }
    info!("Finished measuring");
    Ok(())
}
