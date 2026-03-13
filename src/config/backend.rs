use serde::{Deserialize, Serialize};

use crate::config::config_traits::{Configuration, ConfigurationList};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfig {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_name: Option<String>,
    pub build_command: String,
    pub run_command: String,
}

impl Configuration for BackendConfig {
    type ConfigListType = BackendConfigList;
    fn benchmark_name(&self) -> String {
        self.benchmark_name
            .clone()
            .expect("'benchmark-name' must be set in the backend entry or in the top-level defaults")
    }
}

/// Top-level defaults applied to every backend entry that does not set the field explicitly.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BackendDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_command: Option<String>,
}

fn is_default_backend_defaults(d: &BackendDefaults) -> bool {
    *d == BackendDefaults::default()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfigList {
    #[serde(rename = "backends")]
    pub configs: Vec<BackendConfig>,
    #[serde(default, skip_serializing_if = "is_default_backend_defaults")]
    pub defaults: BackendDefaults,
}

impl BackendConfigList {
    fn merged(&self) -> impl Iterator<Item = BackendConfig> + '_ {
        self.configs.iter().map(|c| {
            let mut c = c.clone();
            if c.benchmark_name.is_none() {
                c.benchmark_name = self.defaults.benchmark_name.clone();
            }
            if c.build_command.is_empty() {
                if let Some(ref cmd) = self.defaults.build_command {
                    c.build_command = cmd.clone();
                }
            }
            if c.run_command.is_empty() {
                if let Some(ref cmd) = self.defaults.run_command {
                    c.run_command = cmd.clone();
                }
            }
            c
        })
    }
}
impl ConfigurationList for BackendConfigList {
    type ConfigType = BackendConfig;

    fn configs(&self) -> impl Iterator<Item = Self::ConfigType> {
        self.merged().collect::<Vec<_>>().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_benchmark_config() {
        let config = BackendConfig {
            name: "scylladb-nodejs-rs-driver".to_owned(),
            benchmark_name: Some("select".to_owned()),
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
            benchmark_name: Some("benchmark_".to_owned() + name),
            build_command: "build_cmd_".to_owned() + name,
            run_command: "run_cmd_".to_owned() + name,
        };

        let backend_config_list = BackendConfigList {
            configs: vec![config("a"), config("b")],
            defaults: BackendDefaults::default(),
        };

        assert!(backend_config_list.find_config("name: c").is_none());

        let found = backend_config_list.find_config("benchmark_a").unwrap();
        assert_eq!(found, config("a"));
    }
}
