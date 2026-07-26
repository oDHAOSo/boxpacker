use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use boxpacker::model::InputData;
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::bin_packing::{BinPackingBackend, BinPackingStrategy};
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::u_nesting::{UNestingBackend, UNestingStrategy};
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const SCALE_INPUT: &str = include_str!("fixtures/generated/scale_8x77.json");

fn instance(json: &str) -> PackingInstance {
    let input: InputData =
        serde_json::from_str(json).expect("evaluation fixture should deserialize");
    PackingInstance::try_from(&input).expect("evaluation fixture should validate")
}

fn request(limit: Duration) -> SolveRequest {
    SolveRequest::new(limit, 29, NonZeroUsize::new(1).expect("one is non-zero"))
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

#[test]
fn fixed_seed_and_effort_reproduce_every_candidate() {
    let instance = instance(CURRENT_INPUT);

    for backend in backends() {
        let first = backend
            .solve(&instance, &request(Duration::from_secs(10)))
            .unwrap_or_else(|error| panic!("{} first run failed: {error}", backend.name()));
        let second = backend
            .solve(&instance, &request(Duration::from_secs(10)))
            .unwrap_or_else(|error| panic!("{} second run failed: {error}", backend.name()));
        let first_summary = validate_solution(&instance, first.solution())
            .unwrap_or_else(|error| panic!("{} first run invalid: {error}", backend.name()));
        let second_summary = validate_solution(&instance, second.solution())
            .unwrap_or_else(|error| panic!("{} second run invalid: {error}", backend.name()));

        assert_eq!(first.solution(), second.solution(), "{}", backend.name());
        assert_eq!(
            ObjectiveValue::from_summary(&first_summary),
            ObjectiveValue::from_summary(&second_summary),
            "{}",
            backend.name()
        );
    }
}

#[test]
fn bounded_backends_return_valid_scale_incumbents_with_shutdown_allowance() {
    let instance = instance(SCALE_INPUT);
    let limit = Duration::from_millis(5);
    let allowance = Duration::from_millis(250);

    for backend in [
        Box::new(ConstructiveBackend) as Box<dyn SolverBackend>,
        Box::new(UNestingBackend::new(UNestingStrategy::ExtremePoint)) as Box<dyn SolverBackend>,
    ] {
        let started_at = Instant::now();
        let outcome = backend
            .solve(&instance, &request(limit))
            .unwrap_or_else(|error| panic!("{} failed: {error}", backend.name()));
        let wall_time = started_at.elapsed();

        validate_solution(&instance, outcome.solution())
            .unwrap_or_else(|error| panic!("{} returned invalid output: {error}", backend.name()));
        assert!(
            wall_time <= limit + allowance,
            "{} took {wall_time:?} for a {limit:?} budget",
            backend.name()
        );
    }
}
