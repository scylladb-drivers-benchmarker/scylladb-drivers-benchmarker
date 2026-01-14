use crate::parsing::ParsedParams;
use crate::parsing::parse_all;
use scylladb_drivers_benchmarker::access_database;
use scylladb_drivers_benchmarker::{
    OutputFormat, PlotKind, PlotSettings, command,
    database::Database,
    flame_graph::BenchMeasure,
    measurement::MeasurementMethod,
    utilities::{BenchmarkMode, RepoNameWithTags, RepoPathWithCommits},
};
use std::path::PathBuf;

mod parsing;
use crate::parsing::benchmark::MeasureSubcommand;

pub struct BenchmarkParams {
    pub benchmark_name: String,

    pub measure: Option<MeasureSubcommand>,

    pub backend_config_path: PathBuf,

    pub benchmark_config_path: PathBuf,

    pub benchmark_mode: BenchmarkMode,
}

pub struct PlotParams {
    pub benchmark_name: String,

    pub measurement_method: MeasurementMethod,

    pub benchmark_config_path: PathBuf,

    pub from: Vec<RepoNameWithTags>, // TODO 2 separates args for this are bad
    pub resolved: Vec<RepoPathWithCommits>,

    pub plot_settings: PlotSettings,
}

fn print_error<T>(err: impl std::error::Error) -> T {
    println!("{}", err);
    std::process::exit(1);
}

fn main() {
    let mut input: ParsedParams = parse_all().unwrap_or_else(print_error);

    match input.params {
        crate::parsing::Subcommands::Benchmark(BenchmarkParams {
            benchmark_name,
            measure,
            backend_config_path,
            benchmark_config_path,
            benchmark_mode,
        }) => {
            let bench_measure = match measure.unwrap_or(MeasureSubcommand::Time) {
                MeasureSubcommand::Time => BenchMeasure::Time,
                MeasureSubcommand::PerfStat => BenchMeasure::PerfStat,
                MeasureSubcommand::FlameGraph {
                    flame_repo,
                    frequency,
                    store_dir,
                } => {
                    input.aliasing_config.store_dir = input.aliasing_config.store_dir.or(store_dir);
                    BenchMeasure::FlameGraph {
                        flame_repo: flame_repo
                            .or(input.aliasing_config.flame_path)
                            .unwrap_or_default(),
                        frequency,
                    }
                }
                MeasureSubcommand::Command { command } => BenchMeasure::Command(command),
            };

            scylladb_drivers_benchmarker::run_benchmarks(
                &input.database,
                &benchmark_name,
                &benchmark_config_path,
                bench_measure,
                backend_config_path.as_path(),
                benchmark_mode,
                input.aliasing_config.store_dir,
            )
            .unwrap();
        }
        crate::parsing::Subcommands::Plot(PlotParams {
            benchmark_name,
            measurement_method,
            benchmark_config_path,
            from,
            resolved,
            plot_settings,
        }) => {
            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                &benchmark_name,
                &benchmark_config_path,
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
