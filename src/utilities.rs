use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

use plotters::coord::types::RangedCoordu64;

use crate::commit_hash::{CommitHash, CommitHashError};

pub type BenchmarkPoint = u64;
pub type RangedCoordBenchmarkPoint = RangedCoordu64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub commit_hash: CommitHash,
    pub benchmark_name: String,
    pub benchmark_point: BenchmarkPoint,
    pub measurement_method: String,
}

impl BenchmarkParams {
    pub fn new(
        commit_hash: CommitHash,
        benchmark_name: String,
        benchmark_point: BenchmarkPoint,
        measurement_method: String,
    ) -> BenchmarkParams {
        BenchmarkParams {
            commit_hash,
            benchmark_name,
            benchmark_point,
            measurement_method,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkRecord {
    Data(String),
    Timeout,
}

impl BenchmarkRecord {
    pub fn is_timeout(&self) -> bool {
        matches!(self, BenchmarkRecord::Timeout)
    }
}

impl From<Option<String>> for BenchmarkRecord {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(s) => BenchmarkRecord::Data(s),
            None => BenchmarkRecord::Timeout,
        }
    }
}

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWithCommits {
    pub repo_path: PathBuf,
    pub commits: Vec<String>,
}

#[justerror::Error]
pub enum RepositoryWithCommitsParsingError {
    PathNotSupplied,
    Infallible(#[from] std::convert::Infallible),
}

impl FromStr for RepositoryWithCommits {
    type Err = RepositoryWithCommitsParsingError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (repo_path_str, commits_str) = string
            .split_once(':')
            .ok_or(RepositoryWithCommitsParsingError::PathNotSupplied)?;

        Ok(RepositoryWithCommits {
            repo_path: PathBuf::from_str(repo_path_str)?,
            commits: commits_str.split(',').map(str::to_owned).collect(),
        })
    }
}

impl RepositoryWithCommits {
    pub fn to_commit_hashes(self) -> Result<Vec<CommitHash>, CommitHashError> {
        self.commits
            .into_iter()
            .map(|commit| CommitHash::new(&self.repo_path, commit))
            .collect()
    }
}
