use std::path::Path;

use assert_cmd::cargo;
use scylladb_drivers_benchmarker::{commit_hash::CommitHash, database, utilities::BenchmarkParams};

mod local;

#[test]
fn gather_data_cpp() {
    let db = database::Database::new(Path::new("./tests/data/cpp/test.db").to_owned()).unwrap();
    db.drop_table().unwrap();
    let benchmark_name = "regex";

    let mut command = std::process::Command::new(cargo::cargo_bin!("scylladb-drivers-benchmarker"));
    command
        .current_dir("./tests/data/cpp")
        .arg("-d")
        .arg("test.db")
        .arg("-b")
        .arg("../config.yml")
        .arg(benchmark_name)
        .arg("run");
    let output = command.output().unwrap();
    println!(
        "my_stdout: {}",
        String::from_utf8_lossy(output.stdout.as_slice())
    );
    if !output.status.success() {
        println!(
            "my_stderr: {}",
            String::from_utf8_lossy(output.stderr.as_slice())
        );
        assert!(output.status.success());
    }

    let db_data = db.get_all_data().unwrap();

    let commit_hash = CommitHash::from_current_repository().unwrap();

    let get_params = |point| {
        BenchmarkParams::new(
            commit_hash.clone(),
            "regex".to_owned(),
            point,
            "time".to_owned(),
        )
    };

    for (params, record) in &db_data {
        if *params != get_params(params.benchmark_point) {
            println!("{:?}", params);
            assert!(*params == get_params(params.benchmark_point));
        }
        if record.is_timeout() {
            assert!(!record.is_timeout());
        }
    }
    assert_eq!(db_data.len(), 4 as usize);
}
