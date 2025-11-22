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
