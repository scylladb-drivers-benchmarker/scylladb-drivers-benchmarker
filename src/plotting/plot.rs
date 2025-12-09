use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::render::{Renderable, RenderableSeries};
use super::series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, VisKind};

use plotters::prelude::*;

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

        let (mut y_min, mut y_max) = self.results.iter().filter_map(|r| r.range()).fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min_acc, max_acc), (min, max)| (min_acc.min(min), max_acc.max(max)),
        );

        if !y_min.is_finite() {
            y_min = 0.0;
        }
        if !y_max.is_finite() {
            y_max = 1.0;
        }

        let root = BitMapBackend::new("test.png", (1024, 768)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("Benchmark {} Results", &self.benchmark_name),
                ("sans-serif", 40),
            )
            .margin(10)
            .x_label_area_size(30)
            .y_label_area_size(40)
            .build_cartesian_2d(x_start..x_end, y_min..y_max)?;

        chart.configure_mesh().draw()?;

        for series in self.results.iter() {
            series.add_to_plot(&mut chart)?;
        }

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::MiddleRight)
            .border_style(BLACK)
            .background_style(WHITE.mix(0.8))
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

    impl Into<f64> for Dummy {
        fn into(self) -> f64 {
            self.0
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
