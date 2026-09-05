mod executor;

use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use log::{debug, info, trace, warn};

use super::database::{Database, DatabaseError};
use crate::benchmarking::executor::command_executor::CommandExecutor;
use crate::benchmarking::executor::flame_executor::FlameExecutor;
use crate::benchmarking::executor::output_executor::OutputExecutor;
use crate::benchmarking::executor::{
    CompileError, MeasuringEquipment, PlainCommandError, run_plain_command,
};
use crate::command::{Command, CommandParsingError};
use crate::commit_hash::CommitHash;
use crate::config::api::ApiSetup;
use crate::config::benchmark::BenchmarkData;
use crate::config::driver::DriverSpec;
use crate::database::utilities::{BenchmarkFilters, BenchmarkRecord, Provenance};
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
    NoStoreDir {
        required_by: MeasurementMethod,
    },
    #[error(desc = "environment preparation script failed")]
    EnvPrepareFailed(#[source] PlainCommandError),
    #[error(desc = "the 'prepare' phase of benchmark '{benchmark}' at point {point} failed")]
    PrepareFailed {
        benchmark: String,
        point: BenchmarkPoint,
        #[source]
        source: PlainCommandError,
    },
    #[error(
        desc = "benchmark '{benchmark}' failed at point {point} (pass --keep-going to continue past failing points)"
    )]
    PointFailed {
        benchmark: String,
        point: BenchmarkPoint,
        #[source]
        source: Box<dyn Error + 'static>,
    },
    Measurement(#[from] Box<dyn Error + 'static>),
}

/// Everything identifying and driving one benchmarking session:
/// the driver under test and the api directory to run in.
pub struct Session {
    pub driver: DriverSpec,
    pub driver_commit: CommitHash,
    pub api: ApiSetup,
    pub benchmarks_commit: String,
}

impl Session {
    /// The run command for one phase of one benchmark, with all contract
    /// env vars except STEP (added per point by the executors).
    fn phase_command(
        &self,
        phase: &str,
        benchmark: &BenchmarkData,
    ) -> Result<Command, CommandParsingError> {
        Ok(Command::from_str(&self.api.config.run_command)?
            .with_cwd(&self.api.api_dir)
            .with_env("PHASE", phase)
            .with_env("BENCHMARK", benchmark.workload.clone())
            .with_env("PARAM_MODE", benchmark.param_mode.as_str())
            .with_env("DRIVER_PACKAGE", self.driver.package.clone()))
    }

    fn plain_api_command(&self, command: &str) -> Result<Command, CommandParsingError> {
        Ok(Command::from_str(command)?
            .with_cwd(&self.api.api_dir)
            .with_envs(self.driver.env_vars()))
    }

    /// Runs the api's env-prepare script (links the driver under test).
    pub fn env_prepare(&self) -> Result<(), BenchmarkingError> {
        info!("Preparing the benchmarking environment (linking the driver)...");
        let command = self.plain_api_command(&self.api.config.env_prepare)?;
        run_plain_command(command, None).map_err(BenchmarkingError::EnvPrepareFailed)?;
        Ok(())
    }

    /// Runs the api's env-cleanup script (best-effort; failures are logged).
    pub fn env_cleanup(&self) {
        info!("Cleaning up the benchmarking environment...");
        match self.plain_api_command(&self.api.config.env_cleanup) {
            Ok(command) => {
                if let Err(e) = run_plain_command(command, None) {
                    warn!("Environment cleanup failed: {e}");
                }
            }
            Err(e) => warn!("Environment cleanup command is invalid: {e}"),
        }
    }

    /// Builds the benchmark project (once per session, unmeasured).
    pub fn build(&self) -> Result<(), BenchmarkingError> {
        info!("Building...");
        let command = self.plain_api_command(&self.api.config.build_command)?;
        executor::build_source(command)?;
        Ok(())
    }

    fn provenance(&self) -> Provenance {
        Provenance {
            api: self.driver.api.clone(),
            benchmarks_commit: self.benchmarks_commit.clone(),
        }
    }
}

/// Guard ensuring env-cleanup runs even when benchmarking fails.
pub struct EnvCleanupGuard<'a>(pub &'a Session);

impl Drop for EnvCleanupGuard<'_> {
    fn drop(&mut self) {
        self.0.env_cleanup();
    }
}

