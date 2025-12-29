use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct AliasingConfig {
    pub dp_path: Option<PathBuf>,
    pub repo_path: HashMap<String, PathBuf>,
}
