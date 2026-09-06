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
use tempfile::NamedTempFile;

use crate::BackendWithCommit;
use crate::config::benchmark::BenchmarkData;
use crate::database::Database;
use crate::measurement::MeasurementMethod;
use crate::perf_stat::PerfStatData;

const IMAGE_WIDTH: u32 = 1920;
const IMAGE_HEIGHT: u32 = 1080;
const IMAGE_SIZE: (u32, u32) = (IMAGE_WIDTH, IMAGE_HEIGHT);

const GRID_COLS: usize = 2;

#[derive(Debug, Clone)]
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

fn render_benchmark_to_png_bytes(
    kind: &PlotKind,
    database: &Database,
    benchmark: &BenchmarkData,
    series: &[BackendWithCommit],
    size: (u32, u32),
) -> Result<Vec<u8>, PlotError> {
    let tmp = NamedTempFile::with_suffix(".png")?;
    let path = tmp.path().to_string_lossy().to_string();
    let series_iter = series.iter().cloned().map(|s| (s.backend_name, s.commit, s.tag));
    match kind {
        PlotKind::Series { measurement_method, visualization_kind } => {
            let dataset: BenchmarkDataset<f64> =
                BenchmarkDataset::new(database, benchmark, series_iter, measurement_method)?;
            let plot = SeriesPlot::from_dataset(
                dataset,
                benchmark.name.clone(),
                *visualization_kind,
                measurement_method.y_axis_label(),
            )?;
            plot.plot(BitMapBackend::new(&path, size))?;
        }
        PlotKind::PerfStat { events } => {
            let dataset: BenchmarkDataset<PerfStatData> =
                BenchmarkDataset::new(database, benchmark, series_iter, &MeasurementMethod::Perf)?;
            let plot =
                PerfStatPlot::from_dataset(dataset, benchmark.name.clone(), events.clone())?;
            plot.plot(BitMapBackend::new(&path, size))?;
        }
        PlotKind::FlameGraph { .. } => {
            return Err(PlotError::Internal(
                "FlameGraph plots cannot be combined into a grid".to_owned(),
            ));
        }
    }
    Ok(std::fs::read(tmp.path())?)
}

fn render_benchmark_to_svg_string(
    kind: &PlotKind,
    database: &Database,
    benchmark: &BenchmarkData,
    series: &[BackendWithCommit],
    size: (u32, u32),
) -> Result<String, PlotError> {
    let tmp = NamedTempFile::with_suffix(".svg")?;
    let path = tmp.path().to_string_lossy().to_string();
    let series_iter = series.iter().cloned().map(|s| (s.backend_name, s.commit, s.tag));
    match kind {
        PlotKind::Series { measurement_method, visualization_kind } => {
            let dataset: BenchmarkDataset<f64> =
                BenchmarkDataset::new(database, benchmark, series_iter, measurement_method)?;
            let plot = SeriesPlot::from_dataset(
                dataset,
                benchmark.name.clone(),
                *visualization_kind,
                measurement_method.y_axis_label(),
            )?;
            plot.plot(SVGBackend::new(&path, size))?;
        }
        PlotKind::PerfStat { events } => {
            let dataset: BenchmarkDataset<PerfStatData> =
                BenchmarkDataset::new(database, benchmark, series_iter, &MeasurementMethod::Perf)?;
            let plot =
                PerfStatPlot::from_dataset(dataset, benchmark.name.clone(), events.clone())?;
            plot.plot(SVGBackend::new(&path, size))?;
        }
        PlotKind::FlameGraph { .. } => {
            return Err(PlotError::Internal(
                "FlameGraph plots cannot be combined into a grid".to_owned(),
            ));
        }
    }
    Ok(std::fs::read_to_string(tmp.path())?)
}

/// Positions an SVG at `(x, y)` by injecting `x`/`y` attributes into its root `<svg>` element
/// and stripping any leading `<?xml?>` declaration. Used for nested-SVG grid composition.
fn nest_svg(svg: &str, x: u32, y: u32) -> String {
    let start = svg.find("<svg").unwrap_or(0);
    let svg = &svg[start..];
    let tag_end = svg.find('>').unwrap_or(svg.len());
    format!("{} x=\"{x}\" y=\"{y}\"{}",  &svg[..tag_end], &svg[tag_end..])
}

