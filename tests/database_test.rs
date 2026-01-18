use std::fs::File;
use std::io::Write;
use std::path::Path;

use fs_err::exists;
use scylladb_drivers_benchmarker::commit_hash::CommitHash;
use scylladb_drivers_benchmarker::database::utilities::{BenchmarkParams, BenchmarkRecord};
use scylladb_drivers_benchmarker::database::{self};
use scylladb_drivers_benchmarker::utilities::FlatBenchmarkRecord;
use tempfile::Builder;

mod utilities;
use utilities::run_utilities::{run_no_output, run_only_stdout, sdb_command};

fn test_dir() -> &'static Path {
    Path::new("./tests/database_test")
}

#[test]
fn database() {
    let db_file = Builder::new()
        .suffix(".db")
        .tempfile_in(test_dir())
        .unwrap();

    let db = database::Database::new(db_file.path()).unwrap();

    let commits = [
        CommitHash::new_unchecked("commit1".to_owned()),
        CommitHash::new_unchecked("commit2".to_owned()),
        CommitHash::new_unchecked("commit3".to_owned()),
    ];

    for (i, commit) in commits.iter().enumerate() {
        let params = BenchmarkParams::new(
            commit.clone(),
            "test-bench".to_owned(),
            i as u64,
            "time -f \"%e\"".to_owned(),
        );

        db.insert_data(params.clone(), BenchmarkRecord::Data("value".to_owned()))
            .unwrap();
    }

    let print_output = run_only_stdout(
        sdb_command()
            .arg("-d")
            .arg(db_file.path())
            .arg("database")
            .arg("print"),
    );

    assert!(print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(print_output.contains("commit3"));

    run_no_output(
        sdb_command()
            .arg("-d")
            .arg(db_file.path())
            .arg("database")
            .arg("drop")
            .arg("--commit-hash")
            .arg("commit1:commit3"),
    );

    let print_output = run_only_stdout(
        sdb_command()
            .arg("-d")
            .arg(db_file.path())
            .arg("database")
            .arg("print"),
    );
    assert!(!print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(!print_output.contains("commit3"));
}

#[test]
fn database_file() {
    let db_file = Builder::new()
        .suffix(".db")
        .tempfile_in(test_dir())
        .unwrap();

    let db = database::Database::new(db_file.path()).unwrap();

    // Create file with some data and insert it to database.
    let tmp_file_path = Path::new("foo.txt");
    let mut tmp_file = File::create(tmp_file_path).unwrap();
    let tmp_file_full_path = tmp_file_path.canonicalize().unwrap();

    let file_content = "hello from file";
    write!(tmp_file, "{}", file_content).unwrap();

    let commit = CommitHash::new_unchecked("commit-file".to_owned());

    let params = BenchmarkParams::new(
        commit.clone(),
        "file-bench".to_owned(),
        0,
        "time -f \"%e\"".to_owned(),
    );

    db.insert_data(
        params.clone(),
        BenchmarkRecord::FilePath(tmp_file_full_path.clone()),
    )
    .unwrap();

    {
        // Read from database, flatten and check content.

        let read_result = db
            .get_result(params.clone())
            .unwrap()
            .expect("Should get one result");

        let flat: FlatBenchmarkRecord = read_result.flatten().unwrap();

        match flat {
            FlatBenchmarkRecord::Data(s) => {
                assert_eq!(s, file_content, "Flattened data should match file content");
            }
            FlatBenchmarkRecord::Timeout => panic!("Should not be Timeout"),
        }
    }

    {
        // Integration test database print.
        let print_output = run_only_stdout(
            sdb_command()
                .arg("-d")
                .arg(db_file.path())
                .arg("database")
                .arg("print")
                .arg("--benchmark-name")
                .arg("file-bench")
                .arg("test"),
        );

        assert!(print_output.contains("FilePath"));
        assert!(
            print_output.contains(
                &tmp_file_path
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            )
        );
    }

    {
        // Integration test database drop.
        run_no_output(
            sdb_command()
                .arg("-d")
                .arg(db_file.path())
                .arg("database")
                .arg("drop")
                .arg("--commit-hash")
                .arg("commit-file")
                .arg(commit.to_string()),
        );

        assert!(!exists(tmp_file_full_path).unwrap());
    }
}
