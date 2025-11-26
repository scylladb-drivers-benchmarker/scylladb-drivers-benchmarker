use crate::commit_hash::CommitHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub commit_hash: CommitHash,
    pub benchmark_type: String,
    pub benchmark_argument: u64,
    pub measurement_method: String,
}

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
