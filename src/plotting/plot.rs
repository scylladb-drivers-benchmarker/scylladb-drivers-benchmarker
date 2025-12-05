use std::error::Error;
use super::data::PlotData;
use super::series::{SeriesKind, SeriesValue, LinearSeries, LogSeries, VisKind};
use super::result::{PlottableSeries, PlottableResult};

use plotters::prelude::*;

pub(crate) trait Plot {
    fn plot(&self) -> Result<(), Box<dyn Error>>;
}

pub(crate) struct SeriesPlot {
    pub benchmark_name: String,
    pub results: Vec<PlottableSeries>,
}

impl SeriesPlot {
    pub(crate) fn new(benchmark_name: String, results: Vec<PlottableSeries>) -> Self {
        SeriesPlot {
            benchmark_name,
            results,
        }
    }

    pub(crate) fn from_plot_data<T: SeriesValue>(
        plot_data: PlotData<T>,
        benchmark_name: String,
        names: &[String],
        visualization_kind: VisKind,
    ) -> Result<Self, Box<dyn Error>> {
        let mut results = Vec::new();

        for (id, (name, series_values)) in names
            .iter()
            .zip(plot_data.results.into_iter())
            .enumerate()
        {
            let series: SeriesKind<T> = match visualization_kind {
                VisKind::Linear => SeriesKind::Linear(LinearSeries { y: series_values }),
                VisKind::Log => SeriesKind::Log(LogSeries { y: series_values }),
            };

            let color = Palette99::pick(id);

            results.push(PlottableSeries::new(
                name.clone(),
                plot_data.points.clone(),
                series.series(),
                color,
                series.range(),
            ));
        }

        Ok(SeriesPlot::new(benchmark_name, results))
    }
}

impl Plot for SeriesPlot {
    fn plot(&self) -> Result<(), Box<dyn Error>> {
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

        Ok(())
    }
}