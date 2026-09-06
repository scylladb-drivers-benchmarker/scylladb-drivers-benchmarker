use std::fmt::Write;

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
    pub driver_name: String,
    pub measurement_method: String,
}

impl BenchmarkParamsBuilder {
    #[must_use]
    pub fn new(
        commit_hash: CommitHash,
        benchmark_name: String,
        driver_name: String,
        measurement_method: String,
    ) -> Self {
        BenchmarkParamsBuilder {
            commit_hash,
            benchmark_name,
            driver_name,
            measurement_method,
        }
    }

    #[must_use]
    pub fn finalize(&self, benchmark_point: BenchmarkPoint) -> BenchmarkParams {
        BenchmarkParams::new(
            self.commit_hash.clone(),
            self.benchmark_name.clone(),
            self.driver_name.clone(),
            benchmark_point,
            self.measurement_method.clone(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlatBenchmarkRecord {
    Data(String),
    Timeout,
    TimedData { mean: f64, stddev: f64 },
}

impl BenchmarkRecord {
    pub fn flatten(self) -> Result<FlatBenchmarkRecord, std::io::Error> {
        Ok(match self {
            BenchmarkRecord::Data(s) => FlatBenchmarkRecord::Data(s),
            BenchmarkRecord::FilePath(path) => {
                FlatBenchmarkRecord::Data(fs::read_to_string(&path)?)
            }
            BenchmarkRecord::Timeout => FlatBenchmarkRecord::Timeout,
            BenchmarkRecord::TimedData { mean, stddev } => {
                FlatBenchmarkRecord::TimedData { mean, stddev }
            }
        })
    }
}

#[must_use]
pub fn format_entry(params: &BenchmarkParams, record: &BenchmarkRecord) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "--- Benchmark Entry ---");
    let _ = writeln!(out, "Commit Hash:         {}", params.commit_hash);
    let _ = writeln!(out, "Benchmark Name:      {}", params.benchmark_name);
    let _ = writeln!(out, "Driver Name:         {}", params.driver_name);
    let _ = writeln!(out, "Benchmark Point:     {}", params.benchmark_point);
    let _ = writeln!(out, "Measurement Method:  {}", params.measurement_method);

    match record {
        BenchmarkRecord::Data(s) => {
            let _ = writeln!(out, "Record Type:         Data");
            let _ = writeln!(out, "Data Content:        {s}");
        }
        BenchmarkRecord::FilePath(path) => {
            let _ = writeln!(out, "Record Type:         FilePath");
            let _ = writeln!(out, "Data Content:        {}", path.display());
        }
        BenchmarkRecord::Timeout => {
            let _ = writeln!(out, "Record Type:         Timeout");
        }
        BenchmarkRecord::TimedData { mean, stddev } => {
            let _ = writeln!(out, "Record Type:         TimedData");
            let _ = writeln!(out, "Mean:                {mean:.6}");
            let _ = writeln!(out, "Std Dev:             {stddev:.6}");
        }
    }

    out
}

/// Adds a small padding above and below a y range so data points and error bars
/// are never clipped by the axis boundary.
/// Uses 5 % of the span, with a fallback for zero-span ranges.
pub fn pad_y_range(y_min: f64, y_max: f64) -> (f64, f64) {
    let span = y_max - y_min;
    let pad = if span == 0.0 { y_max.abs() * 0.05 + 1.0 } else { span * 0.05 };
    (y_min - pad, y_max + pad)
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
