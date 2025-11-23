mod command;
mod errors;
mod execution;

use std::{
    error::Error,
    iter::zip,
    path::PathBuf,
    process::{self, Output},
};

use command::Command;
use execution::{CompiledSource, Executor, SourceCode, compile};

use crate::{
    benchmarking::errors::{GitFailed, WrongCommitHash},
    cmd,
    config::{
        backend::{self, BackendConfig},
        benchmark::{self, BenchmarkConfig, BenchmarkConfigList},
        find_config,
    },
};

use super::database::*;

use clap::{Parser, builder::Str};

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
    git_hash: String,
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

    let git_get_hash = cmd!("git", "rev-parse", "--verify", "HEAD");
    let output = git_get_hash.process().output()?;

    let git_hash = String::from_utf8(output.stdout)?;

    if !output.status.success() {
        return Err(Box::new(GitFailed {
            status: output.status,
            stderr: output.stderr,
        }));
    }

    if !git_hash.chars().all(|char| char.is_alphanumeric()) {
        return Err(Box::new(WrongCommitHash {
            got: git_hash,
            reason: "git hashes should be alphanumeric".to_string(),
        }));
    }

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
