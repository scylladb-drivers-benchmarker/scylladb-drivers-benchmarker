use std::{path::Path, str::FromStr};

use crate::{
    benchmarking::BenchmarkingError,
    commit_hash::{CommitHash, CommitHashError},
    config::{ConfigError, benchmark::BenchmarkConfig, find_config},
    database::Database,
    plotting::error::PlotError,
    utilities::RepositoryWithCommits,
};

mod benchmarking;
mod command;
pub mod commit_hash;
mod config;
pub mod database;
mod plotting;
pub mod utilities;

#[justerror::Error]
pub enum RunBenchmarksError {
    Benchmarking(#[from] BenchmarkingError),
    BenchmarkConfig(#[from] ConfigError),
    CommitHash(#[from] CommitHashError),
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    Plotting(#[from] PlotError),

    BenchmarkConfig(#[from] ConfigError),

    CommitHash(#[from] CommitHashError),
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
        .collect::<Vec<String>>();

    let commit_hashes: Vec<CommitHash> = from
        .into_iter()
        .map(|repo| repo.to_commit_hashes())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    // Fix this unwrap
    match command::Command::from_str(measurement_method).unwrap().program() {
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
            if let Some(visualization) = visualization_kind {
                return Err(PlotBenchmarksError::Plotting(
                    PlotError::UnknownMeasureKind(visualization),
                ));
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
        other => Err(PlotBenchmarksError::Plotting(
            PlotError::UnknownMeasureKind(other.to_owned()),
        )),
    }
}
