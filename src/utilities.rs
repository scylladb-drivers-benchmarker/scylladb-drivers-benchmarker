use clap::ValueEnum;
use fs_err as fs;
use plotters::coord::types::RangedCoordu64;

use crate::commit_hash::CommitHash;
use crate::database::utilities::{BenchmarkParams, BenchmarkRecord};

pub type BenchmarkPoint = u64;
pub type RangedCoordBenchmarkPoint = RangedCoordu64;

/// Often `BenchmarkParams` params differ only by benchmarkPoint.
/// This struct makes it easier to create them.
pub struct BenchmarkParamsBuilder {
    pub commit_hash: CommitHash,
    pub benchmark_name: String,
    pub measurement_method: String,
}

impl BenchmarkParamsBuilder {
    #[must_use] 
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

    #[must_use] 
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
    pub fn flatten(self) -> Result<FlatBenchmarkRecord, std::io::Error> {
        Ok(match self {
            BenchmarkRecord::Data(s) => FlatBenchmarkRecord::Data(s),
            BenchmarkRecord::FilePath(path) => {
                FlatBenchmarkRecord::Data(fs::read_to_string(&path)?)
            }
            BenchmarkRecord::Timeout => FlatBenchmarkRecord::Timeout,
        })
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)] // TODO again rust idiomatic, input benchmark mode and real
pub enum BenchmarkMode {
    UseCached,
    ForceRerun,
}

#[must_use] 
pub fn format_entry(params: &BenchmarkParams, record: &BenchmarkRecord) -> String {
    let mut out = String::new();

    out.push_str("--- Benchmark Entry ---\n");
    out.push_str(&format!("Commit Hash:         {}\n", params.commit_hash));
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
            out.push_str(&format!("Data Content:        {s:?}\n"));
        }
        BenchmarkRecord::FilePath(path) => {
            out.push_str("Record Type:         FilePath\n");
            out.push_str(&format!("Data Content:        {path:?}\n"));
        }
        BenchmarkRecord::Timeout => {
            out.push_str("Record Type:         Timeout\n");
        }
    }

    out
}

/// Calculates the minimum and maximum over an iterator of tuples conforming to the (min, max) constraint.
pub fn calc_min_max(iter: impl Iterator<Item = (f64, f64)>) -> Option<(f64, f64)> {
    iter.fold(
        None,
        |acc: Option<(f64, f64)>, (min, max): (f64, f64)| match acc {
            Some((acc_min, acc_max)) => Some((acc_min.min(min), acc_max.max(max))),
            None => Some((min, max)),
        },
    )
}
