use super::data::BenchmarkDataset;
use super::error::PlotError;
use super::plot::*;
use super::render::{Renderable, RenderableFlamegraph};

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
            let artifacts: Vec<ArtifactFile> = match &artifacts_dir {
                Some(dir) => {
                    let mut artifacts_result = Vec::new();

                    for (i, folded) in data.iter().enumerate() {
                        let artifact = match folded {
                            Some(folded_data) => {
                                let path = dir.join(format!("{}_{}_{}.svg", benchmark_name, name, i));
                                let artifact = ArtifactFile::from_path(path);

                                let mut child = Command::new(flame_repo.join("flamegraph.pl"))
                                    .arg("--width")
                                    .arg("1920") // TODO const
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn()
                                    .map_err(|e| {
                                        PlotError::from_io_with_path(
                                            e,
                                            artifact.path().to_string_lossy(),
                                        )
                                    })?;

                                {
                                    let stdin = child.stdin.as_mut().ok_or_else(|| {
                                        PlotError::Internal(
                                            "Failed to open stdin for flamegraph.pl".into(),
                                        )
                                    })?;

                                    stdin.write_all(folded_data.as_bytes()).map_err(|e| {
                                        PlotError::from_io_with_path(
                                            e,
                                            artifact.path().to_string_lossy(),
                                        )
                                    })?;
                                }

                                let output = child.wait_with_output().map_err(|e| {
                                    PlotError::from_io_with_path(
                                        e,
                                        artifact.path().to_string_lossy(),
                                    )
                                })?;

                                if !output.status.success() {
                                    return Err(PlotError::Internal(format!(
                                        "flamegraph.pl failed on {} data point {}",
                                        name,
                                        i
                                    )));
                                }

                                let mut stdout_str = String::from_utf8(output.stdout.to_vec())
                                    .map_err(|_| PlotError::Internal(
                                        "stdout UTF-8 parsing failed".into()
                                    ))?;

                                stdout_str = stdout_str.replace(">Flame Graph<", format!("{} at {}", name, i).as_str());

                                fs::write(artifact.path(), &stdout_str).map_err(|e| {
                                    PlotError::from_io_with_path(
                                        e,
                                        artifact.path().to_string_lossy(),
                                    )
                                })?;

                                artifact
                            }
                            None => ArtifactFile::temp(),
                        };

                        artifacts_result.push(artifact);
                    }

                    artifacts_result
                }
                None => {
                    let mut artifacts_result = Vec::new();

                    for folded in data.iter() {
                        let artifact = match folded {
                            Some(folded_data) => {
                                let artifact = ArtifactFile::temp();

                                let mut child = Command::new(flame_repo.join("flamegraph.pl"))
                                    .arg("--width")
                                    .arg("1920")
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn()
                                    .map_err(|e| {
                                        PlotError::from_io_with_path(
                                            e,
                                            artifact.path().to_string_lossy(),
                                        )
                                    })?;

                                {
                                    let stdin = child.stdin.as_mut().ok_or_else(|| {
                                        PlotError::Internal(
                                            "Failed to open stdin for flamegraph.pl".into(),
                                        )
                                    })?;

                                    stdin.write_all(folded_data.as_bytes()).map_err(|e| {
                                        PlotError::from_io_with_path(
                                            e,
                                            artifact.path().to_string_lossy(),
                                        )
                                    })?;
                                }

                                let output = child.wait_with_output().map_err(|e| {
                                    PlotError::from_io_with_path(
                                        e,
                                        artifact.path().to_string_lossy(),
                                    )
                                })?;

                                if !output.status.success() {
                                    return Err(PlotError::InvalidData(
                                        "flamegraph.pl failed for temp artifact".to_string(),
                                    ));
                                }

                                fs::write(artifact.path(), &output.stdout).map_err(|e| {
                                    PlotError::from_io_with_path(
                                        e,
                                        artifact.path().to_string_lossy(),
                                    )
                                })?;

                                artifact
                            }
                            None => ArtifactFile::temp(),
                        };

                        artifacts_result.push(artifact);
                    }

                    artifacts_result
                }
            };

            results.push(RenderableFlamegraph::new(
                name.clone(),
                dataset.points.clone(),
                data,
                output.clone(),
                artifacts,
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

impl Plot for FlamegraphPlot {
    fn plot<DB: plotters_backend::DrawingBackend + BackendWithKind>(
        &self,
        _: DB,
    ) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static,
    {
        {
            let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(true)
            .open(&self.output)
            .map_err(|e| PlotError::from_io_with_path(e, self.output.display().to_string()))?;

            writeln!(file, "{}", format!(r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Benchmark {} results</title>
</head>
<body style="margin:0">
	<h1>Benchmark {} results</h1>"#, self.benchmark_name, self.benchmark_name))?;
        }

        for renderable in &self.results {
            <RenderableFlamegraph as Renderable<'_, DB>>::add_to_plot(renderable, &mut [])?;
        }

        {
            let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output)
            .map_err(|e| PlotError::from_io_with_path(e, self.output.display().to_string()))?;

            writeln!(file, "{}", format!(r#"</body></html>"#))?;
        }

        Ok(())
    }
}
