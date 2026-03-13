use std::fmt::Debug;
use std::str::FromStr;

use log::{debug, info, trace};

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
    pub std_devs: Vec<Vec<Option<f64>>>,
    pub names: Vec<String>,
}

impl<T: PlottableValue> BenchmarkDataset<T> {
    pub fn new(
        database: &Database,
        benchmark_config: &BenchmarkData,
        commits: impl Iterator<Item = (CommitHash, String)>,
        measurement_method: &MeasurementMethod,
    ) -> Result<BenchmarkDataset<T>, PlotError> {
        info!("Searching the database for results...");

        let mut results = Vec::new();
        let mut std_devs = Vec::new();
        let mut names = Vec::new();

        for (commit_hash, tag) in commits {
            let backend_names = database
                .get_backend_names(&commit_hash, &benchmark_config.name, &measurement_method.to_string())
                .map_err(PlotError::Database)?;

            if backend_names.is_empty() {
                return Err(PlotError::MissingBenchmark {
                    commit_hash: commit_hash.as_str().to_owned(),
                    benchmark: benchmark_config.name.clone(),
                    measurement_method: measurement_method.to_string(),
                });
            }

            for backend_name in backend_names {
                let (series, devs) = Self::get_benchmark_results(
                    database,
                    &commit_hash,
                    benchmark_config,
                    measurement_method,
                    &backend_name,
                )?;
                names.push(format!("{}@{}", backend_name, tag));
                results.push(series);
                std_devs.push(devs);
            }
        }

        Ok(BenchmarkDataset {
            points: benchmark_config.points.clone(),
            results,
            std_devs,
            names,
        })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkData,
        measurement_method: &MeasurementMethod,
        backend_name: &str,
    ) -> Result<(Vec<Option<T>>, Vec<Option<f64>>), PlotError> {
        let builder = BenchmarkParamsBuilder {
            commit_hash: commit_hash.clone(),
            benchmark_name: benchmark_config.name.clone(),
            backend_name: backend_name.to_owned(),
            measurement_method: measurement_method.to_string(),
        };

        let mut results = Vec::new();
        let mut std_devs = Vec::new();
        let mut missing = Vec::new();

        debug!("Retrieving data for commit_hash: {commit_hash}...");
        for point in benchmark_config.points.iter().cloned() {
            let params = builder.finalize(point);

            match database
                .get_result(params)?
                .map(crate::database::utilities::BenchmarkRecord::flatten)
            {
                Some(Ok(FlatBenchmarkRecord::Data(text))) => {
                    let value =
                        T::from_str(&text).map_err(|_| PlotError::InvalidData(text.clone()))?;

                    trace!("Retrieved result for {point}");
                    results.push(Some(value));
                    std_devs.push(None);
                }
                Some(Ok(FlatBenchmarkRecord::TimedData { mean, stddev })) => {
                    let text = mean.to_string();
                    let value =
                        T::from_str(&text).map_err(|_| PlotError::InvalidData(text.clone()))?;

                    trace!("Retrieved timed result for {point}");
                    results.push(Some(value));
                    std_devs.push(Some(stddev));
                }
                Some(Ok(FlatBenchmarkRecord::Timeout)) => {
                    trace!("Retrieved timeout for {point}");
                    results.push(None);
                    std_devs.push(None);
                }
                Some(Err(e)) => return Err(PlotError::Io(e)),
                None => missing.push(point),
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

        Ok((results, std_devs))
    }
}
