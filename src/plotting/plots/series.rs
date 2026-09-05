use plotters::drawing::DrawingArea;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use crate::plotting::core::{
    BACKGROUND_COLOR, BackendWithKind, BenchmarkDataset, LABEL_FONT, LEGEND_AREA_SIZE,
    LEGEND_BORDER_COLOR, LEGEND_BORDER_SIZE, LEGEND_FONT, LEGEND_MARGIN, LinearSeries, LogSeries,
    MARGIN_RIGHT, MARGIN_SIZE, MARGIN_TOP, Plot, Renderable, RenderableSeries, SeriesValue,
    TICK_FONT, TITLE_FONT, TITLE_MARGIN_TOP, ValueTransformation, X_LABEL_AREA_SIZE,
    Y_LABEL_AREA_SIZE,
};
use crate::plotting::{PlotError, VisKind};
use crate::utilities::{calc_min_max, pad_y_range};

/// Vertical gap between the axis and the group labels drawn under it.
const GROUP_LABEL_OFFSET: i32 = 12;

pub struct SeriesPlot {
    benchmark_name: String,
    results: Vec<RenderableSeries>,
    visualization_kind: VisKind,
    y_label: String,
}

impl SeriesPlot {
    fn new(
        benchmark_name: String,
        results: Vec<RenderableSeries>,
        visualization_kind: VisKind,
        y_label: String,
    ) -> Self {
        SeriesPlot {
            benchmark_name,
            results,
            visualization_kind,
            y_label,
        }
    }

    pub fn from_dataset<T: SeriesValue>(
        dataset: BenchmarkDataset<T>,
        benchmark_name: String,
        visualization_kind: VisKind,
        y_label: impl Into<String>,
    ) -> Result<Self, PlotError> {
        let y_label = match visualization_kind {
            // The point is not a query count in every benchmark (ser/deser run
            // point^2 queries), so the unit stays generic.
            VisKind::Throughput => "Throughput [points/s]".to_owned(),
            _ => y_label.into(),
        };
        let series_count = dataset.names.len();
        let mut results = Vec::new();

        for (id, (name, (series_values, raw_std_devs))) in dataset
            .names
            .iter()
            .zip(dataset.results.into_iter().zip(dataset.std_devs.into_iter()))
            .enumerate()
        {
            let (series, range) = match visualization_kind {
                VisKind::Throughput => {
                    // The recorded value is a duration; the throughput it stands
                    // for is the benchmark point divided by it. A non-positive
                    // duration would divide to an infinity, so it counts as
                    // missing instead.
                    let throughput: Vec<Option<f64>> = dataset
                        .points
                        .iter()
                        .zip(series_values.iter())
                        .map(|(point, value)| {
                            let seconds: Option<f64> = value.clone().map(Into::into);
                            seconds.filter(|s| *s > 0.0).map(|s| *point as f64 / s)
                        })
                        .collect();
                    let range =
                        calc_min_max(throughput.iter().filter_map(|&v| v.map(|val| (val, val))));
                    (throughput, range)
                }
                VisKind::Linear | VisKind::Log => {
                    let series: ValueTransformation<T> = match visualization_kind {
                        VisKind::Log => ValueTransformation::Log(LogSeries { y: series_values }),
                        _ => ValueTransformation::Linear(LinearSeries { y: series_values }),
                    };
                    (series.series()?, series.range()?)
                }
            };

            // Compute error bar bounds in chart coordinates from linear-space std devs.
            let error_bars: Vec<Option<(f64, f64)>> = series
                .iter()
                .copied()
                .zip(raw_std_devs.iter().copied())
                .map(|(v_opt, d_opt)| match (v_opt, d_opt) {
                    (Some(v), Some(d)) => match visualization_kind {
                        VisKind::Linear => Some((v - d, v + d)),
                        // Deliberately none yet: under the reciprocal the bounds
                        // become asymmetric, which is a follow-up.
                        VisKind::Throughput => None,
                        VisKind::Log => {
                            // v is log10(original); back-transform to apply stddev in linear space
                            let original = 10f64.powf(v);
                            let lo = (original - d).max(f64::EPSILON).log10();
                            let hi = (original + d).log10();
                            Some((lo, hi))
                        }
                    },
                    _ => None,
                })
                .collect();

            // Expand chart y-range to encompass the error bar extremes.
            let expanded_range = {
                let mut bounds = range;
                for (lo, hi) in error_bars.iter().flatten() {
                    bounds = Some(match bounds {
                        None => (*lo, *hi),
                        Some((b_lo, b_hi)) => (b_lo.min(*lo), b_hi.max(*hi)),
                    });
                }
                bounds
            };

            let color = Palette99::pick(id);

            let mut renderable = RenderableSeries::new(
                name.clone(),
                dataset.points.clone(),
                series,
                error_bars,
                color,
                expanded_range,
            );
            if visualization_kind == VisKind::Throughput {
                renderable.draw_as_column(id, series_count);
            }
            results.push(renderable);
        }

        Ok(SeriesPlot::new(benchmark_name, results, visualization_kind, y_label))
    }
}

