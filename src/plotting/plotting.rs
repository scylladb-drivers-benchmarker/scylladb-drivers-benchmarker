use std::error::Error;

use crate::database::Database;
use crate::utilities::{BenchmarkParams};
use crate::config::benchmark::BenchmarkConfig;
use crate::commit_hash::CommitHash;

pub struct PlotData {
    pub points: Vec<u32>,
    pub results: Vec<Vec<Option<u64>>>,
}

impl PlotData {
    pub fn new_plot(
        database: &Database,
        benchmark_config: &BenchmarkConfig,
        commit_hashes: &Vec<CommitHash>,
        measurement_method: String
    ) -> Result<PlotData, Box<dyn Error>> {
        let points = benchmark_config
            .data
            .benchmark_points()
            .collect::<Vec<u32>>();

        let results = commit_hashes
            .iter()
            .map(|commit_hash| {
                get_benchmark_results(
                    database,
                    commit_hash,
                    benchmark_config,
                    measurement_method.clone()
                )
            }).collect::<Result<Vec<Vec<Option<u64>>>, Box<dyn Error>>>()?;

            Ok(PlotData {
                points,
                results,
            })
    }
}

pub fn get_benchmark_results(
    database: &Database,
    commit_hash: &CommitHash,
    benchmark_config: &BenchmarkConfig,
    measurement_method: String
) -> Result<Vec<Option<u64>>, Box<dyn Error>> {
    let BenchmarkConfig {
        name: benchmark_name,
        data: benchmark_data,
    } = benchmark_config;

    let benchmark_params = |param: u32| {
        BenchmarkParams::new(
            commit_hash.clone(),
            benchmark_name.clone(),
            param.into(),
            measurement_method.clone(),
        )
    };

    let results = benchmark_data
        .benchmark_points()
        .map(|point| -> Result<_, Box<dyn Error>> {
            let record = database.get_data(benchmark_params(point))?;

            let value = record
                .and_then(|r| r.data_json)
                .and_then(|text| text.parse::<u64>().ok());

            Ok(value)
        }).collect::<Result<Vec<Option<u64>>, _>>()?;

    Ok(results)
}