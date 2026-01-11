mod data;
pub mod error;
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

use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::{commit_hash::CommitHash, measurement::MeasurementMethod};

use error::PlotError;
use perf_stat_plot::PerfStatPlot;
use plot::Plot;
pub use series::VisKind;
use series_plot::SeriesPlot;

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
    Flamegraph,

    /// Generate a perf-stat plot
    PerfStat {
        #[arg(short, long)]
        #[clap(value_delimiter=',', num_args(1..))]
        events: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Png,
    Svg,
}

impl OutputFormat {
    pub fn to_string(&self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Svg => "svg",
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
    }
}

pub fn plot(
    plot_kind: PlotKind,
    database: &Database,
    benchmark_config: BenchmarkConfig,
    measurement_method: &MeasurementMethod,
    commit_hashes: impl Iterator<Item = CommitHash>,
    names: &[String],
    format: OutputFormat,
    output: &str,
) -> Result<(), PlotError> {
    match plot_kind {
        PlotKind::Series { visualization_kind } => {
            let plot = SeriesPlot::build(
                database,
                benchmark_config,
                measurement_method,
                commit_hashes,
                names,
                visualization_kind,
            )?;

            plot_on_backend(plot, output, format)
        }

        PlotKind::Flamegraph => {
            unimplemented!("Flamegraph plotting is not yet implemented");
            // let plot = FlamegraphPlot::build(
            //     database,
            //     benchmark_name,
            //     benchmark_config,
            //     measurement_method,
            //     commit_hashes,
            //     names,
            // )?;

            // plot_on_backend(plot, output, format)
        }

        PlotKind::PerfStat { events } => {
            let plot = PerfStatPlot::build(
                database,
                benchmark_config,
                measurement_method,
                commit_hashes,
                names,
                events,
            )?;

            plot_on_backend(plot, output, format)
        }
    }
}
