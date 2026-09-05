use std::iter;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::config_traits::{Configuration, ConfigurationList};
use crate::measurement::MeasurementMethod;
use crate::utilities::BenchmarkPoint;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressType {
    Multiplicative,
    Additive,
}

/// How the workload interprets the benchmark point value (the STEP env var).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ParamMode {
    /// The point is the number of queries.
    #[default]
    Count,
    /// The point is the concurrency level (query count stays at the workload default).
    Concurrency,
}

impl ParamMode {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ParamMode::Count => "count",
            ParamMode::Concurrency => "concurrency",
        }
    }
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
    /// Workload implementation to run; defaults to the scenario name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_mode: Option<ParamMode>,
    pub starting_step: BenchmarkPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_steps: Option<BenchmarkPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_progress: Option<BenchmarkPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_type: Option<ProgressType>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "humantime_serde"
    )]
    pub timeout: Option<Duration>,
    #[serde(default = "default_num_runs", skip_serializing_if = "is_default_num_runs")]
    pub num_runs: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasurementMethod>,
}

impl BenchmarkConfig {
    pub fn benchmark_points(&self) -> impl Iterator<Item = BenchmarkPoint> {
        let mut starting_step = self.starting_step;
        let no_steps = self.no_steps
            .expect("'no-steps' must be set in the benchmark entry or in the top-level defaults");
        let step_progress = self.step_progress
            .expect("'step-progress' must be set in the benchmark entry or in the top-level defaults");
        let progress_type = self.progress_type
            .expect("'progress-type' must be set in the benchmark entry or in the top-level defaults");
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
        .take(no_steps as usize)
    }
}

impl Configuration for BenchmarkConfig {
    type ConfigListType = BenchmarkConfigList;
    fn benchmark_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkData {
    pub name: String,
    /// Workload implementation name (the BENCHMARK env var).
    pub workload: String,
    /// How the workload interprets the point value (the PARAM_MODE env var).
    pub param_mode: ParamMode,
    pub points: Vec<BenchmarkPoint>,
    pub timeout: Option<Duration>,
    pub num_runs: u32,
    pub measure: Option<MeasurementMethod>,
}

impl From<BenchmarkConfig> for BenchmarkData {
    fn from(value: BenchmarkConfig) -> Self {
        BenchmarkData {
            points: value.benchmark_points().collect(),
            workload: value.workload.unwrap_or_else(|| value.name.clone()),
            param_mode: value.param_mode.unwrap_or_default(),
            name: value.name,
            timeout: value.timeout,
            num_runs: value.num_runs,
            measure: value.measure,
        }
    }
}

/// Top-level defaults applied to every benchmark entry that does not set the field explicitly.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_mode: Option<ParamMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_steps: Option<BenchmarkPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_progress: Option<BenchmarkPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_type: Option<ProgressType>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "humantime_serde")]
    pub timeout: Option<Duration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_runs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasurementMethod>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkConfigList {
    #[serde(rename = "benchmarks")]
    pub configs: Vec<BenchmarkConfig>,
    #[serde(default, skip_serializing_if = "is_default_benchmark_defaults")]
    pub defaults: BenchmarkDefaults,
}

fn is_default_benchmark_defaults(d: &BenchmarkDefaults) -> bool {
    *d == BenchmarkDefaults::default()
}

impl BenchmarkConfigList {
    fn merged(&self) -> impl Iterator<Item = BenchmarkConfig> + '_ {
        self.configs.iter().map(|c| {
            let mut c = c.clone();
            if c.param_mode.is_none() { c.param_mode = self.defaults.param_mode; }
            if c.no_steps.is_none() { c.no_steps = self.defaults.no_steps; }
            if c.step_progress.is_none() { c.step_progress = self.defaults.step_progress; }
            if c.progress_type.is_none() { c.progress_type = self.defaults.progress_type; }
            if c.timeout.is_none() { c.timeout = self.defaults.timeout; }
            if c.num_runs == default_num_runs() {
                if let Some(n) = self.defaults.num_runs { c.num_runs = n; }
            }
            if c.measure.is_none() { c.measure = self.defaults.measure.clone(); }
            c
        })
    }
}
impl ConfigurationList for BenchmarkConfigList {
    type ConfigType = BenchmarkConfig;

    fn configs(&self) -> impl Iterator<Item = Self::ConfigType> {
        self.merged().collect::<Vec<_>>().into_iter()
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
            workload: None,
            param_mode: None,
            starting_step: 7,
            no_steps: Some(3),
            step_progress: Some(2),
            progress_type: Some(ProgressType::Additive),
            timeout: None,
            num_runs: 1,
            measure: None,
        };
        let points: Vec<u64> = data.benchmark_points().collect();
        assert_eq!(points, vec![7, 9, 11]);
    }

    #[test]
    fn points_multiplicative() {
        let data = BenchmarkConfig {
            name: String::new(),
            workload: None,
            param_mode: None,
            starting_step: 3,
            no_steps: Some(3),
            step_progress: Some(2),
            progress_type: Some(ProgressType::Multiplicative),
            timeout: None,
            num_runs: 1,
            measure: None,
        };
        let points: Vec<u64> = data.benchmark_points().collect();
        assert_eq!(points, vec![3, 6, 12]);
    }

    #[test]
    fn serde_benchmark_config() {
        let config = BenchmarkConfig {
            name: "benchmark_name".to_owned(),
            workload: None,
            param_mode: None,
            starting_step: 1,
            no_steps: Some(5),
            step_progress: Some(2),
            progress_type: Some(ProgressType::Multiplicative),
            timeout: Some(Duration::from_secs(3)),
            num_runs: 1,
            measure: None,
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
            workload: None,
            param_mode: None,
            starting_step: 1,
            no_steps: Some(5),
            step_progress: Some(2),
            progress_type: Some(ProgressType::Multiplicative),
            timeout: None,
            num_runs: 1,
            measure: None,
        };

        let config2 = BenchmarkConfig {
            name: "benchmark_name2".to_owned(),
            workload: None,
            param_mode: None,
            starting_step: 2,
            no_steps: Some(6),
            step_progress: Some(3),
            progress_type: Some(ProgressType::Additive),
            timeout: Some(Duration::from_secs(2 * 60)),
            num_runs: 1,
            measure: None,
        };

        let config_list = BenchmarkConfigList {
            configs: vec![config1, config2],
            defaults: BenchmarkDefaults::default(),
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
