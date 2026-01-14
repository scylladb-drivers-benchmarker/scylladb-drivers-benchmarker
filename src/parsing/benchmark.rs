use crate::BenchmarkMode;
use crate::BenchmarkParams;
use crate::command;
use clap::Args;
use scylladb_drivers_benchmarker::flame_graph::FlameFrequency;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum MeasureSubcommand {
    Time,
    PerfStat,
    FlameGraph {
        #[arg(short = 'r', long)]
        flame_repo: Option<PathBuf>,
        #[arg(short, long, default_value_t = FlameFrequency::Number(99))]
        frequency: FlameFrequency,
        /// Directory in which to store the results
        #[arg(short, long)]
        store_dir: Option<PathBuf>,
    },
    Command {
        command: command::Command,
    },
}

#[derive(Args, Debug)]
pub struct BenchmarkCommand {
    pub benchmark_name: String,

    #[arg(short = 'B', long, default_value = "./config.yml")]
    pub backend_config_path: PathBuf,

    #[arg(short, long, default_value = "./config.yml")]
    pub benchmark_config_path: PathBuf,

    #[arg(long, short = 'M', value_enum, default_value_t = BenchmarkMode::UseCached)]
    pub benchmark_mode: BenchmarkMode,

    #[clap(subcommand)]
    pub measure: Option<MeasureSubcommand>,
}

impl BenchmarkCommand {
    pub fn finalize(self) -> Result<BenchmarkParams, Box<dyn std::error::Error>> {
        // TODO IMPROVE, MODIFY bench_params
        Ok(BenchmarkParams {
            benchmark_name: self.benchmark_name,
            measure: self.measure,

            backend_config_path: self.backend_config_path,
            benchmark_config_path: self.benchmark_config_path,
            benchmark_mode: self.benchmark_mode,
        })
    }
}
