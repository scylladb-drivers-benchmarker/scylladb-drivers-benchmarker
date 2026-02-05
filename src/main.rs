use clap::Parser;
use log::{Log, error, info, log};
use scylladb_drivers_benchmarker::benchmarking::{BenchMeasure, BenchmarkMode};
use scylladb_drivers_benchmarker::config::backend::BackendConfig;
use scylladb_drivers_benchmarker::config::benchmark::BenchmarkData;
use scylladb_drivers_benchmarker::database::Database;
use scylladb_drivers_benchmarker::database::utilities::BenchmarkFilters;
use scylladb_drivers_benchmarker::repo_with_commits::{RepoNameWithTags, RepoPathWithCommits};
use scylladb_drivers_benchmarker::{PlotKind, PlotSettings, command};
use simple_logger::SimpleLogger;

use crate::parsing::App;

mod parsing;
pub struct BenchmarkParams {
    pub bench_measure: BenchMeasure,

    pub backend_config: BackendConfig,
    pub benchmark_config: BenchmarkData,

    pub benchmark_mode: BenchmarkMode,
}

pub struct PlotParams {
    pub benchmark_config: BenchmarkData,

    pub from: Vec<RepoNameWithTags>,
    pub resolved: Vec<RepoPathWithCommits>,

    pub plot_settings: PlotSettings,
}

pub struct PrintDatabaseParams {
    pub filters: BenchmarkFilters,
}

pub struct DropDatabaseParams {
    pub filters: BenchmarkFilters,
}

fn print_error<T>(err: impl std::error::Error) -> T {
    error!("{err}");
    std::process::exit(1);
}

fn main() {
    SimpleLogger::new().env().without_timestamps().init().unwrap_or_else(print_error);

    let input = App::parse().finalize().unwrap_or_else(print_error);

    match input.params {
        crate::parsing::Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            backend_config,
            benchmark_config,
            benchmark_mode,
        }) => scylladb_drivers_benchmarker::run_benchmarks(
            &input.database,
            benchmark_config,
            bench_measure,
            backend_config,
            benchmark_mode,
        )
        .unwrap_or_else(print_error),
        crate::parsing::Subcommands::Plot(PlotParams {
            benchmark_config,
            from,
            resolved,
            plot_settings,
        }) => {
            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                benchmark_config,
                from,
                resolved,
            )
        }
        .unwrap_or_else(print_error),
        crate::parsing::Subcommands::PrintDatabase(PrintDatabaseParams { filters }) => {
            scylladb_drivers_benchmarker::print_database(&input.database, filters)
                .unwrap_or_else(print_error);
        }
        crate::parsing::Subcommands::DropDatabase(DropDatabaseParams { filters }) => {
            scylladb_drivers_benchmarker::drop_database(&input.database, filters)
                .unwrap_or_else(print_error);
        }
    }
}
