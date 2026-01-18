use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::FromStr;

use scylladb_drivers_benchmarker::config::benchmark::{BenchmarkConfig, BenchmarkData};
use scylladb_drivers_benchmarker::config::{ConfigError, find_config};
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;

use crate::parsing::ParsingError;
use crate::parsing::aliasing::AliasingConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkSetup {
    Path(PathBuf),
    Points(Vec<BenchmarkPoint>),
}

impl BenchmarkSetup {
    pub fn into_config(self, name: &str) -> Result<BenchmarkData, ConfigError> {
        match self {
            BenchmarkSetup::Path(path) => Ok(find_config::<BenchmarkConfig>(name, &path)?.into()),
            BenchmarkSetup::Points(points) => Ok(BenchmarkData {
                name: name.to_owned(),
                points,
                timeout: None,
            }),
        }
    }

    pub fn into_points(self, name: &str) -> Result<Vec<BenchmarkPoint>, ConfigError> {
        match self {
            BenchmarkSetup::Path(path) => Ok(find_config::<BenchmarkConfig>(name, &path)?
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

impl BenchmarkSetup {
    pub fn finalize(
        benchmark_setup: Option<Self>,
        benchmark_name: &str,
        aliasing_config: &AliasingConfig,
    ) -> Result<BenchmarkData, ParsingError> {
        if let Some(benchmark_setup) = benchmark_setup {
            Ok(benchmark_setup.into_config(benchmark_name)?)
        } else if let Some(config_path) = &aliasing_config.benchmark_config {
            match find_config::<BenchmarkConfig>(benchmark_name, config_path) {
                Ok(config) => Ok(config.into()),
                Err(ConfigError::ConfigurationNotFound { .. }) => {
                    Err(ParsingError::NoBenchmarkConfiguration)
                }
                Err(err) => Err(err.into()),
            }
        } else {
            Err(ParsingError::NoBenchmarkConfiguration)
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::fmt::Debug;
    use std::fs;
    use std::time::Duration;

    use scylladb_drivers_benchmarker::config::benchmark::{
        BenchmarkConfig, BenchmarkConfigList, ProgressType,
    };
    use serde::{Deserialize, Serialize};
    use tempfile::NamedTempFile;

    use crate::parsing::aliasing::AliasingConfig;
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

    struct Configs {
        wrong_config: NamedTempFile,
        aconfig: AliasingConfig,
    }

    fn aliasing_config() -> Configs {
        let bconfig = BenchmarkConfig {
            name: "Aliased".to_owned(),
            starting_step: 100,
            no_steps: 1,
            step_progress: 100,
            progress_type: ProgressType::Multiplicative,
            timeout: Some(Duration::from_secs(60 * 60 * 100)), // 100 hours, from_hours may not be available.
        };
        let wrong_config = write_assert(BenchmarkConfigList {
            configs: vec![bconfig],
        });

        let aconfig = AliasingConfig {
            dp_path: None,
            repo_path: HashMap::new(),
            flame_path: None,
            store_dir: None,
            benchmark_config: Some(wrong_config.path().to_owned()),
        };

        Configs {
            wrong_config,
            aconfig,
        }
    }

    #[test]
    fn setup_aliased_config() {
        let Configs {
            wrong_config: _wrong_config,
            aconfig,
        } = aliasing_config();

        let output = BenchmarkSetup::finalize(None, "Aliased", &aconfig).unwrap();
        assert_eq!(output.name, "Aliased");
        assert_eq!(output.points, vec![100]);
        assert_eq!(output.timeout, Some(Duration::from_secs(60 * 60 * 100))); // 100 hours, from_hours may not be available.
    }

    #[test]
    fn setup_finalize_points() {
        let output = BenchmarkSetup::finalize(
            Some(BenchmarkSetup::Points(vec![1, 2, 3])),
            "bname",
            &AliasingConfig::default(),
        )
        .unwrap();
        assert_eq!(output.name, "bname");
        assert_eq!(output.points, vec![1, 2, 3]);
        assert_eq!(output.timeout, None);
    }

    #[test]
    fn setup_finalize_path() {
        let bconfig = BenchmarkConfig {
            name: "bname".to_owned(),
            starting_step: 1,
            no_steps: 3,
            step_progress: 1,
            progress_type: ProgressType::Additive,
            timeout: Some(Duration::from_secs(3)),
        };

        let bconfig_file = write_assert(BenchmarkConfigList {
            configs: vec![bconfig],
        });
        let Configs {
            wrong_config: _wrong_config,
            aconfig,
        } = aliasing_config();

        let output = BenchmarkSetup::finalize(
            Some(BenchmarkSetup::Path(bconfig_file.path().to_owned())),
            "bname",
            &aconfig,
        )
        .unwrap();
        assert_eq!(output.name, "bname");
        assert_eq!(output.points, vec![1, 2, 3]);
        assert_eq!(output.timeout, Some(Duration::from_secs(3)));
    }
}
