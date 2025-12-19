use std::{path::Path, str::FromStr};

use crate::{
    benchmarking::BenchmarkingError,
    command::CommandParsingError,
    commit_hash::{CommitHash, CommitHashError},
    config::{ConfigError, backend::BackendConfig, benchmark::BenchmarkConfig, find_config},
    database::{Database, DatabaseError},
    plotting::{PlotKind, error::PlotError},
    utilities::{
        BenchmarkFilters, BenchmarkMode, DatabaseCommand, RepositoryWithCommits, format_entry,
    },
};

pub use plotting::VisKind;

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
    BenchmarkConfig(#[source] ConfigError),
    BackendConfig(#[source] ConfigError),
    CommitHash(#[from] CommitHashError),
}

#[justerror::Error]
pub enum PlotBenchmarksError {
    Plotting(#[from] PlotError),
    BenchmarkConfig(#[from] ConfigError),
    CommitHash(#[from] CommitHashError),
    CommandParsingError(#[from] CommandParsingError),
}

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: String,
    backend_config_path: &Path,
    bechmark_mode: BenchmarkMode,
) -> Result<(), RunBenchmarksError> {
    let benchmark_config: BenchmarkConfig = find_config(benchmark_name, benchmark_config_path)
        .map_err(RunBenchmarksError::BenchmarkConfig)?;

    let backend_config: BackendConfig = find_config(benchmark_name, backend_config_path)
        .map_err(RunBenchmarksError::BackendConfig)?;

    let commit_hash: CommitHash = CommitHash::from_current_repository()?;

    Ok(benchmarking::benchmark(
        database,
        commit_hash,
        benchmark_config,
        backend_config,
        measurement_method,
        bechmark_mode,
    )?)
}

pub fn plot_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: &str,
    visualization_kind: Option<VisKind>,
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

    let commit_hashes: Vec<CommitHash> = from.into_iter().try_fold(
        Vec::new(),
        |mut acc, repo| -> Result<_, PlotBenchmarksError> {
            acc.extend(repo.to_commit_hashes()?);
            Ok(acc)
        },
    )?;

    let cmd = command::Command::from_str(measurement_method)?;

    let plot_kind = match cmd.program() {
        "time" => {
            // Default to linear if no vis kind provided
            let vis_kind = visualization_kind.unwrap_or(VisKind::Linear);
            PlotKind::Series(vis_kind)
        }
        "flamegraph" => {
            if visualization_kind.is_some() {
                return Err(PlotBenchmarksError::Plotting(
                    PlotError::UnexpectedVisualizationKind(),
                ));
            }
            PlotKind::Flamegraph
        }
        other => {
            return Err(PlotBenchmarksError::Plotting(
                PlotError::UnknownMeasureKind(other.to_owned()),
            ));
        }
    };

    plotting::plot(
        plot_kind,
        database,
        benchmark_name,
        &benchmark_config,
        measurement_method,
        &commit_hashes,
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
            let results = database.get_data(&BenchmarkFilters::from_input_commands(filters))?;
            for (params, result) in results.into_iter() {
                print!("{}", format_entry(&params, &result));
            }
        }
        DatabaseCommand::Drop { filters } => {
            database.drop_data(&BenchmarkFilters::from_input_commands(filters))?
        }
    }

    Ok(())
}
