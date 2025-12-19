use super::VisKind;

use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::render::{Renderable, RenderableSeries};
use super::series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, calc_min_max};

use plotters::prelude::*;

const IMAGE_SIZE: (u32, u32) = (1024, 768);
const MARGIN_SIZE: u32 = 10;
const X_LABEL_AREA_SIZE: u32 = 30;
const Y_LABEL_AREA_SIZE: u32 = 40;

const FONT_FAMILY: &str = "sans-serif";
const FONT_SIZE: u32 = 40;
const FONT: (&str, u32) = (FONT_FAMILY, FONT_SIZE);

const BACKGROUND_COLOR: RGBColor = WHITE;
const LEGEND_BORDER_COLOR: RGBColor = BLACK;

pub(crate) trait Plot {
    fn plot(&self) -> Result<(), PlotError>;
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
}

impl Plot for SeriesPlot {
    fn plot(&self) -> Result<(), PlotError> {
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

        let root = BitMapBackend::new("test.png", IMAGE_SIZE).into_drawing_area();
        root.fill(&BACKGROUND_COLOR)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Benchmark {} Results", &self.benchmark_name), FONT)
            .margin(MARGIN_SIZE)
            .x_label_area_size(X_LABEL_AREA_SIZE)
            .y_label_area_size(Y_LABEL_AREA_SIZE)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        chart.configure_mesh().draw()?;

        for series in self.results.iter() {
            series.add_to_plot(&mut chart)?;
        }

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::MiddleRight)
            .border_style(LEGEND_BORDER_COLOR)
            .background_style(BACKGROUND_COLOR)
            .draw()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Clone, Debug, PartialOrd, Deserialize)]
    struct Dummy(f64);

    impl From<Dummy> for f64 {
        fn from(val: Dummy) -> Self {
            val.0
        }
    }

    impl PartialEq for Dummy {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    #[test]
    fn series_plot_runs() {
        let dataset = BenchmarkDataset {
            points: vec![1, 2, 3],
            results: vec![
                vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
                vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
            ],
        };

        let names = vec!["first".to_string(), "second".to_string()];
        let plot = SeriesPlot::from_dataset(
            dataset,
            "TestBenchmark".to_string(),
            &names,
            VisKind::Linear,
        )
        .unwrap();

        let result = plot.plot();
        assert!(result.is_ok());

        // As of now, plot does not accept a path.
        std::fs::remove_file("test.png").unwrap();
    }
}
