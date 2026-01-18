use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use fs_err as fs;

use crate::plotting::core::{
    ArtifactFile, BackendWithKind, BenchmarkDataset, Plot, Renderable, RenderableFlamegraph,
};
use crate::plotting::{IMAGE_WIDTH, PlotError};
use crate::utilities::BenchmarkPoint;

pub struct FlamegraphPlot {
    benchmark_name: String,
    results: Vec<RenderableFlamegraph>,
    output: PathBuf,
}

impl FlamegraphPlot {
    fn new(benchmark_name: String, results: Vec<RenderableFlamegraph>, output: PathBuf) -> Self {
        FlamegraphPlot {
            benchmark_name,
            results,
            output,
        }
    }

    fn populate_artifact(
        artifact: &ArtifactFile,
        data: &String,
        flame_repo: &Path,
        name: &String,
        point: BenchmarkPoint,
    ) -> Result<(), PlotError> {
        let mut child = Command::new(flame_repo.join("flamegraph.pl"))
            .arg("--width")
            .arg(IMAGE_WIDTH.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        child
            .stdin
            .as_mut()
            .ok_or_else(|| PlotError::Internal("Failed to open stdin for flamegraph.pl".into()))?
            .write_all(data.as_bytes())?;

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(PlotError::Internal(format!(
                "flamegraph.pl failed on {name} data point {point}"
            )));
        }

        let mut stdout_str = String::from_utf8(output.stdout)
            .map_err(|_| PlotError::Internal("stdout UTF-8 parsing failed".into()))?;

        stdout_str = stdout_str.replace(">Flame Graph<", format!(">{name} at {point}<").as_str());

        fs::write(artifact.path(), &stdout_str)?;
        Ok(())
    }

    pub fn from_dataset(
        dataset: BenchmarkDataset<String>,
        benchmark_name: String,
        names: &[String],
        output: PathBuf,
        flame_repo: PathBuf,
        artifacts_dir: Option<PathBuf>,
    ) -> Result<Self, PlotError> {
        let mut results = Vec::new();

        for (name, data) in names.iter().zip(dataset.results.into_iter()) {
            let make_artifact = |folded_data: Option<&_>,
                                 point: BenchmarkPoint|
             -> Result<ArtifactFile, PlotError> {
                let artifact = match folded_data {
                    Some(point_data) => {
                        let artifact = if let Some(dir) = &artifacts_dir {
                            let path = dir.join(format!("{benchmark_name}_{name}_{point}.svg"));
                            ArtifactFile::from_path(path)?
                        } else {
                            ArtifactFile::temp()
                        };

                        FlamegraphPlot::populate_artifact(
                            &artifact,
                            point_data,
                            &flame_repo,
                            name,
                            point,
                        )?;
                        artifact
                    }
                    None => ArtifactFile::temp(),
                };

                Ok(artifact)
            };

            let artifacts: Vec<ArtifactFile> = data
                .iter()
                .zip(dataset.points.iter())
                .map(|(folded_data, point)| make_artifact(folded_data.as_ref(), *point))
                .collect::<Result<_, PlotError>>()?;

            results.push(RenderableFlamegraph::new(output.clone(), artifacts));
        }

        Ok(FlamegraphPlot::new(benchmark_name, results, output))
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
                .write(true)
                .truncate(true)
                .open(&self.output)?;

            let header = format!(
                r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Benchmark {} results</title>
</head>
<body style="margin:0">
	<h1>Benchmark {} results</h1>"#,
                self.benchmark_name, self.benchmark_name
            );

            writeln!(file, "{header}")?;
        }

        for renderable in &self.results {
            <RenderableFlamegraph as Renderable<'_, DB>>::add_to_plot(renderable, &mut [])?;
        }

        let mut file = OpenOptions::new().append(true).open(&self.output)?;

        let footer = r"</body></html>";
        writeln!(file, "{footer}")?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "flamegraph plot"
    }
}
