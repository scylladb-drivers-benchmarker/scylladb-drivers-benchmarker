mod data;
mod result;
mod series;
mod plot;

use std::error::Error;

use crate::commit_hash::CommitHash;
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;

use data::PlotData;
use plot::{Plot, SeriesPlot};
use series::VisKind;

// TODO: this overall is terrible
pub fn plot(
    database: &Database,
    benchmark_name: &str,
    benchmark_config: &BenchmarkConfig,
    measurement_method: &str,
    visualization_kind: Option<String>,
    commit_hashes: &Vec<CommitHash>,
    names: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if let None = visualization_kind {
        return Err(format!("No visualization kind").into());
    }

    match visualization_kind.as_deref() {
        Some("log") => {
            let plot_data: PlotData<f64> = PlotData::new(
                database,
                benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot = SeriesPlot::from_plot_data(
                plot_data,
                benchmark_name.to_string(),
                &names,
                VisKind::Log,
            )?;

            plot.plot()?;
            Ok(())
        }
        Some("linear") => {
            let plot_data: PlotData<f64> = PlotData::new(
                database,
                benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot = SeriesPlot::from_plot_data(
                plot_data,
                benchmark_name.to_string(),
                &names,
                VisKind::Linear,
            )?;

            plot.plot()?;
            Ok(())
        }
        Some(other) => return Err(format!("Unknown visualization kind: {}", other).into()),
        None => return Ok(()),
    }
}
