use assert_cmd::cargo;
use scylladb_drivers_benchmarker::{
    commit_hash::CommitHash,
    database::{self},
    utilities::{BenchmarkParams, BenchmarkRecord},
};
use std::path::Path;

fn run_bin(args: &[&str]) -> String {
    let bin = cargo::cargo_bin!("scylladb-drivers-benchmarker");
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("Failed to run binary");

    if !output.status.success() {
        println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        panic!("Binary failed");
    }

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn database() {
    let db_path = "./tests/database_test/test.db";
    let db = database::Database::new(Path::new(db_path).to_owned()).unwrap();
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

    let print_output = run_bin(&["-d", db_path, "database", "print"]);

    assert!(print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(print_output.contains("commit3"));

    let print_output = run_bin(&[
        "-d",
        db_path,
        "database",
        "drop",
        "--commit-hash",
        "commit1:commit3",
    ]);

    assert_eq!(print_output, "");

    let print_output = run_bin(&["-d", db_path, "database", "print"]);

    assert!(!print_output.contains("commit1"));
    assert!(print_output.contains("commit2"));
    assert!(!print_output.contains("commit3"));
}
