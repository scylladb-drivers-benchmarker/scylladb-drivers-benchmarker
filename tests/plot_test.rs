mod utilities;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use fs::File;
use fs_err as fs;
use scylladb_drivers_benchmarker::VisKind;
use scylladb_drivers_benchmarker::commit_hash::CommitHash;
use scylladb_drivers_benchmarker::database::utilities::{BenchmarkParams, BenchmarkRecord};
use scylladb_drivers_benchmarker::database::{self};
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;
use tempfile::{Builder, NamedTempFile, TempDir};
use utilities::image_compare::check_files_equality;
use utilities::run_utilities::{print_flamegraph_information, run, run_no_output, sdb_command};

fn init_git_repo(path: &Path, num_commits: usize) -> Vec<CommitHash> {
    if !path.join(".git").exists() {
        run(std::process::Command::new("git")
            .current_dir(path)
            .arg("init"));
    }

    let mut hashes = Vec::new();

    for i in 0..num_commits {
        let file_path = path.join(format!("file{}.txt", i));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "dummy content {}", i).unwrap();

        run(std::process::Command::new("git")
            .current_dir(path)
            .arg("add")
            .arg("."));

        run(std::process::Command::new("git")
            .current_dir(path)
            .arg("commit")
            .arg("-m")
            .arg(format!("Commit {}", i)));

        let hash = CommitHash::new(path, "HEAD".to_owned()).unwrap();
        hashes.push(hash);
    }

    hashes
}

struct TestData {
    db_file: NamedTempFile,
    repo_dir: TempDir,
    repo_hashes: Vec<CommitHash>,
}

