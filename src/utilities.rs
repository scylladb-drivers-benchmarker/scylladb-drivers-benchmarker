use clap::Parser;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use plotters::coord::types::RangedCoordu64;

use crate::commit_hash::CommitHash;

pub type BenchmarkPoint = u64;
pub type RangedCoordBenchmarkPoint = RangedCoordu64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub commit_hash: CommitHash,
    pub benchmark_name: String,
    pub benchmark_point: BenchmarkPoint,
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
    pub fn to_commit_hashes(self) -> Vec<CommitHash> {
        self.commits
            .into_iter()
            .map(|x| CommitHash::new(&self.repo_path, x).unwrap())
            .collect()
    }
}
