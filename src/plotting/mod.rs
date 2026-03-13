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

use log::info;

use crate::BackendWithCommit;
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
        "png" => {
            info!("Creating output file {output}");
            plot.plot(BitMapBackend::new(output, IMAGE_SIZE))
        }
        "svg" => {
            info!("Creating output file {output}");
            plot.plot(SVGBackend::new(output, IMAGE_SIZE))
        }
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
    series: impl Iterator<Item = BackendWithCommit>,
) -> Result<(), PlotError> {
    match plot_settings.plot_kind {
        PlotKind::Series {
            measurement_method,
            visualization_kind,
        } => {
            let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                series.map(|s| (s.backend_name, s.commit, s.tag)),
                &measurement_method,
            )?;

            let plot = SeriesPlot::from_dataset(
                dataset,
                benchmark_config.name,
                visualization_kind,
                measurement_method.y_axis_label(),
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
                series.map(|s| (s.backend_name, s.commit, s.tag)),
                &MeasurementMethod::FlameGraph,
            )?;

            let plot = FlameGraphPlot::from_dataset(
                dataset,
                benchmark_config.name,
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
                series.map(|s| (s.backend_name, s.commit, s.tag)),
                &MeasurementMethod::Perf,
            )?;

            let plot = PerfStatPlot::from_dataset(dataset, benchmark_config.name, events)?;

            plot_on_backend(plot, &plot_settings.output)
        }
    }
}
