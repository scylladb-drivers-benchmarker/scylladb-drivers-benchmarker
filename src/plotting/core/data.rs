use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::database::utilities::BenchmarkParams;
use crate::measurement::MeasurementMethod;
use crate::utilities::{BenchmarkPoint, FlatBenchmarkRecord};
use std::fmt::Debug;
use std::str::FromStr;

use crate::plotting::error::PlotError;

pub(crate) trait PlottableValue: Sized + Debug + Clone + FromStr {}

impl<T> PlottableValue for T where T: FromStr + Sized + Debug + Clone {}

#[derive(Debug)]
pub(crate) struct BenchmarkDataset<T: PlottableValue> {
    pub(crate) points: Vec<BenchmarkPoint>,
    pub(crate) results: Vec<Vec<Option<T>>>,
}

impl<T: PlottableValue> BenchmarkDataset<T> {
    pub(crate) fn new(
        database: &Database,
        benchmark_config: &BenchmarkConfig,
        commit_hashes: impl Iterator<Item = CommitHash>,
        measurement_method: &MeasurementMethod,
    ) -> Result<BenchmarkDataset<T>, PlotError> {
        let points = benchmark_config
            .data
            .benchmark_points()
            .collect::<Vec<BenchmarkPoint>>();

        let results = commit_hashes
            .map(|commit_hash| {
                Self::get_benchmark_results(
                    database,
                    &commit_hash,
                    benchmark_config,
                    measurement_method,
                )
            })
            .collect::<Result<Vec<Vec<Option<T>>>, PlotError>>()?;

        Ok(BenchmarkDataset { points, results })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &MeasurementMethod,
    ) -> Result<Vec<Option<T>>, PlotError> {
        let BenchmarkConfig {
            name: benchmark_name,
            data: benchmark_data,
        } = benchmark_config;

        let benchmark_params = |param: BenchmarkPoint| {
            BenchmarkParams::new(
                commit_hash.clone(),
                benchmark_name.clone(),
                param,
                measurement_method.to_string(),
            )
        };

        let mut results = Vec::new();
        let mut missing = Vec::new();

        for point in benchmark_data.benchmark_points() {
            let params = benchmark_params(point);

            match database.get_result(params)?.map(|r| r.flatten()) {
                Some(FlatBenchmarkRecord::Data(text)) => {
                    let value =
                        T::from_str(&text).map_err(|_| PlotError::InvalidData(text.clone()))?;
                    results.push(Some(value));
                }
                Some(FlatBenchmarkRecord::Timeout) => results.push(None),
                None => missing.push(point), // This invalidates the result, but for better errors, we continue
            }
        }

        if !missing.is_empty() {
            if missing.len() == benchmark_data.no_steps as usize {
                return Err(PlotError::MissingBenchmark {
                    commit_hash: commit_hash.as_str().to_owned(),
                    benchmark: benchmark_name.clone(),
                    measurement_method: measurement_method.to_string(),
                });
            } else {
                return Err(PlotError::MissingRecords {
                    commit_hash: commit_hash.as_str().to_owned(),
                    benchmark: benchmark_name.clone(),
                    points: missing,
                    measurement_method: measurement_method.to_string(),
                });
            }
        }

        Ok(results)
    }
}
