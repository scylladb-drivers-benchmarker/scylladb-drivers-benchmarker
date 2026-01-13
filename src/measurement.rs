use std::fmt::{self, Debug, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::command::{self, CommandParsingError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementMethod {
    Time,
    Perf,
    Flamegraph,
    Command(command::Command),
}

#[justerror::Error]
pub enum MeasurementMethodParsingError {
    #[error(desc = "measuring method, not one of default, and not a command")]
    ParsingFailed(#[from] CommandParsingError),
    #[error(desc = "given flamegraph path is not a directory")]
    FlamegraphNotInADirectory(String),
}

impl Display for MeasurementMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeasurementMethod::Flamegraph => write!(f, "flamegraph"),
            MeasurementMethod::Perf => write!(f, "perf"),
            MeasurementMethod::Time => write!(f, "time"),
            MeasurementMethod::Command(command) => write!(f, "{command}"),
        }
    }
}

impl FromStr for MeasurementMethod {
    type Err = MeasurementMethodParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "perf" => Ok(MeasurementMethod::Perf),
            "flamegraph" => Ok(MeasurementMethod::Flamegraph),
            "time" => Ok(MeasurementMethod::Time),
            value => Ok(MeasurementMethod::Command(value.parse()?)),
        }
    }
}
