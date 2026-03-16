use clap::Parser;
use log::error;
use scylladb_drivers_benchmarker::benchmarking::{BenchMeasure, BenchmarkMode};
use scylladb_drivers_benchmarker::config::backend::BackendConfig;
use scylladb_drivers_benchmarker::config::benchmark::BenchmarkData;
use scylladb_drivers_benchmarker::database::Database;
use scylladb_drivers_benchmarker::database::utilities::BenchmarkFilters;
use scylladb_drivers_benchmarker::{BackendWithCommit, PlotKind, PlotSettings, command};

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
    pub series: Vec<BackendWithCommit>,
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
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env)
        .format_target(false)
        .format_timestamp(None)
        .init();

    let input = App::parse().finalize().unwrap_or_else(print_error);

    match input.params {
        crate::parsing::Subcommands::Benchmark(params_list) => {
            for BenchmarkParams {
                bench_measure,
                backend_config,
                benchmark_config,
                benchmark_mode,
            } in params_list
            {
                scylladb_drivers_benchmarker::run_benchmarks(
                    &input.database,
                    benchmark_config,
                    bench_measure,
                    backend_config,
                    benchmark_mode,
                )
                .unwrap_or_else(print_error);
            }
        }
        crate::parsing::Subcommands::Plot(PlotParams {
            benchmark_config,
            series,
            plot_settings,
        }) => {
            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                benchmark_config,
                series,
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
