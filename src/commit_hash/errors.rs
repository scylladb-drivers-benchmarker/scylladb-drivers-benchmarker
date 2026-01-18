use std::fmt::Display;
use std::path::PathBuf;

use crate::command::{self, PrintableOutput};

#[justerror::Error(desc = "Failed to retrieve commit hash")]
pub struct FailedToRetrieveCommitHash {
    pub command: command::Command,
    pub repo_path: RepoPath,
    #[source]
    pub source: ErrorSource,
}

#[derive(Debug)]
pub struct RepoPath(pub Option<PathBuf>);

impl Display for RepoPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(path) => path.display().fmt(f),
            None => write!(f, "unspecified(current)"),
        }
    }
}

#[justerror::Error]
pub enum ErrorSource {
    #[error(desc = "Failed constructing the git command")]
    GitCommandConstructionFailure {
        #[from]
        source: std::io::Error,
    },
    #[error(desc = "Failed running the git command")]
    GitCommandFailure { output: PrintableOutput },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidUtf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidHash { hash: String },
}
