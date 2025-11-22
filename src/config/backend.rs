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
    fn name(&self) -> String {
        self.name.clone()
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
            name: "scylladb-nodejs-rs-driver".to_string(),
            benchmark_name: "select".to_string(),
            build_command: "npm run build".to_string(),
            run_command: "node benchmark/logic/select.js scylladb-nodejs-rs-driver".to_string(),
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
            name: "name: ".to_string() + name,
            benchmark_name: "benchmark_".to_string() + name,
            build_command: "build_cmd_".to_string() + name,
            run_command: "run_cmd_".to_string() + name,
        };

        let backend_config_list = BackendConfigList {
            configs: vec![config("a"), config("b")],
        };

        assert!(backend_config_list.find_config("name: c").is_none());

        let found = backend_config_list.find_config("name: a").unwrap();
        assert_eq!(found, config("a"));
    }
}
