pub use plotting::{PlotKind, PlotSettings, VisKind};

use crate::benchmarking::{BenchMeasure, BenchmarkMode, BenchmarkingError, EnvCleanupGuard, Session};
use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkData;
use crate::database::utilities::BenchmarkFilters;
use crate::database::{Database, DatabaseError};
use crate::plotting::error::PlotError;
use crate::utilities::format_entry;

/// A specific (backend, commit, label) triple identifying one data series to plot.
#[derive(Clone)]
pub struct DriverWithCommit {
    pub driver_name: String,
    pub commit: CommitHash,
    pub tag: String,
}
pub mod benchmarking;
pub mod command;
pub mod commit_hash;
pub mod config;
pub mod database;
pub mod flame_graph;
pub mod measurement;
pub mod perf_stat;
pub mod plotting;
pub mod repo_with_commits;
pub mod utilities;

#[justerror::Error]
pub enum RunBenchmarksError {
    Benchmarking(#[from] BenchmarkingError),
    CommitHash(#[from] Box<commit_hash::FailedToRetrieveCommitHash>),
}

/// Runs one benchmarking session: env-prepare + build once, then for every
/// benchmark and point the prepare (unmeasured) / run (measured) / teardown
/// (unmeasured) phases. Env-cleanup runs at the end, also on failure.
pub fn run_benchmarks(
    database: &Database,
    session: &Session,
    benchmarks: Vec<BenchmarkData>,
    bench_measure: BenchMeasure,
    benchmark_mode: BenchmarkMode,
    keep_going: bool,
) -> Result<(), RunBenchmarksError> {
    session.env_prepare()?;
    let _cleanup = EnvCleanupGuard(session);

    session.build()?;

    for benchmark_config in benchmarks {
        benchmarking::benchmark(
            database,
            session,
            benchmark_config,
            bench_measure.clone(),
            benchmark_mode,
            keep_going,
        )?;
    }
    Ok(())
}

pub fn plot_benchmarks(
    plot_settings: PlotSettings,
    database: &Database,
    benchmarks: Vec<BenchmarkData>,
    series: Vec<DriverWithCommit>,
) -> Result<(), PlotError> {
    plotting::plot(plot_settings, database, benchmarks, series.into_iter())
}

pub fn drop_database(database: &Database, filters: BenchmarkFilters) -> Result<(), DatabaseError> {
    database.drop_data(&filters)?;
    Ok(())
}

pub fn print_database(database: &Database, filters: BenchmarkFilters) -> Result<(), DatabaseError> {
    let results = database.get_data(&filters)?;
    for (params, result) in results {
        print!("{}", format_entry(&params, &result));
    }
    Ok(())
}
