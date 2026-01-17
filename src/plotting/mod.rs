mod core;
pub mod error;
mod plots;

#[cfg(test)]
mod tests;

use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::perf_stat::PerfStatData;
use crate::{commit_hash::CommitHash, measurement::MeasurementMethod};

use core::BenchmarkDataset;
pub use core::VisKind;
use core::{NullBackend, Plot};
use error::PlotError;
use plots::FlamegraphPlot;
use plots::PerfStatPlot;
use plots::SeriesPlot;
use std::path::PathBuf;

use plotters::backend::{BitMapBackend, SVGBackend};

pub(crate) const IMAGE_WIDTH: u32 = 1920;
pub(crate) const IMAGE_HEIGHT: u32 = 1080;
pub(crate) const IMAGE_SIZE: (u32, u32) = (IMAGE_WIDTH, IMAGE_HEIGHT);

#[derive(Debug, clap::Subcommand)]
pub enum PlotKind {
    /// Generate a series plot
    Series {
        #[arg(short, long, default_value_t = MeasurementMethod::Time)]
        measurement_method: MeasurementMethod,

        #[arg(short, long, value_enum, default_value_t = VisKind::Linear)]
        visualization_kind: VisKind,
    },

    /// Generate a flamegraph plot
    Flamegraph {
        #[arg(short, long, value_name = "DIR")]
        artifacts_dir: Option<PathBuf>,

        #[arg(short, long, value_name = "DIR")]
        flame_repo: Option<PathBuf>,
    },

    /// Generate a perf-stat plot
    PerfStat {
        #[arg(short, long)]
        #[clap(value_delimiter=',', num_args(1..))]
        events: Vec<String>,
    },
}

pub struct PlotSettings {
    pub plot_kind: PlotKind,
    output: String,
}

impl PlotSettings {
    pub fn new(plot_kind: PlotKind, output: String) -> Self {
        PlotSettings { plot_kind, output }
    }
}

pub(crate) fn plot_on_backend(plot: impl Plot, output: &str) -> Result<(), PlotError> {
    let extension = output.rsplit('.').next().unwrap_or("").to_string();

    match extension.as_str() {
        "png" => plot.plot(BitMapBackend::new(output, IMAGE_SIZE)),
        "svg" => plot.plot(SVGBackend::new(output, IMAGE_SIZE)),
        "html" => plot.plot(NullBackend {}),
        _ => Err(PlotError::IncompatibleOutputFormat {
            format: extension,
            plot: plot.name().to_owned(),
        }),
    }
}

pub fn plot(
    plot_settings: PlotSettings,
    database: &Database,
    benchmark_config: BenchmarkConfig,
    commit_hashes: impl Iterator<Item = CommitHash>,
    names: &[String],
) -> Result<(), PlotError> {
    match plot_settings.plot_kind {
        PlotKind::Series {
            measurement_method,
            visualization_kind,
        } => {
            let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                &measurement_method,
            )?;

            let plot = SeriesPlot::from_dataset(
                dataset,
                benchmark_config.name,
                names,
                visualization_kind,
            )?;

            plot_on_backend(plot, &plot_settings.output)
        }

        PlotKind::Flamegraph {
            artifacts_dir,
            flame_repo,
        } => {
            let flame_repo = flame_repo.ok_or_else(|| {
                PlotError::InvalidData(
                    "Flamegraph repository path is missing; please provide `--flame-repo` or configure it in the global config".to_owned(),
                )
            })?;

            let dataset: BenchmarkDataset<String> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                &MeasurementMethod::Flamegraph,
            )?;

            let plot = FlamegraphPlot::from_dataset(
                dataset,
                benchmark_config.name,
                names,
                PathBuf::from(plot_settings.output.clone()),
                flame_repo,
                artifacts_dir,
            )?;

            plot_on_backend(plot, &plot_settings.output)
        }

        PlotKind::PerfStat { events } => {
            let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                &MeasurementMethod::Perf,
            )?;

            let plot = PerfStatPlot::from_dataset(dataset, benchmark_config.name, names, events)?;

            plot_on_backend(plot, &plot_settings.output)
        }
    }
}
