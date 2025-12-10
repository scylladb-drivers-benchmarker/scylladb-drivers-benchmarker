mod data;
pub mod error;
mod plot;
mod render;
mod series;

use crate::{VisKind, commit_hash::CommitHash};
use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;

use data::BenchmarkDataset;
use error::PlotError;
use plot::{Plot, SeriesPlot};

pub enum PlotKind {
    Series(VisKind),
    Flamegraph,
}

pub fn plot(
    plot_kind: PlotKind,
    database: &Database,
    benchmark_name: &str,
    benchmark_config: &BenchmarkConfig,
    measurement_method: &str,
    commit_hashes: &[CommitHash],
    names: &[String],
) -> Result<(), PlotError> {
    match plot_kind {
        PlotKind::Series(vis_kind) => {
            // Not sure how to handle this f64 here.
            // Ideally type would be inferred wrt 'measurement_method',
            // but no such functionality is implemented.
            // Only later abstractions would require Into<64>, as they do now.
            let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
                database,
                benchmark_config,
                commit_hashes,
                measurement_method,
            )?;

            let plot =
                SeriesPlot::from_dataset(dataset, benchmark_name.to_string(), names, vis_kind)?;

            plot.plot()
        }
        PlotKind::Flamegraph => todo!(),
    }
}
