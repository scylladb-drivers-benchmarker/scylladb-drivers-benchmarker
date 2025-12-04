use assert_cmd::{Command, cargo};

mod local;

#[test]
fn basic() {
    let db = local::LocalDatabase::new();

    let mut command = std::process::Command::new(cargo::cargo_bin!("scylladb-drivers-benchmarker"));
    command
        .arg("-d")
        .arg(db.file.path().as_os_str())
        .arg("select")
        .arg("run");
    let output = command.output().unwrap();
    println!("my_stdout: {:?}", output.stdout);
    println!("my_stderr: {:?}", String::from_utf8_lossy(output.stderr.as_slice()));
    assert!(output.status.success());
}