fn plot_grid_png(
    kind: &PlotKind,
    database: &Database,
    benchmarks: Vec<BenchmarkData>,
    series: &[BackendWithCommit],
    output: &str,
) -> Result<(), PlotError> {
    let rows = benchmarks.len().div_ceil(GRID_COLS) as u32;
    let total_w = IMAGE_WIDTH * GRID_COLS as u32;
    let total_h = IMAGE_HEIGHT * rows;
    info!(
        "Rendering {}-benchmark PNG grid ({total_w}x{total_h}) to {output}...",
        benchmarks.len()
    );

    let mut canvas =
        image::RgbImage::from_fn(total_w, total_h, |_, _| image::Rgb([255u8, 255, 255]));

    for (i, benchmark) in benchmarks.into_iter().enumerate() {
        let png_bytes =
            render_benchmark_to_png_bytes(kind, database, &benchmark, series, IMAGE_SIZE)?;
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| PlotError::Internal(e.to_string()))?
            .to_rgb8();
        let col = (i % GRID_COLS) as u32;
        let row = (i / GRID_COLS) as u32;
        image::imageops::overlay(
            &mut canvas,
            &img,
            (col * IMAGE_WIDTH) as i64,
            (row * IMAGE_HEIGHT) as i64,
        );
    }

    canvas.save(output).map_err(|e| PlotError::Internal(e.to_string()))
}

fn plot_grid_svg(
    kind: &PlotKind,
    database: &Database,
    benchmarks: Vec<BenchmarkData>,
    series: &[BackendWithCommit],
    output: &str,
) -> Result<(), PlotError> {
    let rows = benchmarks.len().div_ceil(GRID_COLS) as u32;
    let total_w = IMAGE_WIDTH * GRID_COLS as u32;
    let total_h = IMAGE_HEIGHT * rows;
    info!(
        "Rendering {}-benchmark SVG grid ({total_w}x{total_h}) to {output}...",
        benchmarks.len()
    );

    let mut result = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{total_w}\" height=\"{total_h}\">\n"
    );

    for (i, benchmark) in benchmarks.into_iter().enumerate() {
        let svg =
            render_benchmark_to_svg_string(kind, database, &benchmark, series, IMAGE_SIZE)?;
        let col = (i % GRID_COLS) as u32;
        let row = (i / GRID_COLS) as u32;
        result.push_str(&nest_svg(&svg, col * IMAGE_WIDTH, row * IMAGE_HEIGHT));
        result.push('\n');
    }

    result.push_str("</svg>");
    std::fs::write(output, &result)?;
    Ok(())
}

fn plot_grid(
    plot_settings: PlotSettings,
    database: &Database,
    benchmarks: Vec<BenchmarkData>,
    series: Vec<BackendWithCommit>,
) -> Result<(), PlotError> {
    let output = &plot_settings.output;
    let extension = output.rsplit('.').next().unwrap_or("").to_owned();
    match extension.as_str() {
        "png" => plot_grid_png(&plot_settings.plot_kind, database, benchmarks, &series, output),
        "svg" => plot_grid_svg(&plot_settings.plot_kind, database, benchmarks, &series, output),
        _ => Err(PlotError::IncompatibleOutputFormat {
            format: extension,
            plot: "grid".to_owned(),
        }),
    }
}

fn plot_single(
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

pub fn plot(
    plot_settings: PlotSettings,
    database: &Database,
    mut benchmarks: Vec<BenchmarkData>,
    series: impl Iterator<Item = BackendWithCommit>,
) -> Result<(), PlotError> {
    let series: Vec<BackendWithCommit> = series.collect();
    if benchmarks.len() == 1 {
        let benchmark_config = benchmarks.remove(0);
        plot_single(plot_settings, database, benchmark_config, series.into_iter())
    } else if benchmarks.is_empty() {
        Ok(())
    } else {
        plot_grid(plot_settings, database, benchmarks, series)
    }
}
