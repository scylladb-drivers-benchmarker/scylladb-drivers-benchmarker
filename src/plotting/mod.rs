mod core;
pub mod error;
mod plots;

#[cfg(test)]
mod tests;

pub use core::VisKind;
use core::{BenchmarkDataset, NullBackend, Plot};
use std::path::PathBuf;

use error::PlotError;
use plots::{FlameGraphPlot, PerfStatPlot, SeriesPlot};
use plotters::backend::{BitMapBackend, SVGBackend};

use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkData;
use crate::database::Database;
use crate::measurement::MeasurementMethod;
use crate::perf_stat::PerfStatData;

const IMAGE_WIDTH: u32 = 1920;
const IMAGE_HEIGHT: u32 = 1080;
const IMAGE_SIZE: (u32, u32) = (IMAGE_WIDTH, IMAGE_HEIGHT);

#[derive(Debug)]
pub enum PlotKind {
    Series {
        measurement_method: MeasurementMethod,
        visualization_kind: VisKind,
    },

    FlameGraph {
        artifacts_dir: Option<PathBuf>,
        flame_repo: PathBuf,
    },

    PerfStat {
        events: Vec<String>,
    },
}

pub struct PlotSettings {
    pub plot_kind: PlotKind,
    output: String,
}

impl PlotSettings {
    #[must_use]
    pub fn new(plot_kind: PlotKind, output: String) -> Self {
        PlotSettings { plot_kind, output }
    }
}

fn plot_on_backend(plot: impl Plot, output: &str) -> Result<(), PlotError> {
    let extension = output.rsplit('.').next().unwrap_or("").to_owned();

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
    benchmark_config: BenchmarkData,
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

        PlotKind::FlameGraph {
            artifacts_dir,
            flame_repo,
        } => {
            let dataset: BenchmarkDataset<String> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                &MeasurementMethod::FlameGraph,
            )?;

            let plot = FlameGraphPlot::from_dataset(
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
