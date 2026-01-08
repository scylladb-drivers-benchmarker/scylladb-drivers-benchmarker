mod data;
pub mod error;
mod plot;
mod render;
mod series;

use crate::config::benchmark::BenchmarkConfig;
use crate::database::Database;
use crate::{commit_hash::CommitHash, measurement::MeasurementMethod};

use error::PlotError;
use plot::{Plot, SeriesPlot};
pub use series::VisKind;

use plotters::backend::{BitMapBackend, SVGBackend};

const IMAGE_SIZE: (u32, u32) = (1920, 1080);

#[derive(Debug, clap::Subcommand)]
pub enum PlotKind {
    /// Generate a series plot
    Series {
        #[arg(short, long, value_enum, default_value_t = VisKind::Linear)]
        visualization_kind: VisKind,
    },

    /// Generate a flamegraph plot
    Flamegraph,

    /// Generate a perf-stat plot
    PerfStat {
        #[arg(short, long)]
        #[clap(value_delimiter=',', num_args(1..))]
        events: Vec<String>,
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Png,
    Svg,
}

fn plot_on_backend<P: Plot>(plot: P, output: &str, format: OutputFormat) -> Result<(), PlotError> {
    let extension = output.rsplit('.').next().unwrap_or("").to_string();

    match format {
        OutputFormat::Png => {
            if extension != "png" {
                return Err(PlotError::IncompatibleFileExtension {
                    extension,
                    format: "png".to_string(),
                });
            }

            let backend = BitMapBackend::new(output, IMAGE_SIZE);
            plot.plot(backend)
        }

        OutputFormat::Svg => {
            if extension != "svg" {
                return Err(PlotError::IncompatibleFileExtension {
                    extension,
                    format: "svg".to_string(),
                });
            }
            let backend = SVGBackend::new(output, IMAGE_SIZE);
            plot.plot(backend)
        }
    }
}

pub fn plot(
    plot_kind: PlotKind,
    database: &Database,
    benchmark_name: &str,
    benchmark_config: &BenchmarkConfig,
    measurement_method: &MeasurementMethod,
    commit_hashes: impl Iterator<Item = CommitHash>,
    names: &[String],
    format: OutputFormat,
    output: &str,
) -> Result<(), PlotError> {
    match plot_kind {
        PlotKind::Series { visualization_kind } => {
            let plot = SeriesPlot::build(
                database,
                benchmark_name,
                benchmark_config,
                measurement_method,
                commit_hashes,
                names,
                visualization_kind,
            )?;

            plot_on_backend(plot, output, format)
        }

        PlotKind::Flamegraph => {
            unimplemented!("Flamegraph plotting is not yet implemented");
            // let plot = FlamegraphPlot::build(
            //     database,
            //     benchmark_name,
            //     benchmark_config,
            //     measurement_method,
            //     commit_hashes,
            //     names,
            // )?;

            // plot_on_backend(plot, output, format)
        }

        PlotKind::PerfStat { events } => {
            events.into_iter().try_for_each(|_event| -> Result<(), PlotError> {
                todo!()  // plot for event
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data::BenchmarkDataset;
    use serde::Deserialize;
    use tempfile::NamedTempFile;

    #[derive(Clone, Debug, PartialOrd, Deserialize)]
    struct Dummy(f64);

    impl From<Dummy> for f64 {
        fn from(val: Dummy) -> Self {
            val.0
        }
    }

    impl PartialEq for Dummy {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    #[test]
    fn series_plot_runs() {
        let dataset = BenchmarkDataset {
            points: vec![1, 2, 3],
            results: vec![
                vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
                vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
            ],
        };

        let names = vec!["first".to_string(), "second".to_string()];
        let plot = SeriesPlot::from_dataset(
            dataset,
            "TestBenchmark".to_string(),
            &names,
            VisKind::Linear,
        )
        .unwrap();

        let mut tmp_path = NamedTempFile::new().unwrap().path().to_path_buf();
        tmp_path.set_extension("png");
        let path_png = tmp_path.to_str().unwrap();
        let result = plot_on_backend(plot, path_png, OutputFormat::Png);
        assert!(result.is_ok());
    }

    #[test]
    fn series_plot_fails_file_not_found() {
        let dataset = BenchmarkDataset {
            points: vec![1, 2, 3],
            results: vec![
                vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
                vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
            ],
        };

        let names = vec!["first".to_string(), "second".to_string()];
        let plot = SeriesPlot::from_dataset(
            dataset,
            "TestBenchmark".to_string(),
            &names,
            VisKind::Linear,
        )
        .unwrap();

        let result = plot_on_backend(
            plot,
            "/this/path/should/not/exist/lmao.png",
            OutputFormat::Png,
        );

        assert!(matches!(result.unwrap_err(), PlotError::Plotters(problem)
        if problem == "backend error: Drawing backend error: ImageError(IoError(Os { code: 2, kind: NotFound, message: \"No such file or directory\" }))"));
    }

    #[cfg(unix)]
    #[test]
    fn series_plot_fails_on_permission_denied() {
        use std::fs::{self, File};
        use std::os::unix::fs::PermissionsExt;
        use std::path::Path;

        let path = Path::new("no_write.png");
        File::create(path).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(path, perms).unwrap();

        let dataset = BenchmarkDataset {
            points: vec![1, 2, 3],
            results: vec![
                vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
                vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
            ],
        };

        let names = vec!["first".to_string(), "second".to_string()];
        let plot = SeriesPlot::from_dataset(
            dataset,
            "TestBenchmark".to_string(),
            &names,
            VisKind::Linear,
        )
        .unwrap();

        let result = plot_on_backend(plot, "no_write.png", OutputFormat::Png);

        assert!(matches!(result.unwrap_err(), PlotError::Plotters(msg)
        if msg == "backend error: Drawing backend error: ImageError(IoError(Os { code: 13, kind: PermissionDenied, message: \"Permission denied\" }))"));

        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn series_plot_fails_incompatible_file_extension() {
        let dataset = BenchmarkDataset {
            points: vec![1, 2, 3],
            results: vec![
                vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
                vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
            ],
        };

        let names = vec!["first".to_string(), "second".to_string()];
        let plot = SeriesPlot::from_dataset(
            dataset,
            "TestBenchmark".to_string(),
            &names,
            VisKind::Linear,
        )
        .unwrap();

        let mut tmp_path = NamedTempFile::new().unwrap().path().to_path_buf();
        tmp_path.set_extension("svg");
        let path_png = tmp_path.to_str().unwrap();
        let result = plot_on_backend(plot, path_png, OutputFormat::Png);

        assert!(
            matches!(result.unwrap_err(), PlotError::IncompatibleFileExtension { extension, format} 
            if extension == "svg" && format == "png")
        );
    }
}
