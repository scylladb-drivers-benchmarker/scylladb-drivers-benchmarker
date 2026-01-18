use std::path::PathBuf;

use clap::Parser;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
use scylladb_drivers_benchmarker::repo_with_commits::RepoNameWithTags;

use crate::parsing::database::InputDatabaseCommand;
use crate::parsing::plot::{InputPlotKind, InputVisKind};
use crate::parsing::{App, AppSubcommands, BenchmarkCommand, PlotCommand};

#[test]
fn basic_run() {
    let args = App::parse_from(vec!["scylladb-drivers-benchmarker", "run", "select"]);

    let AppSubcommands::Run(BenchmarkCommand { benchmark_name, .. }) = args.subcommand else {
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
        "-o",
        "plot.svg",
        "series",
    ]);

    let AppSubcommands::Plot(PlotCommand {
        benchmark_name,
        benchmark_config_path: _,
        from,
        output,
        plot_kind,
    }) = args.subcommand
    else {
        panic!("Not a plot");
    };

    let from: Vec<RepoNameWithTags> = from.into_iter().map(From::from).collect();

    assert_eq!(benchmark_name, "select");

    assert!(matches!(plot_kind, InputPlotKind::Series { .. }));
    match plot_kind {
        InputPlotKind::Series {
            measurement_method,
            visualization_kind,
        } => {
            assert!(matches!(measurement_method, MeasurementMethod::Time));
            assert!(matches!(visualization_kind, InputVisKind::Linear));
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
    assert_eq!(output, PathBuf::from("plot.svg".to_owned()));
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
    let AppSubcommands::Database(command) = args.subcommand else {
        panic!("Expected Database subcommand");
    };

    let InputDatabaseCommand::Print { filters } = command.command else {
        panic!("Expected DatabaseCommand::Print");
    };

    assert_eq!(filters.commit_hashes, vec!["test", "21123123", "ff"]);
    assert_eq!(filters.benchmark_names, vec!["my", "benchmark", ""]);
    assert_eq!(filters.benchmark_points, vec![1, 2, 5, 3]);
    assert_eq!(filters.measurement_methods, vec!["m1", "m2", "m4"]);
}
