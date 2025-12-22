use crate::{database::DatabaseError, utilities::BenchmarkPoint};
use plotters::prelude::DrawingAreaErrorKind;
use std::error::Error;

#[justerror::Error(desc = "plotting failed")]
pub enum PlotError {
    Database(#[from] DatabaseError),

    #[error(desc = "visualization kinds do not apply to flamegraph")]
    UnexpectedVisualizationKind(),

    #[error(desc = "unknown measurement method")]
    UnknownMeasureKind(String),

    SerdeJson(#[from] serde_json::Error),

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
    }
}

impl<E> From<DrawingAreaErrorKind<E>> for PlotError
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: DrawingAreaErrorKind<E>) -> Self {
        PlotError::Plotters(e.to_string())
    }
}
