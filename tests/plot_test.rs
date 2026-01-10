use image::{RgbaImage, open};
use resvg::{tiny_skia, usvg};
use scylladb_drivers_benchmarker::{
    OutputFormat, VisKind,
    commit_hash::CommitHash,
    database::{
        self,
        utilities::{BenchmarkParams, BenchmarkRecord},
    },
};
use serial_test::file_serial;
mod common;
use crate::common::{run, run_assert_empty, sdb_command};
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
use tempfile::TempDir;

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

fn load_image(path: &str, format: OutputFormat) -> RgbaImage {
    match format {
        OutputFormat::Png => open(path).unwrap().to_rgba8(),
        OutputFormat::Svg => svg_to_rgba(path.as_ref()),
    }
}

fn svg_to_rgba(path: &std::path::Path) -> RgbaImage {
    let svg_data = fs::read(path).unwrap();
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();

    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec()).unwrap()
}

fn check_files_equality(output: &str, expected_output: &str, format: OutputFormat) {
    let result = image_compare::rgba_hybrid_compare(
        &load_image(output, format),
        &load_image(expected_output, format),
    )
    .expect("Images had different dimensions");
    assert!(result.score > 0.95, "similarity too low: {}", result.score);
}

fn setup_initial_data(
    data_generator: fn(usize, &CommitHash, BenchmarkPoint) -> (BenchmarkParams, BenchmarkRecord),
) -> (String, TempDir, Vec<CommitHash>) {
    let db_path = "./tests/plot_test/test.db";
    let db = database::Database::new(Path::new(db_path)).unwrap();
    db.drop_all_data().unwrap();

    let tmp_repo = TempDir::new().unwrap();
    let commits = init_git_repo(tmp_repo.path(), 3);

    let points = 1u64..101u64;

    for (commit_idx, commit) in commits.iter().enumerate() {
        for point in points.clone() {
            let (params, record) = data_generator(commit_idx, commit, point);
            db.insert_data(params, record).unwrap();
        }
    }
    (db_path.to_owned(), tmp_repo, commits)
}

fn plot_series_generic_test(
    output: &str,
    expected_output: &str,
    vis_kind: VisKind,
    format: OutputFormat,
) {
    let (db_path, tmp_repo, commits) = setup_initial_data(generate_series_data);

    let from_arg = format!(
        "--from={}:{},{},{}",
        tmp_repo.path().to_str().unwrap(),
        commits[0],
        commits[1],
        commits[2]
    );

    run_assert_empty(
        sdb_command()
            .arg("-d")
            .arg(&db_path)
            .arg("plot")
            .arg("test-bench")
            .arg("-b")
            .arg("./tests/plot_test/config.yml")
            .arg(from_arg.as_str())
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
    let (db_path, tmp_repo, commits) = setup_initial_data(generate_perf_data);

    let from_arg = format!(
        "--from={}:{},{},{}",
        tmp_repo.path().to_str().unwrap(),
        commits[0],
        commits[1],
        commits[2]
    );

    run_assert_empty(
        sdb_command()
            .arg("-d")
            .arg(&db_path)
            .arg("plot")
            .arg("test-bench")
            .arg("-b")
            .arg("./tests/plot_test/config.yml")
            .arg(from_arg.as_str())
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

// Compiling rust by two tests in parallel sometimes fails.
// We chose to serialize those two tests to avoid unpredictable test failures.

#[test]
#[file_serial]
fn plot_series() {
    let output_base = "./tests/plot_test/series";
    let expected_base = "./tests/plot_test/expected_series";

    let vis_kinds = [VisKind::Linear, VisKind::Log];
    let formats = [OutputFormat::Png, OutputFormat::Svg];

    for vis_kind in &vis_kinds {
        for format in &formats {
            let sufix = format!("_{}.{}", vis_kind.to_string(), format.to_string());

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
#[file_serial]
fn plot_perf() {
    let output_base = "./tests/plot_test/perf";
    let expected_base = "./tests/plot_test/expected_perf";

    let formats = [OutputFormat::Png, OutputFormat::Svg];

    for format in &formats {
        let sufix = format!(".{}", format.to_string());

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
