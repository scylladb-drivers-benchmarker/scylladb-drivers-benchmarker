use std::path::Path;

use crate::utilities::run_utilities::run;

pub fn setup_git(repo: impl AsRef<Path>) {
    if Path::new(&repo.as_ref().join(".git/")).is_dir() {
        return;
    }

    run(std::process::Command::new("git")
        .current_dir(&repo)
        .arg("init"));
    run(std::process::Command::new("git")
        .current_dir(&repo)
        .arg("add")
        .arg("-A"));
    run(std::process::Command::new("git")
        .current_dir(&repo)
        .arg("commit")
        .arg("-m")
        .arg("\"initial\""));
}
