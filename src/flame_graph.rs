use std::fmt::Display;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

use crate::command;
use crate::measurement::MeasurementMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlameFrequency {
    Number(u32),
    Max,
}

#[justerror::Error]
pub struct NotAFlameFrequency(#[from] ParseIntError);

impl FromStr for FlameFrequency {
    type Err = NotAFlameFrequency;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "max" {
            return Ok(FlameFrequency::Max);
        }
        match s.parse() {
            Ok(number) => Ok(FlameFrequency::Number(number)),
            Err(error) => Err(NotAFlameFrequency(error)),
        }
    }
}

impl Display for FlameFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{}", number),
            Self::Max => write!(f, "max"),
        }
    }
}
