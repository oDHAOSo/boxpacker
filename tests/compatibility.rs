use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use boxpacker::cli::{
    Cli, DEFAULT_INPUT_PATH, DEFAULT_OUTPUT_PATH, SearchPreset, html_output_path,
};
use boxpacker::model::{InputData, OutputData};
use clap::Parser;

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const CURRENT_SAVED_OUTPUT: &str = include_str!("fixtures/current/saved_output.json");

#[test]
fn current_input_deserializes_through_compatibility_dtos() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");

    assert_eq!(input.containers.len(), 6);
    assert_eq!(input.contents.len(), 57);
    assert_eq!(input.containers[0].name, "Uhaul1");
    assert_eq!(input.contents[0].name, "Arc Yujin");
}

#[test]
fn current_saved_output_deserializes_through_compatibility_dtos() {
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");

    assert_eq!(output.containers.len(), 6);
    assert_eq!(
        output
            .containers
            .iter()
            .map(|container| container.placed_items.len())
            .sum::<usize>(),
        49
    );
    assert_eq!(output.unplaced_items.len(), 8);
}

#[test]
fn output_serialization_preserves_the_legacy_json_shape() {
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");
    let serialized = serde_json::to_value(output).expect("output DTO should serialize");

    let root = serialized
        .as_object()
        .expect("output document should be an object");
    assert_eq!(root.len(), 2);
    assert!(root.contains_key("containers"));
    assert!(root.contains_key("unplaced_items"));

    let container = root["containers"][0]
        .as_object()
        .expect("container should be an object");
    assert_eq!(container.len(), 5);
    assert!(container.contains_key("placed_items"));

    let placed_item = container["placed_items"][0]
        .as_object()
        .expect("placed item should be an object");
    assert_eq!(placed_item.len(), 3);
    assert!(placed_item.contains_key("coords"));
    assert!(placed_item.contains_key("color"));

    let coords = placed_item["coords"]
        .as_object()
        .expect("coordinates should be an object");
    assert_eq!(coords.len(), 6);
    for field in ["x", "y", "z", "w", "l", "h"] {
        assert!(coords.contains_key(field));
    }
}

#[test]
fn cli_defaults_preserve_legacy_paths_and_add_search_controls() {
    let cli = Cli::try_parse_from(["boxpacker"]).expect("default CLI should parse");

    assert_eq!(cli.input, PathBuf::from(DEFAULT_INPUT_PATH));
    assert_eq!(cli.output, PathBuf::from(DEFAULT_OUTPUT_PATH));
    assert_eq!(cli.html_output_path(), PathBuf::from("output.html"));
    assert_eq!(cli.preset, SearchPreset::Balanced);
    assert_eq!(cli.time_limit, None);
    assert_eq!(cli.seed, 0);
    assert_eq!(cli.threads, NonZeroUsize::new(1).unwrap());
}

#[test]
fn cli_accepts_legacy_short_paths_and_explicit_search_controls() {
    let cli = Cli::try_parse_from([
        "boxpacker",
        "-i",
        "fixtures/input data.json",
        "-o",
        "results/packed.data",
        "--preset",
        "thorough",
        "--time-limit",
        "2.5",
        "--seed",
        "42",
        "--threads",
        "3",
    ])
    .expect("custom CLI should parse");

    assert_eq!(cli.input, PathBuf::from("fixtures/input data.json"));
    assert_eq!(cli.output, PathBuf::from("results/packed.data"));
    assert_eq!(
        html_output_path(&cli.output),
        PathBuf::from("results/packed.html")
    );
    assert_eq!(cli.preset, SearchPreset::Thorough);
    assert_eq!(cli.time_limit, Some(Duration::from_millis(2_500)));
    assert_eq!(cli.seed, 42);
    assert_eq!(cli.threads, NonZeroUsize::new(3).unwrap());
}

#[test]
fn cli_rejects_non_positive_search_bounds() {
    assert!(Cli::try_parse_from(["boxpacker", "--time-limit", "0"]).is_err());
    assert!(Cli::try_parse_from(["boxpacker", "--time-limit", "NaN"]).is_err());
    assert!(Cli::try_parse_from(["boxpacker", "--threads", "0"]).is_err());
}
