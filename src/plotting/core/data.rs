use std::fmt::Debug;
use std::str::FromStr;

use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkData;
use crate::database::Database;
use crate::measurement::MeasurementMethod;
use crate::plotting::PlotError;
use crate::utilities::{BenchmarkParamsBuilder, BenchmarkPoint, FlatBenchmarkRecord};

pub(crate) trait PlottableValue: FromStr + Debug + Clone {}

impl<T> PlottableValue for T where T: FromStr + Debug + Clone {}

#[derive(Debug)]
pub struct BenchmarkDataset<T: PlottableValue> {
    pub points: Vec<BenchmarkPoint>,
    pub results: Vec<Vec<Option<T>>>,
}

impl<T: PlottableValue> BenchmarkDataset<T> {
    pub fn new(
        database: &Database,
        benchmark_config: &BenchmarkData,
        commit_hashes: impl Iterator<Item = CommitHash>,
        measurement_method: &MeasurementMethod,
    ) -> Result<BenchmarkDataset<T>, PlotError> {
        let results = commit_hashes
            .map(|commit_hash| {
                Self::get_benchmark_results(
                    database,
                    &commit_hash,
                    benchmark_config,
                    measurement_method,
                )
            })
            .collect::<Result<_, _>>()?;

        Ok(BenchmarkDataset {
            points: benchmark_config.points.clone(),
            results,
        })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkData,
        measurement_method: &MeasurementMethod,
    ) -> Result<Vec<Option<T>>, PlotError> {
        let builder = BenchmarkParamsBuilder {
            commit_hash: commit_hash.clone(),
            benchmark_name: benchmark_config.name.clone(),
            measurement_method: measurement_method.to_string(),
        };

        let mut results = Vec::new();
        let mut missing = Vec::new();

        for point in benchmark_config.points.iter().cloned() {
            let params = builder.finalize(point);

            match database
                .get_result(params)?
                .map(crate::database::utilities::BenchmarkRecord::flatten)
            {
                Some(Ok(FlatBenchmarkRecord::Data(text))) => {
                    let value =
                        T::from_str(&text).map_err(|_| PlotError::InvalidData(text.clone()))?;
                    results.push(Some(value));
                }
                Some(Ok(FlatBenchmarkRecord::Timeout)) => results.push(None),
                Some(Err(e)) => return Err(PlotError::Io(e)),
                None => missing.push(point), // This invalidates the result, but for better errors, we continue
            }
        }

        if !missing.is_empty() {
            if missing.len() == benchmark_config.points.len() {
                return Err(PlotError::MissingBenchmark {
                    commit_hash: commit_hash.as_str().to_owned(),
                    benchmark: benchmark_config.name.clone(),
                    measurement_method: measurement_method.to_string(),
                });
            }
            return Err(PlotError::MissingRecords {
                commit_hash: commit_hash.as_str().to_owned(),
                benchmark: benchmark_config.name.clone(),
                points: missing,
                measurement_method: measurement_method.to_string(),
            });
        }

        Ok(results)
    }
}
