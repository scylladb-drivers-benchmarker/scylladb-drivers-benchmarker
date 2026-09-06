use std::path::PathBuf;

use clap::{Args, ValueEnum};
use scylladb_drivers_benchmarker::benchmarking::{BenchMeasure, BenchmarkMode, Session};
use scylladb_drivers_benchmarker::commit_hash::CommitHash;
use scylladb_drivers_benchmarker::config::api::ApiSetup;
use scylladb_drivers_benchmarker::config::benchmark::{BenchmarkConfigList, BenchmarkData};
use scylladb_drivers_benchmarker::config::config_traits::ConfigurationList;
use scylladb_drivers_benchmarker::config::driver::DriverSpec;
use scylladb_drivers_benchmarker::config::open_config;
use scylladb_drivers_benchmarker::flame_graph::FlameFrequency;

use crate::BenchmarkParams;
use crate::command;
use crate::parsing::{ParsingError, Subcommands};

#[derive(Debug, Clone, clap::Args)]
pub(crate) struct FlameOptions {
    #[arg(short = 'r', long)]
    flame_repo: PathBuf,
    #[arg(short, long, default_value_t = FlameFrequency::Number(99))]
    frequency: FlameFrequency,
    /// Directory in which to store the results
    #[arg(short, long)]
    store_dir: PathBuf,
}

impl FlameOptions {
    fn finalize(self) -> Result<BenchMeasure, ParsingError> {
        let mut store_dir = self.store_dir;

        if !store_dir.is_absolute() {
            store_dir = store_dir.canonicalize()?;
        }

        if !store_dir.is_dir() {
            return Err(ParsingError::StoreDirNotADir);
        }

        Ok(BenchMeasure::FlameGraph {
            flame_repo: self.flame_repo,
            frequency: self.frequency,
            store_dir,
        })
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub(crate) enum MeasureSubcommand {
    Time,
    PerfStat,
    FlameGraph(#[command(flatten)] FlameOptions),
    Command { command: command::Command },
}

impl MeasureSubcommand {
    fn finalize(self) -> Result<BenchMeasure, ParsingError> {
        match self {
            MeasureSubcommand::Time => Ok(BenchMeasure::Time),
            MeasureSubcommand::PerfStat => Ok(BenchMeasure::PerfStat),
            MeasureSubcommand::FlameGraph(flame_options) => flame_options.finalize(),
            MeasureSubcommand::Command { command } => Ok(BenchMeasure::Command(command)),
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum InputBenchmarkMode {
    UseCached,
    ForceRerun,
}

impl InputBenchmarkMode {
    fn finalize(self) -> BenchmarkMode {
        match self {
            InputBenchmarkMode::UseCached => BenchmarkMode::UseCached,
            InputBenchmarkMode::ForceRerun => BenchmarkMode::ForceRerun,
        }
    }
}

#[derive(Args, Debug)]
pub(crate) struct BenchmarkCommand {
    /// Path to the local repository of the driver under test
    /// (must contain benchmark-config.yml at its root).
    #[arg(long, conflicts_with = "driver")]
    pub driver_path: Option<PathBuf>,

    /// Published driver under test: published:<api>:<package>@<version>,
    /// e.g. published:nodejs:cassandra-driver@4.8.0
    #[arg(long, required_unless_present = "driver_path")]
    pub driver: Option<String>,

    /// Override for the recorded driver name (defaults to `driver-name` from
    /// benchmark-config.yml, or the package name for published drivers).
    /// Useful to record a published version under the same name as its
    /// repository, enabling version-vs-branch comparisons of one series name.
    #[arg(long)]
    pub driver_name: Option<String>,

    /// Path to the benchmarks repository
    /// (containing scenarios/config.yml and apis/<api>/).
    #[arg(long, short = 'p')]
    pub benchmarks_path: PathBuf,

    /// Scenario to run; may be repeated. Defaults to all scenarios.
    #[arg(long, short = 's')]
    pub scenario: Vec<String>,

    /// Override for the scenarios config
    /// (default: <benchmarks-path>/scenarios/config.yml).
    #[arg(long, short = 'b')]
    pub scenarios_config: Option<PathBuf>,

    #[arg(long, short = 'M', value_enum, default_value_t = InputBenchmarkMode::UseCached)]
    pub benchmark_mode: InputBenchmarkMode,

    /// Continue with the remaining points/scenarios when a point fails,
    /// instead of aborting (the failed point is not recorded).
    #[arg(long)]
    pub keep_going: bool,

    #[clap(subcommand)]
    pub measure: Option<MeasureSubcommand>,
}

impl BenchmarkCommand {
    pub(crate) fn finalize(self) -> Result<Subcommands, ParsingError> {
        let bench_measure = self.measure.unwrap_or(MeasureSubcommand::Time).finalize()?;
        let benchmark_mode = self.benchmark_mode.finalize();

        let mut driver = match (&self.driver_path, &self.driver) {
            (Some(path), None) => DriverSpec::from_repo(&path.canonicalize()?)?,
            (None, Some(spec)) => DriverSpec::from_published_spec(spec)?,
            // clap guarantees exactly one is present.
            _ => unreachable!("clap enforces exactly one of --driver-path/--driver"),
        };
        if let Some(name) = self.driver_name {
            driver.name = name;
        }
        let driver_commit = driver.commit_id()?;

        let benchmarks_path = self.benchmarks_path.canonicalize()?;
        let api = ApiSetup::load(&benchmarks_path, &driver.api)?;
        let benchmarks_commit = CommitHash::new(&benchmarks_path, "HEAD".to_owned())
            .map(|h| h.to_string())
            .unwrap_or_else(|_| "unknown".to_owned());

        let scenarios_config_path = self
            .scenarios_config
            .unwrap_or_else(|| benchmarks_path.join("scenarios").join("config.yml"));
        let all_scenarios: Vec<BenchmarkData> =
            open_config::<BenchmarkConfigList>(&scenarios_config_path)?
                .configs()
                .map(BenchmarkData::from)
                .collect();

        let benchmarks = if self.scenario.is_empty() {
            all_scenarios
        } else {
            let mut selected = Vec::new();
            for name in &self.scenario {
                let scenario = all_scenarios
                    .iter()
                    .find(|b| &b.name == name)
                    .ok_or_else(|| ParsingError::UnknownScenario {
                        name: name.clone(),
                        config_path: scenarios_config_path.clone(),
                    })?;
                selected.push(scenario.clone());
            }
            selected
        };

        Ok(Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            session: Session {
                driver,
                driver_commit,
                api,
                benchmarks_commit,
            },
            benchmarks,
            benchmark_mode,
            keep_going: self.keep_going,
        }))
    }
}
