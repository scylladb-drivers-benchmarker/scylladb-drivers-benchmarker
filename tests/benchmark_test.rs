use std::io::Write;
use std::path::Path;

use fs::File;
use fs_err as fs;
use scylladb_drivers_benchmarker::commit_hash::CommitHash;
use scylladb_drivers_benchmarker::database::Database;
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
    data: Vec<(BenchmarkParams, BenchmarkRecord)>,
) {
    let param_builder =
        BenchmarkParamsBuilder::new(commit_hash.clone(), "regex".to_owned(), "time".to_owned());

    assert_eq!(data.len(), 8usize);
    for (params, record) in data {
        if params != param_builder.finalize(params.benchmark_point) {
            println!("{:?}", params);
            assert!(params == param_builder.finalize(params.benchmark_point));
        }

        
        let record=  match record {
            BenchmarkRecord::Data(data) => data,
            BenchmarkRecord::FilePath(file) => panic!("File in database: {}", file.display()),
            BenchmarkRecord::Timeout => continue,
        };

        println!("ddd{record}ddd");
        record.parse::<f64>().expect("Should be parsable");
    }
}

struct CppVsRust {
    db: Database,
}

impl CppVsRust {
    fn new() -> Self {
        setup_git("./tests/cpp_vs_rust_test/cpp/");
        setup_git("./tests/cpp_vs_rust_test/rust/");
        CppVsRust {
            db: open_clean_db(Path::new("./tests/cpp_vs_rust_test/test.db")),
        }
    }

    fn gather_data(&self, path: &str, command: &mut std::process::Command) -> CommitHash {
        run_safe(command.current_dir(path), |output| {
            output.status.success() && output.stdout.is_empty() && output.stderr.is_empty()
        });

        CommitHash::new(Path::new(path), "HEAD".to_owned()).unwrap()
    }

    fn run(&self, command: &mut std::process::Command) {
        let hash_cpp = self.gather_data("./tests/cpp_vs_rust_test/cpp/", command);
        let hash_rust = self.gather_data("./tests/cpp_vs_rust_test/rust/", command);
        assert!(hash_cpp != hash_rust);

        let db_data = self.db.get_all_data().unwrap();

        let data_cpp = db_data
            .iter()
            .filter(|(params, _)| params.commit_hash == hash_cpp)
            .map(Clone::clone);
        check_data(&hash_cpp, data_cpp.collect());

        let data_rust = db_data
            .iter()
            .filter(|(params, _)| params.commit_hash == hash_rust)
            .map(Clone::clone);
        check_data(&hash_rust, data_rust.collect());
    }
}

// Compiling rust by two tests in parallel sometimes fails.
// We chose to serialize those two tests to avoid unpredictable test failures.

#[test]
#[file_serial]
fn simple() {
    let mut command = sdb_command();
    command
        .arg("-d")
        .arg("../test.db")
        .arg("run")
        .arg("-b")
        .arg("../config.yml")
        .arg("regex");

    CppVsRust::new().run(&mut command);
}

#[test]
#[file_serial]
fn aliasing_db() {
    let path = Path::new(file!()).parent().unwrap().canonicalize().unwrap();

    let db_path = path.join("cpp_vs_rust_test").join("test.db");
    let config_path = path.join("cpp_vs_rust_test").join("aliasing.yml");

    let mut config_file = File::create("./tests/cpp_vs_rust_test/aliasing.yml")
        .expect("Cannot create and write an aliasing file");
    config_file
        .write_all(format!("db-path: {db_path:?}\n").as_bytes())
        .unwrap();
    let mut command = sdb_command();

    command
        .env("SDB_CONFIG", config_path)
        .arg("run")
        .arg("-b")
        .arg("../config.yml")
        .arg("regex");

    CppVsRust::new().run(&mut command);
}

#[test]
fn flame_graph() {
    let test_dir = Path::new("./tests/flame_graph_bench_test/");
    let benchmark_name = "recurse";
    let config_name = Path::new("flame-path.yml");
    let db = open_clean_db(&test_dir.join("test.db"));

    print_flame_graph_information(&test_dir.join(config_name));

    run(sdb_command()
        .args([
            "-d",
            "./test.db",
            "-a",
            Path::new("./").join(config_name).to_str().unwrap(),
            "run",
            "-b",
            "./bench.yml",
            "-B",
            "./back.yml",
            benchmark_name,
            "flame-graph",
            "-s",
            "./store",
        ])
        .current_dir(test_dir));

    let param_builder = BenchmarkParamsBuilder::new(
        CommitHash::new(test_dir, "HEAD".to_owned()).unwrap(),
        benchmark_name.to_owned(),
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

#[test]
fn utility_test() {
    let path = Path::new(file!())
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join("utility_test/");

    let dp_path = path.join("db.db");
    let bc_path = path.join("bconfig.yml");
    let ac_path = path.join("alias.yml");

    write!(
        File::create(&ac_path).unwrap(),
        "db-path: {dp_path:?}\nbenchmark-config: {bc_path:?}"
    )
    .unwrap();

    setup_git(&path);
    let commit_hash = CommitHash::new(&path, "HEAD".to_owned()).unwrap();

    let dp = open_clean_db(&dp_path);

    run(sdb_command()
        .env("SDB_CONFIG", ac_path)
        .args(["run", "utility", "-b", "1,5,2", "command", "echo"])
        .current_dir("./tests/utility_test/"));

    let data = dp.get_all_data().unwrap();
    assert_eq!(
        data[0],
        (
            BenchmarkParams::new(
                commit_hash.clone(),
                "utility".to_owned(),
                1,
                "echo".to_owned()
            ),
            BenchmarkRecord::Data("sleep 1 >/dev/null".to_owned())
        ),
    );
    assert_eq!(
        data[1],
        (
            BenchmarkParams::new(
                commit_hash.clone(),
                "utility".to_owned(),
                2,
                "echo".to_owned()
            ),
            BenchmarkRecord::Data("sleep 2 >/dev/null".to_owned())
        )
    );
    assert_eq!(
        data[2],
        (
            BenchmarkParams::new(
                commit_hash.clone(),
                "utility".to_owned(),
                5,
                "echo".to_owned()
            ),
            BenchmarkRecord::Data("sleep 5 >/dev/null".to_owned())
        ),
    );
}
