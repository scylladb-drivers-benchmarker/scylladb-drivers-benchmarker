use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::CommitHash;
use crate::utilities::BenchmarkPoint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub commit_hash: CommitHash,
    pub benchmark_name: String,
    pub benchmark_point: BenchmarkPoint,
    pub measurement_method: String,
}

impl BenchmarkParams {
    #[must_use]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum BenchmarkRecord {
    Data(String),
    FilePath(PathBuf),
    Timeout,
}

impl BenchmarkRecord {
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self, BenchmarkRecord::Timeout)
    }
}

pub struct BenchmarkFilters {
    pub commit_hashes: Vec<String>,
    pub benchmark_names: Vec<String>,
    pub benchmark_points: Vec<BenchmarkPoint>,
    pub measurement_methods: Vec<String>,
}

impl BenchmarkFilters {
    #[must_use]
    pub fn all() -> Self {
        BenchmarkFilters {
            commit_hashes: Vec::new(),
            benchmark_names: Vec::new(),
            benchmark_points: Vec::new(),
            measurement_methods: Vec::new(),
        }
    }

    #[must_use]
    pub fn filter_exact_param(params: &BenchmarkParams) -> Self {
        BenchmarkFilters {
            commit_hashes: vec![params.commit_hash.clone().to_string()],
            benchmark_names: vec![params.benchmark_name.clone()],
            benchmark_points: vec![params.benchmark_point],
            measurement_methods: vec![params.measurement_method.clone()],
        }
    }
}
