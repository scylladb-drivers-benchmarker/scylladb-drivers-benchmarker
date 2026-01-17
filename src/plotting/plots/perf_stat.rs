use std::collections::HashMap;

use crate::plotting::data::BenchmarkDataset;
use crate::plotting::error::PlotError;
use crate::plotting::plot::*;
use crate::plotting::render::{Renderable, RenderablePerfStat};

use crate::perf_stat::PerfStatData;
use crate::utilities::calc_min_max;

use plotters::coord::Shift;
use plotters::drawing::DrawingArea;
use plotters::prelude::*;

pub(crate) struct PerfStatPlot {
    benchmark_name: String,
    events: Vec<String>,
    units: Vec<String>,
    results: Vec<RenderablePerfStat>,
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

    fn add_legend<DB: BackendWithKind + DrawingBackend>(
        &self,
        scale_factor: f64,
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
        let text_width = max_label_width as f64 * scale_factor;

        let entry_height = Self::LEGEND_MARKER_HEIGHT.max(Self::LEGEND_CHAR_HEIGHT);

        let legend_width = Self::LEGEND_PADDING_X * 2
            + Self::LEGEND_MARKER_WIDTH
            + Self::LEGEND_MARKER_TEXT_GAP
            + text_width as i32;

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
    fn plot<DB: DrawingBackend + BackendWithKind>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let backend_kind = backend.kind();

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

        let scale_factor = match backend_kind {
            BackendKind::Svg => 0.9,
            _ => 1.0,
        };

        self.add_legend(scale_factor, plot_area)?;

        root.present()?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "perf-stat plot"
    }
}
