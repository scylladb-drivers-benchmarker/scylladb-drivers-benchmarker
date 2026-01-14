use crate::parsing::ParsedParams;
use crate::parsing::parse_all;
use scylladb_drivers_benchmarker::access_database;
use scylladb_drivers_benchmarker::repo_with_commits::resolve_repo_tags;
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

/*pub enum DatabaseSucommand {
    Print(BenchmarkFilters),
    Drop(BenchmarkFilters),
}*/

pub struct PlotParams {
    pub benchmark_name: String,

    pub measurement_method: MeasurementMethod,

    pub benchmark_config_path: PathBuf,

    pub from: Vec<RepoNameWithTags>,

    pub plot_settings: PlotSettings,
    /*

        plot_settings: PlotSettings,
        database: &Database,
        benchmark_name: &str,
        benchmark_config_path: &Path,
        measurement_method: &MeasurementMethod,
        from: Vec<RepoNameWithTags>,
        resolved: Vec<RepoPathWithCommits>,

    */
}

fn print_error<T>(err: impl std::error::Error) -> T {
    println!("{}", err);
    std::process::exit(1);
}

fn main() {
    let mut input: ParsedParams = parse_all().unwrap(); // TODO

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
                } =>  {
                    input.aliasing_config.store_dir = input.aliasing_config.store_dir.or(store_dir);
                    BenchMeasure::FlameGraph {
                    flame_repo: flame_repo
                        .or(input.aliasing_config.flame_path)
                        .unwrap_or_default(),
                    frequency,
                }
            },
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
            plot_settings,
        }) => {
            let parsed: Vec<RepoNameWithTags> = from;

            let resolved = parsed
                .iter()
                .map(|repo| resolve_repo_tags(repo.clone(), &input.aliasing_config.repo_path))
                .collect::<Result<Vec<RepoPathWithCommits>, _>>()
                .unwrap_or_else(print_error);

            scylladb_drivers_benchmarker::plot_benchmarks(
                plot_settings,
                &input.database,
                &benchmark_name,
                &benchmark_config_path,
                &measurement_method,
                parsed,
                resolved,
            )
            .unwrap();
        }
        crate::parsing::Subcommands::Database(database_sucommand) => {
            access_database(&input.database, database_sucommand).unwrap(); // TODO
        }
    }
}

#[cfg(test)]
mod test {
    /*use clap::Parser;

    use crate::{App, AppSubcommand, DatabaseCommand};
    use scylladb_drivers_benchmarker::{
        measurement::MeasurementMethod, utilities::RepoNameWithTags,
    };

    use super::{OutputFormat, PlotKind};
    use scylladb_drivers_benchmarker::VisKind;

    #[test]
    fn basic_run() {
        let args = App::parse_from(vec!["scylladb-drivers-benchmarker", "run", "select"]);

        let AppSubcommand::Run { benchmark_name, .. } = args.subcommand else {
            panic!("Not a run")
        };

        assert_eq!(benchmark_name, "select");
    }

    #[test]
    fn advanced_plot() {
        let args = App::parse_from(vec![
            "scylladb-drivers-benchmarker",
            "plot",
            "select",
            "--from=repo:branch",
            "--from",
            "repo2:commit",
            "--format=svg",
            "series",
        ]);

        let AppSubcommand::Plot {
            benchmark_name,
            measurement_method,
            benchmark_config_path: _,
            from,
            output,
            format,
            plot_kind,
        } = args.subcommand
        else {
            panic!("Not a plot");
        };

        let from: Vec<RepoNameWithTags> = from.into_iter().map(From::from).collect();

        assert_eq!(benchmark_name, "select");
        assert_eq!(measurement_method, MeasurementMethod::Time);

        assert!(matches!(plot_kind, PlotKind::Series { .. }));
        match plot_kind {
            PlotKind::Series { visualization_kind } => {
                assert!(matches!(visualization_kind, VisKind::Linear))
            }
            _ => panic!("Expected PlotKind::Series"),
        }

        assert_eq!(
            from,
            vec!(
                RepoNameWithTags {
                    name: "repo".to_owned(),
                    tags: vec!("branch".to_owned())
                },
                RepoNameWithTags {
                    name: "repo2".to_owned(),
                    tags: vec!("commit".to_owned())
                }
            )
        );
        assert_eq!(output, None);
        assert!(matches!(format, OutputFormat::Svg));
    }

    #[test]
    fn advanced_database() {
        let args = App::parse_from(vec![
            "scylladb-drivers-benchmarker",
            "database",
            "print",
            "--commit-hash=test:21123123:ff",
            "--benchmark-name=my:benchmark:",
            "--benchmark-point=1:2:5:3",
            "--measurement-method=m1:m2:m4",
        ]);
        let AppSubcommand::Database { command } = args.subcommand else {
            panic!("Expected Database subcommand");
        };

        let DatabaseCommand::Print { filters } = command else {
            panic!("Expected DatabaseCommand::Print");
        };

        assert_eq!(filters.commit_hashes, vec!["test", "21123123", "ff"]);
        assert_eq!(filters.benchmark_names, vec!["my", "benchmark", ""]);
        assert_eq!(filters.benchmark_points, vec![1, 2, 5, 3]);
        assert_eq!(filters.measurement_methods, vec!["m1", "m2", "m4"]);
    }*/
}
