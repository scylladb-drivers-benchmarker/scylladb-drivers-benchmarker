use std::{
    error::Error,
    path::Path,
};

use crate::{
    commit_hash::CommitHash,
    config::{benchmark::BenchmarkConfig, find_config},
    database::Database,
    utilities::RepositoryWithCommits,
};

#[allow(dead_code)]
mod benchmarking;
mod command;
mod commit_hash;
mod config;
pub mod database;
mod plotting;
pub mod utilities;

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: String,
    backend_config_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig = find_config(&benchmark_name, &benchmark_config_path)?;

    let commit_hash: CommitHash = CommitHash::from_repository()?;
    benchmarking::benchmark(
        &database,
        commit_hash,
        benchmark_config,
        backend_config_path,
        measurement_method,
    )
}

pub fn plot_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: &str,
    visualization_kind: Option<String>,
    from: Vec<RepositoryWithCommits>,
) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig = find_config(&benchmark_name, &benchmark_config_path)?;

    let commit_hashes = vec![]; // TODO: how to get hashes here? from_repo? new_unchecked? whats the format
    let names = from
        .iter()
        .flat_map(|repo| {
            let repo_name = repo.repo_path.to_string_lossy();

            repo.commits
                .iter()
                .map(move |commit| format!("{}@{}", repo_name, commit))
        })
        .collect();

    plotting::plot(
        &database,
        &benchmark_name,
        &benchmark_config,
        measurement_method,
        visualization_kind,
        &commit_hashes,
        &names,
    )
}
