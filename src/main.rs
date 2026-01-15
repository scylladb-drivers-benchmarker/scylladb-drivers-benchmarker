use crate::parsing::App;
use clap::Parser;
use scylladb_drivers_benchmarker::access_database;
use scylladb_drivers_benchmarker::config::backend::BackendConfig;
use scylladb_drivers_benchmarker::config::benchmark::BenchmarkConfig;
use scylladb_drivers_benchmarker::{
    OutputFormat, PlotKind, PlotSettings, command,
    database::Database,
    flame_graph::BenchMeasure,
    measurement::MeasurementMethod,
    utilities::{BenchmarkMode, RepoNameWithTags, RepoPathWithCommits},
};

mod parsing;
pub struct BenchmarkParams {
    pub bench_measure: BenchMeasure,

    pub backend_config: BackendConfig,

    pub benchmark_config: BenchmarkConfig,

    pub benchmark_mode: BenchmarkMode,
}

pub struct PlotParams {
    pub measurement_method: MeasurementMethod,

    pub benchmark_config: BenchmarkConfig,

    pub from: Vec<RepoNameWithTags>, // TODO 2 separated args for this are bad IMO
    pub resolved: Vec<RepoPathWithCommits>,

    pub plot_settings: PlotSettings,
}

fn print_error<T>(err: impl std::error::Error) -> T {
    println!("{}", err);
    std::process::exit(1);
}

fn main() {
    let input = App::parse().finalize().unwrap_or_else(print_error);

    match input.params {
        crate::parsing::Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            backend_config,
            benchmark_config,
            benchmark_mode,
        }) => {
            scylladb_drivers_benchmarker::run_benchmarks(
                &input.database,
                benchmark_config,
                bench_measure,
                backend_config,
                benchmark_mode,
            )
            .unwrap_or_else(print_error);
        }
        crate::parsing::Subcommands::Plot(PlotParams {
            measurement_method,
            benchmark_config,
            from,
            resolved,
            plot_settings,
        }) => {
            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                benchmark_config,
                &measurement_method,
                from,
                resolved,
            )
            .unwrap();
        }
        crate::parsing::Subcommands::Database(database_sucommand) => {
            access_database(&input.database, database_sucommand).unwrap(); // TODO
        }
    }
}
