use std::error::Error;

use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::utilities::BenchmarkParams;

use serde::de::DeserializeOwned;

use std::fmt::Debug;

use plotters::coord::types::{RangedCoordf64, RangedCoordu32};
use plotters::prelude::*;

pub enum VisKind {
    Linear,
    Log,
}

pub trait PlottableValue: Sized + Debug + Clone {
    fn from_json(s: &str) -> Option<Self>;
}

impl<T> PlottableValue for T
where
    T: DeserializeOwned + Sized + Debug + Clone,
{
    fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

pub struct PlotData<T: PlottableValue> {
    pub points: Vec<u32>,
    pub results: Vec<Vec<Option<T>>>,
}

impl<T: PlottableValue> PlotData<T> {
    pub fn new(
        database: &Database,
        benchmark_config: &BenchmarkConfig,
        commit_hashes: &Vec<CommitHash>,
        measurement_method: String,
    ) -> Result<PlotData<T>, Box<dyn Error>> {
        let points = benchmark_config
            .data
            .benchmark_points()
            .collect::<Vec<u32>>();

        let results = commit_hashes
            .iter()
            .map(|commit_hash| {
                Self::get_benchmark_results(
                    database,
                    commit_hash,
                    benchmark_config,
                    measurement_method.clone(),
                )
            })
            .collect::<Result<Vec<Vec<Option<T>>>, Box<dyn Error>>>()?;

        Ok(PlotData { points, results })
    }

    fn get_benchmark_results(
        database: &Database,
        commit_hash: &CommitHash,
        benchmark_config: &BenchmarkConfig,
        measurement_method: String,
    ) -> Result<Vec<Option<T>>, Box<dyn Error>> {
        let BenchmarkConfig {
            name: benchmark_name,
            data: benchmark_data,
        } = benchmark_config;

        let benchmark_params = |param: u32| {
            BenchmarkParams::new(
                commit_hash.clone(),
                benchmark_name.clone(),
                param.into(),
                measurement_method.clone(),
            )
        };

        let results = benchmark_data
            .benchmark_points()
            .map(|point| -> Result<_, Box<dyn Error>> {
                let record = database.get_data(benchmark_params(point))?;

                let value = record
                    .and_then(|r| r.data_json)
                    .and_then(|text| T::from_json(&text));

                Ok(value)
            })
            .collect::<Result<Vec<Option<T>>, _>>()?;

        Ok(results)
    }
}

pub trait Plot {
    fn plot(&self) -> Result<(), Box<dyn Error>>;
}

pub trait PlottableResult<'a, BC> {
    fn add_to_plot(&self, backend: &mut BC) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait SingleSeries<T>
where
    T: PlottableValue + PartialOrd + Into<f64>,
{
    fn series(&self) -> Vec<Option<f64>>;
    fn range(&self) -> Option<(f64, f64)>;
}

pub struct LinearSeries<T: PlottableValue> {
    y: Vec<Option<T>>,
}

impl<T> SingleSeries<T> for LinearSeries<T>
where
    T: PlottableValue + PartialOrd + Into<f64>,
{
    fn series(&self) -> Vec<Option<f64>> {
        self.y
            .iter()
            .cloned()
            .map(|y| y.map(|v| v.into()))
            .collect()
    }

    fn range(&self) -> Option<(f64, f64)> {
        let mut iter = self.y.iter().filter_map(|v| v.clone());
        let first = iter.next()?;
        let (mut min, mut max) = (first.clone(), first.clone());

        for value in iter {
            if value < min {
                min = value.clone();
            }
            if value > max {
                max = value.clone();
            }
        }
        Some((min.into(), max.into()))
    }
}

pub struct LogSeries<T: PlottableValue> {
    y: Vec<Option<T>>,
}

impl<T> SingleSeries<T> for LogSeries<T>
where
    T: PlottableValue + PartialOrd + Into<f64>,
{
    fn series(&self) -> Vec<Option<f64>> {
        self.y
            .iter()
            .cloned()
            .map(|y| y.map(|v| v.into().log10()))
            .collect()
    }

    fn range(&self) -> Option<(f64, f64)> {
        let mut iter = self.y.iter().filter_map(|v| v.clone());
        let first = iter.next()?;
        let (mut min, mut max) = (first.clone(), first.clone());

        for value in iter {
            if value < min {
                min = value.clone();
            }
            if value > max {
                max = value.clone();
            }
        }
        Some((min.into().log10(), max.into().log10()))
    }
}

pub struct PlottableSeries {
    name: String,
    points: Vec<u32>,
    series: Vec<Option<f64>>,
    color: PaletteColor<Palette99>,
    range: Option<(f64, f64)>,
}

impl PlottableSeries {
    pub fn new(
        name: String,
        points: Vec<u32>,
        series: Vec<Option<f64>>,
        color: PaletteColor<Palette99>,
        range: Option<(f64, f64)>,
    ) -> Self {
        PlottableSeries {
            name,
            points,
            series,
            color,
            range,
        }
    }

    fn range(&self) -> Option<(f64, f64)> {
        self.range.clone()
    }
}

impl<'a>
    PlottableResult<
        'a,
        ChartContext<'a, BitMapBackend<'a>, Cartesian2d<RangedCoordu32, RangedCoordf64>>,
    > for PlottableSeries
{
    fn add_to_plot(
        &self,
        chart: &mut ChartContext<
            'a,
            BitMapBackend<'a>,
            Cartesian2d<RangedCoordu32, RangedCoordf64>,
        >,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let color = self.color.to_rgba();
        let name = self.name.clone();
        let y_max = chart.as_coord_spec().y_spec().range().end;

        let mut line_points: Vec<(u32, f64)> = Vec::new();

        for (&x, y_opt) in self.points.iter().zip(self.series.iter()) {
            match y_opt {
                Some(y) => line_points.push((x, *y)),
                None => {
                    chart.draw_series(std::iter::once(Cross::new((x, y_max / 2.0), 5, color)))?;
                }
            }
        }

        chart
            .draw_series(LineSeries::new(line_points, color))?
            .label(name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));

        chart
            .configure_series_labels()
            .border_style(&BLACK)
            .background_style(&WHITE.mix(0.8))
            .draw()?;

        Ok(())
    }
}

pub struct SeriesPlot {
    pub benchmark_name: String,
    pub results: Vec<PlottableSeries>,
}

impl SeriesPlot {
    pub fn new(benchmark_name: String, results: Vec<PlottableSeries>) -> Self {
        SeriesPlot {
            benchmark_name,
            results,
        }
    }
}

impl SeriesPlot {
    pub fn from_plot_data<T: PlottableValue + PartialOrd + Into<f64>>(
        plot_data: PlotData<T>,
        benchmark_name: String,
        names: &Vec<String>,
        visualization_kind: VisKind,
    ) -> Result<Self, Box<dyn Error>> {
        let mut results = Vec::new();

        for (id, (name, series_values)) in names
            .into_iter()
            .zip(plot_data.results.into_iter())
            .enumerate()
        {
            let series: Box<dyn SingleSeries<T>> = match visualization_kind {
                VisKind::Linear => Box::new(LinearSeries { y: series_values }),
                VisKind::Log => Box::new(LogSeries { y: series_values }),
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
