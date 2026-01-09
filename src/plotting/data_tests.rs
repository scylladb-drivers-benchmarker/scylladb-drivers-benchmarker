use std::fs::OpenOptions;

use super::*;
use crate::database::utilities::BenchmarkParams;
use crate::database::utilities::{BenchmarkFilters, BenchmarkRecord};
use crate::plotting::data::BenchmarkDataset;
use crate::*;
use tempfile::NamedTempFile;

fn get_db() -> (Database, NamedTempFile, std::path::PathBuf) {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_path_buf();
    (Database::new(&path).unwrap(), file, path)
}

// Insert record to database, with provided commit_hash, config, point and result
// Measurement method is always "time".
macro_rules! insert_bench {
    ($db:expr, $hash:expr, $conf:expr, $step:expr, $val:expr) => {
        $db.insert_data(
            BenchmarkParams::new($hash.clone(), $conf.name.clone(), $step, "time".to_string()),
            BenchmarkRecord::Data($val.to_string()),
        )
        .unwrap();
    };
}

// Creates mock data and initialises structs with it.
fn init_db() -> (
    Database,
    NamedTempFile,
    std::path::PathBuf,
    Vec<BenchmarkConfig>,
    Vec<CommitHash>,
    MeasurementMethod,
) {
    let (db, file, path) = get_db();

    let config1 = BenchmarkConfig {
        name: "benchmark1".to_string(),
        data: config::benchmark::BenchmarkData {
            starting_step: 1,
            no_steps: 3,
            step_progress: 1,
            progress_type: config::benchmark::ProgressType::Additive,
            timeout: None,
        },
    };

    let config2 = BenchmarkConfig {
        name: "benchmark2".to_string(),
        data: config::benchmark::BenchmarkData {
            starting_step: 10,
            no_steps: 1,
            step_progress: 1,
            progress_type: config::benchmark::ProgressType::Additive,
            timeout: None,
        },
    };

    let commit_hash_1 = CommitHash::new_unchecked("1".to_string());
    let commit_hash_2 = CommitHash::new_unchecked("2".to_string());
    let commit_hash_3 = CommitHash::new_unchecked("3".to_string());

    insert_bench!(db, commit_hash_1, config1, 1, "1.5");
    insert_bench!(db, commit_hash_1, config1, 2, "2.5");
    insert_bench!(db, commit_hash_1, config1, 3, "4.5");

    insert_bench!(db, commit_hash_2, config1, 1, "2");
    insert_bench!(db, commit_hash_2, config1, 2, "3.5");
    insert_bench!(db, commit_hash_2, config1, 3, "5.5");

    insert_bench!(db, commit_hash_3, config2, 10, "3.5");

    (
        db,
        file,
        path,
        vec![config1, config2],
        vec![commit_hash_1, commit_hash_2, commit_hash_3],
        MeasurementMethod::Time,
    )
}

#[test]
fn extract() {
    let (db, _file, _path, configs, hashes, measure) = init_db();

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

macro_rules! drop_and_expect_missing {
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
    let (db, _file, path, configs, hashes, measure) = init_db();

    drop_and_expect_missing!(db, &configs[0], &hashes[0], measure, vec![3],
        PlotError::MissingRecords { commit_hash, benchmark, points, measurement_method }
            if commit_hash == hashes[0].as_str()
            && benchmark == configs[0].name
            && points == vec![3]
            && measurement_method == measure.to_string()
    );

    drop_and_expect_missing!(db, &configs[0], &hashes[0], measure, vec![2],
        PlotError::MissingRecords { commit_hash, benchmark, points, measurement_method }
            if commit_hash == hashes[0].as_str()
            && benchmark == configs[0].name
            && points == vec![2,3]
            && measurement_method == measure.to_string()
    );

    drop_and_expect_missing!(db, &configs[0], &hashes[0], measure, vec![1],
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
