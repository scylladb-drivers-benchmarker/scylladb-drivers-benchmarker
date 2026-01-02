use super::VisKind;

use super::BenchmarkConfig;
use super::CommitHash;
use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::render::{Renderable, RenderableSeries};
use super::series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, calc_min_max};
use crate::Database;

use plotters::prelude::*;

const MARGIN_SIZE: u32 = 10;
const X_LABEL_AREA_SIZE: u32 = 30;
const Y_LABEL_AREA_SIZE: u32 = 40;

const FONT_FAMILY: &str = "sans-serif";
const FONT_SIZE: u32 = 40;
const FONT: (&str, u32) = (FONT_FAMILY, FONT_SIZE);

const BACKGROUND_COLOR: RGBColor = WHITE;
const LEGEND_BORDER_COLOR: RGBColor = BLACK;

pub(crate) trait Plot {
    fn plot<DB: DrawingBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static;
}

pub(crate) struct SeriesPlot {
    pub benchmark_name: String,
    pub results: Vec<RenderableSeries>,
}

impl SeriesPlot {
    pub(crate) fn new(benchmark_name: String, results: Vec<RenderableSeries>) -> Self {
        SeriesPlot {
            benchmark_name,
            results,
        }
    }

    pub(crate) fn from_dataset<T: SeriesValue>(
        dataset: BenchmarkDataset<T>,
        benchmark_name: String,
        names: &[String],
        visualization_kind: VisKind,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        for (id, (name, series_values)) in names.iter().zip(dataset.results.into_iter()).enumerate()
        {
            let series: ValueTransformation<T> = match visualization_kind {
                VisKind::Linear => ValueTransformation::Linear(LinearSeries { y: series_values }),
                VisKind::Log => ValueTransformation::Log(LogSeries { y: series_values }),
            };

            let range = series.range()?;
            let series = series.series()?;

            let color = Palette99::pick(id);

            results.push(RenderableSeries::new(
                name.clone(),
                dataset.points.clone(),
                series,
                color,
                range,
            ));
        }

        Ok(SeriesPlot::new(benchmark_name, results))
    }
    pub fn build(
        database: &Database,
        benchmark_name: &str,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &str,
        commit_hashes: impl Iterator<Item = CommitHash>,
        names: &[String],
        vis_kind: VisKind,
    ) -> Result<Self, PlotError> {
        let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
            database,
            benchmark_config,
            commit_hashes,
            measurement_method,
        )?;

        SeriesPlot::from_dataset(dataset, benchmark_name.to_string(), names, vis_kind)
    }
}

impl Plot for SeriesPlot {
    fn plot<DB: DrawingBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let root = DrawingArea::from(backend);
        root.fill(&BACKGROUND_COLOR)?;

        let x_start = *self
            .results
            .first()
            .and_then(|r| r.points.first())
            .unwrap_or(&0);
        let x_end = *self
            .results
            .first()
            .and_then(|r| r.points.last())
            .unwrap_or(&1);
        let (y_min, y_max) =
            calc_min_max(self.results.iter().filter_map(|r| r.range())).unwrap_or((0.0, 1.0));

        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Benchmark {} Results", &self.benchmark_name), FONT)
            .margin(MARGIN_SIZE)
            .x_label_area_size(X_LABEL_AREA_SIZE)
            .y_label_area_size(Y_LABEL_AREA_SIZE)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        chart.configure_mesh().draw()?;

        for series in &self.results {
            series.add_to_plot(&mut chart)?;
        }

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::MiddleRight)
            .border_style(LEGEND_BORDER_COLOR)
            .background_style(BACKGROUND_COLOR)
            .draw()?;

        root.present()?;

        Ok(())
    }
}
