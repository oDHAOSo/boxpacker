use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use boxpacker::app::{AppError, run};
use boxpacker::cli::Cli;
use boxpacker::compatibility::adapt_saved_solution;
use boxpacker::model::{InputData, OutputData};
use boxpacker::solver::OptimalityStatus;
use boxpacker::validate::{PackingInstance, validate_solution};
use clap::Parser;

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const DATA_ELEMENT_START: &str = r#"<script id="boxpacker-report-data" type="application/json">"#;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "boxpacker-pipeline-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("test directory should be removable");
    }
}

#[test]
fn current_input_runs_to_compatible_validated_json_and_html() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("input.json");
    let output_path = directory.path().join("packing.json");
    let html_path = directory.path().join("packing.html");
    fs::write(&input_path, CURRENT_INPUT).expect("input fixture should be writable");

    let cli = Cli::try_parse_from([
        "boxpacker",
        "--input",
        input_path.to_str().expect("temporary path should be UTF-8"),
        "--output",
        output_path
            .to_str()
            .expect("temporary path should be UTF-8"),
        "--preset",
        "fast",
        "--seed",
        "73",
        "--threads",
        "1",
    ])
    .expect("test CLI should parse");
    let run_summary = run(&cli).expect("pipeline should complete");

    assert_eq!(run_summary.solution().placed_item_count(), 53);
    assert_eq!(run_summary.solution().unplaced_item_count(), 4);
    assert_eq!(run_summary.solution().placed_volume(), 587_815_524);
    assert_eq!(run_summary.optimality(), OptimalityStatus::Heuristic);
    assert_eq!(run_summary.metrics().validated_candidates(), 2);
    assert_eq!(run_summary.json_output_path(), output_path);
    assert_eq!(run_summary.html_output_path(), html_path);

    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("input fixture should deserialize");
    let instance = PackingInstance::try_from(&input).expect("input fixture should validate");
    let output_json = fs::read_to_string(&output_path).expect("JSON output should exist");
    assert!(output_json.ends_with('\n'));
    let output: OutputData =
        serde_json::from_str(&output_json).expect("JSON output should remain compatible");
    let adapted = adapt_saved_solution(&instance, output.clone())
        .expect("generated compatibility output should adapt");
    let adapted_summary = validate_solution(&instance, &adapted.to_solution())
        .expect("generated compatibility output should validate independently");
    assert_eq!(adapted_summary, *run_summary.solution());

    let html = fs::read_to_string(&html_path).expect("HTML output should exist");
    let embedded_json = html
        .split_once(DATA_ELEMENT_START)
        .expect("report should embed JSON")
        .1
        .split_once("</script>")
        .expect("embedded JSON should be closed")
        .0;
    let embedded: OutputData =
        serde_json::from_str(embedded_json).expect("embedded report data should deserialize");
    assert_eq!(embedded, output);
    assert!(html.contains("Logistics Overview"));

    let mut entries = fs::read_dir(directory.path())
        .expect("test directory should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should be readable")
                .file_name()
        })
        .collect::<Vec<_>>();
    entries.sort();
    assert_eq!(entries, ["input.json", "packing.html", "packing.json"]);
}

#[test]
fn output_extension_cannot_make_json_and_html_share_a_path() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("input.json");
    let output_path = directory.path().join("packing.html");
    fs::write(&input_path, CURRENT_INPUT).expect("input fixture should be writable");
    let cli = Cli::try_parse_from([
        "boxpacker",
        "--input",
        input_path.to_str().expect("temporary path should be UTF-8"),
        "--output",
        output_path
            .to_str()
            .expect("temporary path should be UTF-8"),
    ])
    .expect("test CLI should parse");

    let error = run(&cli).expect_err("colliding output paths should be rejected");

    assert!(error.to_string().contains("resolve to the same file"));
    assert!(!output_path.exists());
}

