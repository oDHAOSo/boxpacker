use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::compatibility::adapt_saved_solution;
use boxpacker::model::OutputData;
use boxpacker::model::{InputContainer, InputData, Item};
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::{OptimalityStatus, SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const CURRENT_SAVED_OUTPUT: &str = include_str!("fixtures/current/saved_output.json");

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

fn request(time_limit: Duration) -> SolveRequest {
    SolveRequest::new(
        time_limit,
        7,
        NonZeroUsize::new(1).expect("one is non-zero"),
    )
}

#[test]
fn maximal_spaces_pack_items_at_face_derived_coordinates() {
    let instance = instance(
        &[[10.0, 10.0, 10.0]],
        &[[3.7, 10.0, 10.0], [6.3, 10.0, 10.0]],
    );

    let outcome = ConstructiveBackend
        .solve(&instance, &request(Duration::from_secs(1)))
        .expect("constructive backend should solve");
    let summary = validate_solution(&instance, outcome.solution())
        .expect("constructive result should independently validate");
    let mut x_origins = outcome
        .solution()
        .placements()
        .iter()
        .map(|placement| placement.bounds().origin().x().get())
        .collect::<Vec<_>>();
    x_origins.sort_unstable();

    assert_eq!(summary.placed_item_count(), 2);
    assert_eq!(summary.unplaced_item_count(), 0);
    assert_eq!(x_origins, [0, 63]);
    assert_eq!(outcome.optimality(), OptimalityStatus::Heuristic);
}

#[test]
fn unique_rotations_allow_an_item_to_fit() {
    let instance = instance(&[[2.0, 3.0, 4.0]], &[[4.0, 2.0, 3.0]]);

    let outcome = ConstructiveBackend
        .solve(&instance, &request(Duration::from_secs(1)))
        .expect("rotated item should solve");
    let summary =
        validate_solution(&instance, outcome.solution()).expect("rotated solution should validate");

    assert_eq!(summary.placed_item_count(), 1);
    assert_eq!(summary.unplaced_item_count(), 0);
}

#[test]
fn items_that_fit_no_container_are_reported_unplaced() {
    let instance = instance(&[[2.0, 2.0, 2.0]], &[[3.0, 1.0, 1.0]]);

    let outcome = ConstructiveBackend
        .solve(&instance, &request(Duration::from_secs(1)))
        .expect("no-fit instance should still return a solution");
    let summary =
        validate_solution(&instance, outcome.solution()).expect("no-fit solution should validate");

    assert_eq!(summary.placed_item_count(), 0);
    assert_eq!(summary.unplaced_item_count(), 1);
}

#[test]
fn an_expired_deadline_returns_a_complete_valid_incumbent() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0], [2.0, 2.0, 2.0]]);

    let outcome = ConstructiveBackend
        .solve(&instance, &request(Duration::ZERO))
        .expect("expired search should return all items unplaced");
    let summary = validate_solution(&instance, outcome.solution())
        .expect("deadline incumbent should validate");

    assert_eq!(summary.placed_item_count(), 0);
    assert_eq!(summary.unplaced_item_count(), 2);
}

#[test]
fn current_fixture_constructive_result_is_valid_and_reproducible() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");

    let first = ConstructiveBackend
        .solve(&instance, &request(Duration::from_secs(10)))
        .expect("current fixture should solve");
    let second = ConstructiveBackend
        .solve(&instance, &request(Duration::from_secs(10)))
        .expect("current fixture should solve reproducibly");
    let summary = validate_solution(&instance, first.solution())
        .expect("current constructive result should validate");
    let saved_output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output should deserialize");
    let saved =
        adapt_saved_solution(&instance, saved_output).expect("current saved output should adapt");
    let saved_summary =
        validate_solution(&instance, &saved.to_solution()).expect("saved baseline should validate");

    assert_eq!(first.solution(), second.solution());
    assert_eq!(
        summary.placed_item_count() + summary.unplaced_item_count(),
        57
    );
    assert!(first.metrics().explored_candidates() > 0);
    assert!(
        ObjectiveValue::from_summary(&summary) >= ObjectiveValue::from_summary(&saved_summary),
        "constructive baseline must match or improve the saved result"
    );
    assert_eq!(summary.placed_item_count(), 53);
    assert_eq!(summary.placed_volume(), 587_815_524);
}
