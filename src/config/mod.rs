pub mod backend;
pub mod benchmark;
pub mod config_traits;

use std::path::{Path, PathBuf};

use config_traits::{Configuration, ConfigurationList};

use fs_err as fs;

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
    let file = fs::File::open(config_path).map_err(|source| ConfigError::FileOperationError {
        source,
        path: config_path.to_path_buf(),
    })?;
    serde_yml::from_reader(file).map_err(|source| ConfigError::ParseError {
        source,
        path: config_path.to_path_buf(),
    })
}

pub fn find_config<ConfigType: Configuration>(
    benchmark_name: &str,
    config_path: &Path,
) -> Result<ConfigType, ConfigError> {
    let config_list: ConfigType::ConfigListType = open_config(config_path)?;
    config_list
        .find_config(benchmark_name.as_ref())
        .ok_or(ConfigError::ConfigurationNotFound {
            path: config_path.to_path_buf(),
            benchmark_name: benchmark_name.to_owned(),
        })
}

#[cfg(test)]
mod tests {
    use crate::config::backend::BackendConfig;
    use crate::config::{ConfigError, find_config};
    use std::path::Path;

    #[test]
    fn open_config() {
        let config: BackendConfig =
            find_config("select", Path::new("./configs/backend_config.yml")).unwrap();

        assert_eq!(config.name, "scylladb-nodejs-rs-driver");
        assert_eq!(config.benchmark_name, "select");
        assert_eq!(config.build_command, "npm run build");
        assert_eq!(
            config.run_command,
            "node benchmark/logic/select.js scylladb-nodejs-rs-driver"
        );
    }

    #[test]
    fn config_error() {
        let benchmark_name = "drop_table";
        let config_path = Path::new("./configs/backend_config.yml");

        let error: ConfigError =
            find_config::<BackendConfig>(benchmark_name, config_path).unwrap_err();
        let ConfigError::ConfigurationNotFound {
            path,
            benchmark_name,
        } = error
        else {
            panic!("expected configuration not found, but got: {:?}", error);
        };
        assert_eq!(benchmark_name, "drop_table");
        assert!(path.ends_with(config_path));
    }
}
