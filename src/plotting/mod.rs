mod data;
pub mod error;
mod flamegraph_plot;
mod perf_stat_plot;
mod plot;
mod render;
mod series;
mod series_plot;

#[cfg(test)]
mod data_tests;
#[cfg(test)]
mod plot_tests;
#[cfg(test)]
mod render_tests;

use crate::config::benchmark::{self, BenchmarkConfig};
use crate::database::Database;
use crate::perf_stat::PerfStatData;
use crate::plotting::flamegraph_plot::FlamegraphPlot;
use crate::{commit_hash::CommitHash, measurement::MeasurementMethod};

use data::BenchmarkDataset;
use error::PlotError;
use perf_stat_plot::PerfStatPlot;
use plot::{NullBackend, Plot};
pub use series::VisKind;
use series_plot::SeriesPlot;
use std::fmt;
use std::path::PathBuf;

use plotters::backend::{BitMapBackend, SVGBackend};

pub(crate) const IMAGE_WIDTH: u32 = 1920;
pub(crate) const IMAGE_HEIGHT: u32 = 1080;
pub(crate) const IMAGE_SIZE: (u32, u32) = (IMAGE_WIDTH, IMAGE_HEIGHT);

#[derive(Debug, clap::Subcommand)]
pub enum PlotKind {
    /// Generate a series plot
    Series {
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

impl fmt::Display for PlotKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PlotKind::Series { .. } => "series",
            PlotKind::PerfStat { .. } => "perf-stat",
            PlotKind::Flamegraph { .. } => "flamegraph",
        })
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq)]
pub enum OutputFormat {
    Png,
    Svg,
    Html,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OutputFormat::Png => "png",
            OutputFormat::Svg => "svg",
            OutputFormat::Html => "html",
        })
    }
}

pub struct PlotSettings {
    pub plot_kind: PlotKind,
    format: OutputFormat,
    output: String,
}

impl PlotSettings {
    pub fn new(plot_kind: PlotKind, format: OutputFormat, output: String) -> Self {
        PlotSettings {
            plot_kind,
            format,
            output,
        }
    }
}

fn plot_on_backend<P: Plot>(plot: P, output: &str, format: OutputFormat) -> Result<(), PlotError> {
    let extension = output.rsplit('.').next().unwrap_or("").to_string();

    if format.to_string() != extension {
        return Err(PlotError::IncompatibleFileExtension {
            extension,
            format: format.to_string().to_owned(),
        });
    }

    match format {
        OutputFormat::Png => plot.plot(BitMapBackend::new(output, IMAGE_SIZE)),
        OutputFormat::Svg => plot.plot(SVGBackend::new(output, IMAGE_SIZE)),
        OutputFormat::Html => plot.plot(NullBackend {}),
    }
}

pub fn plot(
    plot_settings: PlotSettings,
    database: &Database,
    benchmark_config: BenchmarkConfig,
    measurement_method: &MeasurementMethod,
    commit_hashes: impl Iterator<Item = CommitHash>,
    names: &[String],
) -> Result<(), PlotError> {
    match plot_settings.plot_kind {
        PlotKind::Series { visualization_kind } => {
            if plot_settings.format == OutputFormat::Html {
                return Err(PlotError::IncompatibleOutputFormat {
                    format: OutputFormat::Html.to_string(),
                    plot: PlotKind::Series { visualization_kind }.to_string(),
                });
            }

            let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot = SeriesPlot::from_dataset(
                dataset,
                benchmark_config.name,
                names,
                visualization_kind,
            )?;

            plot_on_backend(plot, &plot_settings.output, plot_settings.format)
        }

        PlotKind::Flamegraph {
            artifacts_dir,
            flame_repo,
        } => {
            if plot_settings.format != OutputFormat::Html {
                return Err(PlotError::IncompatibleOutputFormat {
                    format: plot_settings.format.to_string(),
                    plot: PlotKind::Flamegraph {
                        artifacts_dir,
                        flame_repo,
                    }
                    .to_string(),
                });
            }

            let flame_repo = flame_repo.ok_or_else(|| {
                PlotError::InvalidData(
                    "Flamegraph repository path is missing; please provide `--flame-repo` or configure it in the global config".to_string(),
                )
            })?;

            let dataset: BenchmarkDataset<String> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot = FlamegraphPlot::from_dataset(
                dataset,
                benchmark_config.name,
                names,
                PathBuf::from(plot_settings.output.clone()),
                flame_repo,
                artifacts_dir,
            )?;

            plot_on_backend(plot, &plot_settings.output, plot_settings.format)
        }

        PlotKind::PerfStat { events } => {
            if plot_settings.format == OutputFormat::Html {
                return Err(PlotError::IncompatibleOutputFormat {
                    format: OutputFormat::Html.to_string(),
                    plot: PlotKind::PerfStat { events }.to_string(),
                });
            }

            let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset::new(
                database,
                &benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot = PerfStatPlot::from_dataset(dataset, benchmark_config.name, names, events)?;

            plot_on_backend(plot, &plot_settings.output, plot_settings.format)
        }
    }
}
