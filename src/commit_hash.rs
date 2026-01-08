use crate::cmd;
use crate::command::PrintableOutput;
use std::fmt::Display;
use std::path::Path;
use std::process;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash(String);

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct GitCommand(process::Command);

impl Display for GitCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[justerror::Error()]
pub enum CommitHashError {
    GitCommandConstructionFailed {
        source: std::io::Error,
        git_command: GitCommand,
    },
    GitCommandFailure {
        output: PrintableOutput,
        git_command: GitCommand,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidUtf8 {
        source: std::string::FromUtf8Error,
        git_command: GitCommand,
    },
    #[error(desc = "Git returned an invalid commit hash")]
    InvalidHash {
        hash: String,
        git_command: GitCommand,
    },
}

impl CommitHash {
    fn validate(value: &str) -> bool {
        value.chars().all(|c| c.is_ascii_hexdigit()) && (value.len() == 40 || value.len() == 64)
    }

    pub fn new_unchecked(value: String) -> Self {
        CommitHash(value)
    }

    pub fn new(path: &Path, commit: String) -> Result<CommitHash, CommitHashError> {
        Self::new_impl(Some(path), commit)
    }

    pub fn from_current_repository() -> Result<CommitHash, CommitHashError> {
        Self::new_impl(None, "HEAD".to_owned())
    }

    fn new_impl(path: Option<&Path>, commit: String) -> Result<CommitHash, CommitHashError> {
        let mut command = cmd!("git", "rev-parse", "--verify", commit).process();
        if let Some(path) = path {
            command.current_dir(path);
        }

        let mut git_command = GitCommand(command);
        let output = match git_command.0.output() {
            Ok(output) => output,
            Err(error) => {
                return Err(CommitHashError::GitCommandConstructionFailed {
                    source: error,
                    git_command,
                });
            }
        };

        if !output.status.success() {
            return Err(CommitHashError::GitCommandFailure {
                output: output.into(),
                git_command,
            });
        }

        let mut value = match String::from_utf8(output.stdout) {
            Ok(string) => string,
            Err(error) => {
                return Err(CommitHashError::InvalidUtf8 {
                    source: error,
                    git_command,
                });
            }
        };
        value.truncate(value.trim_end().len()); // Remove endl

        // basic validation
        if !Self::validate(&value) {
            return Err(CommitHashError::InvalidHash {
                hash: value,
                git_command,
            });
        }

        Ok(CommitHash(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::commit_hash::CommitHashError;

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

        let CommitHashError::GitCommandFailure { .. } = error else {
            panic!("git should have failed on invalid commit/branch/.. name");
        };
    }
}
