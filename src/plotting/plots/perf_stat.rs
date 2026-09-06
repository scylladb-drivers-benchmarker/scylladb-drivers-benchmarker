use std::collections::HashMap;

use plotters::coord::Shift;
use plotters::drawing::DrawingArea;
use plotters::prelude::*;

use crate::perf_stat::PerfStatData;
use crate::plotting::PlotError;
use crate::plotting::core::{
    BACKGROUND_COLOR, BackendKind, BackendWithKind, BenchmarkDataset, CAPTION_FONT,
    LABEL_FONT, LEGEND_BORDER_COLOR, LEGEND_BORDER_SIZE, MARGIN_RIGHT, MARGIN_SIZE,
    MARGIN_TOP, Plot,
    Renderable, RenderablePerfStat, TICK_FONT, TITLE_FONT, TITLE_MARGIN_TOP, X_LABEL_AREA_SIZE,
    Y_LABEL_AREA_SIZE,
};
use crate::utilities::{calc_min_max, pad_y_range};

pub struct PerfStatPlot {
    benchmark_name: String,
    events: Vec<String>,
    units: Vec<String>,
    results: Vec<RenderablePerfStat>,
}

impl PerfStatPlot {
    const LEGEND_PADDING_X: i32 = 20; // inner horizontal padding
    const LEGEND_PADDING_Y: i32 = 20; // inner vertical padding
    const LEGEND_MARKER_WIDTH: i32 = 30; // color rectangle width
    const LEGEND_MARKER_HEIGHT: i32 = 20; // color rectangle height
    const LEGEND_MARKER_TEXT_GAP: i32 = 10; // gap between marker and text
    const LEGEND_ENTRY_GAP: i32 = 60; // horizontal gap between entries
    const LEGEND_CHAR_HEIGHT: i32 = LABEL_FONT.1 as i32; // legend text character height
    const LEGEND_STRIP_HEIGHT: u32 = 100; // height of the dedicated legend row above charts

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

