mod backend;
mod benchmark;
mod config_errors;
mod config_traits;

use std::{error::Error, path::Path};

use backend::BackendConfig;
use config_errors::ConfigurationNotFound;
use config_traits::{Configuration, ConfigurationList};

pub fn find_config<ConfigListType: ConfigurationList>(
    config_name: String,
    config_path: &Path,
) -> Result<ConfigListType::ConfigType, Box<dyn Error>> {
    let file = std::fs::File::open(config_path)?;
    let config_list: ConfigListType = serde_yml::from_reader(file)?;
    config_list
        .find_config(config_name.as_str())
        .ok_or(Box::new(ConfigurationNotFound {
            path: config_path.to_owned(),
            name: config_name,
        }))
}

#[cfg(test)]
mod tests {
    use crate::config::backend::BackendConfigList;

    use super::*;

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
