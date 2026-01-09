use super::*;
use data::BenchmarkDataset;
use std::str::FromStr;
use tempfile::NamedTempFile;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
struct Dummy(f64);

impl From<Dummy> for f64 {
    fn from(val: Dummy) -> Self {
        val.0
    }
}

impl FromStr for Dummy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<f64>().map(Dummy).map_err(|_| ())
    }
}

// Creates SeriesPlot with mock data.
fn setup_test_plot() -> SeriesPlot {
    let dataset = BenchmarkDataset {
        points: vec![1, 2, 3],
        results: vec![
            vec![Some(Dummy(10.0)), Some(Dummy(20.0)), None],
            vec![Some(Dummy(5.0)), Some(Dummy(15.0)), Some(Dummy(20.0))],
        ],
    };
    let names = vec!["first".to_string(), "second".to_string()];
    SeriesPlot::from_dataset(
        dataset,
        "TestBenchmark".to_string(),
        &names,
        VisKind::Linear,
    )
    .unwrap()
}

#[test]
fn series_plot_runs() {
    let plot = setup_test_plot();

    let path = NamedTempFile::new().unwrap().path().with_extension("png");
    let result = plot_on_backend(plot, path.to_str().unwrap(), OutputFormat::Png);
    assert!(result.is_ok());
}

#[test]
fn series_plot_fails_file_not_found() {
    let result = plot_on_backend(
        setup_test_plot(),
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
    fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();

    let plot = setup_test_plot();

    let result = plot_on_backend(plot, "no_write.png", OutputFormat::Png);

    assert!(matches!(result.unwrap_err(), PlotError::Plotters(msg)
        if msg == "backend error: Drawing backend error: ImageError(IoError(Os { code: 13, kind: PermissionDenied, message: \"Permission denied\" }))"));

    fs::remove_file(path).unwrap();
}

#[test]
fn series_plot_fails_incompatible_file_extension() {
    let plot = setup_test_plot();

    let path = NamedTempFile::new()
        .unwrap()
        .path()
        .to_path_buf()
        .with_extension("svg");
    let result = plot_on_backend(plot, path.to_str().unwrap(), OutputFormat::Png);

    assert!(
        matches!(result.unwrap_err(), PlotError::IncompatibleFileExtension { extension, format}
            if extension == "svg" && format == "png")
    );
}
