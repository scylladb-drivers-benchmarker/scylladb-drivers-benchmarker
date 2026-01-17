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

#[derive(Debug, Clone, clap::Args)]
pub struct FlameOptions {
    #[arg(short = 'r', long)]
    flame_repo: Option<PathBuf>,
    #[arg(short, long, default_value_t = FlameFrequency::Number(99))]
    frequency: FlameFrequency,
    /// Directory in which to store the results
    #[arg(short, long)]
    store_dir: Option<PathBuf>,
}

impl FlameOptions {
    pub fn finalize(
        mut self,
        aliasing_config: AliasingConfig,
    ) -> Result<BenchMeasure, ParsingError> {
        self.store_dir = self.store_dir.or(aliasing_config.store_dir);

        let Some(mut store_dir) = self.store_dir else {
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

        Ok(BenchMeasure::FlameGraph {
            flame_repo: self
                .flame_repo
                .or(aliasing_config.flame_path.clone())
                .unwrap_or_default(),
            frequency: self.frequency,
            store_dir,
        })
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum MeasureSubcommand {
    Time,
    PerfStat,
    FlameGraph(#[command(flatten)] FlameOptions),
    Command { command: command::Command },
}

impl MeasureSubcommand {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<BenchMeasure, ParsingError> {
        match self {
            MeasureSubcommand::Time => Ok(BenchMeasure::Time),
            MeasureSubcommand::PerfStat => Ok(BenchMeasure::PerfStat),
            MeasureSubcommand::FlameGraph(flame_options) => flame_options.finalize(aliasing_config),
            MeasureSubcommand::Command { command } => Ok(BenchMeasure::Command(command)),
        }
    }
}

#[derive(Args, Debug)]
pub struct BenchmarkCommand {
    pub benchmark_name: String,
    #[arg(short = 'B', long, default_value = "./config.yml")]
    pub backend_config_path: PathBuf,
    #[arg(short = 'b', long)]
    pub benchmark_configuration: Option<BenchmarkSetup>,
    #[arg(long, short = 'M', value_enum, default_value_t = BenchmarkMode::UseCached)]
    pub benchmark_mode: BenchmarkMode,
    #[clap(subcommand)]
    pub measure: Option<MeasureSubcommand>,
}

impl BenchmarkCommand {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<Subcommands, ParsingError> {
        let benchmark_config = BenchmarkSetup::finalize(
            self.benchmark_configuration,
            &self.benchmark_name,
            &aliasing_config,
        )?;

        let measure = self.measure.unwrap_or(MeasureSubcommand::Time);
        let bench_measure = measure.finalize(aliasing_config)?;

        Ok(Subcommands::Benchmark(BenchmarkParams {
            bench_measure,
            backend_config: find_config(&self.benchmark_name, &self.backend_config_path)?,
            benchmark_config,
            benchmark_mode: self.benchmark_mode,
        }))
    }
}
