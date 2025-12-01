mod plotting;

use std::error::Error;

use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;

use plotting::PlotData;

use plotters::prelude::*;

pub fn main(
    database: &Database,
    benchmark_name: &str,
    benchmark_config: &BenchmarkConfig,
    measurement_method: String,
    visualization_kind: Option<String>,
    commit_hashes: &Vec<CommitHash>,
    names: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let plot_data = PlotData::new_plot(
        database,
        benchmark_config,
        commit_hashes,
        measurement_method,
    )?;

    let x_start = *plot_data.points.first().unwrap_or(&0);
    let x_end = *plot_data.points.last().unwrap_or(&1);

    let y_max = plot_data
        .results
        .iter()
        .flatten()
        .filter_map(|&y| y)
        .max()
        .unwrap_or(1);

    let root = BitMapBackend::new("test.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Benchmark {} Results", benchmark_name),
            ("sans-serif", 40),
        )
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(x_start..x_end, 0u64..y_max)?;

    chart.configure_mesh().draw()?;

    for (idx, (name, series)) in names.iter().zip(plot_data.results.iter()).enumerate() {
        let color = Palette99::pick(idx);

        let mut line_points: Vec<(u32, u64)> = Vec::new();

        for (&x, y_opt) in plot_data.points.iter().zip(series.iter()) {
            match y_opt {
                Some(y) => line_points.push((x, *y)),
                None => {
                    chart.draw_series(std::iter::once(Cross::new((x, y_max / 2), 5, &color)))?;
                }
            }
        }

        chart
            .draw_series(LineSeries::new(line_points, &color))?
            .label(name.clone())
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &color));

        chart
            .configure_series_labels()
            .border_style(&BLACK)
            .background_style(&WHITE.mix(0.8))
            .draw()?;
    }

    Ok(())
}
