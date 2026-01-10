use image::{RgbaImage, open};
use resvg::{tiny_skia, usvg};
use scylladb_drivers_benchmarker::{
    OutputFormat, VisKind,
    commit_hash::CommitHash,
    database,
    database::utilities::{BenchmarkParams, BenchmarkRecord},
};
use serial_test::file_serial;

use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};
use tempfile::TempDir;

use crate::common::{run, sdb_command};

mod common;

// TODO: eliminate this
fn run_bin(args: &[&str]) -> String {
    let output = run(sdb_command().args(args));
    String::from_utf8_lossy(&output.stdout).to_string()
}

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

        let hash = CommitHash::new(path, "HEAD".to_string()).unwrap();
        hashes.push(hash);
    }

    hashes
}

fn init_db(db_path: &str) -> database::Database {
    let db = database::Database::new(Path::new(db_path)).unwrap();
    db.drop_all_data().unwrap();
    db
}

fn svg_to_rgba(path: &std::path::Path) -> RgbaImage {
    let svg_data = std::fs::read(path).unwrap();
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();

    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .unwrap()
}

fn plot_series_generic_test(
    output: &str,
    expected_output: &str,
    vis_kind: VisKind,
    format: OutputFormat,
) {
    let db_path = "./tests/plot_test/test.db";
    let db = init_db(db_path);

    let tmp_repo = TempDir::new().unwrap();
    let commits = init_git_repo(tmp_repo.path(), 3);

    let points = 1u64..101u64;

    for (commit_idx, commit) in commits.iter().enumerate() {
        for point in points.clone() {
            let params = BenchmarkParams::new(
                commit.clone(),
                "test-bench".to_string(),
                point,
                "time".to_string(),
            );

            let value = commit_idx as f64 * 100.0 + point as f64 * point as f64;

            db.insert_data(params, BenchmarkRecord::Data(value.to_string()))
                .unwrap();
        }
    }

    let from_arg = format!(
        "--from={}:{},{},{}",
        tmp_repo.path().to_str().unwrap(),
        commits[0],
        commits[1],
        commits[2]
    );
    let config_path = "./tests/plot_test/config.yml";

    run_bin(&[
        "-d",
        db_path,
        "plot",
        "test-bench",
        "-b",
        config_path,
        from_arg.as_str(),
        "-m",
        "time",
        "-o",
        output,
        "-f",
        format.to_string(),
        "series",
        "-v",
        vis_kind.to_string(),
    ]);

    assert!(Path::new(output).exists());

    match format {
        OutputFormat::Png => {
            let f1 = open(output).unwrap().to_rgba8();
            let f2 = open(expected_output).unwrap().to_rgba8();
            let result = image_compare::rgba_hybrid_compare(&f1, &f2)
                .expect("Images had different dimensions");
            assert!(result.score > 0.98);
        }
        OutputFormat::Svg => {
            let f1 = svg_to_rgba(output.as_ref());
            let f2 = svg_to_rgba(expected_output.as_ref());

            let result = image_compare::rgba_hybrid_compare(&f1, &f2)
                .expect("Images had different dimensions");

            assert!(result.score > 0.98,);
        }
    };

    fs::remove_file(output).unwrap();
}

fn plot_perf_generic_test(output: &str, format: OutputFormat, delete_result: bool) {
    let db_path = "./tests/plot_test/test.db";
    let db = init_db(db_path);

    let tmp_repo = TempDir::new().unwrap();
    let commits = init_git_repo(tmp_repo.path(), 3);

    let points = 1u64..101u64;

    for (commit_idx, commit) in commits.iter().enumerate() {
        for point in points.clone() {
            let params = BenchmarkParams::new(
                commit.clone(),
                "test-bench".to_string(),
                point,
                "perf".to_string(),
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

            db.insert_data(params, BenchmarkRecord::Data(json_value.to_string()))
                .unwrap();
        }
    }

    let from_arg = format!(
        "--from={}:{},{},{}",
        tmp_repo.path().to_str().unwrap(),
        commits[0],
        commits[1],
        commits[2]
    );
    let config_path = "./tests/plot_test/config.yml";

    let format_str = match format {
        OutputFormat::Png => "png",
        OutputFormat::Svg => "svg",
    };

    run_bin(&[
        "-d",
        db_path,
        "plot",
        "test-bench",
        "-b",
        config_path,
        from_arg.as_str(),
        "-m",
        "perf",
        "-o",
        output,
        "-f",
        format_str,
        "perf-stat",
        "-e",
        "task-clock,context-switches,page-faults",
    ]);

    assert!(Path::new(output).exists());

    if delete_result {
        fs::remove_file(output).unwrap();
    }
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
    // To see the results of the test, set this to false.
    let delete_results = true;

    let output_base = "./tests/plot_test/perf";

    let formats = [OutputFormat::Png, OutputFormat::Svg];

    for format in &formats {
        let output_file = match format {
            OutputFormat::Png => format!("{}.png", output_base),
            OutputFormat::Svg => format!("{}.svg", output_base),
        };

        //    plot_perf_generic_test(&output_file, *format, delete_results);
    }
}
