use serde::{Deserialize, Serialize};

use crate::config::config_traits::{Configuration, ConfigurationList};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfig {
    pub name: String,
    pub benchmark_name: String,
    pub build_command: String,
    pub run_command: String,
}

impl Configuration for BackendConfig {
    type ConfigListType = BackendConfigList;
    fn benchmark_name(&self) -> String {
        self.benchmark_name.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfigList {
    #[serde(rename = "backends")]
    pub configs: Vec<BackendConfig>,
}

impl ConfigurationList for BackendConfigList {
    type ConfigType = BackendConfig;

    fn configs(&self) -> impl Iterator<Item = Self::ConfigType> {
        self.configs.iter().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_benchmark_config() {
        let config = BackendConfig {
            name: "scylladb-nodejs-rs-driver".to_owned(),
            benchmark_name: "select".to_owned(),
            build_command: "npm run build".to_owned(),
            run_command: "node benchmark/logic/select.js scylladb-nodejs-rs-driver".to_owned(),
        };

        let serialized: String = serde_yml::to_string(&config).unwrap();
        let expected: &str = "\
name: scylladb-nodejs-rs-driver
benchmark-name: select
build-command: npm run build
run-command: node benchmark/logic/select.js scylladb-nodejs-rs-driver
";
        assert_eq!(serialized, expected);
    }

    #[test]
    fn find_config() {
        let config = |name| BackendConfig {
            name: "name: ".to_owned() + name,
            benchmark_name: "benchmark_".to_owned() + name,
            build_command: "build_cmd_".to_owned() + name,
            run_command: "run_cmd_".to_owned() + name,
        };

        let backend_config_list = BackendConfigList {
            configs: vec![config("a"), config("b")],
        };

        assert!(backend_config_list.find_config("name: c").is_none());

        let found = backend_config_list.find_config("benchmark_a").unwrap();
        assert_eq!(found, config("a"));
    }
}
