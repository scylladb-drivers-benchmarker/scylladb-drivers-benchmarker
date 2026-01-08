use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;
use std::path::PathBuf;

use plotters::coord::types::RangedCoordu64;

use crate::commit_hash::CommitHash;
use crate::database::utilities::{BenchmarkFilters, BenchmarkParams, BenchmarkRecord};

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
    #[arg(long = "commit-hash", value_delimiter = ':', num_args(1..))]
    pub commit_hashes: Vec<String>,

    #[arg(long = "benchmark-name", value_delimiter = ':', num_args(1..))]
    pub benchmark_names: Vec<String>,

    #[arg(long = "benchmark-point", value_delimiter = ':', num_args(1..))]
    pub benchmark_points: Vec<BenchmarkPoint>,

    #[arg(long = "measurement-method", value_delimiter = ':', num_args(1..))]
    pub measurement_methods: Vec<String>,
}

impl From<InputDatabaseFilters> for BenchmarkFilters {
    fn from(input: InputDatabaseFilters) -> Self {
        BenchmarkFilters {
            commit_hashes: input.commit_hashes,
            benchmark_names: input.benchmark_names,
            benchmark_points: input.benchmark_points,
            measurement_methods: input.measurement_methods,
        }
    }
}

/// Often BenchmarkParams params differ only by benchmarkPoint.
/// This struct makes it easier to create them.
pub struct BenchmarkParamsBuilder {
    pub commit_hash: CommitHash,
    pub benchmark_name: String,
    pub measurement_method: String,
}

impl BenchmarkParamsBuilder {
    pub fn new(
        commit_hash: CommitHash,
        benchmark_name: String,
        measurement_method: String,
    ) -> Self {
        BenchmarkParamsBuilder {
            commit_hash,
            benchmark_name,
            measurement_method,
        }
    }

    pub fn finalize(&self, benchmark_point: BenchmarkPoint) -> BenchmarkParams {
        BenchmarkParams::new(
            self.commit_hash.clone(),
            self.benchmark_name.clone(),
            benchmark_point,
            self.measurement_method.clone(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatBenchmarkRecord {
    Data(String),
    Timeout,
}

impl BenchmarkRecord {
    pub fn flatten(self) -> FlatBenchmarkRecord {
        match self {
            BenchmarkRecord::Data(s) => FlatBenchmarkRecord::Data(s),
            BenchmarkRecord::FilePath(path) => {
                let s = std::fs::read_to_string(&path)
                    .unwrap_or_else(|_| format!("Failed to read file: {}", path));
                FlatBenchmarkRecord::Data(s)
            }
            BenchmarkRecord::Timeout => FlatBenchmarkRecord::Timeout,
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum BenchmarkMode {
    UseCached,
    ForceRerun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoPathWithCommits {
    pub repo_path: PathBuf,
    pub git_hashes: Vec<CommitHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoNameWithTags {
    pub name: String,
    pub tags: Vec<String>,
}

pub fn format_entry(params: &BenchmarkParams, record: &BenchmarkRecord) -> String {
    let mut out = String::new();

    out.push_str("--- Benchmark Entry ---\n");
    out.push_str(&format!(
        "Commit Hash:         {}\n",
        params.commit_hash.to_string()
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
        BenchmarkRecord::FilePath(path) => {
            out.push_str("Record Type:         FilePath\n");
            out.push_str(&format!("Data Content:        {:?}\n", path));
        }
        BenchmarkRecord::Timeout => {
            out.push_str("Record Type:         Timeout\n");
        }
    }

    out
}
