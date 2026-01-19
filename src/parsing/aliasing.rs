use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct AliasingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub repo_path: HashMap<String, PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flame_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_config: Option<PathBuf>,
}

#[justerror::Error(desc = "Failed reading the main config file")]
pub(crate) enum MainConfigError {
    FailedOpening(#[from] std::io::Error),
    FailedParsing(#[from] serde_yml::Error),
}

impl AliasingConfig {
    pub fn read_config(path: &Path) -> Result<Self, MainConfigError> {
        Ok(serde_yml::from_slice(&fs::read(path)?)?)
    }
}
