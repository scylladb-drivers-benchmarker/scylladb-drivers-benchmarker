mod execution;

use std::error::Error;
use std::path::Path;

use crate::commit_hash::CommitHash;
use crate::config::backend::BackendConfigList;
use crate::config::config_traits::ConfigurationList;
use crate::config::open_config;
use crate::utilities::BenchmarkParams;
use execution::{Executor, SourceCode, build_source};

use crate::config::{backend::BackendConfig, benchmark::BenchmarkConfig};

use super::database::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct BenchmarkingArguments {
    #[arg(short, long)]
    pub driver_name: String,

    #[arg(short, long)]
    pub measurement_method: String,
}

pub fn benchmark(
    database: &Database,
    commit_hash: CommitHash,
    benchmark_config: BenchmarkConfig,
    backend_config_path: &Path,
    measurement_method: String,
) -> Result<(), Box<dyn Error>> {
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

    let points = benchmark_data
        .benchmark_points()
        .filter_map(
            |point| match database.data_exists(benchmark_params(point)) {
                Ok(true) => Some(Ok(point)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            },
        )
        .collect::<Result<Vec<u32>, sqlite::Error>>()?;

    if points.is_empty() {
        return Ok(());
    }

    let backend_config: BackendConfig = open_config::<BackendConfigList>(backend_config_path)?
        .configs()
        .find(|config| config.benchmark_name == benchmark_name)
        .expect("backend not found"); // TODO: fix

    let built_source = build_source(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let mut executor = Executor::new(built_source, backend_config.run_command.as_str())?;
    executor.measure(measurement_method.as_str())?;

    for point in points.iter().cloned() {
        let benchmark_result = executor.run(point)?;
        database.insert_data(benchmark_params(point), benchmark_result)?;
    }

    Ok(())
}
