use std::path::Path;

use assert_cmd::cargo;
use scylladb_drivers_benchmarker::{
    commit_hash::{self, CommitHash},
    database::{self, test_utils::*},
    utilities::{BenchmarkParams, BenchmarkRecord},
};

fn setup_git(repo: &str) {
    println!("{}", repo);

    if Path::new(&(repo.to_owned() + "/.git/")).is_dir() {
        return;
    }
    
    assert!(
        std::process::Command::new("git")
            .current_dir(repo)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .current_dir(repo)
            .arg("add")
            .arg("-A")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .current_dir(repo)
            .arg("commit")
            .arg("-m")
            .arg("\"initial\"")
            .output()
            .unwrap()
            .status
            .success()
    );
}

fn gather_data(path: &str) -> CommitHash {
    let mut command = std::process::Command::new(cargo::cargo_bin!("scylladb-drivers-benchmarker"));
    command
        .current_dir(path)
        .arg("-d")
        .arg("../test.db")
        .arg("-b")
        .arg("../config.yml")
        .arg("regex")
        .arg("run");
    let output = command.output().unwrap();
    if !output.status.success() {
        println!(
            "my_stdout: {}",
            String::from_utf8_lossy(output.stdout.as_slice())
        );
        println!(
            "my_stderr: {}",
            String::from_utf8_lossy(output.stderr.as_slice())
        );
        println!("{}", path);
        assert!(output.status.success());
    }
    let commit_hash = CommitHash::new(Path::new(path), "HEAD".to_owned()).unwrap();
    commit_hash
}

fn check_data(
    commit_hash: &CommitHash,
    data: impl Iterator<Item = (BenchmarkParams, BenchmarkRecord)>,
) {
    let get_params = |point| {
        BenchmarkParams::new(
            commit_hash.clone(),
            "regex".to_owned(),
            point,
            "time -f \"%e\"".to_owned(),
        )
    };

    let mut cnt = 0;
    for (params, record) in data {
        cnt += 1;
        if params != get_params(params.benchmark_point) {
            println!("{:?}", params);
            assert!(params == get_params(params.benchmark_point));
        }

        assert!(!matches!(record, BenchmarkRecord::Timeout));
    }
    assert_eq!(cnt, 8usize);
}

#[test]
fn test_cpp_vs_rust() {
    setup_git("./tests/data/cpp/");
    setup_git("./tests/data/rust/");

    let db = database::Database::new(Path::new("./tests/data/test.db").to_owned()).unwrap();
    drop_table(&db).unwrap();

    let hash_cpp = gather_data("./tests/data/cpp/");
    let hash_rust = gather_data("./tests/data/rust/");
    assert!(hash_cpp != hash_rust);

    let db_data = get_all_data(&db).unwrap();

    let data_cpp = db_data
        .iter()
        .filter(|(params, _)| params.commit_hash == hash_cpp)
        .map(Clone::clone);
    check_data(&hash_cpp, data_cpp);

    let data_rust = db_data
        .iter()
        .filter(|(params, _)| params.commit_hash == hash_rust)
        .map(Clone::clone);
    check_data(&hash_rust, data_rust);

}
