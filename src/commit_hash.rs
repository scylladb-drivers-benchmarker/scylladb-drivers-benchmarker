
use crate::cmd;
use crate::command::Command;
use std::error::Error;
use std::fmt;
use std::process::ExitStatus;

#[derive(Debug)]
pub struct GitFailed {
    pub status: ExitStatus,
    pub stderr: Vec<u8>,
}

impl fmt::Display for GitFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stderr = String::from_utf8_lossy(&self.stderr);

        write!(
            f,
            "Git command failed with {}.\nStderr: {}",
            self.status,
            stderr.trim() // Trim whitespace for cleaner output
        )
    }
}

impl Error for GitFailed {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash {
    value: String,
}

impl From<CommitHash> for String {
    fn from(value: CommitHash) -> Self {
        value.value
    }
}

impl CommitHash {
    pub fn new_unchecked(value: String) -> Self {
        CommitHash { value }
    }

    pub fn from_repository_relative(relative: i32) -> Result<CommitHash, Box<dyn Error>> {
        let git_get_hash = cmd!(
            "git",
            "rev-parse",
            "--verify",
            "HEAD~".to_owned() + relative.to_string().as_str()
        );
        let output = git_get_hash.process().output()?;
        if !output.status.success() {
            return Err(Box::new(GitFailed {
                status: output.status,
                stderr: output.stderr,
            }));
        }

        let value = String::from_utf8(output.stdout)?;
        for char in value.chars() {
            assert!(
                char.is_lowercase() || char.is_ascii_digit(),
                "Got an incorrect git hash from a passing git process. Character: {} is not a number or a lowercase letter.",
                char
            );
        }

        Ok(CommitHash { value })
    }

    pub fn from_repository() -> Result<CommitHash, Box<dyn Error>> {
        Self::from_repository_relative(0)
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    } 

    pub fn value(&self) -> &String {
        &self.value
    }
}
