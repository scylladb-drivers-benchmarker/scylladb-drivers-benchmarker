use super::VisKind;

use super::BenchmarkConfig;
use super::CommitHash;
use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::render::{Renderable, RenderablePerfStat, RenderableSeries};
use super::series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, calc_min_max};
use crate::Database;
use crate::measurement::MeasurementMethod;
use crate::perf_stat::PerfStatData;

use plotters::prelude::*;

const MARGIN_SIZE: u32 = 10;
const X_LABEL_AREA_SIZE: u32 = 30;
const Y_LABEL_AREA_SIZE: u32 = 40;

const FONT_FAMILY: &str = "sans-serif";
const TITLE_FONT_SIZE: u32 = 40;
const TITLE_FONT: (&str, u32) = (FONT_FAMILY, TITLE_FONT_SIZE);
const CAPTION_FONT_SIZE: u32 = 24;
const CAPTION_FONT: (&str, u32) = (FONT_FAMILY, CAPTION_FONT_SIZE);

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

pub(crate) struct PerfStatPlot {
    pub benchmark_name: String,
    pub events: Vec<String>,
    pub results: Vec<RenderablePerfStat>,
}

impl SeriesPlot {
    fn new(benchmark_name: String, results: Vec<RenderableSeries>) -> Self {
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

    pub(crate) fn build(
        database: &Database,
        benchmark_name: &str,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &MeasurementMethod,
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

        let plot_area = root.titled(
            &format!("Benchmark {} Results", &self.benchmark_name),
            TITLE_FONT,
        )?;

        let mut chart = ChartBuilder::on(&plot_area)
            .margin(MARGIN_SIZE)
            .x_label_area_size(X_LABEL_AREA_SIZE)
            .y_label_area_size(Y_LABEL_AREA_SIZE)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        chart.configure_mesh().draw()?;

        let mut charts: [ChartContext<_, _>; 1] = [chart];
        for series in &self.results {
            series.add_to_plot(&mut charts)?;
        }

        charts[0]
            .configure_series_labels()
            .position(SeriesLabelPosition::MiddleRight)
            .border_style(LEGEND_BORDER_COLOR)
            .background_style(BACKGROUND_COLOR)
            .draw()?;

        root.present()?;

        Ok(())
    }
}

impl PerfStatPlot {
    fn new(benchmark_name: String, events: Vec<String>, results: Vec<RenderablePerfStat>) -> Self {
        PerfStatPlot {
            benchmark_name,
            events,
            results,
        }
    }

    pub(crate) fn from_dataset(
        dataset: BenchmarkDataset<PerfStatData>,
        benchmark_name: String,
        names: &[String],
        events: Vec<String>,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        for (id, (name, values)) in names.iter().zip(dataset.results.into_iter()).enumerate() {
            let values_per_event: Vec<Vec<Option<f64>>> = events
                .iter()
                .map(|event_name| {
                    values
                        .iter()
                        .map(|data| {
                            data.as_ref().and_then(|perfstat| {
                                perfstat.filter_value(event_name).map(|e| e.value)
                            })
                        })
                        .collect::<Vec<Option<f64>>>()
                })
                .collect();

            let color = Palette99::pick(id);

            let ranges = values_per_event
                .iter()
                .map(|vals| calc_min_max(vals.iter().filter_map(|&v| v.map(|val| (val, val)))))
                .collect();

            results.push(RenderablePerfStat::new(
                name.clone(),
                dataset.points.clone(),
                values_per_event,
                color,
                ranges,
            ));
        }

        Ok(PerfStatPlot::new(benchmark_name, events, results))
    }

    pub(crate) fn build(
        database: &Database,
        benchmark_name: &str,
        benchmark_config: &BenchmarkConfig,
        measurement_method: &MeasurementMethod,
        commit_hashes: impl Iterator<Item = CommitHash>,
        names: &[String],
        events: Vec<String>,
    ) -> Result<Self, PlotError> {
        let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset::new(
            database,
            benchmark_config,
            commit_hashes,
            measurement_method,
        )?;

        PerfStatPlot::from_dataset(dataset, benchmark_name.to_string(), names, events)
    }
}

impl Plot for PerfStatPlot {
    fn plot<DB: DrawingBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let root = DrawingArea::from(backend);
        root.fill(&BACKGROUND_COLOR)?;

        let (title_legend_area, plot_area) = root.split_vertically(20);

        let legend_area = title_legend_area.titled(
            &format!("Benchmark {} Results", &self.benchmark_name),
            TITLE_FONT,
        )?;

        // Somehow plot legend_area; has to be done experimentally, currently no way to do it properly.
        // For now each subplot has its own legend.

        let subareas = plot_area.split_evenly((self.events.len(), 1));

        let mut charts: Vec<_> = subareas
            .into_iter()
            .enumerate()
            .map(|(id, area)| {
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
                let(y_min, y_max) = calc_min_max(
                    self.results
                        .iter()
                        .filter_map(|r| r.ranges()[id])
                ).unwrap_or((0.0, 1.0));

                let mut chart = ChartBuilder::on(&area)
                    .caption(self.events[id].clone(), CAPTION_FONT)
                    .margin(MARGIN_SIZE)
                    .x_label_area_size(X_LABEL_AREA_SIZE)
                    .y_label_area_size(Y_LABEL_AREA_SIZE)
                    .build_cartesian_2d(x_start..x_end, y_min..y_max)
                    .map_err(|e| PlotError::Plotters(e.to_string()))?;

                chart.configure_mesh().draw()?;

                Ok(chart)
            })
            .collect::<Result<Vec<_>, PlotError>>()?;

        for r in &self.results {
            r.add_to_plot(&mut charts)?;
        }

        for chart in &mut charts {
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::MiddleRight)
                .border_style(LEGEND_BORDER_COLOR)
                .background_style(BACKGROUND_COLOR)
                .draw()?;
        }

        root.present()?;
        Ok(())
    }
}