fn build_from_arg(path: &TempDir, commits: Vec<CommitHash>) -> String {
    format!(
        "--from={}:{}",
        path.path().to_str().unwrap(),
        commits
            .into_iter()
            .map(|x| x.as_str().to_owned())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn setup_initial_data(
    max: u64,
    data_generator: fn(usize, &CommitHash, BenchmarkPoint) -> (BenchmarkParams, BenchmarkRecord),
) -> TestData {
    let db_file = Builder::new().suffix(".db").tempfile().unwrap();

    let db = database::Database::new(db_file.path()).unwrap();

    let repo_dir = TempDir::new().unwrap();
    let repo_hashes = init_git_repo(repo_dir.path(), 3);

    let points = 1u64..max;

    for (commit_idx, commit) in repo_hashes.iter().enumerate() {
        for point in points.clone() {
            let (params, record) = data_generator(commit_idx, commit, point);
            db.insert_data(params, record).unwrap();
        }
    }
    TestData {
        db_file,
        repo_dir,
        repo_hashes,
    }
}

fn plot_series_generic_test(
    output: &Path,
    config: &str,
    expected_output: &Path,
    vis_kind: VisKind,
) {
    let test_data = setup_initial_data(101, generate_series_data);

    run_no_output(sdb_command().args([
        "-d",
        test_data.db_file.path().to_str().unwrap(),
        "plot",
        "test-bench",
        "-b",
        config,
        &build_from_arg(&test_data.repo_dir, test_data.repo_hashes),
        "-o",
        output.to_str().unwrap(),
        "series",
        "-v",
        &vis_kind.to_string(),
        "-m",
        "time",
    ]));

    check_files_equality(output, expected_output);

    fs::remove_file(output).unwrap();
}

fn plot_perf_generic_test(output: &Path, expected_output: &Path) {
    let test_data = setup_initial_data(101, generate_perf_data);

    run_no_output(sdb_command().args([
        "-d",
        test_data.db_file.path().to_str().unwrap(),
        "plot",
        "test-bench",
        "-b",
        "./tests/plot_test/config.yml",
        &build_from_arg(&test_data.repo_dir, test_data.repo_hashes),
        "-o",
        output.to_str().unwrap(),
        "perf-stat",
        "-e",
        "task-clock,context-switches,page-faults",
    ]));

    check_files_equality(output, expected_output);
    fs::remove_file(output).unwrap();
}

#[test]
fn plot_series() {
    let output_base = Path::new("./tests/plot_test/");
    let expected_base = Path::new("./tests/plot_test/expected");

    for vis_kind in [VisKind::Linear, VisKind::Log] {
        for format in ["png", "svg"] {
            let filename = PathBuf::from(format!("series_{vis_kind}.{format}"));

            plot_series_generic_test(
                &output_base.join(&filename),
                "./tests/plot_test/config.yml",
                &expected_base.join(&filename),
                vis_kind,
            );
        }
    }
}

#[test]
fn plot_series_points() {
    let output_base = Path::new("./tests/plot_test/");
    let expected_base = Path::new("./tests/plot_test/expected");

    for vis_kind in [VisKind::Linear, VisKind::Log] {
        for format in ["png", "svg"] {
            let filename = PathBuf::from(format!("series_points_{vis_kind}.{format}"));

            plot_series_generic_test(
                &output_base.join(&filename),
                "1,20,3",
                &expected_base.join(&filename),
                vis_kind,
            );
        }
    }
}

#[test]
fn plot_perf() {
    let output_base = Path::new("./tests/plot_test/");
    let expected_base = Path::new("./tests/plot_test/expected");

    for format in ["png", "svg"] {
        let filename = PathBuf::from(format!("perf.{format}"));

        plot_perf_generic_test(&output_base.join(&filename), &expected_base.join(&filename));
    }
}

#[test]
fn plot_flamegraph() {
    print_flamegraph_information(Path::new("./tests/plot_test/flame-path.yml"));

    let test_data = setup_initial_data(5, generate_flame_data);

    let output = "./tests/plot_test/flame_output.html";
    let _expected_output = "./tests/plot_test/expected_flame.html";

    run_no_output(sdb_command().args([
        "-d",
        test_data.db_file.path().to_str().unwrap(),
        "-a",
        "./tests/plot_test/flame-path.yml",
        "plot",
        "flame-bench",
        "-b",
        "./tests/plot_test/config.yml",
        &build_from_arg(&test_data.repo_dir, test_data.repo_hashes),
        "-o",
        output,
        "flamegraph",
        "-a",
        "./tests/plot_test",
    ]));

    assert!(fs::exists(output).unwrap());
    // It is hard to compare generated html files (they may differ as they contain svg with floating numbers).
    fs::remove_file(output).unwrap();
}

fn generate_series_data(
    commit_idx: usize,
    commit: &CommitHash,
    point: BenchmarkPoint,
) -> (BenchmarkParams, BenchmarkRecord) {
    let params = BenchmarkParams::new(
        commit.clone(),
        "test-bench".to_owned(),
        point,
        "time".to_owned(),
    );

    let value = commit_idx as f64 * 100.0 + point as f64 * point as f64;
    (params, BenchmarkRecord::Data(value.to_string()))
}

fn generate_perf_data(
    commit_idx: usize,
    commit: &CommitHash,
    point: BenchmarkPoint,
) -> (BenchmarkParams, BenchmarkRecord) {
    let params = BenchmarkParams::new(
        commit.clone(),
        "test-bench".to_owned(),
        point,
        "perf".to_owned(),
    );

    let value = commit_idx as f64 * 1000.0 + point as f64 * point as f64;

    // Some perf data multiplied by arbitrary factors to simulate semi-realistic values.
    let json_value = format!(
        r#"{{
"counter-value":"{task_clock_val:.6}","unit":"msec","event":"task-clock","event-runtime":{task_clock_runtime},"pcnt-running":100.0,"metric-value":"{task_clock_metric:.6}","metric-unit":"CPUs utilized"
}}
{{
"counter-value":"{ctx_switch_val:.6}","unit":"","event":"context-switches","event-runtime":{ctx_switch_runtime},"pcnt-running":100.0,"metric-value":"{ctx_switch_metric:.6}","metric-unit":"K/sec"
}}
{{
"counter-value":"{cpu_mig_val:.6}","unit":"","event":"cpu-migrations","event-runtime":{cpu_mig_runtime},"pcnt-running":100.0,"metric-value":"{cpu_mig_metric:.6}","metric-unit":"/sec"
}}
{{
"counter-value":"{page_fault_val:.6}","unit":"","event":"page-faults","event-runtime":{page_fault_runtime},"pcnt-running":100.0,"metric-value":"{page_fault_metric:.6}","metric-unit":"K/sec"
}}
{{
"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.0,"metric-value":"0.000000","metric-unit":""
}}
{{
"counter-value":"{core_cycles_val:.6}","unit":"","event":"cpu_core/cycles/","event-runtime":{core_cycles_runtime},"pcnt-running":100.0,"metric-value":"{core_cycles_metric:.6}","metric-unit":"GHz"
}}"#,
        task_clock_val = value * 0.00374,
        task_clock_runtime = 374_411 + commit_idx as u64,
        task_clock_metric = value * 0.000374,
        ctx_switch_val = value * 0.01,
        ctx_switch_runtime = 374_411,
        ctx_switch_metric = value * 0.02670862,
        cpu_mig_val = value * 0.0,
        cpu_mig_runtime = 374_411,
        cpu_mig_metric = value * 0.0,
        page_fault_val = value * 75.0,
        page_fault_runtime = 374_411,
        page_fault_metric = value * 2.00314628,
        core_cycles_val = value * 1_461_835.0,
        core_cycles_runtime = 374_411,
        core_cycles_metric = value * 3.904359,
    );
    (params, BenchmarkRecord::Data(json_value.to_string()))
}

fn generate_flame_data(
    _commit_idx: usize,
    commit: &CommitHash,
    point: BenchmarkPoint,
) -> (BenchmarkParams, BenchmarkRecord) {
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    static INPUTS: [&str; 2] = [
        "./tests/plot_test/input_flame_0",
        "./tests/plot_test/input_flame_1",
    ];

    let params = BenchmarkParams::new(
        commit.clone(),
        "flame-bench".to_owned(),
        point,
        "flamegraph".to_owned(),
    );

    let idx = CALL_COUNT.fetch_add(1, Ordering::Relaxed) % INPUTS.len();

    let path: PathBuf = INPUTS[idx].into();
    (
        params,
        BenchmarkRecord::FilePath(path.canonicalize().unwrap()),
    )
}
