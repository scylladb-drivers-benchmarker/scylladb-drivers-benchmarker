use super::BenchmarkConfig;
use super::CommitHash;
use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::plot::*;
use super::render::{Renderable, RenderableFlamegraph};

use crate::Database;
use crate::measurement::MeasurementMethod;

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
    benchmark_name: String,
    results: Vec<RenderableFlamegraph>,
    output: PathBuf,
    flame_repo: PathBuf,
    artifacts_dir: Option<PathBuf>,
}

impl FlamegraphPlot {
    fn new(
        benchmark_name: String,
        results: Vec<RenderableFlamegraph>,
        output: PathBuf,
        flame_repo: PathBuf,
        artifacts_dir: Option<PathBuf>,
    ) -> Self {
        FlamegraphPlot {
            benchmark_name,
            results,
            output,
            flame_repo,
            artifacts_dir,
        }
    }

    pub(crate) fn from_dataset(
        dataset: BenchmarkDataset<String>,
        benchmark_name: String,
        names: &[String],
        output: PathBuf,
        flame_repo: PathBuf,
        artifacts_dir: Option<PathBuf>,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        for (name, data) in names.iter().zip(dataset.results.into_iter()) {
            let artifact: ArtifactFile = match &artifacts_dir {
                Some(path) => ArtifactFile::from_path(path.clone()),
                None => ArtifactFile::temp(),
            };

            results.push(RenderableFlamegraph::new(
                name.clone(),
                dataset.points.clone(),
                data,
                output.clone(),
                artifact,
            ));
        }

        Ok(FlamegraphPlot::new(
            benchmark_name,
            results,
            output,
            flame_repo,
            artifacts_dir,
        ))
    }
}
