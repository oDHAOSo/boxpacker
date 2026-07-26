use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use boxpacker::cli::{
    Cli, DEFAULT_INPUT_PATH, DEFAULT_OUTPUT_PATH, SearchPreset, html_output_path,
};
use boxpacker::geometry::{LengthConversionError, MAX_EXACT_SCALED_LENGTH, SCALE};
use boxpacker::model::{InputContainer, InputData, Item, OutputData};
use boxpacker::validate::{DimensionField, InputSection, InputValidationError, PackingInstance};
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
fn current_input_converts_once_to_exact_internal_geometry() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");

    assert_eq!(instance.containers().len(), 6);
    assert_eq!(instance.items().len(), 57);
    assert_eq!(instance.containers()[0].id().index(), 0);
    assert_eq!(instance.containers()[0].dimensions().width().get(), 600);
    assert_eq!(instance.containers()[0].dimensions().length().get(), 460);
    assert_eq!(instance.containers()[0].dimensions().height().get(), 610);
    assert_eq!(instance.items()[0].id().index(), 0);
    assert_eq!(instance.items()[0].dimensions().width().get(), 190);
    assert_eq!(instance.items()[0].dimensions().length().get(), 168);
    assert_eq!(instance.items()[0].dimensions().height().get(), 248);
}

#[test]
fn duplicate_names_receive_distinct_stable_internal_ids() {
    let input = InputData {
        containers: vec![
            InputContainer {
                name: "duplicate".to_owned(),
                width: 10.0,
                length: 10.0,
                height: 10.0,
            },
            InputContainer {
                name: "duplicate".to_owned(),
                width: 20.0,
                length: 20.0,
                height: 20.0,
            },
        ],
        contents: vec![
            Item {
                name: "duplicate".to_owned(),
                width: 1.0,
                length: 2.0,
                height: 3.0,
            },
            Item {
                name: "duplicate".to_owned(),
                width: 4.0,
                length: 5.0,
                height: 6.0,
            },
        ],
    };

    let instance = PackingInstance::try_from(&input).expect("duplicate names should be legal");

    assert_eq!(instance.containers()[0].id().index(), 0);
    assert_eq!(instance.containers()[1].id().index(), 1);
    assert_eq!(instance.items()[0].id().index(), 0);
    assert_eq!(instance.items()[1].id().index(), 1);
    assert_eq!(instance.items()[0].name(), instance.items()[1].name());
}

#[test]
fn invalid_dimensions_report_every_compatibility_field_path() {
    let input = InputData {
        containers: vec![InputContainer {
            name: "invalid container".to_owned(),
            width: f64::NAN,
            length: 0.0,
            height: 1.25,
        }],
        contents: vec![Item {
            name: "invalid item".to_owned(),
            width: f64::INFINITY,
            length: -1.0,
            height: f64::MAX,
        }],
    };

    let errors = PackingInstance::try_from(&input)
        .expect_err("invalid dimensions should fail compatibility conversion");

    assert_eq!(errors.errors().len(), 6);
    assert!(matches!(
        errors.errors()[0],
        InputValidationError::InvalidDimension {
            section: InputSection::Containers,
            index: 0,
            field: DimensionField::Width,
            value,
            reason: LengthConversionError::NonFinite,
        } if value.is_nan()
    ));
    assert!(matches!(
        errors.errors()[1],
        InputValidationError::InvalidDimension {
            section: InputSection::Containers,
            index: 0,
            field: DimensionField::Length,
            value: 0.0,
            reason: LengthConversionError::NonPositive,
        }
    ));
    assert!(matches!(
        errors.errors()[2],
        InputValidationError::InvalidDimension {
            section: InputSection::Containers,
            index: 0,
            field: DimensionField::Height,
            value: 1.25,
            reason: LengthConversionError::OverPrecision,
        }
    ));
    assert!(matches!(
        errors.errors()[3],
        InputValidationError::InvalidDimension {
            section: InputSection::Contents,
            index: 0,
            field: DimensionField::Width,
            value,
            reason: LengthConversionError::NonFinite,
        } if value.is_infinite()
    ));
    assert!(matches!(
        errors.errors()[4],
        InputValidationError::InvalidDimension {
            section: InputSection::Contents,
            index: 0,
            field: DimensionField::Length,
            value: -1.0,
            reason: LengthConversionError::NonPositive,
        }
    ));
    assert!(matches!(
        errors.errors()[5],
        InputValidationError::InvalidDimension {
            section: InputSection::Contents,
            index: 0,
            field: DimensionField::Height,
            value,
            reason: LengthConversionError::OutOfRange,
        } if value == f64::MAX
    ));

    let message = errors.to_string();
    for path in [
        "containers[0].width",
        "containers[0].length",
        "containers[0].height",
        "contents[0].width",
        "contents[0].length",
        "contents[0].height",
    ] {
        assert!(
            message.contains(path),
            "missing field path {path}: {message}"
        );
    }
}

#[test]
fn input_conversion_rejects_scaled_volume_overflow() {
    let maximum_input_length = MAX_EXACT_SCALED_LENGTH as f64 / SCALE as f64;
    let input = InputData {
        containers: vec![InputContainer {
            name: "too much volume".to_owned(),
            width: maximum_input_length,
            length: maximum_input_length,
            height: maximum_input_length,
        }],
        contents: Vec::new(),
    };

    let errors =
        PackingInstance::try_from(&input).expect_err("scaled volume overflow must be rejected");

    assert_eq!(
        errors.errors(),
        [InputValidationError::VolumeOverflow {
            section: InputSection::Containers,
            index: 0,
        }]
    );
    assert!(errors.to_string().contains("containers[0]"));
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
