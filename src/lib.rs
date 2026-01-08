use std::path::Path;

use crate::{
    benchmarking::BenchmarkingError,
    command::CommandParsingError,
    commit_hash::{CommitHash},
    config::{ConfigError, find_config},
    database::{Database, DatabaseError},
    measurement::MeasurementMethod,
    plotting::error::PlotError,
    utilities::{
        BenchmarkMode, DatabaseCommand, RepoNameWithTags, RepoPathWithCommits, format_entry,
    },
};

pub use plotting::{OutputFormat, PlotKind, VisKind};

mod benchmarking;
mod command;
pub mod commit_hash;
mod config;
pub mod database;
pub mod measurement;
mod perf_stat;
mod plotting;
pub mod utilities;

#[justerror::Error]
pub enum RunBenchmarksError {
    Benchmarking(#[from] BenchmarkingError),
    BenchmarkConfig(#[source] ConfigError),
    BackendConfig(#[source] ConfigError),
    CommitHash(#[from] commit_hash::errors::Error),
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    Plotting(#[from] PlotError),
    BenchmarkConfig(#[from] ConfigError),
    CommitHash(#[from] commit_hash::errors::Error),
    CommandParsingError(#[from] CommandParsingError),
}

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: MeasurementMethod,
    backend_config_path: &Path,
    benchmark_mode: BenchmarkMode,
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
        measurement_method,
        benchmark_mode,
    )?)
}

pub fn plot_benchmarks(
    plot_kind: PlotKind,
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: &MeasurementMethod,
    from: Vec<RepoNameWithTags>,
    resolved: Vec<RepoPathWithCommits>,
    format: OutputFormat,
    output: Option<&Path>,
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
        plot_kind,
        database,
        benchmark_name,
        &benchmark_config,
        measurement_method,
        commit_hashes,
        &names,
        format,
        output.and_then(|p| p.to_str()).unwrap_or("plot.png"),
    )?;

    Ok(())
}

pub fn access_database(
    database: &Database,
    operation: DatabaseCommand,
) -> Result<(), DatabaseError> {
    match operation {
        DatabaseCommand::Print { filters } => {
            let results = database.get_data(&filters.into())?;
            for (params, result) in results.into_iter() {
                print!("{}", format_entry(&params, &result));
            }
        }
        DatabaseCommand::Drop { filters } => database.drop_data(&filters.into())?,
    }

    Ok(())
}
