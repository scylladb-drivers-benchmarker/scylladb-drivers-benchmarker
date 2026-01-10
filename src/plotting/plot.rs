use std::collections::HashMap;

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

use plotters::coord::Shift;
use plotters::drawing::DrawingArea;
use plotters::prelude::*;

const MARGIN_SIZE: u32 = 20;
const X_LABEL_AREA_SIZE: u32 = 40;
const Y_LABEL_AREA_SIZE: u32 = 70;

const FONT_FAMILY: &str = "sans-serif";
const TITLE_FONT_SIZE: u32 = 40;
const TITLE_FONT: (&str, u32) = (FONT_FAMILY, TITLE_FONT_SIZE);
const CAPTION_FONT_SIZE: u32 = 24;
const CAPTION_FONT: (&str, u32) = (FONT_FAMILY, CAPTION_FONT_SIZE);
const LABEL_FONT_SIZE: u32 = 16;
const LABEL_FONT: (&str, u32) = (FONT_FAMILY, LABEL_FONT_SIZE);

const BACKGROUND_COLOR: RGBColor = WHITE;
const LEGEND_BORDER_COLOR: RGBColor = BLACK;

const LEGEND_BORDER_SIZE: u32 = 1;

pub(crate) trait Plot {
    fn plot<DB: DrawingBackend + NamedBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static;
}

// A very hacky way to differentiate backends, necessary as text has different pixel size.
pub(crate) trait NamedBackend {
    fn name(&self) -> &'static str;
}

impl<'a> NamedBackend for BitMapBackend<'a> {
    fn name(&self) -> &'static str {
        "bitmap"
    }
}

impl<'a> NamedBackend for SVGBackend<'a> {
    fn name(&self) -> &'static str {
        "svg"
    }
}

pub(crate) struct SeriesPlot {
    pub benchmark_name: String,
    pub results: Vec<RenderableSeries>,
    pub visualization_kind: VisKind,
}

pub(crate) struct PerfStatPlot {
    pub benchmark_name: String,
    pub events: Vec<String>,
    pub units: Vec<String>,
    pub results: Vec<RenderablePerfStat>,
}

impl SeriesPlot {
    fn new(
        benchmark_name: String,
        results: Vec<RenderableSeries>,
        visualization_kind: VisKind,
    ) -> Self {
        SeriesPlot {
            benchmark_name,
            results,
            visualization_kind,
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

        Ok(SeriesPlot::new(benchmark_name, results, visualization_kind))
    }

    pub(crate) fn build(
        database: &Database,
        benchmark_config: BenchmarkConfig,
        measurement_method: &MeasurementMethod,
        commit_hashes: impl Iterator<Item = CommitHash>,
        names: &[String],
        vis_kind: VisKind,
    ) -> Result<Self, PlotError> {
        let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
            database,
            &benchmark_config,
            commit_hashes,
            measurement_method,
        )?;

        SeriesPlot::from_dataset(dataset, benchmark_config.name, names, vis_kind)
    }
}

impl Plot for SeriesPlot {
    fn plot<DB: DrawingBackend + NamedBackend>(&self, backend: DB) -> Result<(), PlotError>
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

        let log_text = match self.visualization_kind {
            VisKind::Log => " (log scale)",
            _ => "",
        };

        let plot_area = root.titled(
            &format!("Benchmark {} Results{}", &self.benchmark_name, log_text),
            TITLE_FONT,
        )?;

        let mut chart = ChartBuilder::on(&plot_area)
            .margin(MARGIN_SIZE)
            .x_label_area_size(X_LABEL_AREA_SIZE)
            .y_label_area_size(Y_LABEL_AREA_SIZE)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        chart
            .configure_mesh()
            .label_style(LABEL_FONT)
            .y_desc("Benchmark value")
            .y_label_style(LABEL_FONT)
            .x_desc("Input size")
            .x_label_style(LABEL_FONT)
            .draw()?;

        let mut charts: [ChartContext<_, _>; 1] = [chart];
        for series in &self.results {
            series.add_to_plot(&mut charts)?;
        }

