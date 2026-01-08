use std::{fmt::Display, path::PathBuf};

use crate::command::{self, PrintableOutput};

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

#[justerror::Error(desc = "Failed to retrieve commit hash")]
pub struct Error {
    pub command: command::Command,
    pub repo_path: RepoPath,
    #[source]
    pub source: ErrorSource,
}

#[justerror::Error]
pub enum ErrorSource {
    IO {
        #[from]
        source: std::io::Error,
    },
    GitCommandFailure {
        output: PrintableOutput,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidUtf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidHash {
        hash: String,
    },
}
