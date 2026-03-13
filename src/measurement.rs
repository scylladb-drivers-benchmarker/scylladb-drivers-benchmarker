use std::fmt::{self, Debug, Display};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::command::{self, CommandParsingError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementMethod {
    Time,
    Perf,
    FlameGraph,
    Command(command::Command),
}

impl Serialize for MeasurementMethod {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MeasurementMethod {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[justerror::Error]
pub enum MeasurementMethodParsingError {
    #[error(desc = "measuring method, not one of default, and not a command")]
    ParsingFailed(#[from] CommandParsingError),
    #[error(desc = "given flame-graph path is not a directory")]
    FlameGraphNotInADirectory(String),
}

impl MeasurementMethod {
    #[must_use]
    pub fn y_axis_label(&self) -> &'static str {
        match self {
            MeasurementMethod::Time => "Time [s]",
            MeasurementMethod::Perf => "Perf value",
            MeasurementMethod::FlameGraph => "Frequency",
            MeasurementMethod::Command(_) => "Value",
        }
    }
}

impl Display for MeasurementMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeasurementMethod::FlameGraph => write!(f, "flame-graph"),
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
            "flame-graph" => Ok(MeasurementMethod::FlameGraph),
            "time" => Ok(MeasurementMethod::Time),
            value => Ok(MeasurementMethod::Command(value.parse()?)),
        }
    }
}
