use std::iter;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::config_traits::{Configuration, ConfigurationList};
use crate::utilities::BenchmarkPoint;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressType {
    Multiplicative,
    Additive,
}

fn default_num_runs() -> u32 {
    1
}

fn is_default_num_runs(n: &u32) -> bool {
    *n == 1
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkConfig {
    pub name: String,
    pub starting_step: BenchmarkPoint,
    pub no_steps: BenchmarkPoint,
    pub step_progress: BenchmarkPoint,
    pub progress_type: ProgressType,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "humantime_serde"
    )]
    pub timeout: Option<Duration>,
    #[serde(default = "default_num_runs", skip_serializing_if = "is_default_num_runs")]
    pub num_runs: u32,
}

impl BenchmarkConfig {
    pub fn benchmark_points(&self) -> impl Iterator<Item = BenchmarkPoint> {
        let mut starting_step = self.starting_step;
        let step_progress = self.step_progress;
        let progress_type = self.progress_type;
        iter::from_fn(move || {
            let ret = starting_step;
            match progress_type {
                ProgressType::Additive => {
                    starting_step += step_progress;
                }
                ProgressType::Multiplicative => {
                    starting_step *= step_progress;
                }
            }
            Some(ret)
        })
        .take(self.no_steps as usize)
    }
}

impl Configuration for BenchmarkConfig {
    type ConfigListType = BenchmarkConfigList;
    fn benchmark_name(&self) -> String {
        self.name.clone()
    }
}

pub struct BenchmarkData {
    pub name: String,
    pub points: Vec<BenchmarkPoint>,
    pub timeout: Option<Duration>,
    pub num_runs: u32,
}

impl From<BenchmarkConfig> for BenchmarkData {
    fn from(value: BenchmarkConfig) -> Self {
        BenchmarkData {
            points: value.benchmark_points().collect(),
            name: value.name,
            timeout: value.timeout,
            num_runs: value.num_runs,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkConfigList {
    #[serde(rename = "benchmarks")]
    pub configs: Vec<BenchmarkConfig>,
}

impl ConfigurationList for BenchmarkConfigList {
    type ConfigType = BenchmarkConfig;

    fn configs(&self) -> impl Iterator<Item = Self::ConfigType> {
        self.configs.iter().cloned()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn points_additive() {
        let data = BenchmarkConfig {
            name: String::new(),
            starting_step: 7,
            no_steps: 3,
            step_progress: 2,
            progress_type: ProgressType::Additive,
            timeout: None,
            num_runs: 1,
        };
        let points: Vec<u64> = data.benchmark_points().collect();
        assert_eq!(points, vec![7, 9, 11]);
    }

    #[test]
    fn points_multiplicative() {
        let data = BenchmarkConfig {
            name: String::new(),
            starting_step: 3,
            no_steps: 3,
            step_progress: 2,
            progress_type: ProgressType::Multiplicative,
            timeout: None,
            num_runs: 1,
        };
        let points: Vec<u64> = data.benchmark_points().collect();
        assert_eq!(points, vec![3, 6, 12]);
    }

    #[test]
    fn serde_benchmark_config() {
        let config = BenchmarkConfig {
            name: "benchmark_name".to_owned(),
            starting_step: 1,
            no_steps: 5,
            step_progress: 2,
            progress_type: ProgressType::Multiplicative,
            timeout: Some(Duration::from_secs(3)),
            num_runs: 1,
        };

        let serialized: String = serde_yml::to_string(&config).unwrap();
        let expected: &str = "\
name: benchmark_name
starting-step: 1
no-steps: 5
step-progress: 2
progress-type: multiplicative
timeout: '3s'
";
        assert_eq!(serialized, expected);
        assert_eq!(
            serde_yml::from_str::<BenchmarkConfig>(expected).unwrap(),
            config
        );
    }

    #[test]
    fn serde_benchmark_config_list() {
        let config1 = BenchmarkConfig {
            name: "benchmark_name1".to_owned(),
            starting_step: 1,
            no_steps: 5,
            step_progress: 2,
            progress_type: ProgressType::Multiplicative,
            timeout: None,
            num_runs: 1,
        };

        let config2 = BenchmarkConfig {
            name: "benchmark_name2".to_owned(),
            starting_step: 2,
            no_steps: 6,
            step_progress: 3,
            progress_type: ProgressType::Additive,
            timeout: Some(Duration::from_secs(2 * 60)),
            num_runs: 1,
        };

        let config_list = BenchmarkConfigList {
            configs: vec![config1, config2],
        };

        let serialized: String = serde_yml::to_string(&config_list).unwrap();
        let expected: &str = "\
benchmarks:
- name: benchmark_name1
  starting-step: 1
  no-steps: 5
  step-progress: 2
  progress-type: multiplicative
- name: benchmark_name2
  starting-step: 2
  no-steps: 6
  step-progress: 3
  progress-type: additive
  timeout: '2m'
";
        assert_eq!(serialized, expected);
        assert_eq!(
            serde_yml::from_str::<BenchmarkConfigList>(expected).unwrap(),
            config_list
        );
    }
}
