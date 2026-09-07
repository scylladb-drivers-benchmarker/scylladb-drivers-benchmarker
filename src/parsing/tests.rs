use std::path::PathBuf;

use clap::Parser;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;

use crate::parsing::database::InputDatabaseCommand;
use crate::parsing::plot::{InputPlotKind, InputQuantity, InputScale, ParsableDriverWithCommit};
use crate::parsing::{App, AppSubcommands, BenchmarkCommand, PlotCommand};

#[test]
fn basic_run() {
    let args = App::parse_from(vec![
        "scylladb-drivers-benchmarker",
        "run",
        "--driver-path",
        "/repo/driver",
        "--benchmarks-path",
        "/repo/benchmarks",
        "--scenario",
        "select",
        "time",
    ]);

    let AppSubcommands::Run(BenchmarkCommand {
        driver_path,
        driver,
        benchmarks_path,
        scenario,
        keep_going,
        ..
    }) = args.subcommand
    else {
        panic!("Not a run")
    };
    assert_eq!(driver_path, Some(PathBuf::from("/repo/driver")));
    assert_eq!(driver, None);
    assert_eq!(benchmarks_path, PathBuf::from("/repo/benchmarks"));
    assert_eq!(scenario, vec!["select".to_owned()]);
    assert!(!keep_going);

    let args = App::parse_from(vec![
        "scylladb-drivers-benchmarker",
        "plot",
        "select",
        "--series=backend-a@/repo/a:branch",
        "--series",
        "backend-b@/repo/b:abc123=v1.0",
        "-o",
        "plot.svg",
        "series",
    ]);

    let AppSubcommands::Plot(PlotCommand {
        benchmark_name,
        benchmark_setup: _,
        series,
        output,
        plot_kind,
    }) = args.subcommand
    else {
        panic!("Not a plot");
    };

    assert_eq!(benchmark_name, Some("select".to_owned()));
    assert_eq!(output, Some(PathBuf::from("plot.svg")));
    assert!(matches!(plot_kind, InputPlotKind::Series { .. }));
    match plot_kind {
        InputPlotKind::Series {
            measurement_method,
            quantity,
            visualization_kind,
        } => {
            assert!(matches!(measurement_method, MeasurementMethod::Time));
            assert!(matches!(quantity, InputQuantity::Time));
            assert!(matches!(visualization_kind, InputScale::Linear));
        }
        _ => panic!("Expected PlotKind::Series"),
    }

    assert_eq!(series.len(), 2);
    let s0: &ParsableDriverWithCommit = &series[0];
    assert_eq!(s0.driver_name, "backend-a");
    assert_eq!(s0.repo, "/repo/a");
    assert_eq!(s0.git_ref, "branch");
    assert_eq!(s0.display_tag, "branch");

    let s1: &ParsableDriverWithCommit = &series[1];
    assert_eq!(s1.driver_name, "backend-b");
    assert_eq!(s1.repo, "/repo/b");
    assert_eq!(s1.git_ref, "abc123");
    assert_eq!(s1.display_tag, "v1.0");
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
