use clap::Parser;
use log::error;
use scylladb_drivers_benchmarker::benchmarking::{BenchMeasure, BenchmarkMode, Session};
use scylladb_drivers_benchmarker::config::benchmark::BenchmarkData;
use scylladb_drivers_benchmarker::database::Database;
use scylladb_drivers_benchmarker::database::utilities::BenchmarkFilters;
use scylladb_drivers_benchmarker::{DriverWithCommit, PlotKind, PlotSettings, command};

use crate::parsing::App;

mod parsing;
pub struct BenchmarkParams {
    pub bench_measure: BenchMeasure,

    pub session: Session,
    pub benchmarks: Vec<BenchmarkData>,

    pub benchmark_mode: BenchmarkMode,
    pub keep_going: bool,
}

pub struct PlotParams {
    pub benchmarks: Vec<BenchmarkData>,
    pub series: Vec<DriverWithCommit>,
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
        crate::parsing::Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            session,
            benchmarks,
            benchmark_mode,
            keep_going,
        }) => {
            scylladb_drivers_benchmarker::run_benchmarks(
                &input.database,
                &session,
                benchmarks,
                bench_measure,
                benchmark_mode,
                keep_going,
            )
            .unwrap_or_else(print_error);
        }
        crate::parsing::Subcommands::Plot(PlotParams {
            benchmarks,
            series,
            plot_settings,
        }) => {
            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                benchmarks,
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
