use std::path::{Path, PathBuf};

use crate::utilities::DatabaseCommand;
use crate::{
    benchmarking::BenchmarkingError,
    command::CommandParsingError,
    commit_hash::CommitHash,
    config::{ConfigError, find_config},
    database::{Database, DatabaseError},
    flame_graph::BenchMeasure,
    measurement::MeasurementMethod,
    plotting::error::PlotError,
    utilities::{BenchmarkMode, RepoNameWithTags, RepoPathWithCommits, format_entry},
};
pub use plotting::{OutputFormat, PlotKind, PlotSettings, VisKind};
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
    BenchmarkConfig(#[source] ConfigError),
    BackendConfig(#[source] ConfigError),
    CommitHash(#[from] Box<commit_hash::FailedToRetrieveCommitHash>),
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    Plotting(#[from] PlotError),
    BenchmarkConfig(#[from] ConfigError),
    CommitHash(#[from] Box<commit_hash::FailedToRetrieveCommitHash>),
    CommandParsingError(#[from] CommandParsingError),
}

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    bench_measure: BenchMeasure,
    backend_config_path: &Path,
    benchmark_mode: BenchmarkMode,
    store_dir: Option<PathBuf>,
) -> Result<(), RunBenchmarksError> {
    let benchmark_config = find_config(benchmark_name, benchmark_config_path)
        .map_err(RunBenchmarksError::BenchmarkConfig)?;

    let backend_config = find_config(benchmark_name, backend_config_path)
        .map_err(RunBenchmarksError::BackendConfig)?;

    let commit_hash = CommitHash::from_current_repository()?;

    Ok(benchmarking::benchmark(
        database,
        commit_hash,
        benchmark_config,
        backend_config,
        bench_measure,
        benchmark_mode,
        store_dir,
    )?)
}

pub fn plot_benchmarks(
    plot_settings: PlotSettings,
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: &MeasurementMethod,
    from: Vec<RepoNameWithTags>,
    resolved: Vec<RepoPathWithCommits>,
) -> Result<(), PlotBenchmarksError> {
    let benchmark_config = find_config(benchmark_name, benchmark_config_path)?;

    let names = from
        .into_iter()
        .flat_map(|repo| {
            repo.tags
                .into_iter()
                .map(move |commit| format!("{}@{}", repo.name, commit))
        })
        .collect::<Vec<String>>();

    let commit_hashes = resolved.into_iter().flat_map(|repo| repo.git_hashes);

    plotting::plot(
        plot_settings,
        database,
        benchmark_config,
        measurement_method,
        commit_hashes,
        &names,
    )?;

    Ok(())
}

pub fn access_database(
    database: &Database,
    operation: DatabaseCommand,
) -> Result<(), DatabaseError> {
    match operation {
        DatabaseCommand::Print { filters } => {
            let results = database.get_data(&filters)?;
            for (params, result) in results.into_iter() {
                print!("{}", format_entry(&params, &result));
            }
        }
        DatabaseCommand::Drop { filters } => database.drop_data(&filters)?,
    }

    Ok(())
}
