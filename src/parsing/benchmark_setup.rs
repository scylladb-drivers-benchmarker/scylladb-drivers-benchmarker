use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

use log::trace;
use scylladb_drivers_benchmarker::config::benchmark::{BenchmarkConfig, BenchmarkData};
use scylladb_drivers_benchmarker::config::{ConfigError, find_config};
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;

use crate::parsing::ParsingError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BenchmarkSetup {
    Path(PathBuf),
    Points(Vec<BenchmarkPoint>),
}

impl BenchmarkSetup {
    fn into_config(self, name: &str) -> Result<BenchmarkData, ConfigError> {
        match self {
            BenchmarkSetup::Path(path) => Ok(find_config::<BenchmarkConfig>(name, &path)?.into()),
            BenchmarkSetup::Points(mut points) => {
                points.sort();
                Ok(BenchmarkData {
                    name: name.to_owned(),
                    workload: name.to_owned(),
                    param_mode: Default::default(),
                    points,
                    timeout: None,
                    num_runs: 1,
                    measure: None,
                })
            }
        }
    }
}

#[justerror::Error]
pub(crate) enum BenchmarkSetupError {
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

impl BenchmarkSetup {
    pub(crate) fn finalize(
        benchmark_setup: Option<Self>,
        benchmark_name: &str,
    ) -> Result<BenchmarkData, ParsingError> {
        trace!("Finalizing the benchmark setup...");
        if let Some(benchmark_setup) = benchmark_setup {
            trace!("Found literal setup: {:?}", benchmark_setup);
            Ok(benchmark_setup.into_config(benchmark_name)?)
        } else {
            Err(ParsingError::NoBenchmarkConfiguration)
        }
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use std::fs;
    use std::time::Duration;

    use scylladb_drivers_benchmarker::config::benchmark::{
        BenchmarkConfig, BenchmarkConfigList, BenchmarkDefaults, ProgressType,
    };
    use serde::{Deserialize, Serialize};
    use tempfile::NamedTempFile;

    use crate::parsing::benchmark_setup::BenchmarkSetup;

    fn write_assert<T: Serialize + Debug + Eq + for<'a> Deserialize<'a>>(val: T) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        serde_yml::ser::to_writer(&file, &val).unwrap();
        assert_eq!(
            serde_yml::from_slice::<T>(&fs::read(file.path()).unwrap()).unwrap(),
            val
        );
        file
    }

    #[test]
    fn setup_finalize_points() {
        let output =
            BenchmarkSetup::finalize(Some(BenchmarkSetup::Points(vec![1, 2, 3])), "bname").unwrap();
        assert_eq!(output.name, "bname");
        assert_eq!(output.points, vec![1, 2, 3]);
        assert_eq!(output.timeout, None);
    }

    #[test]
    fn setup_finalize_path() {
        let bconfig = BenchmarkConfig {
            name: "bname".to_owned(),
            workload: None,
            param_mode: None,
            starting_step: 1,
            no_steps: Some(3),
            step_progress: Some(1),
            progress_type: Some(ProgressType::Additive),
            timeout: Some(Duration::from_secs(3)),
            num_runs: 1,
            measure: None,
        };

        let bconfig_file = write_assert(BenchmarkConfigList {
            configs: vec![bconfig],
            defaults: BenchmarkDefaults::default(),
        });

        let output = BenchmarkSetup::finalize(
            Some(BenchmarkSetup::Path(bconfig_file.path().to_owned())),
            "bname",
        )
        .unwrap();
        assert_eq!(output.name, "bname");
        assert_eq!(output.points, vec![1, 2, 3]);
        assert_eq!(output.timeout, Some(Duration::from_secs(3)));
    }
}
