use plotters::drawing::DrawingArea;
use plotters::prelude::*;

use crate::plotting::core::{
    BACKGROUND_COLOR, BackendWithKind, BenchmarkDataset, LABEL_FONT, LEGEND_AREA_SIZE,
    LEGEND_BORDER_COLOR, LEGEND_BORDER_SIZE, LEGEND_FONT, LEGEND_MARGIN, LinearSeries, LogSeries,
    MARGIN_RIGHT, MARGIN_SIZE, MARGIN_TOP, Plot, Renderable, RenderableSeries, SeriesValue,
    TICK_FONT, TITLE_FONT, TITLE_MARGIN_TOP, ValueTransformation, X_LABEL_AREA_SIZE,
    Y_LABEL_AREA_SIZE,
};
use crate::plotting::{PlotError, VisKind};
use crate::utilities::calc_min_max;

pub struct SeriesPlot {
    benchmark_name: String,
    results: Vec<RenderableSeries>,
    visualization_kind: VisKind,
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

    pub fn from_dataset<T: SeriesValue>(
        dataset: BenchmarkDataset<T>,
        benchmark_name: String,
        visualization_kind: VisKind,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        for (id, (name, (series_values, raw_std_devs))) in dataset
            .names
            .iter()
            .zip(dataset.results.into_iter().zip(dataset.std_devs.into_iter()))
            .enumerate()
        {
            let series: ValueTransformation<T> = match visualization_kind {
                VisKind::Linear => ValueTransformation::Linear(LinearSeries { y: series_values }),
                VisKind::Log => ValueTransformation::Log(LogSeries { y: series_values }),
            };

            let range = series.range()?;
            let series = series.series()?;

            // Compute error bar bounds in chart coordinates from linear-space std devs.
            let error_bars: Vec<Option<(f64, f64)>> = series
                .iter()
                .copied()
                .zip(raw_std_devs.iter().copied())
                .map(|(v_opt, d_opt)| match (v_opt, d_opt) {
                    (Some(v), Some(d)) => match visualization_kind {
                        VisKind::Linear => Some((v - d, v + d)),
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

            results.push(RenderableSeries::new(
                name.clone(),
                dataset.points.clone(),
                series,
                error_bars,
                color,
                expanded_range,
            ));
        }

        Ok(SeriesPlot::new(benchmark_name, results, visualization_kind))
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
        let (y_min, y_max) = calc_min_max(
            self.results
                .iter()
                .filter_map(super::super::core::render::RenderableSeries::range),
        )
        .unwrap_or((0.0, 1.0));

        let log_text = match self.visualization_kind {
            VisKind::Log => " (log scale)",
            _ => "",
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

        chart
            .configure_mesh()
            .label_style(LABEL_FONT)
            .y_labels(5)
            .y_desc("Benchmark value")
            .y_label_style(TICK_FONT)
            .x_desc("Input size")
            .x_label_style(TICK_FONT)
            .draw()?;

        let mut charts: [ChartContext<_, _>; 1] = [chart];
        for series in &self.results {
            series.add_to_plot(&mut charts)?;
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
