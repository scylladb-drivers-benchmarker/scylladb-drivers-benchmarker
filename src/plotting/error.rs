use std::error::Error;

use plotters::prelude::DrawingAreaErrorKind;

use crate::database::DatabaseError;
use crate::utilities::BenchmarkPoint;

#[justerror::Error(desc = "plotting failed")]
pub enum PlotError {
    Database(#[from] DatabaseError),

    InvalidData(String),

    #[error(desc = "Plotters error")]
    Plotters(String),

    #[error(desc = "invalid value for logarithmic plot")]
    InvalidLogValue(f64),

    #[error(desc = "missing records in the database", fmt = debug)]
    MissingRecords {
        commit_hash: String,
        benchmark: String,
        points: Vec<BenchmarkPoint>,
        measurement_method: String,
    },

    #[error(desc = "no records in the database, this benchmark most likely did not run", fmt = debug)]
    MissingBenchmark {
        commit_hash: String,
        benchmark: String,
        measurement_method: String,
    },

    #[error(desc = "incompatible output format for the selected plot", fmt = debug)]
    IncompatibleOutputFormat {
        format: String,
        plot: String,
    },

    #[error(
        desc = "Flamegraph repository path is missing; provide --flame-repo or configure global path"
    )]
    MissingFlameRepo,

    #[error(desc = "I/O error occurred", fmt = debug)]
    Io(#[from] std::io::Error),

    #[error(desc = "fatal internal error", fmt = debug)]
    Internal(String),
}

impl<E> From<DrawingAreaErrorKind<E>> for PlotError
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: DrawingAreaErrorKind<E>) -> Self {
        PlotError::Plotters(e.to_string())
    }
}
