mod execution;

use std::{
    error::Error,
    iter::zip,
    path::PathBuf,
};

use crate::commit_hash::CommitHash;
use execution::{Executor, SourceCode, compile};

use crate::config::{
    backend::BackendConfig,
    benchmark::BenchmarkConfig,
    find_config,
};

use super::database::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct BenchmarkingArguments {
    #[arg(short, long)]
    pub benchmark_name: String,

    #[arg(long, default_value = "./config.yml")]
    pub benchmark_config_path: PathBuf,

    #[arg(short, long)]
    pub driver_name: String,

    #[arg(long, default_value = "./config.yml")]
    pub driver_config_path: PathBuf,

    #[arg(short, long)]
    pub instrumentation: String,
}

fn benchmark(
    args: &BenchmarkingArguments,
    benchmark_config: &BenchmarkConfig,
    backend_config: &BackendConfig,
) -> Result<Vec<String>, Box<dyn Error>> {
    let compiled_source = compile(
        backend_config.build_command.as_str(),
        SourceCode { path: None },
    )?;

    let mut executor = Executor::new(compiled_source, backend_config.run_command.as_str())?;
    executor.measure(args.instrumentation.as_ref())?;

    let mut outputs: Vec<String> = Vec::new();
    for benchmark_point in benchmark_config.iter_points() {
        let output = executor.execute(benchmark_point)?;
        let output = String::from_utf8(output.stdout)?;
        outputs.push(output);
    }

    Ok(outputs)
}

fn save_benchmark_results(
    args: &BenchmarkingArguments,
    git_hash: CommitHash,
    results: Vec<String>,
    benchmark_config: &BenchmarkConfig,
    backend_config: &BackendConfig,
    database: &Database,
) -> Result<(), Box<dyn Error>> {
    for (param, result) in zip(benchmark_config.iter_points(), results) {
        database.insert_data(
            BenchmarkParams::new(
                git_hash.clone(),
                args.benchmark_name.clone(),
                param.into(),
                args.instrumentation.clone(),
            ),
            BenchmarkResult::new(Some(result)),
        )?
    }

    Ok(())
}

fn main(args: &BenchmarkingArguments, database: &Database) -> Result<(), Box<dyn Error>> {
    let benchmark_config: BenchmarkConfig =
        find_config(&args.benchmark_name, &args.driver_config_path)?;
    let backend_config: BackendConfig = find_config(&args.driver_name, &args.driver_config_path)?;

    let git_hash = CommitHash::from_repository()?;
    let results = benchmark(args, &benchmark_config, &backend_config)?;
    save_benchmark_results(
        args,
        git_hash,
        results,
        &benchmark_config,
        &backend_config,
        database,
    )
}
