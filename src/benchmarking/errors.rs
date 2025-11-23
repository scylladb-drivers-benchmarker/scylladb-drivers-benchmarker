use std::error::Error;
use std::fmt;
use std::io::Stderr;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

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

#[derive(Debug)]
pub struct WrongCommitHash {
    pub got: String,
    pub reason: String,
}

impl fmt::Display for WrongCommitHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not a valid commit hash: {}, because: {}", self.got, self.reason)
    }
}

impl Error for WrongCommitHash {}
