use crate::database::DatabaseError;
use plotters::prelude::DrawingAreaErrorKind;
use std::error::Error;

#[justerror::Error(desc = "plotting failed")]
pub enum PlotError {
    DatabaseError(
        #[from]
        DatabaseError,
    ),

    #[error(fmt = "unsupported visualization kind")]
    UnsupportedVisKind,

    #[error(fmt = "unknown measurement method")]
    UnknownMeasureKind,

    SerdeJson(
        #[from]
        serde_json::Error,
    ),

    #[error(desc = "Plotters error")]
    Plotters(
        #[from]
        Box<dyn std::error::Error>,
    ),

    #[error(fmt = "invalid value for logarithmic plot")]
    InvalidLogValue,

    #[error(fmt = "missing records in database")]
    MissingRecords,
}

impl<E> From<DrawingAreaErrorKind<E>> for PlotError
where
    E: Error + Send + Sync + 'static,
{
    fn from(e: DrawingAreaErrorKind<E>) -> Self {
        PlotError::Plotters(Box::new(e))
    }
}
