use super::*;
use data::BenchmarkDataset;
use fs_err as fs;
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
    let names = vec!["first".to_owned(), "second".to_owned()];
    SeriesPlot::from_dataset(dataset, "TestBenchmark".to_owned(), &names, VisKind::Linear).unwrap()
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
    use fs::{self, File};
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    // TODO zmienic na tmp lokalizacje, jesli w trakcje debugu ten plik nie zostanie usuniety, to test failuje
    let path = Path::new("no_write.png");
    File::create(path).unwrap();
    fs::set_permissions(path, Permissions::from_mode(0o444)).unwrap();

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

#[test]
fn perf_stat_plot_runs() {
    use crate::perf_stat::PerfStatData;
    let points = vec![1, 2];

    let prog1_step1 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000374","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"2,670862","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog1_step2 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000474","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"3,670862","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog2_step1 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000500","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"2,000000","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog2_step2 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000600","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"3,000000","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset {
        points: points.clone(),
        results: vec![
            vec![Some(prog1_step1), Some(prog1_step2)],
            vec![Some(prog2_step1), Some(prog2_step2)],
        ],
    };

    let plot = PerfStatPlot::from_dataset(
        dataset,
        "TestPerfStat".to_owned(),
        &["prog1".to_owned(), "prog2".to_owned()],
        vec!["task-clock".to_owned(), "context-switches".to_owned()],
    )
    .unwrap();

    let mut tmp_path = NamedTempFile::new().unwrap().path().to_path_buf();
    tmp_path.set_extension("png");
    let path_png = tmp_path.to_str().unwrap();
    let result = plot_on_backend(plot, path_png, OutputFormat::Png);
    assert!(result.is_ok());
}

#[test]
fn invalid_perf_metric() {
    use crate::perf_stat::PerfStatData;
    let points = vec![1, 2];

    let prog1_step1 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000374","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"2,670862","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog1_step2 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000474","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"3,670862","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog2_step1 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000500","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"2,000000","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let prog2_step2 = PerfStatData::from_str(
        r#"
        {"event":"task-clock","metric-value":"0,000600","metric-unit":"CPUs utilized"}
        {"event":"context-switches","metric-value":"3,000000","metric-unit":"K/sec"}
    "#,
    )
    .unwrap();

    let dataset: BenchmarkDataset<PerfStatData> = BenchmarkDataset {
        points: points.clone(),
        results: vec![
            vec![Some(prog1_step1), Some(prog1_step2)],
            vec![Some(prog2_step1), Some(prog2_step2)],
        ],
    };

    let result = PerfStatPlot::from_dataset(
        dataset,
        "TestPerfStat".to_owned(),
        &["prog1".to_owned(), "prog2".to_owned()],
        vec!["incorrect-metric".to_owned()],
    );

    assert!(matches!(
        result,
        Err(PlotError::InvalidData(ref error)) if error == "Event 'incorrect-metric' not found in dataset"
    ));
}
