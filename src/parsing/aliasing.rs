use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AliasingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub repo_path: HashMap<String, PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flame_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_dir: Option<PathBuf>,
}

#[justerror::Error(desc = "Failed reading the main config file")]
pub enum MainConfigError {
    FailedOpening(#[from] io::Error),
    FailedParsing(#[from] serde_yml::Error),
}

impl AliasingConfig {
    pub fn read_config(path: &Path) -> Result<Self, MainConfigError> {
        let file = File::open(path)?;
        Ok(serde_yml::from_reader(file)?)
    }
}
