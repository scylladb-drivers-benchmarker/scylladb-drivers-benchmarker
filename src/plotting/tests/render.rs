use crate::plotting::core::{Renderable, RenderablePerfStat, RenderableSeries};
use plotters::prelude::*;
use tempfile::NamedTempFile;

use fs_err as fs;

#[test]
fn renderable_series_runs() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    let backend = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();
    backend.fill(&WHITE).unwrap();

    let chart = ChartBuilder::on(&backend)
        .margin(10)
        .build_cartesian_2d(0u64..10u64, 0f64..100f64)
        .unwrap();

    let series = RenderableSeries::new(
        "Test series".into(),
        (0..10).collect(),
        vec![
            Some(10.0),
            Some(20.0),
            Some(30.0),
            None,
            Some(50.0),
            Some(60.0),
            Some(70.0),
            Some(80.0),
            Some(90.0),
            Some(100.0),
        ],
        Palette99::pick(0),
        Some((10.0, 100.0)),
    );

    let result = series.add_to_plot(&mut [chart]);
    assert!(result.is_ok());
}

#[test]
fn renderable_perf_stat_runs() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    let backend = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();
    backend.fill(&WHITE).unwrap();

    let areas = backend.split_evenly((3, 1));

    let mut charts: Vec<_> = areas
        .into_iter()
        .map(|area| {
            ChartBuilder::on(&area)
                .build_cartesian_2d(0u64..10u64, 0f64..100f64)
                .unwrap()
        })
        .collect();

    let perfstat = RenderablePerfStat::new(
        "Test perfstat".into(),
        (0..10).collect(),
        vec![
            vec![
                Some(10.0),
                Some(20.0),
                None,
                Some(40.0),
                Some(50.0),
                None,
                Some(70.0),
                Some(80.0),
                Some(90.0),
                Some(100.0),
            ],
            vec![
                Some(5.0),
                None,
                Some(25.0),
                Some(35.0),
                None,
                Some(55.0),
                Some(65.0),
                None,
                Some(85.0),
                Some(95.0),
            ],
            vec![
                None,
                Some(15.0),
                Some(30.0),
                None,
                Some(45.0),
                Some(60.0),
                None,
                Some(75.0),
                Some(90.0),
                None,
            ],
        ],
        Palette99::pick(1),
        vec![Some((10.0, 100.0)), Some((5.0, 95.0)), Some((15.0, 90.0))],
    );

    let result = perfstat.add_to_plot(&mut charts);
    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn renderable_perf_stat_view() {
    use fs::{self, File};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    let path = Path::new("test.png");
    File::create(path).unwrap();
    let backend = BitMapBackend::new(&path, (1920, 1080)).into_drawing_area();
    backend.fill(&WHITE).unwrap();

    let areas = backend.split_evenly((3, 1));

    let mut charts: Vec<_> = areas
        .into_iter()
        .map(|area| {
            ChartBuilder::on(&area)
                .build_cartesian_2d(0u64..10u64, 0f64..100f64)
                .unwrap()
        })
        .collect();

    let perfstat = RenderablePerfStat::new(
        "Test perfstat".into(),
        (0..10).collect(),
        vec![
            vec![
                Some(10.0),
                Some(20.0),
                None,
                Some(40.0),
                Some(50.0),
                None,
                Some(70.0),
                Some(80.0),
                Some(90.0),
                Some(100.0),
            ],
            vec![
                Some(5.0),
                None,
                Some(25.0),
                Some(35.0),
                None,
                Some(55.0),
                Some(65.0),
                None,
                Some(85.0),
                Some(95.0),
            ],
            vec![
                None,
                Some(15.0),
                Some(30.0),
                None,
                Some(45.0),
                Some(60.0),
                None,
                Some(75.0),
                Some(90.0),
                None,
            ],
        ],
        Palette99::pick(1),
        vec![Some((10.0, 100.0)), Some((5.0, 95.0)), Some((15.0, 90.0))],
    );

    let result = perfstat.add_to_plot(&mut charts);
    assert!(result.is_ok());

    for chart in &mut charts {
        chart
            .configure_series_labels()
            .border_style(BLACK)
            .draw()
            .unwrap();
    }

    // To view the semi-rendered image, comment the following lines.
    drop(charts);
    drop(backend);
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(path, perms).unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
#[should_panic(expected = "Number of charts must match number of metrics")]
fn renderable_perfstat_panics_on_mismatch() {
    let perfstat = RenderablePerfStat::new(
        "bad".into(),
        vec![0, 1],
        vec![vec![Some(1.0), Some(2.0)]],
        Palette99::pick(0),
        vec![Some((1.0, 2.0))],
    );

    let file = NamedTempFile::new().unwrap();
    let backend = BitMapBackend::new(file.path(), (100, 100)).into_drawing_area();

    let areas = backend.split_evenly((1, 3));

    let mut charts: Vec<_> = areas
        .into_iter()
        .map(|area| {
            ChartBuilder::on(&area)
                .build_cartesian_2d(0u64..10u64, 0f64..100f64)
                .unwrap()
        })
        .collect();

    let _ = perfstat.add_to_plot(&mut charts);
}
