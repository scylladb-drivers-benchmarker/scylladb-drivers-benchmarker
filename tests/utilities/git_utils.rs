use std::path::Path;

use crate::utilities::run_utilities::run;

pub fn setup_git(repo: &str) {
    if Path::new(&(repo.to_owned() + "/.git/")).is_dir() {
        return;
    }

    run(std::process::Command::new("git")
        .current_dir(repo)
        .arg("init"));
    run(std::process::Command::new("git")
        .current_dir(repo)
        .arg("add")
        .arg("-A"));
    run(std::process::Command::new("git")
        .current_dir(repo)
        .arg("commit")
        .arg("-m")
        .arg("\"initial\""));
}
