use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::model::{InputContainer, InputData, Item};
use boxpacker::solver::bin_packing::{BinPackingBackend, BinPackingStrategy};
use boxpacker::solver::u_nesting::{UNestingBackend, UNestingStrategy};
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");

fn instance(container_dimensions: &[[f64; 3]], item_dimensions: &[[f64; 3]]) -> PackingInstance {
    let input = InputData {
        containers: container_dimensions
            .iter()
            .enumerate()
            .map(|(index, dimensions)| InputContainer {
                name: format!("container {index}"),
                width: dimensions[0],
                length: dimensions[1],
                height: dimensions[2],
            })
            .collect(),
        contents: item_dimensions
            .iter()
            .enumerate()
            .map(|(index, dimensions)| Item {
                name: format!("item {index}"),
                width: dimensions[0],
                length: dimensions[1],
                height: dimensions[2],
            })
            .collect(),
    };
    PackingInstance::try_from(&input).expect("test instance should validate")
}

fn request() -> SolveRequest {
    SolveRequest::new(
        Duration::from_secs(10),
        17,
        NonZeroUsize::new(1).expect("one is non-zero"),
    )
}

fn candidate_backends() -> Vec<Box<dyn SolverBackend>> {
    vec![
        Box::new(BinPackingBackend::new(
            BinPackingStrategy::ExtremePointsContactPoint,
        )),
        Box::new(UNestingBackend::new(UNestingStrategy::ExtremePoint)),
    ]
}

#[test]
fn adapters_hide_dependency_types_and_preserve_heterogeneous_inventory() {
    let instance = instance(
        &[[4.0, 4.0, 4.0], [2.0, 2.0, 2.0]],
        &[[4.0, 4.0, 4.0], [2.0, 2.0, 2.0]],
    );

    for backend in candidate_backends() {
        let outcome = backend
            .solve(&instance, &request())
            .unwrap_or_else(|error| panic!("{} failed: {error}", backend.name()));
        let summary = validate_solution(&instance, outcome.solution())
            .unwrap_or_else(|error| panic!("{} returned invalid output: {error}", backend.name()));

        assert_eq!(summary.placed_item_count(), 2, "{}", backend.name());
        assert_eq!(summary.unplaced_item_count(), 0, "{}", backend.name());
        assert_eq!(summary.used_container_count(), 2, "{}", backend.name());
    }
}

#[test]
fn adapters_return_independently_valid_rotations_and_unplaced_items() {
    let rotating = instance(&[[2.0, 3.0, 4.0]], &[[4.0, 2.0, 3.0]]);
    let no_fit = instance(&[[2.0, 2.0, 2.0]], &[[3.0, 3.0, 1.0]]);

    for backend in candidate_backends() {
        let rotated = backend
            .solve(&rotating, &request())
            .unwrap_or_else(|error| panic!("{} rotation failed: {error}", backend.name()));
        let rotated_summary = validate_solution(&rotating, rotated.solution())
            .unwrap_or_else(|error| panic!("{} rotation invalid: {error}", backend.name()));
        assert_eq!(rotated_summary.placed_item_count(), 1, "{}", backend.name());

        let unplaced = backend
            .solve(&no_fit, &request())
            .unwrap_or_else(|error| panic!("{} no-fit failed: {error}", backend.name()));
        let unplaced_summary = validate_solution(&no_fit, unplaced.solution())
            .unwrap_or_else(|error| panic!("{} no-fit invalid: {error}", backend.name()));
        assert_eq!(
            unplaced_summary.unplaced_item_count(),
            1,
            "{}",
            backend.name()
        );
    }
}

#[test]
fn current_fixture_candidate_outputs_are_independently_valid() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");

    for (backend, expected_placed, expected_volume) in [
        (
            Box::new(BinPackingBackend::new(
                BinPackingStrategy::ExtremePointsContactPoint,
            )) as Box<dyn SolverBackend>,
            41,
            535_042_896,
        ),
        (
            Box::new(UNestingBackend::new(UNestingStrategy::ExtremePoint))
                as Box<dyn SolverBackend>,
            49,
            568_460_714,
        ),
    ] {
        let outcome = backend
            .solve(&instance, &request())
            .unwrap_or_else(|error| panic!("{} failed: {error}", backend.name()));
        let summary = validate_solution(&instance, outcome.solution())
            .unwrap_or_else(|error| panic!("{} returned invalid output: {error}", backend.name()));

        assert_eq!(
            summary.placed_item_count() + summary.unplaced_item_count(),
            57,
            "{}",
            backend.name()
        );
        assert_eq!(
            summary.placed_item_count(),
            expected_placed,
            "{}",
            backend.name()
        );
        assert_eq!(
            summary.placed_volume(),
            expected_volume,
            "{}",
            backend.name()
        );
    }
}

#[test]
fn bin_packing_dimension_cap_is_reported_without_truncation() {
    let instance = instance(&[[4_000.0, 1.0, 1.0]], &[[1.0, 1.0, 1.0]]);
    let error = BinPackingBackend::default()
        .solve(&instance, &request())
        .expect_err("bin-packing u32 safety cap should be explicit");

    assert!(error.to_string().contains("maximum is 32768"));
}
