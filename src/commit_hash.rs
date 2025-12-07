use crate::cmd;
use crate::command::Command;
use std::env;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash {
    value: String,
}

impl From<CommitHash> for String {
    fn from(value: CommitHash) -> Self {
        value.value
    }
}

#[justerror::Error(desc = "Failed to retrieve commit hash")]
pub enum CommitHashError {
    IoError {
        #[from]
        source: std::io::Error,
    },
    GitCommandFailure(String),
    InvalidUtf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidHash {
        hash: String,
    },
}

impl CommitHash {
    fn validate(value: &str) -> bool {
        value.chars().all(|c| c.is_ascii_hexdigit()) && (value.len() == 40 || value.len() == 64)
    }

    pub fn new_unchecked(value: String) -> Self {
        CommitHash { value }
    }

    pub fn new(path: &Path, commit: String) -> Result<CommitHash, CommitHashError> {
        let output = cmd!("git", "rev-parse", "--verify", &commit)
            .process()
            .current_dir(path)
            .output()?;

        if !output.status.success() {
            return Err(CommitHashError::GitCommandFailure(format!(
                "git rev-parse failed for '{}': {}",
                commit,
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let mut value = String::from_utf8(output.stdout)?;
        value.truncate(value.trim_end().len()); // Remove endl

        // basic validation
        if !Self::validate(&value) {
            return Err(CommitHashError::InvalidHash { hash: value });
        }

        Ok(CommitHash { value })
    }

    pub fn from_current_repository() -> Result<CommitHash, CommitHashError> {
        CommitHash::new(&env::current_dir()?, "HEAD".to_string())
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn value(&self) -> &String {
        &self.value
    }
}
