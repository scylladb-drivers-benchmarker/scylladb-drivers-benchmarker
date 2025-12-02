use assert_cmd::cargo;

mod local;

#[test]
fn basic() {
    let db = local::LocalDatabase::new();

    assert_cmd::Command::cargo_bin("scylladb-drivers-benchmarker")
        .unwrap()
        .args([
            "-d",
            db.file
                .path()
                .as_os_str()
                .to_str()
                .expect("database file should exist"),
            "select",
            "run",
        ])
        .assert()
        .success();
}
