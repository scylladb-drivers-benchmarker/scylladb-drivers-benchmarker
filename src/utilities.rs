use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use std::path::PathBuf;
use std::str::FromStr;

use plotters::coord::types::RangedCoordu64;

use crate::commit_hash::{CommitHash, CommitHashError};

pub type BenchmarkPoint = u64;
pub type RangedCoordBenchmarkPoint = RangedCoordu64;

#[derive(Subcommand, Debug)]
pub enum DatabaseCommand {
    Print {
        #[command(flatten)]
        filters: InputDatabaseFilters,
    },

    Drop {
        #[command(flatten)]
        filters: InputDatabaseFilters,
    },
}

#[derive(Args, Debug)]
pub struct InputDatabaseFilters {
    #[arg(long = "commit-hash", value_parser = parse_list::<String>)]
    pub commit_hashes: Vec<String>,

    #[arg(long = "benchmark-name", value_parser = parse_list::<String>)]
    pub benchmark_names: Vec<String>,

    #[arg(long = "benchmark-point", value_parser = parse_list::<BenchmarkPoint>)]
    pub benchmark_points: Vec<BenchmarkPoint>,

    #[arg(long = "measurement-method", value_parser = parse_list::<String>)]
    pub measurement_methods: Vec<String>,
}

fn parse_list<T>(s: &str) -> Result<Vec<T>, String>
where
    T: FromStr,
    <T as FromStr>::Err: ToString,
{
    s.split(':')
        .map(|v| v.parse::<T>().map_err(|e| e.to_string()))
        .collect()
}

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

impl From<Option<String>> for BenchmarkRecord {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(s) => BenchmarkRecord::Data(s),
            None => BenchmarkRecord::Timeout,
        }
    }
}

pub struct BenchmarkFilters {
    pub commit_hashes: Vec<String>,
    pub benchmark_names: Vec<String>,
    pub benchmark_points: Vec<BenchmarkPoint>,
    pub measurement_methods: Vec<String>,
}

impl BenchmarkFilters {
    pub fn all() -> Self {
        BenchmarkFilters {
            commit_hashes: Vec::new(),
            benchmark_names: Vec::new(),
            benchmark_points: Vec::new(),
            measurement_methods: Vec::new(),
        }
    }

    pub fn filter_exact_param(params: &BenchmarkParams) -> Self {
        BenchmarkFilters {
            commit_hashes: vec![String::from(params.commit_hash.clone())],
            benchmark_names: vec![params.benchmark_name.clone()],
            benchmark_points: vec![params.benchmark_point],
            measurement_methods: vec![params.measurement_method.clone()],
        }
    }

    pub fn from_input_commands(filters: InputDatabaseFilters) -> Self {
        BenchmarkFilters {
            commit_hashes: filters.commit_hashes,
            benchmark_names: filters.benchmark_names,
            benchmark_points: filters.benchmark_points,
            measurement_methods: filters.measurement_methods,
        }
    }
}

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWithCommits {
    pub repo_path: PathBuf,
    pub commits: Vec<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum BenchmarkMode {
    UseCached,
    ForceRerun,
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

pub fn format_entry(params: &BenchmarkParams, record: &BenchmarkRecord) -> String {
    let mut out = String::new();

    out.push_str("--- Benchmark Entry ---\n");
    out.push_str(&format!(
        "Commit Hash:         {}\n",
        String::from(params.commit_hash.clone())
    ));
    out.push_str(&format!("Benchmark Name:      {}\n", params.benchmark_name));
    out.push_str(&format!(
        "Benchmark Point:     {}\n",
        params.benchmark_point
    ));
    out.push_str(&format!(
        "Measurement Method:  {}\n",
        params.measurement_method
    ));

    match record {
        BenchmarkRecord::Data(s) => {
            out.push_str("Record Type:         Data\n");
            out.push_str(&format!("Data Content:        {:?}\n", s));
        }
        BenchmarkRecord::Timeout => {
            out.push_str("Record Type:         Timeout\n");
        }
    }

    out
}