#[test]
fn malformed_json_reports_location_without_writing_artifacts() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("malformed.json");
    let output_path = directory.path().join("packing.json");
    let html_path = directory.path().join("packing.html");
    fs::write(&input_path, "{\n  \"containers\": [\n").expect("malformed input should be writable");
    let cli = Cli::try_parse_from([
        "boxpacker",
        "--input",
        input_path.to_str().expect("temporary path should be UTF-8"),
        "--output",
        output_path
            .to_str()
            .expect("temporary path should be UTF-8"),
    ])
    .expect("test CLI should parse");

    let error = run(&cli).expect_err("malformed input should fail");
    let message = error.to_string();

    assert!(matches!(error, AppError::ParseInput { .. }));
    assert!(message.contains(&input_path.display().to_string()));
    assert!(message.contains("line 3 column 0"));
    assert!(!output_path.exists());
    assert!(!html_path.exists());
}

#[test]
fn wrong_json_type_reports_the_exact_document_path() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("wrong-type.json");
    let output_path = directory.path().join("packing.json");
    let html_path = directory.path().join("packing.html");
    let input = r#"{
        "containers": [
            {"name": "box", "width": 10, "length": 10, "height": 10}
        ],
        "contents": [
            {"name": "item", "width": 1, "length": 1, "height": "high"}
        ]
    }"#;
    fs::write(&input_path, input).expect("wrong-type input should be writable");
    let cli = Cli::try_parse_from([
        "boxpacker",
        "--input",
        input_path.to_str().expect("temporary path should be UTF-8"),
        "--output",
        output_path
            .to_str()
            .expect("temporary path should be UTF-8"),
    ])
    .expect("test CLI should parse");

    let error = run(&cli).expect_err("wrong-type input should fail");
    let message = error.to_string();

    assert!(matches!(error, AppError::ParseInput { .. }));
    assert!(message.contains("at contents[0].height"));
    assert!(message.contains("invalid type: string"));
    assert!(!output_path.exists());
    assert!(!html_path.exists());
}

#[test]
fn invalid_dimensions_report_every_field_without_writing_artifacts() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("invalid-dimensions.json");
    let output_path = directory.path().join("packing.json");
    let html_path = directory.path().join("packing.html");
    let input = r#"{
        "containers": [
            {"name": "box", "width": 0, "length": 1.25, "height": 10}
        ],
        "contents": [
            {"name": "item", "width": 1, "length": -2, "height": 1}
        ]
    }"#;
    fs::write(&input_path, input).expect("invalid input should be writable");
    let cli = Cli::try_parse_from([
        "boxpacker",
        "--input",
        input_path.to_str().expect("temporary path should be UTF-8"),
        "--output",
        output_path
            .to_str()
            .expect("temporary path should be UTF-8"),
    ])
    .expect("test CLI should parse");

    let error = run(&cli).expect_err("invalid dimensions should fail");
    let message = error.to_string();

    assert!(matches!(error, AppError::ValidateInput(_)));
    assert!(message.contains("containers[0].width"));
    assert!(message.contains("containers[0].length"));
    assert!(message.contains("contents[0].length"));
    assert!(message.contains("3 error(s)"));
    assert!(!output_path.exists());
    assert!(!html_path.exists());
}

#[test]
fn cli_reports_the_honest_solver_status() {
    let directory = TestDirectory::new();
    let input_path = directory.path().join("input.json");
    let output_path = directory.path().join("packing.json");
    fs::write(&input_path, CURRENT_INPUT).expect("input fixture should be writable");

    let process = Command::new(env!("CARGO_BIN_EXE_boxpacker"))
        .args([
            "--input",
            input_path.to_str().expect("temporary path should be UTF-8"),
            "--output",
            output_path
                .to_str()
                .expect("temporary path should be UTF-8"),
            "--preset",
            "fast",
        ])
        .output()
        .expect("boxpacker process should launch");

    assert!(process.status.success());
    let stdout = String::from_utf8(process.stdout).expect("CLI output should be UTF-8");
    assert!(stdout.contains("Packed 53 of 57 items (status: heuristic)"));
    assert!(output_path.exists());
    assert!(output_path.with_extension("html").exists());
}
