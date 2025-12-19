use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::utilities::{BenchmarkParams, BenchmarkPoint, BenchmarkRecord};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use super::error::PlotError;

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

#[derive(Debug)]
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
    ) -> Result<BenchmarkDataset<T>, PlotError> {
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
            .collect::<Result<Vec<Vec<Option<T>>>, PlotError>>()?;

        Ok(BenchmarkDataset { points, results })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &str,
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

        let results = benchmark_data
            .benchmark_points()
            .map(|point| -> Result<_, PlotError> {
                let params = benchmark_params(point.clone());

                let record = database.get_result(params.clone())?;

                let record = record.ok_or_else(|| PlotError::MissingRecord {
                    commit_hash: commit_hash.as_str().to_owned(),
                    benchmark: benchmark_name.clone(),
                    point,
                    measurement_method: measurement_method.to_owned(),
                })?;

                Ok(match record {
                    BenchmarkRecord::Data(text) => T::from_json(&text),
                    BenchmarkRecord::Timeout => None,
                })
            })
            .collect::<Result<Vec<Option<T>>, _>>()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use tempfile::NamedTempFile;

    fn get_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        (Database::new(path).unwrap(), file)
    }

    fn init_db() -> (
        Database,
        NamedTempFile,
        Vec<BenchmarkConfig>,
        Vec<CommitHash>,
        String,
    ) {
        let (db, file) = get_db();

        let config1 = BenchmarkConfig {
            name: "benchmark1".to_string(),
            data: config::benchmark::BenchmarkData {
                starting_step: 1,
                no_steps: 2,
                step_progress: 1,
                progress_type: config::benchmark::ProgressType::Additive,
                timeout: None,
            },
        };

        let commit_hash_1 = CommitHash::new_unchecked("1".to_string());
        let params1 = BenchmarkParams::new(
            commit_hash_1.clone(),
            config1.name.clone(),
            1,
            "time".to_string(),
        );
        let record1 = utilities::BenchmarkRecord::Data("1.5".to_string());
        db.insert_data(params1, record1).unwrap();

        let params2 = BenchmarkParams::new(
            commit_hash_1.clone(),
            config1.name.clone(),
            2,
            "time".to_string(),
        );
        let record2 = utilities::BenchmarkRecord::Data("2.5".to_string());
        db.insert_data(params2, record2).unwrap();

        let commit_hash_2 = CommitHash::new_unchecked("2".to_string());
        let params3 = BenchmarkParams::new(
            commit_hash_2.clone(),
            config1.name.clone(),
            1,
            "time".to_string(),
        );
        let record3 = utilities::BenchmarkRecord::Data("2".to_string());
        db.insert_data(params3, record3).unwrap();

        let params4 = BenchmarkParams::new(
            commit_hash_2.clone(),
            config1.name.clone(),
            2,
            "time".to_string(),
        );
        let record4 = utilities::BenchmarkRecord::Data("3.5".to_string());
        db.insert_data(params4, record4).unwrap();

        let config2 = BenchmarkConfig {
            name: "benchmark2".to_string(),
            data: config::benchmark::BenchmarkData {
                starting_step: 10,
                no_steps: 1,
                step_progress: 1,
                progress_type: config::benchmark::ProgressType::Additive,
                timeout: None,
            },
        };

        let commit_hash_3 = CommitHash::new_unchecked("3".to_string());
        let params5 = BenchmarkParams::new(
            commit_hash_3.clone(),
            config2.name.clone(),
            10,
            "time".to_string(),
        );
        let record5 = utilities::BenchmarkRecord::Data("3.5".to_string());
        db.insert_data(params5, record5).unwrap();

        (
            db,
            file,
            vec![config1, config2],
            vec![commit_hash_1, commit_hash_2, commit_hash_3],
            "time".to_owned(),
        )
    }

    #[test]
    fn extract() {
        let (db, _file, configs, hashes, measure) = init_db();

        let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
            &db,
            &configs[0],
            &[hashes[0].clone(), hashes[1].clone()],
            &measure,
        )
        .unwrap();
        assert_eq!(dataset.points, vec![1, 2]);
        assert_eq!(
            dataset.results,
            vec![vec![Some(1.5), Some(2.5)], vec![Some(2.0), Some(3.5)]]
        );

        let dataset: BenchmarkDataset<f64> =
            BenchmarkDataset::new(&db, &configs[1], &[hashes[2].clone()], &measure).unwrap();
        assert_eq!(dataset.points, vec![10]);
        assert_eq!(dataset.results, vec![vec![Some(3.5)]]);
    }

    #[test]
    fn extract_failure() {
        let (db, _file, _configs, hashes, measure) = init_db();

        let config = BenchmarkConfig {
            name: "wrong".to_string(),
            data: config::benchmark::BenchmarkData {
                starting_step: 1,
                no_steps: 2,
                step_progress: 3,
                progress_type: config::benchmark::ProgressType::Multiplicative,
                timeout: None,
            },
        };

        let dataset: Result<BenchmarkDataset<f64>, PlotError> =
            BenchmarkDataset::new(&db, &config, &hashes, &measure);

        assert!(matches!(
            dataset.unwrap_err(),
            PlotError::MissingRecord {
                commit_hash,
                benchmark,
                point,
                measurement_method,
            } if commit_hash == "1"
                && benchmark == "wrong"
                && point == 1
                && measurement_method == "time"
        ));

        // No idea how to force db to fail on read.
        // let dataset: Result<BenchmarkDataset<f64>, PlotError> = BenchmarkDataset::new(..., &configs[1], &vec![hashes[2].clone()], &measure);
        // assert!(matches!(dataset.unwrap_err(), PlotError::DatabaseError(_)));
    }
}