fn filter_points(
    database: &Database,
    config_points: impl Iterator<Item = BenchmarkPoint>,
    param_generator: &BenchmarkParamsBuilder,
) -> Result<Vec<BenchmarkPoint>, DatabaseError> {
    debug!("Filtering points...");
    config_points
        .filter_map(|point| {
            trace!("Searching in database point: {point}...");
            match database.result_exists(param_generator.finalize(point)) {
                Ok(true) => None,
                Ok(false) => Some(Ok(point)),
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>()
}

fn remove_points(
    database: &Database,
    config_points: impl Iterator<Item = BenchmarkPoint>,
    param_generator: &BenchmarkParamsBuilder,
) -> Result<Vec<BenchmarkPoint>, DatabaseError> {
    debug!("Deleting previous results...");
    config_points
        .map(|point| {
            trace!("Trying to remove from database point: {point}...");
            database.drop_data(&BenchmarkFilters::filter_exact_param(
                &param_generator.finalize(point),
            ))?;
            Ok(point)
        })
        .collect::<Result<Vec<BenchmarkPoint>, DatabaseError>>()
}

/// Runs the unmeasured `prepare` phase for one point.
fn run_prepare(
    session: &Session,
    benchmark: &BenchmarkData,
    point: BenchmarkPoint,
    timeout: Option<Duration>,
) -> Result<(), BenchmarkingError> {
    trace!("Running the prepare phase...");
    let command = session
        .phase_command("prepare", benchmark)?
        .with_env("STEP", point.to_string());
    run_plain_command(command, timeout).map_err(|source| BenchmarkingError::PrepareFailed {
        benchmark: benchmark.name.clone(),
        point,
        source,
    })
}

/// Runs the unmeasured `teardown` phase for one point (best-effort).
fn run_teardown(session: &Session, benchmark: &BenchmarkData, point: BenchmarkPoint) {
    trace!("Running the teardown phase...");
    let result = session
        .phase_command("teardown", benchmark)
        .map_err(PlainCommandError::from)
        .and_then(|command| run_plain_command(command.with_env("STEP", point.to_string()), None));
    if let Err(e) = result {
        warn!(
            "Teardown of '{}' at point {point} failed (continuing): {e}",
            benchmark.name
        );
    }
}

/// Executes one measured run of the `run` phase.
fn measure_once(
    exec: &dyn MeasuringEquipment,
    point: BenchmarkPoint,
    timeout: Option<Duration>,
) -> Result<BenchmarkRecord, Box<dyn Error + 'static>> {
    match timeout {
        Some(t) => exec.execute_with_timeout(point, t),
        None => exec.execute(point),
    }
}

pub fn benchmark(
    database: &Database,
    session: &Session,
    benchmark_config: BenchmarkData,
    bench_measure: BenchMeasure,
    benchmark_mode: BenchmarkMode,
    keep_going: bool,
) -> Result<(), BenchmarkingError> {
    info!(
        "Setting up benchmarking for '{}' with driver '{}'...",
        benchmark_config.name, session.driver.name
    );

    let is_time = matches!(bench_measure, BenchMeasure::Time);
    let timeout = benchmark_config.timeout;
    let num_runs = benchmark_config.num_runs;

    let measurement_method: MeasurementMethod = bench_measure.clone().into();
    let param_generator = BenchmarkParamsBuilder::new(
        session.driver_commit.clone(),
        benchmark_config.name.clone(),
        session.driver.name.clone(),
        measurement_method.to_string(),
    );

    let points = match benchmark_mode {
        BenchmarkMode::UseCached => filter_points(
            database,
            benchmark_config.points.iter().copied(),
            &param_generator,
        )?,
        BenchmarkMode::ForceRerun => remove_points(
            database,
            benchmark_config.points.iter().copied(),
            &param_generator,
        )?,
    };

    if points.is_empty() {
        info!("All points already in database");
        return Ok(());
    }

    let run_command = session.phase_command("run", &benchmark_config)?;

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

    let no_points = points.len();
    for (idx, point) in (1..).zip(points) {
        info!("Measuring [{idx}/{no_points}] in {point}...");

        let mut values: Vec<f64> = Vec::with_capacity(num_runs as usize);
        let mut record: Option<BenchmarkRecord> = None;
        let mut point_error: Option<BenchmarkingError> = None;

        let runs = if is_time { num_runs as usize } else { 1 };
        for run_idx in 0..runs {
            if runs > 1 {
                info!("  Run [{}/{}]...", run_idx + 1, runs);
            }

            if let Err(e) = run_prepare(session, &benchmark_config, point, timeout) {
                point_error = Some(e);
                break;
            }

            let raw = measure_once(exec, point, timeout);
            // Teardown regardless of the measurement outcome.
            run_teardown(session, &benchmark_config, point);

            match raw {
                Ok(BenchmarkRecord::Timeout) => {
                    record = Some(BenchmarkRecord::Timeout);
                    break;
                }
                Ok(BenchmarkRecord::Data(s)) if is_time => match s.trim().parse::<f64>() {
                    Ok(v) => values.push(v),
                    Err(e) => {
                        point_error = Some(BenchmarkingError::PointFailed {
                            benchmark: benchmark_config.name.clone(),
                            point,
                            source: Box::new(e),
                        });
                        break;
                    }
                },
                Ok(r) => {
                    record = Some(r);
                }
                Err(e) => {
                    point_error = Some(BenchmarkingError::PointFailed {
                        benchmark: benchmark_config.name.clone(),
                        point,
                        source: e,
                    });
                    break;
                }
            }
        }

        if let Some(e) = point_error {
            if keep_going {
                warn!("Point {point} failed: {e}");
                continue;
            }
            return Err(e);
        }

        let record = record.unwrap_or_else(|| {
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            BenchmarkRecord::TimedData {
                mean,
                stddev: variance.sqrt(),
            }
        });

        database.insert_data(
            param_generator.finalize(point),
            session.provenance(),
            record,
        )?;
    }
    info!("Finished measuring");
    Ok(())
}
