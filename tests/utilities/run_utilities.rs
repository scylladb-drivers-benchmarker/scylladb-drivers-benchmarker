use std::process::Output;

use assert_cmd::cargo;
use scylladb_drivers_benchmarker::command;

pub fn sdb_command() -> std::process::Command {
    std::process::Command::new(cargo::cargo_bin!("scylladb-drivers-benchmarker"))
}

pub fn run(cmd: &mut std::process::Command) -> Output {
    run_safe(cmd, |output: &Output| output.status.success())
}

pub fn run_only_stdout(cmd: &mut std::process::Command) -> String {
    let output = run(cmd);
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

pub fn run_no_output(cmd: &mut std::process::Command) {
    assert!(run_only_stdout(cmd).is_empty())
}

pub fn run_safe(cmd: &mut std::process::Command, verifier: impl Fn(&Output) -> bool) -> Output {
    let wrong_output = format!(
        "Command: \"{}\" failed",
        command::Command::from_command_lossy(cmd)
    );

    let output = cmd.output().expect(&wrong_output);
    if !verifier(&output) {
        println!("{}", wrong_output);
        println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        panic!("failed");
    }
    output
}
