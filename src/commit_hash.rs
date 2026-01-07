use crate::cmd;
use std::env;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitHash {
    value: String,
}

impl From<CommitHash> for String {
    /// Converts a commit hash to a string
    /// ```
    /// # use scylladb_drivers_benchmarker::commit_hash::CommitHash;
    /// # use std::convert::From;
    /// let commit_hash = CommitHash::from_current_repository().unwrap();
    /// let stringified: String = String::from(commit_hash);
    ///
    /// println!("{}", stringified);
    /// ```
    fn from(value: CommitHash) -> Self {
        value.value
    }
}

#[justerror::Error(desc = "Failed to retrieve commit hash")]
pub enum CommitHashError {
    IO {
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

        let CommitHashError::GitCommandFailure(_) = error else {
            panic!("git should have failed on invalid commit/branch/.. name");
        };
    }
}
