use std::path::Path;

use scylladb_drivers_benchmarker::commit_hash::CommitHash;
use scylladb_drivers_benchmarker::database::utilities::{BenchmarkParams, BenchmarkRecord};
use scylladb_drivers_benchmarker::utilities::{BenchmarkParamsBuilder, FlatBenchmarkRecord};
use serial_test::file_serial;
use utilities::run_utilities::{run, run_safe, sdb_command};

use crate::utilities::db_utils::open_clean_db;
use crate::utilities::git_utils::setup_git;
use crate::utilities::run_utilities::print_flame_graph_information;

mod utilities;

fn check_data(
    commit_hash: &CommitHash,
    driver_name: &str,
    data: Vec<(BenchmarkParams, BenchmarkRecord)>,
) {
    let param_builder = BenchmarkParamsBuilder::new(
        commit_hash.clone(),
        "regex".to_owned(),
        driver_name.to_owned(),
        "time".to_owned(),
    );

    assert_eq!(data.len(), 8usize);
    for (params, record) in data {
        if params != param_builder.finalize(params.benchmark_point) {
            println!("{:?}", params);
            assert!(params == param_builder.finalize(params.benchmark_point));
        }

        match record {
            BenchmarkRecord::Data(data) => {
                data.parse::<f64>().expect("Should be parsable as f64");
            }
            BenchmarkRecord::TimedData { .. } => {} // mean is always a valid f64
            BenchmarkRecord::FilePath(file) => panic!("File in database: {}", file.display()),
            BenchmarkRecord::Timeout => {}
        };
    }
}

/// Runs the tandem: two fixture "drivers" (separate git repos) benchmarked
/// with the same fixture benchmarks repo, via two different apis.
#[test]
#[file_serial]
fn cpp_vs_rust() {
    let test_dir = Path::new("./tests/cpp_vs_rust_test/");
    setup_git(test_dir.join("drivers/cpp/"));
    setup_git(test_dir.join("drivers/rust/"));
    let db_path = test_dir.join("test.db");
    let db = open_clean_db(&db_path);

    let mut hashes = Vec::new();
    for driver in ["cpp", "rust"] {
        let driver_path = test_dir.join("drivers").join(driver);
        run_safe(
            sdb_command()
                .env("RUST_LOG", "off")
                .arg("-d")
                .arg(&db_path)
                .arg("run")
                .arg("--driver-path")
                .arg(&driver_path)
                .arg("--benchmarks-path")
                .arg(test_dir.join("benchmarks"))
                .arg("--keep-going")
                .arg("time"),
            |output| output.status.success() && output.stdout.is_empty() && output.stderr.is_empty(),
        );
        hashes.push(CommitHash::new(&driver_path, "HEAD".to_owned()).unwrap());
    }
    let (hash_cpp, hash_rust) = (hashes.remove(0), hashes.remove(0));
    assert!(hash_cpp != hash_rust);

    let db_data = db.get_all_data().unwrap();

    let data_cpp = db_data
        .iter()
        .filter(|(params, _)| params.commit_hash == hash_cpp)
        .map(Clone::clone);
    check_data(&hash_cpp, "regex-cpp", data_cpp.collect());

    let data_rust = db_data
        .iter()
        .filter(|(params, _)| params.commit_hash == hash_rust)
        .map(Clone::clone);
    check_data(&hash_rust, "regex-rust", data_rust.collect());
}

#[test]
fn flame_graph() {
    let test_dir = Path::new("./tests/flame_graph_bench_test/");
    let benchmark_name = "recurse";
    let db = open_clean_db(&test_dir.join("test.db"));
    setup_git(test_dir.join("driver"));

    let flame_config_path = test_dir.join("flame-path.yml");
    print_flame_graph_information(&flame_config_path);
    // The FlameGraph repository location requires manual setup, as before.
    let flame_config = std::fs::read_to_string(&flame_config_path)
        .expect("flame-path.yml with `flame-path: <FlameGraph repo>` is required");
    let flame_repo = flame_config
        .lines()
        .find_map(|l| l.strip_prefix("flame-path:"))
        .expect("flame-path key missing in flame-path.yml")
        .trim()
        .to_owned();

    run(sdb_command()
        .env("RUST_LOG", "off")
        .args([
            "-d",
            "./test.db",
            "run",
            "--driver-path",
            "./driver",
            "--benchmarks-path",
            "./benchmarks",
            "--scenario",
            benchmark_name,
            "flame-graph",
            "-r",
            &flame_repo,
            "-s",
            "./store",
        ])
        .current_dir(test_dir));

    let param_builder = BenchmarkParamsBuilder::new(
        CommitHash::new(&test_dir.join("driver"), "HEAD".to_owned()).unwrap(),
        benchmark_name.to_owned(),
        "recurse-c".to_owned(),
        "flame-graph".to_owned(),
    );

    let all_data = db.get_all_data().unwrap();
    assert!(all_data.len() > 1);
    for (params, record) in all_data {
        if params != param_builder.finalize(params.benchmark_point) {
            println!("{:?}", params);
            assert!(params == param_builder.finalize(params.benchmark_point));
        }

        let FlatBenchmarkRecord::Data(record_value) = record.flatten().unwrap() else {
            panic!("Unexpected timeout at point: {}", params.benchmark_point);
        };

        println!("{}", record_value);

        println!("foos: {}", record_value.matches("foo").count());
        println!("goos: {}", record_value.matches("goo").count());
        assert!(params.benchmark_point as usize / 1_000 < record_value.matches("foo").count());
        assert!(params.benchmark_point as usize / 1_000 < record_value.matches("goo").count());
    }
}

/// Custom `command` measurement mode: the measuring command wraps the run
/// command; here `echo` just prints it.
#[test]
fn utility_test() {
    let test_dir = Path::new("./tests/utility_test/");
    setup_git(test_dir.join("driver"));
    let commit_hash = CommitHash::new(&test_dir.join("driver"), "HEAD".to_owned()).unwrap();

    let db_path = test_dir.join("db.db");
    let db = open_clean_db(&db_path);

    run(sdb_command().arg("-d").arg(&db_path).args([
        "run",
        "--driver-path",
        "./tests/utility_test/driver",
        "--benchmarks-path",
        "./tests/utility_test/benchmarks",
        "command",
        "echo",
    ]));

    let expected = |point| {
        (
            BenchmarkParams::new(
                commit_hash.clone(),
                "utility".to_owned(),
                "sleeper".to_owned(),
                point,
                "echo".to_owned(),
            ),
            BenchmarkRecord::Data("./run.sh".to_owned()),
        )
    };

    let mut data = db.get_all_data().unwrap();
    data.sort_by_key(|(params, _)| params.benchmark_point);
    assert_eq!(data, vec![expected(1), expected(2), expected(4)]);
}
