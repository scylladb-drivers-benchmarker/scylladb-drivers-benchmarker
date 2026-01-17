use fs::OpenOptions;

use crate::CommitHash;
use crate::Database;
use crate::config;
use crate::database::utilities::{BenchmarkFilters, BenchmarkParams, BenchmarkRecord};
use crate::measurement::MeasurementMethod;
use crate::plotting::{BenchmarkConfig, PlotError, core::BenchmarkDataset};
use crate::utilities::BenchmarkPoint;
use tempfile::NamedTempFile;

use fs_err as fs;

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
            name: "benchmark1".to_owned(),
            starting_step: 1,
            no_steps: 3,
            step_progress: 1,
            progress_type: config::benchmark::ProgressType::Additive,
            timeout: None,
        },
        BenchmarkConfig {
            name: "benchmark2".to_owned(),
            starting_step: 10,
            no_steps: 1,
            step_progress: 1,
            progress_type: config::benchmark::ProgressType::Additive,
            timeout: None,
        },
    ];

    let hashes = vec![
        CommitHash::new_unchecked("1".to_owned()),
        CommitHash::new_unchecked("2".to_owned()),
        CommitHash::new_unchecked("3".to_owned()),
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

#[test]
fn extract_perfstat_dataset() {
    use crate::assert_perf;
    use crate::perf_stat;

    let (db, _file) = {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        (Database::new(&path).unwrap(), file)
    };

    let config = BenchmarkConfig {
        name: "benchmark_perf".to_owned(),
        starting_step: 1,
        no_steps: 2,
        step_progress: 1,
        progress_type: config::benchmark::ProgressType::Additive,
        timeout: None,
    };

    let commit = CommitHash::new_unchecked("abc".to_owned());

    let perf_json_1 = r#"
{"counter-value":"0,374411","unit":"msec","event":"task-clock","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000374","metric-unit":"CPUs utilized"}
{"counter-value":"1,000000","unit":"","event":"context-switches","event-runtime":374411,"pcnt-running":100.00,"metric-value":"2,670862","metric-unit":"K/sec"}
{"counter-value":"0,000000","unit":"","event":"cpu-migrations","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000000","metric-unit":"/sec"}
{"counter-value":"75,000000","unit":"","event":"page-faults","event-runtime":374411,"pcnt-running":100.00,"metric-value":"200,314628","metric-unit":"K/sec"}
{"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.00,"metric-value":"0,000000","metric-unit":""}
{"counter-value":"1461835,000000","unit":"","event":"cpu_core/cycles/","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,904359","metric-unit":"GHz"}
"#;
    let perf_json_2 = r#"
{"counter-value":"0,374411","unit":"msec","event":"task-clock","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000474","metric-unit":"CPUs utilized"}
{"counter-value":"1,000000","unit":"","event":"context-switches","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,670862","metric-unit":"K/sec"}
{"counter-value":"0,000000","unit":"","event":"cpu-migrations","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,100000","metric-unit":"/sec"}
{"counter-value":"75,000000","unit":"","event":"page-faults","event-runtime":374411,"pcnt-running":100.00,"metric-value":"250,314628","metric-unit":"K/sec"}
{"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.00,"metric-value":"0,100000","metric-unit":""}
{"counter-value":"1461835,000000","unit":"","event":"cpu_core/cycles/","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,954359","metric-unit":"GHz"}
"#;

    db.insert_data(
        BenchmarkParams::new(commit.clone(), config.name.clone(), 1, "perf".to_owned()),
        BenchmarkRecord::Data(perf_json_1.to_string()),
    )
    .unwrap();

    db.insert_data(
        BenchmarkParams::new(commit.clone(), config.name.clone(), 2, "perf".to_owned()),
        BenchmarkRecord::Data(perf_json_2.to_string()),
    )
    .unwrap();

    let dataset: BenchmarkDataset<perf_stat::PerfStatData> = BenchmarkDataset::new(
        &db,
        &config,
        [commit.clone()].into_iter(),
        &MeasurementMethod::Perf,
    )
    .unwrap();

    assert_eq!(dataset.points, vec![1, 2]);
    assert_eq!(dataset.results.len(), 1);
    assert!(dataset.results[0][0].is_some());
    assert!(dataset.results[0][1].is_some());

    let perf_data1 = dataset.results[0][0].as_ref().unwrap();
    let perf_data2 = dataset.results[0][1].as_ref().unwrap();

    assert_perf!(perf_data1, "task-clock", 0.000374, "CPUs utilized");
    assert_perf!(perf_data1, "context-switches", 2.670862, "K/sec");
    assert_perf!(perf_data1, "cpu-migrations", 0.0, "/sec");
    assert_perf!(perf_data1, "page-faults", 200.314628, "K/sec");
    assert_perf!(perf_data1, "cpu_atom/cycles/", 0.0, "");
    assert_perf!(perf_data1, "cpu_core/cycles/", 3.904359, "GHz");

    assert_perf!(perf_data2, "task-clock", 0.000474, "CPUs utilized");
    assert_perf!(perf_data2, "context-switches", 3.670862, "K/sec");
    assert_perf!(perf_data2, "cpu-migrations", 0.1, "/sec");
    assert_perf!(perf_data2, "page-faults", 250.314628, "K/sec");
    assert_perf!(perf_data2, "cpu_atom/cycles/", 0.1, "");
    assert_perf!(perf_data2, "cpu_core/cycles/", 3.954359, "GHz");
}
