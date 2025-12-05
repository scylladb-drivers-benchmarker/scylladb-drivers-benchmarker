use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::utilities::{BenchmarkParams, BenchmarkPoint};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::Debug;

pub(crate) trait PlottableValue: Sized + Debug + Clone {
    fn from_json(s: &str) -> Option<Self>;
}

impl<T> PlottableValue for T
where
    T: DeserializeOwned + Sized + Debug + Clone,
{
    fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

pub(crate) struct BenchmarkDataset<T: PlottableValue> {
    pub(crate) points: Vec<BenchmarkPoint>,
    pub(crate) results: Vec<Vec<Option<T>>>,
}

impl<T: PlottableValue> BenchmarkDataset<T> {
    pub(crate) fn new(
        database: &Database,
        benchmark_config: &BenchmarkConfig,
        commit_hashes: &[CommitHash],
        measurement_method: &str,
    ) -> Result<BenchmarkDataset<T>, Box<dyn Error>> {
        let points = benchmark_config
            .data
            .benchmark_points()
            .collect::<Vec<BenchmarkPoint>>();

        let results = commit_hashes
            .iter()
            .map(|commit_hash| {
                Self::get_benchmark_results(
                    database,
                    commit_hash,
                    benchmark_config,
                    measurement_method,
                )
            })
            .collect::<Result<Vec<Vec<Option<T>>>, Box<dyn Error>>>()?;

        Ok(BenchmarkDataset { points, results })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &str,
    ) -> Result<Vec<Option<T>>, Box<dyn Error>> {
        let BenchmarkConfig {
            name: benchmark_name,
            data: benchmark_data,
        } = benchmark_config;

        let benchmark_params = |param: u64| {
            BenchmarkParams::new(
                commit_hash.clone(),
                benchmark_name.clone(),
                param,
                measurement_method.to_string(),
            )
        };

        let results = benchmark_data
            .benchmark_points()
            .map(|point| -> Result<_, Box<dyn Error>> {
                let record = database.get_data(benchmark_params(point))?;

                let value = record
                    .and_then(|r| r.data_json)
                    .and_then(|text| T::from_json(&text));

                Ok(value)
            })
            .collect::<Result<Vec<Option<T>>, _>>()?;

        Ok(results)
    }
}
