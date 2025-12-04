use std::{
    error::Error,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    commit_hash::CommitHash,
    config::{benchmark::BenchmarkConfig, find_config},
    database::Database,
};

use clap::Parser;

#[allow(dead_code)]
mod benchmarking;
mod command;
mod commit_hash;
mod config;
pub mod database;
mod plotting;
mod utilities;

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWithCommits {
    pub repo_path: PathBuf,
    pub commits: Vec<String>,
}

impl FromStr for RepositoryWithCommits {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut parts = string.split(':');
        let repo_path = parts.next().expect("repo path not supplied"); // TODO: fix

        let commits: Vec<String> = parts.map(str::to_owned).collect();

        Ok(RepositoryWithCommits {
            repo_path: PathBuf::from_str(repo_path)?,
            commits,
        })
    }
}

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