        charts[0]
            .configure_series_labels()
            .position(SeriesLabelPosition::MiddleRight)
            .border_style(LEGEND_BORDER_COLOR.stroke_width(LEGEND_BORDER_SIZE))
            .background_style(BACKGROUND_COLOR)
            .draw()?;

        root.present()?;

        Ok(())
    }
}

impl PerfStatPlot {
    const LEGEND_MARGIN_WIDTH: i32 = 10; // outer right margin width
    const LEGEND_PADDING_X: i32 = 10; // inner horizontal padding
    const LEGEND_PADDING_Y: i32 = 10; // inner vertical padding
    const LEGEND_MARKER_WIDTH: i32 = 10; // color rectangle width
    const LEGEND_MARKER_HEIGHT: i32 = 10; // color rectangle height
    const LEGEND_MARKER_TEXT_GAP: i32 = 5; // gap between marker and text
    const LEGEND_ENTRY_SPACING: i32 = 8; // vertical gap between legend entries
    const LEGEND_CHAR_HEIGHT: i32 = LABEL_FONT_SIZE as i32; // legend text character height

    fn new(
        benchmark_name: String,
        events: Vec<String>,
        units: Vec<String>,
        results: Vec<RenderablePerfStat>,
    ) -> Self {
        PerfStatPlot {
            benchmark_name,
            events,
            units,
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

        let mut unit: HashMap<String, Result<String, ()>> = HashMap::new();

        for (id, (name, values)) in names.iter().zip(dataset.results.into_iter()).enumerate() {
            for event_name in &events {
                let any_data = values.iter().any(|d| {
                    d.as_ref()
                        .and_then(|perf| perf.filter_value(event_name))
                        .is_some()
                });

                if !any_data {
                    return Err(PlotError::InvalidData(format!(
                        "Event '{}' not found in dataset",
                        event_name
                    )));
                }
            }

            let values_per_event: Vec<Vec<Option<f64>>> = events
                .iter()
                .map(|event_name| {
                    values
                        .iter()
                        .map(|data| {
                            data.as_ref().and_then(|perfstat| {
                                perfstat.filter_value(event_name).map(|e| {
                                    unit.entry(event_name.clone())
                                        .and_modify(|existing| {
                                            if let Ok(existing_unit) = existing
                                                && existing_unit != &e.unit
                                            {
                                                *existing = Err(());
                                            }
                                        })
                                        .or_insert_with(|| Ok(e.unit.clone()));

                                    e.value
                                })
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

        let mut units_per_event = Vec::new();

        for event in &events {
            match unit.get(event) {
                Some(Ok(u)) => units_per_event.push(u.clone()),
                Some(Err(_)) => {
                    return Err(PlotError::InvalidData(format!(
                        "different units for event {}",
                        event
                    )));
                }
                None => {
                    units_per_event.push(String::new());
                }
            }
        }

        Ok(PerfStatPlot::new(
            benchmark_name,
            events,
            units_per_event,
            results,
        ))
    }

    pub(crate) fn build(
        database: &Database,
        benchmark_config: BenchmarkConfig,
        measurement_method: &MeasurementMethod,
        commit_hashes: impl Iterator<Item = CommitHash>,
        names: &[String],
        events: Vec<String>,
    ) -> Result<Self, PlotError> {
        let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset::new(
            database,
            &benchmark_config,
            commit_hashes,
            measurement_method,
        )?;

        PerfStatPlot::from_dataset(dataset, benchmark_config.name, names, events)
    }

    fn add_legend<DB: NamedBackend + DrawingBackend>(
        &self,
        backend: &str,
        area: DrawingArea<DB, Shift>,
    ) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let (plot_width, plot_height) = area.dim_in_pixel();

        // For some reason this does not work correctly for svg
        let max_label_width = self
            .results
            .iter()
            .map(|r| {
                let style = TextStyle::from(LABEL_FONT.into_font());
                area.estimate_text_size(&r.name, &style)
                    .expect("failed to estimate text size")
                    .0 // width
            })
            .max()
            .unwrap_or(50);

        // So we scale it in this terrible, hacky, heuristic way
        let scale_factor = match backend {
            "svg" => 0.9f64,
            _ => 1.0f64,
        };

        let entry_height = Self::LEGEND_MARKER_HEIGHT.max(Self::LEGEND_CHAR_HEIGHT);

        let legend_width = Self::LEGEND_PADDING_X * 2
            + Self::LEGEND_MARKER_WIDTH
            + Self::LEGEND_MARKER_TEXT_GAP
            + (max_label_width as f64 * scale_factor) as i32;

        let legend_height = Self::LEGEND_PADDING_Y * 2
            + self.results.len() as i32 * entry_height
            + (self.results.len() as i32 - 1) * Self::LEGEND_ENTRY_SPACING;

        let legend_left = plot_width as i32 - legend_width - Self::LEGEND_MARGIN_WIDTH;
        let legend_top = (plot_height as i32 - legend_height) / 2;

        let legend_rect = [
            (legend_left, legend_top),
            (legend_left + legend_width, legend_top + legend_height),
        ];

        // background
        area.draw(&Rectangle::new(legend_rect, BACKGROUND_COLOR.filled()))?;

        // border
        area.draw(&Rectangle::new(
            legend_rect,
            LEGEND_BORDER_COLOR.stroke_width(LEGEND_BORDER_SIZE),
        ))?;

        let mut y = legend_top + Self::LEGEND_PADDING_Y;
        for r in &self.results {
            area.draw(&Rectangle::new(
                [
                    (legend_left + Self::LEGEND_PADDING_X, y),
                    (
                        legend_left + Self::LEGEND_PADDING_X + Self::LEGEND_MARKER_WIDTH,
                        y + Self::LEGEND_MARKER_HEIGHT,
                    ),
                ],
                r.color.filled(),
            ))?;

            area.draw(&Text::new(
                r.name.clone(),
                (
                    legend_left
                        + Self::LEGEND_PADDING_X
                        + Self::LEGEND_MARKER_WIDTH
                        + Self::LEGEND_MARKER_TEXT_GAP,
                    y,
                ),
                LABEL_FONT,
            ))?;

            y += entry_height + Self::LEGEND_ENTRY_SPACING;
        }

        Ok(())
    }
}

impl Plot for PerfStatPlot {
    fn plot<DB: DrawingBackend + NamedBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let backend_name = backend.name();

        let root = DrawingArea::from(backend);
        root.fill(&BACKGROUND_COLOR)?;

        let plot_area = root.titled(
            &format!("Benchmark {} Results", &self.benchmark_name),
            TITLE_FONT,
        )?;

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
                let (y_min, y_max) =
                    calc_min_max(self.results.iter().filter_map(|r| r.ranges()[id]))
                        .unwrap_or((0.0, 1.0));

                let mut chart = ChartBuilder::on(&area)
                    .caption(self.events[id].clone(), CAPTION_FONT)
                    .margin(MARGIN_SIZE)
                    .x_label_area_size(X_LABEL_AREA_SIZE)
                    .y_label_area_size(Y_LABEL_AREA_SIZE)
                    .build_cartesian_2d(x_start..x_end, y_min..y_max)
                    .map_err(|e| PlotError::Plotters(e.to_string()))?;

                chart
                    .configure_mesh()
                    .label_style(LABEL_FONT)
                    .y_desc(format!(
                        "Value ({})",
                        if self.units[id].is_empty() {
                            "unknown unit"
                        } else {
                            &self.units[id]
                        }
                    ))
                    .y_label_style(LABEL_FONT)
                    .x_desc("Input size")
                    .x_label_style(LABEL_FONT)
                    .draw()?;

                Ok(chart)
            })
            .collect::<Result<Vec<_>, PlotError>>()?;

        for r in &self.results {
            r.add_to_plot(&mut charts)?;
        }

        self.add_legend(backend_name, plot_area)?;

        root.present()?;
        Ok(())
    }
}