impl Plot for SeriesPlot {
    fn plot<DB: DrawingBackend + BackendWithKind>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        let root = DrawingArea::from(backend);
        root.fill(&BACKGROUND_COLOR)?;
        let (_, root) = root.split_vertically(TITLE_MARGIN_TOP);

        let columns = self.visualization_kind == VisKind::Throughput;
        let group_points: Vec<u64> = self
            .results
            .first()
            .map(|r| r.points.clone())
            .unwrap_or_default();

        // Columns are laid out in uniform slots rather than at the benchmark
        // point values, which grow multiplicatively and would crowd together.
        let (x_start, x_end) = if columns {
            let width = RenderableSeries::group_width(self.results.len());
            (0, (group_points.len() as u64) * width)
        } else {
            (
                *self
                    .results
                    .first()
                    .and_then(|r| r.points.first())
                    .unwrap_or(&0),
                *self
                    .results
                    .first()
                    .and_then(|r| r.points.last())
                    .unwrap_or(&1),
            )
        };
        let (y_min, y_max) = calc_min_max(
            self.results
                .iter()
                .filter_map(super::super::core::render::RenderableSeries::range),
        )
        .map(|(lo, hi)| pad_y_range(lo, hi))
        .unwrap_or((0.0, 1.0));
        // Columns are read by their height, so they must stand on zero.
        let y_min = if columns { 0.0 } else { y_min };

        let log_text = match self.visualization_kind {
            VisKind::Log => " (log scale)",
            VisKind::Throughput => " (throughput)",
            VisKind::Linear => "",
        };

        let plot_area = root.titled(
            &format!("Benchmark {} Results{}", &self.benchmark_name, log_text),
            TITLE_FONT,
        )?;

        let mut chart = ChartBuilder::on(&plot_area)
            .margin_top(MARGIN_TOP)
            .margin_right(MARGIN_RIGHT)
            .margin_bottom(MARGIN_SIZE)
            .margin_left(MARGIN_SIZE)
            .x_label_area_size(X_LABEL_AREA_SIZE)
            .y_label_area_size(Y_LABEL_AREA_SIZE)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        let mut mesh = chart.configure_mesh();
        mesh.label_style(LABEL_FONT)
            .y_labels(5)
            .y_desc(&self.y_label)
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
            .x_label_style(TICK_FONT);
        if columns {
            // Slot numbers mean nothing to the reader; each group is labelled
            // with its benchmark point below, once the geometry is known.
            mesh.disable_x_mesh().x_label_formatter(&|_| String::new());
        }
        mesh.draw()?;

        let mut charts: [ChartContext<_, _>; 1] = [chart];
        for series in &self.results {
            series.add_to_plot(&mut charts)?;
        }

        if columns {
            // One label per group, centred under its columns. Drawn by hand
            // because the axis is indexed in slots, so plotters' own ticks
            // would land between groups rather than under them.
            let width = RenderableSeries::group_width(self.results.len());
            let (base_x, base_y) = plot_area.get_base_pixel();
            for (group_index, point) in group_points.iter().enumerate() {
                let first_slot = (group_index as u64) * width;
                let left = charts[0].backend_coord(&(first_slot, y_min)).0;
                let right = charts[0]
                    .backend_coord(&(first_slot + width - 1, y_min))
                    .0;
                let baseline = charts[0].backend_coord(&(first_slot, y_min)).1;
                plot_area.draw(&Text::new(
                    format!("{point}"),
                    (
                        (left + right) / 2 - base_x,
                        baseline - base_y + GROUP_LABEL_OFFSET,
                    ),
                    TextStyle::from(TICK_FONT.into_font())
                        .pos(Pos::new(HPos::Center, VPos::Top)),
                ))?;
            }
        }

        charts[0]
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft)
            .border_style(LEGEND_BORDER_COLOR.stroke_width(LEGEND_BORDER_SIZE))
            .background_style(BACKGROUND_COLOR)
            .margin(LEGEND_MARGIN)
            .label_font(LEGEND_FONT)
            .legend_area_size(LEGEND_AREA_SIZE)
            .draw()?;

        root.present()?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "series plot"
    }
}
