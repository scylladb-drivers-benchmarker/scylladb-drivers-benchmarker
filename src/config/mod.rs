pub mod backend;
pub mod benchmark;
pub mod config_traits;

use std::path::{Path, PathBuf};

use config_traits::{Configuration, ConfigurationList};

#[justerror::Error(desc = "error reading from config")]
pub enum ConfigError {
    FileOperationError {
        source: std::io::Error,
        path: PathBuf,
    },
    ParseError {
        source: serde_yml::Error,
        path: PathBuf,
    },
    #[error(desc = "configuration not found")]
    ConfigurationNotFound {
        path: PathBuf,
        benchmark_name: String,
    },
}

pub fn open_config<ConfigListType: ConfigurationList>(
    config_path: &Path,
) -> Result<ConfigListType, ConfigError> {
    let file =
        std::fs::File::open(config_path).map_err(|source| ConfigError::FileOperationError {
            source,
            path: config_path.to_path_buf(),
        })?;
    serde_yml::from_reader(file).map_err(|source| ConfigError::ParseError {
        source,
        path: config_path.to_path_buf(),
    })
}

pub fn find_config<ConfigType: Configuration>(
    benchmark_name: impl AsRef<str>,
    config_path: &Path,
) -> Result<ConfigType, ConfigError> {
    let config_list: ConfigType::ConfigListType = open_config(config_path)?;
    config_list
        .find_config(benchmark_name.as_ref())
        .ok_or(ConfigError::ConfigurationNotFound {
            path: config_path.to_path_buf(),
            benchmark_name: benchmark_name.as_ref().to_owned()
        })
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
        let config: BackendConfig =
            find_config("select", Path::new("./configs/backend_config.yml")).unwrap();
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
