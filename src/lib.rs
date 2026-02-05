use log::info;
pub use plotting::{PlotKind, PlotSettings, VisKind};

use crate::benchmarking::{BenchMeasure, BenchmarkMode, BenchmarkingError};
use crate::commit_hash::CommitHash;
use crate::config::backend::BackendConfig;
use crate::config::benchmark::BenchmarkData;
use crate::database::utilities::BenchmarkFilters;
use crate::database::{Database, DatabaseError};
use crate::plotting::error::PlotError;
use crate::repo_with_commits::{RepoNameWithTags, RepoPathWithCommits, SafeRepoNameWithTags};
use crate::utilities::format_entry;
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

pub fn run_benchmarks(
    database: &Database,
    benchmark_config: BenchmarkData,
    bench_measure: BenchMeasure,
    backend_config: BackendConfig,
    benchmark_mode: BenchmarkMode,
) -> Result<(), RunBenchmarksError> {
    let commit_hash = CommitHash::from_current_repository()?;

    info!("Gathered data");
    Ok(benchmarking::benchmark(
        database,
        commit_hash,
        benchmark_config,
        backend_config,
        bench_measure,
        benchmark_mode,
    )?)
}

pub fn plot_benchmarks(
    plot_settings: PlotSettings,
    database: &Database,
    benchmark_config: BenchmarkData,
    from: Vec<RepoNameWithTags>,
    resolved: Vec<RepoPathWithCommits>,
) -> Result<(), PlotError> {
    let names = from
        .into_iter()
        .flat_map(|repo| {
            let repo: SafeRepoNameWithTags = repo.into();

            repo.tags
                .into_iter()
                .map(move |commit| format!("{}@{}", repo.safe_name, commit))
        })
        .collect::<Vec<String>>();

    let commit_hashes = resolved.into_iter().flat_map(|repo| repo.git_hashes);

    plotting::plot(
        plot_settings,
        database,
        benchmark_config,
        commit_hashes,
        &names,
    )
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
