use crate::database::DatabaseError;
use plotters::prelude::DrawingAreaErrorKind;
use std::error::Error;

#[justerror::Error(desc = "plotting failed")]
pub enum PlotError {
    Database(#[from] DatabaseError),

    #[error(desc = "unsupported visualization kind: {0}")]
    UnsupportedVisKind(String),

    #[error(desc = "unknown measurement method: {0}")]
    UnknownMeasureKind(String),

    SerdeJson(#[from] serde_json::Error),

    #[error(desc = "Plotters error: {0}")]
    Plotters(String),

    #[error(desc = "invalid value for logarithmic plot: {0}")]
    InvalidLogValue(f64),

    #[error(desc = "missing records in database")]
    MissingRecords,
}

impl<E> From<DrawingAreaErrorKind<E>> for PlotError
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: DrawingAreaErrorKind<E>) -> Self {
        PlotError::Plotters(e.to_string())
    }
}
