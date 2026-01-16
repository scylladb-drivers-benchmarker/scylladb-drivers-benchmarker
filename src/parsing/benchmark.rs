use crate::BenchmarkMode;
use crate::BenchmarkParams;
use crate::command;
use crate::parsing::ParsingError;
use crate::parsing::Subcommands;
use crate::parsing::aliasing::AliasingConfig;
use crate::parsing::bsetup::BenchmarkSetup;
use clap::Args;
use scylladb_drivers_benchmarker::config::find_config;
use scylladb_drivers_benchmarker::flame_graph::BenchMeasure;
use scylladb_drivers_benchmarker::flame_graph::FlameFrequency;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
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

    #[arg(short = 'b', long, default_value = "./config.yml")]
    pub benchmark_configuration: BenchmarkSetup,

    #[arg(long, short = 'M', value_enum, default_value_t = BenchmarkMode::UseCached)]
    pub benchmark_mode: BenchmarkMode,

    #[clap(subcommand)]
    pub measure: Option<MeasureSubcommand>,
}

impl BenchmarkCommand {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<Subcommands, ParsingError> {
        let measure = self.measure.unwrap_or(MeasureSubcommand::Time);
        let bench_measure = match measure {
            MeasureSubcommand::Time => BenchMeasure::Time,
            MeasureSubcommand::PerfStat => BenchMeasure::PerfStat,
            MeasureSubcommand::FlameGraph {
                flame_repo,
                frequency,
                mut store_dir,
            } => {
                store_dir = store_dir.or(aliasing_config.store_dir);

                let Some(mut store_dir) = store_dir else {
                    return Err(ParsingError::NoStoreDir {
                        needed_by: MeasurementMethod::Flamegraph,
                    });
                };

                if !store_dir.is_absolute() {
                    store_dir = store_dir.canonicalize()?;
                }

                if !store_dir.is_absolute() {
                    return Err(ParsingError::StoreDirNotAbsolute);
                }

                if !store_dir.is_dir() {
                    return Err(ParsingError::StoreDirNotADir);
                }

                BenchMeasure::FlameGraph {
                    flame_repo: flame_repo
                        .or(aliasing_config.flame_path.clone())
                        .unwrap_or_default(),
                    frequency,
                    store_dir,
                }
            }
            MeasureSubcommand::Command { command } => BenchMeasure::Command(command),
        };

        Ok(Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            backend_config: find_config(&self.benchmark_name, &self.backend_config_path)?,
            benchmark_config: self
                .benchmark_configuration
                .to_config(self.benchmark_name)?,
            benchmark_mode: self.benchmark_mode,
        }))
    }
}
