use std::{convert::Infallible, num::ParseIntError, path::PathBuf, str::FromStr};

use scylladb_drivers_benchmarker::{
    config::{
        ConfigError,
        benchmark::{BenchmarkConfig, BenchmarkConfiguration, BenchmarkData},
        find_config,
    },
    utilities::BenchmarkPoint,
};

#[derive(Debug, Clone)]
pub enum BenchmarkSetup {
    Path(PathBuf),
    Points(Vec<BenchmarkPoint>),
}

impl BenchmarkSetup {
    pub fn to_config(self, name: String) -> Result<BenchmarkData, ConfigError> {
        match self {
            BenchmarkSetup::Path(path) => Ok(find_config::<BenchmarkConfig>(&name, &path)?.into()),
            BenchmarkSetup::Points(points) => Ok(BenchmarkData {
                name,
                points,
                timeout: None,
            }),
        }
    }

    pub fn to_points(self, name: String) -> Result<Vec<BenchmarkPoint>, ConfigError> {
        match self {
            BenchmarkSetup::Path(path) => Ok(find_config::<BenchmarkConfig>(&name, &path)?
                .data
                .benchmark_points()
                .collect()),
            BenchmarkSetup::Points(points) => Ok(points),
        }
    }
}

#[justerror::Error]
pub enum BenchmarkSetupError {
    FailedParsingPoint(#[from] ParseIntError),
}

impl FromStr for BenchmarkSetup {
    type Err = BenchmarkSetupError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(',');
        if let Ok(num) = parts.next().unwrap().parse::<BenchmarkPoint>() {
            let mut vec = parts
                .map(str::parse::<BenchmarkPoint>)
                .collect::<Result<Vec<BenchmarkPoint>, _>>()?;
            vec.insert(0, num);
            Ok(BenchmarkSetup::Points(vec))
        } else {
            Ok(BenchmarkSetup::Path(s.into()))
        }
    }
}
