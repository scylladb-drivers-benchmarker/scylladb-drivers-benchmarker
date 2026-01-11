use scylladb_drivers_benchmarker::{
    commit_hash::CommitHash,
    database::utilities::{BenchmarkParams, BenchmarkRecord},
    database::{self},
    utilities::FlatBenchmarkRecord,
};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

mod utilities;
use utilities::run_utilities::{run_no_output, run_only_stdout, sdb_command};

#[test]
fn database() {
    let db_path = "./tests/database_test/test.db";
    let db = database::Database::new(Path::new(db_path)).unwrap();
    db.drop_all_data().unwrap();

    let commits = [
        CommitHash::new_unchecked("commit1".to_string()),
        CommitHash::new_unchecked("commit2".to_string()),
        CommitHash::new_unchecked("commit3".to_string()),
    ];

    for (i, commit) in commits.iter().enumerate() {
        let params = BenchmarkParams::new(
            commit.clone(),
            "test-bench".to_string(),
            i as u64,
            "time -f \"%e\"".to_string(),
        );

        db.insert_data(params.clone(), BenchmarkRecord::Data("value".to_string()))
            .unwrap();
    }

    let print_output = run_only_stdout(
        sdb_command()
            .arg("-d")
            .arg(db_path)
            .arg("database")
            .arg("print"),
    );

    assert!(print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(print_output.contains("commit3"));

    run_no_output(
        sdb_command()
            .arg("-d")
            .arg(db_path)
            .arg("database")
            .arg("drop")
            .arg("--commit-hash")
            .arg("commit1:commit3"),
    );

    let print_output = run_only_stdout(
        sdb_command()
            .arg("-d")
            .arg(db_path)
            .arg("database")
            .arg("print"),
    );
    assert!(!print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(!print_output.contains("commit3"));
}

#[test]
fn database_file() {
    let db_path = "./tests/database_test/tmp_file_test.db";
    let db = database::Database::new(Path::new(db_path)).unwrap();
    db.drop_all_data().unwrap();

    // Create temporary file with some data and insert it to database.
    let mut tmp_file = NamedTempFile::new().unwrap();
    let file_content = "hello from file";
    write!(tmp_file, "{}", file_content).unwrap();

    let commit = CommitHash::new_unchecked("commit-file".to_string());

    let params = BenchmarkParams::new(
        commit.clone(),
        "file-bench".to_string(),
        0,
        "time -f \"%e\"".to_string(),
    );

    db.insert_data(
        params.clone(),
        BenchmarkRecord::FilePath(tmp_file.path().to_owned().to_string_lossy().to_string()),
    )
    .unwrap();

    {
        // Read from database, flatten and check content.

        let read_result = db
            .get_result(params.clone())
            .unwrap()
            .expect("Should get one result");

        let flat: FlatBenchmarkRecord = read_result.flatten();

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
                .arg(db_path)
                .arg("database")
                .arg("print")
                .arg("--benchmark-name")
                .arg("file-bench")
                .arg("test"),
        );

        assert!(print_output.contains("FilePath"));
        assert!(print_output.contains(&tmp_file.path().to_owned().to_string_lossy().to_string()));
    }
}