    pub fn from_dataset(
        dataset: BenchmarkDataset<PerfStatData>,
        benchmark_name: String,
        events: Vec<String>,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        let mut unit: HashMap<String, Result<String, ()>> = HashMap::new();

        for (id, (name, values)) in dataset.names.iter().zip(dataset.results.into_iter()).enumerate() {
            for event_name in &events {
                let any_data = values.iter().any(|d| {
                    d.as_ref()
                        .and_then(|perf| perf.filter_value(event_name))
                        .is_some()
                });

                if !any_data {
                    return Err(PlotError::InvalidData(format!(
                        "Event '{event_name}' not found in dataset"
                    )));
                }
            }

            let mut values_per_event = Vec::new();

            for event_name in &events {
                let mut values_for_event = Vec::new();

                for data in &values {
                    let event_data = data
                        .as_ref()
                        .and_then(|perfstat| perfstat.filter_value(event_name));

                    if let Some(e) = event_data {
                        unit.entry(event_name.clone())
                            .and_modify(|existing| {
                                if let Ok(existing_unit) = existing
                                    && existing_unit != &e.unit
                                {
                                    *existing = Err(())
                                }
                            })
                            .or_insert_with(|| Ok(e.unit.clone()));

                        values_for_event.push(Some(e.value));
                    } else {
                        values_for_event.push(None);
                    }
                }

                values_per_event.push(values_for_event);
            }

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
                Some(Err(())) => {
                    return Err(PlotError::InvalidData(format!(
                        "different units for event {event}"
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

    fn add_legend_strip<DB: BackendWithKind + DrawingBackend>(
        &self,
        scale_factor: f64,
        area: DrawingArea<DB, Shift>,
    ) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let (area_width, area_height) = area.dim_in_pixel();
        let entry_height = Self::LEGEND_MARKER_HEIGHT.max(Self::LEGEND_CHAR_HEIGHT);

        // Compute each entry's text width (scaled for the SVG estimation quirk)
        let text_widths: Vec<i32> = self
            .results
            .iter()
            .map(|r| {
                let style = TextStyle::from(LABEL_FONT.into_font());
                let w = area
                    .estimate_text_size(&r.name, &style)
                    .expect("failed to estimate text size")
                    .0;
                (f64::from(w) * scale_factor) as i32
            })
            .collect();

        let entry_widths: Vec<i32> = text_widths
            .iter()
            .map(|&tw| Self::LEGEND_MARKER_WIDTH + Self::LEGEND_MARKER_TEXT_GAP + tw)
            .collect();

        let total_content_width: i32 = entry_widths.iter().sum::<i32>()
            + Self::LEGEND_ENTRY_GAP * (self.results.len() as i32 - 1);

        // Center horizontally and vertically within the strip
        let mut x = (area_width as i32 - total_content_width) / 2;
        let y = (area_height as i32 - entry_height) / 2;

        // Bounding box with padding
        let box_rect = [
            (x - Self::LEGEND_PADDING_X, y - Self::LEGEND_PADDING_Y),
            (
                x + total_content_width + Self::LEGEND_PADDING_X,
                y + entry_height + Self::LEGEND_PADDING_Y,
            ),
        ];
        area.draw(&Rectangle::new(box_rect, BACKGROUND_COLOR.filled()))?;
        area.draw(&Rectangle::new(
            box_rect,
            LEGEND_BORDER_COLOR.stroke_width(LEGEND_BORDER_SIZE),
        ))?;

        for (r, &ew) in self.results.iter().zip(entry_widths.iter()) {
            let marker_y_offset = (entry_height - Self::LEGEND_MARKER_HEIGHT) / 2;
            area.draw(&Rectangle::new(
                [
                    (x, y + marker_y_offset),
                    (x + Self::LEGEND_MARKER_WIDTH, y + marker_y_offset + Self::LEGEND_MARKER_HEIGHT),
                ],
                r.color.filled(),
            ))?;

            area.draw(&Text::new(
                r.name.clone(),
                (x + Self::LEGEND_MARKER_WIDTH + Self::LEGEND_MARKER_TEXT_GAP, y),
                LABEL_FONT,
            ))?;

            x += ew + Self::LEGEND_ENTRY_GAP;
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
        let (_, root) = root.split_vertically(TITLE_MARGIN_TOP);

        let plot_area = root.titled(
            &format!("Benchmark {} Results", &self.benchmark_name),
            TITLE_FONT,
        )?;

        let (legend_strip, chart_area) = plot_area.split_vertically(Self::LEGEND_STRIP_HEIGHT);
        let subareas = chart_area.split_evenly((self.events.len(), 1));

        {
            let mut charts: Vec<_> = subareas
                .iter()
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
                    let (_, y_max) =
                        calc_min_max(self.results.iter().filter_map(|r| r.ranges()[id]))
                            .map(|(lo, hi)| pad_y_range(lo, hi))
                            .unwrap_or((0.0, 1.0));
                    let y_min = 0.0_f64;

                    let mut chart = ChartBuilder::on(area)
                        .caption(self.events[id].clone(), CAPTION_FONT)
                        .margin_top(MARGIN_TOP)
                        .margin_right(MARGIN_RIGHT)
                        .margin_bottom(MARGIN_SIZE)
                        .margin_left(MARGIN_SIZE)
                        .x_label_area_size(X_LABEL_AREA_SIZE)
                        .y_label_area_size(Y_LABEL_AREA_SIZE)
                        .build_cartesian_2d(x_start..x_end, y_min..y_max)
                        .map_err(|e| PlotError::Plotters(e.to_string()))?;

                    chart
                        .configure_mesh()
                        .label_style(LABEL_FONT)
                        .y_labels(5)
                        .y_desc(format!(
                            "Value ({})",
                            if self.units[id].is_empty() {
                                "count"
                            } else {
                                &self.units[id]
                            }
                        ))
                        .y_label_style(TICK_FONT)
                        .y_label_formatter(&|v| {
                            if v.abs() >= 1e6 {
                                let s = format!("{:.2e}", v);
                                let (mantissa, exp) = s.split_once('e').unwrap();
                                let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
                                format!("{mantissa}e{exp}")
                            } else {
                                format!("{}", v)
                            }
                        })
                        .x_desc("Input size")
                        .x_label_style(TICK_FONT)
                        .draw()?;

                    Ok(chart)
                })
                .collect::<Result<Vec<_>, PlotError>>()?;

            for r in &self.results {
                r.add_to_plot(&mut charts)?;
            }
        } // drop charts, releasing borrows on subareas

        let scale_factor = match backend_kind {
            BackendKind::Svg => 0.9,
            _ => 1.0,
        };

        self.add_legend_strip(scale_factor, legend_strip)?;

        root.present()?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "perf-stat plot"
    }
}
