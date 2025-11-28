pub mod backend;
pub mod benchmark;
pub mod config_errors;
pub mod config_traits;

use std::{error::Error, path::Path};

use config_errors::ConfigurationNotFound;
use config_traits::{Configuration, ConfigurationList};

pub fn open_config<ConfigListType: ConfigurationList>(
    config_path: &Path,
) -> Result<ConfigListType, Box<dyn Error>> {
    let file = std::fs::File::open(config_path)?;
    return Ok(serde_yml::from_reader(file)?);
}

pub fn find_config<ConfigType: Configuration>(
    config_name: &str,
    config_path: &Path,
) -> Result<ConfigType, Box<dyn Error>> {
    let config_list: ConfigType::ConfigListType = open_config(config_path)?;
    config_list
        .find_config(config_name)
        .ok_or(Box::new(ConfigurationNotFound {
            path: config_path.to_owned(),
            name: config_name.to_string(),
        }))
}

mod tests {
    #[allow(unused_imports)]
    use crate::config::backend::BackendConfig;
    #[allow(unused_imports)]
    use crate::config::find_config;
    #[allow(unused_imports)]
    use std::path::Path;

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
