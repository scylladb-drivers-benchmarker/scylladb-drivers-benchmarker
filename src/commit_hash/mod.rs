use crate::cmd;
use crate::command::PrintableOutput;
use std::ffi::{OsStr, OsString};
use std::fmt::{Debug, Display, write};
use std::path::{Path, PathBuf};
use std::{env, process};

pub mod errors;
pub use errors::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash {
    value: String,
}

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}


impl CommitHash {
    fn validate(value: &str) -> bool {
        value.chars().all(|c| c.is_ascii_hexdigit()) && (value.len() == 40 || value.len() == 64)
    }

    pub fn new_unchecked(value: String) -> Self {
        CommitHash { value }
    }

    pub fn new(path: &Path, commit: String) -> Result<CommitHash, Error> {
        let mut command = cmd!("git", "rev-parse", "--verify", commit).process();
        Self::from_git_command(command.current_dir(path))
    }

    pub fn from_current_repository() -> Result<CommitHash, Error> {
        Self::from_git_command(&mut cmd!("git", "rev-parse", "--verify", "HEAD").process())
    }

    pub fn from_git_command(command: &mut process::Command) -> Result<CommitHash, Error> {
        Self::from_git_inner(command).map_err(|source| Error {
            command: format!(
                "{} {}",
                command.get_program().to_string_lossy(),
                command
                    .get_args()
                    .map(OsStr::to_string_lossy)
                    .collect::<String>()
            ),
            repo: errors::RepoPath(command.get_current_dir().map(Path::to_owned)),
            source,
        })
    }
    fn from_git_inner(command: &mut process::Command) -> Result<CommitHash, ErrorSource> {
        let output = command.output()?;

        if !output.status.success() {
            return Err(ErrorSource::GitCommandFailure {
                output: output.into(),
            });
        }

        let mut value = String::from_utf8(output.stdout)?;
        value.truncate(value.trim_end().len()); // Remove endl

        // basic validation
        if !Self::validate(&value) {
            return Err(ErrorSource::InvalidHash { hash: value });
        }

        Ok(CommitHash { value })
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
            ErrorSource::GitCommandFailure { .. }
        ))
    }
}
