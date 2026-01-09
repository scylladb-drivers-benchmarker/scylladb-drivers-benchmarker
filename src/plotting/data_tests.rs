use std::fs::OpenOptions;

use crate::CommitHash;
use crate::Database;
use crate::MeasurementMethod;
use crate::PlotError;
use crate::config;
use crate::database::utilities::BenchmarkParams;
use crate::database::utilities::{BenchmarkFilters, BenchmarkRecord};
use crate::plotting::BenchmarkConfig;
use crate::plotting::data::BenchmarkDataset;
use crate::utilities::BenchmarkPoint;
use tempfile::NamedTempFile;

// Insert record to database, with provided commit_hash, config, point and result
// Measurement method is always "time".
fn insert_bench(
    db: &Database,
    hash: &CommitHash,
    conf: &BenchmarkConfig,
    step: BenchmarkPoint,
    val: &str,
) {
    db.insert_data(
        BenchmarkParams::new(hash.clone(), conf.name.clone(), step, String::from("time")),
        BenchmarkRecord::Data(val.to_owned()),
    )
    .unwrap();
}

struct TestSetup {
    db: Database,
    file: NamedTempFile,
    path: std::path::PathBuf,
    configs: Vec<BenchmarkConfig>,
    hashes: Vec<CommitHash>,
    measure: MeasurementMethod,
}

// Creates mock data and initialises structs with it.
fn init_db() -> TestSetup {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    let db = Database::new(&path).unwrap();

    let configs = vec![
        BenchmarkConfig {
            name: "benchmark1".to_string(),
            data: config::benchmark::BenchmarkData {
                starting_step: 1,
                no_steps: 3,
                step_progress: 1,
                progress_type: config::benchmark::ProgressType::Additive,
                timeout: None,
            },
        },
        BenchmarkConfig {
            name: "benchmark2".to_string(),
            data: config::benchmark::BenchmarkData {
                starting_step: 10,
                no_steps: 1,
                step_progress: 1,
                progress_type: config::benchmark::ProgressType::Additive,
                timeout: None,
            },
        },
    ];

    let hashes = vec![
        CommitHash::new_unchecked("1".to_string()),
        CommitHash::new_unchecked("2".to_string()),
        CommitHash::new_unchecked("3".to_string()),
    ];

    insert_bench(&db, &hashes[0], &configs[0], 1, "1.5");
    insert_bench(&db, &hashes[0], &configs[0], 2, "2.5");
    insert_bench(&db, &hashes[0], &configs[0], 3, "4.5");

    insert_bench(&db, &hashes[1], &configs[0], 1, "2");
    insert_bench(&db, &hashes[1], &configs[0], 2, "3.5");
    insert_bench(&db, &hashes[1], &configs[0], 3, "5.5");

    insert_bench(&db, &hashes[2], &configs[1], 10, "3.5");

    TestSetup {
        db,
        file,
        path,
        configs,
        hashes,
        measure: MeasurementMethod::Time,
    }
}

#[test]
fn extract() {
    let TestSetup {
        db,
        file: _file,
        path: _path,
        configs,
        hashes,
        measure,
    } = init_db();

    let dataset: BenchmarkDataset<f64> = BenchmarkDataset::new(
        &db,
        &configs[0],
        [hashes[0].clone(), hashes[1].clone()].into_iter(),
        &measure,
    )
    .unwrap();
    assert_eq!(dataset.points, vec![1, 2, 3]);
    assert_eq!(
        dataset.results,
        vec![
            vec![Some(1.5), Some(2.5), Some(4.5)],
            vec![Some(2.0), Some(3.5), Some(5.5)]
        ]
    );

    let dataset: BenchmarkDataset<f64> =
        BenchmarkDataset::new(&db, &configs[1], [hashes[2].clone()].into_iter(), &measure).unwrap();
    assert_eq!(dataset.points, vec![10]);
    assert_eq!(dataset.results, vec![vec![Some(3.5)]]);
}

macro_rules! drop_and_expect_failure {
    ($db:expr, $conf:expr, $hash:expr, $measure:expr, $drop:expr, $err_pat:pat $(if $guard:expr)?) => {
        $db.drop_data(&BenchmarkFilters {
            commit_hashes: vec![$hash.as_str().to_owned()],
            benchmark_names: vec![$conf.name.clone()],
            benchmark_points: $drop,
            measurement_methods: vec![$measure.to_string()],
        }).unwrap();

        let err = BenchmarkDataset::<f64>::new(&$db, &$conf, [$hash.clone()].into_iter(), &$measure).unwrap_err();
        assert!(matches!(err, $err_pat $(if $guard)?));
    };
}

#[test]
fn extract_failure() {
    let TestSetup {
        db,
        file: _file,
        path,
        configs,
        hashes,
        measure,
    } = init_db();

    drop_and_expect_failure!(db, &configs[0], &hashes[0], measure, vec![3],
        PlotError::MissingRecords { commit_hash, benchmark, points, measurement_method }
            if commit_hash == hashes[0].as_str()
            && benchmark == configs[0].name
            && points == vec![3]
            && measurement_method == measure.to_string()
    );

    drop_and_expect_failure!(db, &configs[0], &hashes[0], measure, vec![2],
        PlotError::MissingRecords { commit_hash, benchmark, points, measurement_method }
            if commit_hash == hashes[0].as_str()
            && benchmark == configs[0].name
            && points == vec![2,3]
            && measurement_method == measure.to_string()
    );

    drop_and_expect_failure!(db, &configs[0], &hashes[0], measure, vec![1],
        PlotError::MissingBenchmark { commit_hash, benchmark, measurement_method }
            if commit_hash == hashes[0].as_str()
            && benchmark == configs[0].name
            && measurement_method == measure.to_string()
    );

    // In order to force db error, break db file by clearing it.
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(0)
        .unwrap();
    let dataset: Result<BenchmarkDataset<f64>, PlotError> =
        BenchmarkDataset::new(&db, &configs[1], [hashes[2].clone()].into_iter(), &measure);
    assert!(matches!(dataset.unwrap_err(), PlotError::Database(_)));
}
