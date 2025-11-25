pub mod backend;
pub mod benchmark;
mod config_errors;
mod config_traits;

use std::{error::Error, path::Path};

use config_errors::ConfigurationNotFound;
use config_traits::{Configuration, ConfigurationList};

pub fn find_config<ConfigType: Configuration>(
    config_name: &str,
    config_path: &Path,
) -> Result<ConfigType, Box<dyn Error>> {
    let file = std::fs::File::open(config_path)?;
    let config_list: ConfigType::ConfigListType = serde_yml::from_reader(file)?;
    config_list
        .find_config(config_name)
        .ok_or(Box::new(ConfigurationNotFound {
            path: config_path.to_owned(),
            name: config_name.to_string(),
        }))
}

mod tests {
    use crate::config::backend::BackendConfig;

    use super::*;

    #[test]
    fn open_config() {
        let config: BackendConfig = find_config(
            "scylladb-nodejs-rs-driver",
            Path::new("./configs/backend_config.yml"),
        )
        .unwrap();
        assert_eq!(
            config,
            BackendConfig {
                name: "scylladb-nodejs-rs-driver".to_string(),
                benchmark_name: "select".to_string(),
                build_command: "npm run build".to_string(),
                run_command: "node benchmark/logic/select.js scylladb-nodejs-rs-driver".to_string()
            }
        )
    }
}
