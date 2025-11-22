use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressType {
    Multiplicative,
    Additive,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkConfig {
    pub name: String,
    pub starting_step: u32,
    pub no_steps: u32,
    pub step_progress: u32,
    pub progress_type: ProgressType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BenchmarkConfigList {
    pub benchmarks: Vec<BenchmarkConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_benchmark_config() {
        let config = BenchmarkConfig {
            name: "benchmark_name".to_string(),
            starting_step: 1,
            no_steps: 5,
            step_progress: 2,
            progress_type: ProgressType::Multiplicative,
        };

        let serialized: String = serde_yml::to_string(&config).unwrap();
        let expected: &str = "\
name: benchmark_name
starting-step: 1
no-steps: 5
step-progress: 2
progress-type: multiplicative
";
        assert_eq!(serialized, expected);
    }
    #[test]
    fn serialize_benchmark_config_list() {
        let config1 = BenchmarkConfig {
            name: "benchmark_name1".to_string(),
            starting_step: 1,
            no_steps: 5,
            step_progress: 2,
            progress_type: ProgressType::Multiplicative,
        };

        let config2 = BenchmarkConfig {
            name: "benchmark_name2".to_string(),
            starting_step: 2,
            no_steps: 6,
            step_progress: 3,
            progress_type: ProgressType::Additive,
        };

        let config_list = BenchmarkConfigList {
            benchmarks: vec!(config1, config2)
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
";
        assert_eq!(serialized, expected);
    }
}
