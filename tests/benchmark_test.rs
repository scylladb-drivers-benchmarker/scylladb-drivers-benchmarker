use std::{env, fs::File, io::Write, path::Path};

use assert_cmd::cargo;
use scylladb_drivers_benchmarker::{
    commit_hash::CommitHash,
    database::{self, Database},
    utilities::{BenchmarkParams, BenchmarkRecord},
};
use serial_test::serial;

fn setup_git(repo: &str) {
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
    for (params, _record) in data {
        cnt += 1;
        if params != get_params(params.benchmark_point) {
            println!("{:?}", params);
            assert!(params == get_params(params.benchmark_point));
        }
    }
    assert_eq!(cnt, 8usize);
}

struct CppVsRust {
    db: Database,
}

impl CppVsRust {
    fn new() -> Self {
        setup_git("./tests/data/cpp/");
        setup_git("./tests/data/rust/");

        let db = database::Database::new(Path::new("./tests/data/test.db").to_owned()).unwrap();
        db.drop_all_data().unwrap();

        CppVsRust { db }
    }

    fn gather_data(&self, path: &str, command: &mut std::process::Command) -> CommitHash {
        let output = command.current_dir(path).output().unwrap();
        if !output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty()  {
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
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
        CommitHash::new(Path::new(path), "HEAD".to_owned()).unwrap()
    }

    fn run(&self, command: &mut std::process::Command) {
        let hash_cpp = self.gather_data("./tests/data/cpp/", command);
        let hash_rust = self.gather_data("./tests/data/rust/", command);
        assert!(hash_cpp != hash_rust);

        let db_data = self.db.get_all_data().unwrap();

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
}

fn sdb_command() -> std::process::Command {
    std::process::Command::new(cargo::cargo_bin!("scylladb-drivers-benchmarker"))
}

#[test]
#[serial]
fn simple() {
    let mut command = sdb_command();
    command
        .arg("-d")
        .arg("../test.db")
        .arg("-b")
        .arg("../config.yml")
        .arg("regex")
        .arg("run");

    CppVsRust::new().run(&mut command);
}

#[test]
#[serial]
fn aliasing_db() {
    let path = Path::new(file!()).parent().unwrap().canonicalize().unwrap();

    let dp_path = path.join("data").join("test.db");
    let config_path = path.join("data").join("aliasing.yml");

    let mut config_file = File::create("./tests/data/aliasing.yml")
        .expect("Cannot create and write an aliasing file");
    config_file
        .write_all(format!("dp-path: {dp_path:?}\n").as_bytes())
        .unwrap();
    let mut command = sdb_command();
    command
        .env("SDB_CONFIG", config_path)
        .arg("-b")
        .arg("../config.yml")
        .arg("regex")
        .arg("run");

    CppVsRust::new().run(&mut command);
}
