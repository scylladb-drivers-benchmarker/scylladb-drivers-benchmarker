use std::path::Path;

use crate::{
    benchmarking::BenchmarkingError,
    commit_hash::{CommitHash, CommitHashError},
    config::{ConfigError, benchmark::BenchmarkConfig, find_config},
    database::Database,
    plotting::error::PlotError,
    utilities::RepositoryWithCommits,
};

#[allow(dead_code)]
mod benchmarking;
mod command;
pub mod commit_hash;
mod config;
pub mod database;
mod plotting;
pub mod utilities;

#[justerror::Error]
pub enum RunBenchmarksError {
    BenchmarkingError(
        #[from]
        #[source]
        BenchmarkingError,
    ),
    BenchmarkConfigError(
        #[from]
        #[source]
        ConfigError,
    ),
    CommitHashError(
        #[from]
        #[source]
        CommitHashError,
    ),
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    PlottingError(
        #[from]
        #[source]
        PlotError,
    ),

    BenchmarkConfigError(
        #[from]
        #[source]
        ConfigError,
    ),

    CommitHashError(
        #[from]
        #[source]
        CommitHashError,
    ),
}

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: String,
    backend_config_path: &Path,
) -> Result<(), RunBenchmarksError> {
    let benchmark_config: BenchmarkConfig = find_config(benchmark_name, benchmark_config_path)?;

    let commit_hash: CommitHash = CommitHash::from_current_repository()?;
    Ok(benchmarking::benchmark(
        database,
        commit_hash,
        benchmark_config,
        backend_config_path,
        measurement_method,
    )?)
}

pub fn plot_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: &str,
    visualization_kind: Option<String>,
    from: Vec<RepositoryWithCommits>,
) -> Result<(), PlotBenchmarksError> {
    let benchmark_config: BenchmarkConfig = find_config(benchmark_name, benchmark_config_path)?;

    let names = from
        .iter()
        .flat_map(|repo| {
            let repo_name = repo.repo_path.to_string_lossy();

            repo.commits
                .iter()
                .map(move |commit| format!("{}@{}", repo_name, commit))
        })
        .collect();

    let commit_hashes: Vec<CommitHash> = from
        .into_iter()
        .map(|repo| repo.to_commit_hashes())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    match measurement_method {
        "time" => {
            // Default to linear if no vis kind provided
            let vis_kind = match visualization_kind.as_deref() {
                Some("log") => crate::plotting::VisKind::Log,
                _ => crate::plotting::VisKind::Linear,
            };

            Ok(plotting::plot(
                plotting::PlotKind::Series(vis_kind),
                database,
                benchmark_name,
                &benchmark_config,
                measurement_method,
                &commit_hashes,
                &names,
            )?)
        }

        "flamegraph" => {
            // Should not provide vis_kind
            if visualization_kind.is_some() {
                Err(PlotError::UnsupportedVisKind)?; // Hacky, maybe change to something more explicit.
            }

            Ok(plotting::plot(
                plotting::PlotKind::Flamegraph,
                database,
                benchmark_name,
                &benchmark_config,
                measurement_method,
                &commit_hashes,
                &names,
            )?)
        }
        _ => Err(PlotError::UnknownMeasureKind)?, // Hacky, maybe change to something more explicit.
    }
}
