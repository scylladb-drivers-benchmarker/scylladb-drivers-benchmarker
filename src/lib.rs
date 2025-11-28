use std::{error::Error, path::Path};

use crate::{commit_hash::CommitHash, config::{benchmark::BenchmarkConfig, find_config}, database::Database};

#[allow(dead_code)]
mod benchmarking;
mod command;
mod commit_hash;
mod config;
pub mod database;
mod plotting;
mod utilities;

pub fn run_benchmarks(
    database: &Database,
    benchmark_name: &str,
    benchmark_config_path: &Path,
    measurement_method: String,
    backend_config_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig = find_config(&benchmark_name, &benchmark_config_path)?;

    let commit_hash: CommitHash = CommitHash::from_repository()?;
    benchmarking::main(&database, commit_hash, benchmark_config, backend_config_path, measurement_method)
}

pub fn plot_benchmarks() -> Result<(), Box<dyn Error>> {
    todo!()
}
