use crate::database::DatabaseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlotError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Unsupported visualization kind")]
    UnsupportedVisKind,

    #[error("Serde JSON error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Plotters error: {0}")]
    Plotters(#[from] Box<dyn std::error::Error>),

    #[error("Invalid value for log plot: {value}")]
    InvalidLogValue { value: f64 },
}
