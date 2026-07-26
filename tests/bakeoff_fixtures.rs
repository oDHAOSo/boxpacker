use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::model::{InputContainer, InputData, Item};
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::bin_packing::{BinPackingBackend, BinPackingStrategy};
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::u_nesting::{UNestingBackend, UNestingStrategy};
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, SolutionSummary, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const SCALE_INPUT: &str = include_str!("fixtures/generated/scale_8x77.json");

fn request() -> SolveRequest {
    SolveRequest::new(
        Duration::from_secs(10),
        23,
        NonZeroUsize::new(1).expect("one is non-zero"),
    )
}

fn backends() -> Vec<Box<dyn SolverBackend>> {
    vec![
        Box::new(ConstructiveBackend),
        Box::new(BinPackingBackend::new(
            BinPackingStrategy::ExtremePointsContactPoint,
        )),
        Box::new(UNestingBackend::new(UNestingStrategy::ExtremePoint)),
    ]
}

fn validated_instance(input: &InputData) -> PackingInstance {
    PackingInstance::try_from(input).expect("bake-off fixture should validate")
}

fn solve(backend: &dyn SolverBackend, instance: &PackingInstance) -> SolutionSummary {
    let outcome = backend
        .solve(instance, &request())
        .unwrap_or_else(|error| panic!("{} failed: {error}", backend.name()));
    validate_solution(instance, outcome.solution())
        .unwrap_or_else(|error| panic!("{} returned invalid output: {error}", backend.name()))
}

fn small_instance(container: [f64; 3], items: &[[f64; 3]]) -> InputData {
    InputData {
        containers: vec![InputContainer {
            name: "known container".to_owned(),
            width: container[0],
            length: container[1],
            height: container[2],
        }],
        contents: items
            .iter()
            .enumerate()
            .map(|(index, dimensions)| Item {
                name: format!("known item {index}"),
                width: dimensions[0],
                length: dimensions[1],
                height: dimensions[2],
            })
            .collect(),
    }
}

#[test]
fn small_known_answer_fills_one_container() {
    let input = small_instance(
        [4.0, 4.0, 4.0],
        &[
            [2.0, 2.0, 4.0],
            [2.0, 2.0, 4.0],
            [2.0, 2.0, 4.0],
            [2.0, 2.0, 4.0],
        ],
    );
    let instance = validated_instance(&input);

    for backend in backends() {
        let summary = solve(backend.as_ref(), &instance);
        assert_eq!(summary.placed_item_count(), 4, "{}", backend.name());
        assert_eq!(summary.unplaced_item_count(), 0, "{}", backend.name());
        assert_eq!(summary.used_container_count(), 1, "{}", backend.name());
        assert_eq!(summary.placed_volume(), 64_000, "{}", backend.name());
    }
}

#[test]
fn adversarial_volume_slack_does_not_imply_geometric_fit() {
    let input = small_instance([5.0, 5.0, 1.0], &[[3.0, 4.0, 1.0], [3.0, 4.0, 1.0]]);
    let instance = validated_instance(&input);

    for backend in backends() {
        let summary = solve(backend.as_ref(), &instance);
        assert_eq!(summary.placed_item_count(), 1, "{}", backend.name());
        assert_eq!(summary.unplaced_item_count(), 1, "{}", backend.name());
        assert_eq!(summary.placed_volume(), 12_000, "{}", backend.name());
    }
}

#[test]
fn current_fixture_and_its_reversal_produce_valid_objectives() {
    let original: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current fixture should deserialize");
    let mut reversed = original.clone();
    reversed.containers.reverse();
    reversed.contents.reverse();
    let original_instance = validated_instance(&original);
    let reversed_instance = validated_instance(&reversed);

    for (backend, (expected_placed, expected_volume)) in
        backends()
            .into_iter()
            .zip([(53, 587_815_524), (41, 535_042_896), (49, 568_460_714)])
    {
        let original_summary = solve(backend.as_ref(), &original_instance);
        let reversed_summary = solve(backend.as_ref(), &reversed_instance);
        assert_eq!(
            original_summary.placed_item_count(),
            expected_placed,
            "{}",
            backend.name()
        );
        assert_eq!(
            original_summary.placed_volume(),
            expected_volume,
            "{}",
            backend.name()
        );
        assert_eq!(
            ObjectiveValue::from_summary(&original_summary),
            ObjectiveValue::from_summary(&reversed_summary),
            "{} objective should be input-order invariant",
            backend.name()
        );
    }
}

#[test]
fn generated_eight_container_seventy_seven_item_scale_fixture_is_valid() {
    let input: InputData =
        serde_json::from_str(SCALE_INPUT).expect("scale fixture should deserialize");
    assert_eq!(input.containers.len(), 8);
    assert_eq!(input.contents.len(), 77);
    let instance = validated_instance(&input);

    for (backend, expected) in backends().into_iter().zip([
        (73, 694_614_920, 8, 199_220, 836_936_280),
        (70, 687_975_920, 8, 362_560, 807_771_800),
        (73, 694_614_920, 8, 282_900, 830_038_000),
    ]) {
        let summary = solve(backend.as_ref(), &instance);
        assert_eq!(
            summary.placed_item_count() + summary.unplaced_item_count(),
            77,
            "{}",
            backend.name()
        );
        assert_eq!(
            (
                summary.placed_item_count(),
                summary.placed_volume(),
                summary.used_container_count(),
                summary.unsupported_area(),
                summary.bounding_volume(),
            ),
            expected,
            "{} scale objective components changed",
            backend.name()
        );
    }
}
