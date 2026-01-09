use crate::{cmd, command};
use std::fmt::{Debug, Display};
use std::path::Path;
use std::process;

pub mod errors;
pub use errors::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash(String);

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl CommitHash {
    pub fn new_unchecked(value: String) -> Self {
        CommitHash(value)
    }

    pub fn new(path: &Path, commit: String) -> Result<CommitHash, Box<FailedToRetrieveCommitHash>> {
        let mut command = cmd!("git", "rev-parse", "--verify", commit).process();
        Self::from_git_command(command.current_dir(path))
    }

    pub fn from_current_repository() -> Result<CommitHash, Box<FailedToRetrieveCommitHash>> {
        Self::from_git_command(&mut cmd!("git", "rev-parse", "--verify", "HEAD").process())
    }

    fn validate(value: &str) -> bool {
        value.chars().all(|c| c.is_ascii_hexdigit()) && (value.len() == 40 || value.len() == 64)
    }

    // Attempts to get commit hash, returns error with context.
    fn from_git_command(
        command: &mut process::Command,
    ) -> Result<CommitHash, Box<FailedToRetrieveCommitHash>> {
        Self::from_git_inner(command).map_err(|source| {
            Box::new(FailedToRetrieveCommitHash {
                command: command::Command::from_command_lossy(command),
                repo_path: errors::RepoPath(command.get_current_dir().map(Path::to_owned)),
                source,
            })
        })
    }

    // Helper function, which attempts to get commit hash, returns lowest level error information.
    // Do not use directly, use from_git_command instead.
    fn from_git_inner(command: &mut process::Command) -> Result<CommitHash, ErrorSource> {
        let output = command.output()?;

        if !output.status.success() {
            return Err(ErrorSource::GitCommandFailure {
                output: output.into(),
            });
        }

        let mut value = String::from_utf8(output.stdout)?;
        value.pop(); // Remove endl

        if !Self::validate(&value) {
            // basic validation
            Err(ErrorSource::InvalidHash { hash: value })
        } else {
            Ok(CommitHash(value))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
