use super::render::{Renderable, RenderableFlamegraph};

use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub(crate) struct ArtifactFile {
    path: PathBuf,
    _tmp: Option<NamedTempFile>,
}

impl ArtifactFile {
    pub(crate) fn from_path(path: PathBuf) -> Self {
        Self { path, _tmp: None }
    }

    pub(crate) fn temp() -> Self {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        Self {
            path,
            _tmp: Some(tmp),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

pub(crate) struct FlamegraphPlot {
    pub benchmark_name: String,
    pub results: Vec<RenderableFlamegraph>,
}
