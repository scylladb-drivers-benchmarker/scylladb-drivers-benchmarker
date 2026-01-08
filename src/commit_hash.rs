use crate::cmd;
use crate::command::PrintableOutput;
use std::ffi::{OsStr, OsString};
use std::fmt::{Debug, Display, write};
use std::path::{Path, PathBuf};
use std::{env, process};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash {
    value: String,
}

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}

#[justerror::Error(desc = "Failed to retrieve commit hash")]
pub struct CommitHashError {
    command: String,
    repo: CommitHashErrorRepoPath,
    #[source]
    source: CommitHashErrorSource,
}

#[derive(Debug)]
pub struct CommitHashErrorRepoPath(Option<PathBuf>);
impl Display for CommitHashErrorRepoPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(path) => path.fmt(f),
            None => Ok(()),
        }
    }
}

#[justerror::Error]
pub enum CommitHashErrorSource {
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

impl CommitHash {
    fn validate(value: &str) -> bool {
        value.chars().all(|c| c.is_ascii_hexdigit()) && (value.len() == 40 || value.len() == 64)
    }

    pub fn new_unchecked(value: String) -> Self {
        CommitHash { value }
    }

    pub fn new(path: &Path, commit: String) -> Result<CommitHash, CommitHashError> {
        let mut command = cmd!("git", "rev-parse", "--verify", commit).process();
        Self::from_git_command(command.current_dir(path))
    }

    pub fn from_current_repository() -> Result<CommitHash, CommitHashError> {
        Self::from_git_command(&mut cmd!("git", "rev-parse", "--verify", "HEAD").process())
    }

    pub fn from_git_command(command: &mut process::Command) -> Result<CommitHash, CommitHashError> {
        Self::from_git_inner(command).map_err(|source| CommitHashError {
            command: format!(
                "{} {}",
                command.get_program().to_string_lossy(),
                command
                    .get_args()
                    .map(OsStr::to_string_lossy)
                    .collect::<String>()
            ),
            repo: CommitHashErrorRepoPath(command.get_current_dir().map(Path::to_owned)),
            source,
        })
    }
    fn from_git_inner(command: &mut process::Command) -> Result<CommitHash, CommitHashErrorSource> {
        let output = command.output()?;

        if !output.status.success() {
            return Err(CommitHashErrorSource::GitCommandFailure {
                output: output.into(),
            });
        }

        let mut value = String::from_utf8(output.stdout)?;
        value.truncate(value.trim_end().len()); // Remove endl

        // basic validation
        if !Self::validate(&value) {
            return Err(CommitHashErrorSource::InvalidHash { hash: value });
        }

        Ok(CommitHash { value })
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::commit_hash::{CommitHashError, CommitHashErrorSource};

    use super::CommitHash;

    #[test]
    fn coherent_commit_hashes() {
        let hash1 = CommitHash::new(Path::new("."), "HEAD".to_owned()).unwrap();
        let hash2 = CommitHash::from_current_repository().unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn erroneous_commit_hash() {
        let error = CommitHash::new(
            Path::new("."),
            "not a valid commit/branch/.. name".to_owned(),
        )
        .unwrap_err();

        assert!(matches!(
            error.source,
            CommitHashErrorSource::GitCommandFailure { .. }
        ))
    }
}
