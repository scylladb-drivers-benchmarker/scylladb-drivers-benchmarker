use std::fmt::{self, Debug, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::command::{self, CommandParsingError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementMethod {
    Time,
    Perf,
    Flamegraph(Option<PathBuf>),
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
            MeasurementMethod::Flamegraph(_) => write!(f, "flamegraph"),
            MeasurementMethod::Perf => write!(f, "perf"),
            MeasurementMethod::Time => write!(f, "time"),
            MeasurementMethod::Command(command) => write!(f, "{command}"),
        }
    }
}

impl FromStr for MeasurementMethod {
    type Err = MeasurementMethodParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(stripped) = s.strip_prefix("flamegraph:") {
            let path = Path::new(stripped);
            if !path.is_dir() {
                return Err(MeasurementMethodParsingError::FlamegraphNotInADirectory(
                    stripped.to_owned(),
                ));
            } else {
                return Ok(MeasurementMethod::Flamegraph(Some(stripped.into())));
            }
        }

        match s {
            "perf" => Ok(MeasurementMethod::Perf),
            "flamegraph" => Ok(MeasurementMethod::Flamegraph(None)),
            "time" => Ok(MeasurementMethod::Time),
            value => Ok(MeasurementMethod::Command(value.parse()?)),
        }
    }
}
