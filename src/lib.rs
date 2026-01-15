use crate::config::backend::BackendConfig;
use crate::config::benchmark::BenchmarkConfig;
use crate::utilities::DatabaseCommand;
use crate::{
    benchmarking::BenchmarkingError,
    commit_hash::CommitHash,
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
    CommitHash(#[from] Box<commit_hash::FailedToRetrieveCommitHash>),
}

pub fn run_benchmarks(
    database: &Database,
    benchmark_config: BenchmarkConfig,
    bench_measure: BenchMeasure,
    backend_config: BackendConfig,
    benchmark_mode: BenchmarkMode,
) -> Result<(), RunBenchmarksError> {
    let commit_hash = CommitHash::from_current_repository()?;

    Ok(benchmarking::benchmark(
        database,
        commit_hash,
        benchmark_config,
        backend_config,
        bench_measure,
        benchmark_mode,
    )?)
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    // TODO
    Plotting(#[from] PlotError),
}

pub fn plot_benchmarks(
    plot_settings: PlotSettings,
    database: &Database,
    benchmark_config: BenchmarkConfig,
    measurement_method: &MeasurementMethod,
    from: Vec<RepoNameWithTags>,
    resolved: Vec<RepoPathWithCommits>,
) -> Result<(), PlotBenchmarksError> {
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
