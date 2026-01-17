use crate::plotting::VisKind;
use crate::plotting::core::BenchmarkDataset;
use crate::plotting::core::{
    BACKGROUND_COLOR, BackendWithKind, LABEL_FONT, LEGEND_BORDER_COLOR, LEGEND_BORDER_SIZE,
    MARGIN_SIZE, Plot, TITLE_FONT, X_LABEL_AREA_SIZE, Y_LABEL_AREA_SIZE,
};
use crate::plotting::core::{LinearSeries, LogSeries, SeriesValue, ValueTransformation};
use crate::plotting::core::{Renderable, RenderableSeries};
use crate::plotting::error::PlotError;

use crate::utilities::calc_min_max;

use plotters::drawing::DrawingArea;
use plotters::prelude::*;

pub(crate) struct SeriesPlot {
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
}

impl Plot for SeriesPlot {
    fn plot<DB: DrawingBackend + BackendWithKind>(&self, backend: DB) -> Result<(), PlotError>
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

    fn name(&self) -> &'static str {
        "series plot"
    }
}
