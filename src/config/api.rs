//! Per-API configuration: `apis/<api>/api-config.yml` in the benchmarks
//! repository. All commands it names are executed with the working directory
//! set to the api directory.

use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};

pub const API_CONFIG_FILE: &str = "api-config.yml";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ApiConfig {
    pub api: String,
    /// Script linking the driver under test into the benchmark project.
    pub env_prepare: String,
    /// Script reverting whatever env-prepare generated.
    pub env_cleanup: String,
    /// Command building the benchmark project (run once, unmeasured).
    pub build_command: String,
    /// Command invoked for every phase of every benchmark (via env vars).
    pub run_command: String,
}

/// An [`ApiConfig`] together with the directory it was loaded from
/// (the cwd for all its commands).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiSetup {
    pub config: ApiConfig,
    pub api_dir: PathBuf,
}

#[justerror::Error(desc = "failed to load the api config")]
pub enum ApiConfigError {
    #[error(desc = "cannot read {path}: {source} - does the benchmarks repository support api '{api}'?")]
    Unreadable {
        api: String,
        path: PathBuf,
        source: std::io::Error,
    },
    Invalid {
        path: PathBuf,
        source: serde_yml::Error,
    },
    #[error(desc = "api-config.yml at {path} declares api '{declared}', but '{requested}' was requested")]
    ApiMismatch {
        path: PathBuf,
        declared: String,
        requested: String,
    },
}

impl ApiSetup {
    /// Loads `apis/<api>/api-config.yml` from the benchmarks repository.
    pub fn load(benchmarks_path: &Path, api: &str) -> Result<Self, ApiConfigError> {
        let api_dir = benchmarks_path.join("apis").join(api);
        let path = api_dir.join(API_CONFIG_FILE);
        let contents = fs::read(&path).map_err(|source| ApiConfigError::Unreadable {
            api: api.to_owned(),
            path: path.clone(),
            source,
        })?;
        let config: ApiConfig = serde_yml::from_slice(&contents)
            .map_err(|source| ApiConfigError::Invalid { path: path.clone(), source })?;
        if config.api != api {
            return Err(ApiConfigError::ApiMismatch {
                path,
                declared: config.api,
                requested: api.to_owned(),
            });
        }
        Ok(ApiSetup { config, api_dir })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_api_config() {
        let config: ApiConfig = serde_yml::from_str(
            "api: rust-v1\nenv-prepare: ./env_prepare.sh\nenv-cleanup: ./env_cleanup.sh\nbuild-command: cargo build --release\nrun-command: ./run.sh\n",
        )
        .unwrap();
        assert_eq!(config.api, "rust-v1");
        assert_eq!(config.env_prepare, "./env_prepare.sh");
        assert_eq!(config.build_command, "cargo build --release");
    }
}
