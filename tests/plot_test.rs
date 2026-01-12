mod utilities;
use utilities::{
    image_compare::check_files_equality,
    run_utilities::{run, run_no_output, sdb_command},
};

use scylladb_drivers_benchmarker::{
    OutputFormat, VisKind,
    commit_hash::CommitHash,
    database::{
        self,
        utilities::{BenchmarkParams, BenchmarkRecord},
    },
    utilities::BenchmarkPoint,
};

use fs::File;
use fs_err as fs;
use std::{io::Write, path::Path};
use tempfile::{Builder, NamedTempFile, TempDir};

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
    data_generator: fn(usize, &CommitHash, BenchmarkPoint) -> (BenchmarkParams, BenchmarkRecord),
) -> TestData {
    let db_file = Builder::new().suffix(".db").tempfile().unwrap();

    let db = database::Database::new(db_file.path()).unwrap();

    let repo_dir = TempDir::new().unwrap();
    let repo_hashes = init_git_repo(repo_dir.path(), 3);

    let points = 1u64..101u64;

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
    output: &str,
    expected_output: &str,
    vis_kind: VisKind,
    format: OutputFormat,
) {
    let test_data = setup_initial_data(generate_series_data);

    run_no_output(
        sdb_command()
            .arg("-d")
            .arg(test_data.db_file.path().to_string_lossy().to_string())
            .arg("plot")
            .arg("test-bench")
            .arg("-b")
            .arg("./tests/plot_test/config.yml")
            .arg(build_from_arg(&test_data.repo_dir, test_data.repo_hashes))
            .arg("-m")
            .arg("time")
            .arg("-o")
            .arg(output)
            .arg("-f")
            .arg(format.to_string())
            .arg("series")
            .arg("-v")
            .arg(vis_kind.to_string()),
    );

    check_files_equality(output, expected_output, format);

    fs::remove_file(output).unwrap();
}

fn plot_perf_generic_test(output: &str, expected_output: &str, format: OutputFormat) {
    let test_data = setup_initial_data(generate_perf_data);

    run_no_output(
        sdb_command()
            .arg("-d")
            .arg(test_data.db_file.path().to_string_lossy().to_string())
            .arg("plot")
            .arg("test-bench")
            .arg("-b")
            .arg("./tests/plot_test/config.yml")
            .arg(build_from_arg(&test_data.repo_dir, test_data.repo_hashes))
            .arg("-m")
            .arg("perf")
            .arg("-o")
            .arg(output)
            .arg("-f")
            .arg(format.to_string())
            .arg("perf-stat")
            .arg("-e")
            .arg("task-clock,context-switches,page-faults"),
    );

    check_files_equality(output, expected_output, format);
    fs::remove_file(output).unwrap();
}

#[test]
fn plot_series() {
    let output_base = "./tests/plot_test/series";
    let expected_base = "./tests/plot_test/expected_series";

    let vis_kinds = [VisKind::Linear, VisKind::Log];
    let formats = [OutputFormat::Png, OutputFormat::Svg];

    for vis_kind in &vis_kinds {
        for format in &formats {
            let sufix = format!("_{}.{}", vis_kind, format);

            plot_series_generic_test(
                &(output_base.to_owned() + &sufix),
                &(expected_base.to_owned() + &sufix),
                *vis_kind,
                *format,
            );
        }
    }
}

#[test]
fn plot_perf() {
    let output_base = "./tests/plot_test/perf";
    let expected_base = "./tests/plot_test/expected_perf";

    let formats = [OutputFormat::Png, OutputFormat::Svg];

    for format in &formats {
        let sufix = format!(".{}", format);

        plot_perf_generic_test(
            &(output_base.to_owned() + &sufix),
            &(expected_base.to_owned() + &sufix),
            *format,
        );
    }
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
