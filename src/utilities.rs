use clap::Parser;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use crate::commit_hash::CommitHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub commit_hash: CommitHash,
    pub benchmark_type: String,
    pub benchmark_argument: u64,
    pub measurement_method: String,
}

pub type BenchmarkResult = Result<BenchmarkRecord, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkRecord {
    pub data_json: Option<String>,
}

impl BenchmarkParams {
    pub fn new(
        commit_hash: CommitHash,
        benchmark_type: String,
        benchmark_argument: u64,
        measurement_method: String,
    ) -> BenchmarkParams {
        BenchmarkParams {
            commit_hash,
            benchmark_type,
            benchmark_argument,
            measurement_method,
        }
    }
}

impl BenchmarkRecord {
    pub fn new(data_json: Option<String>) -> BenchmarkRecord {
        BenchmarkRecord { data_json }
    }

    pub fn is_timeout(&self) -> bool {
        self.data_json.is_none()
    }
}

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWithCommits {
    pub repo_path: PathBuf,
    pub commits: Vec<String>,
}

impl FromStr for RepositoryWithCommits {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut parts = string.split(':');
        let repo_path = parts.next().expect("repo path not supplied"); // TODO: fix

        let commits: Vec<String> = parts.map(str::to_owned).collect();

        Ok(RepositoryWithCommits {
            repo_path: PathBuf::from_str(repo_path)?,
            commits,
        })
    }
}

impl RepositoryWithCommits {
    fn to_commit_hashes(self) -> Vec<CommitHash> {
        self.commits
            .into_iter()
            .map(|x| CommitHash::new(&self.repo_path, x))
            .collect()
    }
}
