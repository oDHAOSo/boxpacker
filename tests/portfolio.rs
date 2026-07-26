use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::model::InputData;
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::portfolio::PortfolioBackend;
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");

fn instance() -> PackingInstance {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current fixture should deserialize");
    PackingInstance::try_from(&input).expect("current fixture should validate")
}

fn request(seed: u64, threads: usize) -> SolveRequest {
    SolveRequest::new(
        Duration::from_secs(10),
        seed,
        NonZeroUsize::new(threads).expect("test thread count should be non-zero"),
    )
}

#[test]
fn fixed_seed_portfolio_is_reproducible_across_thread_counts() {
    let instance = instance();
    let backend = PortfolioBackend::new(NonZeroUsize::new(8).expect("eight is non-zero"));

    let serial = backend
        .solve(&instance, &request(73, 1))
        .expect("serial portfolio should solve");
    let parallel = backend
        .solve(&instance, &request(73, 4))
        .expect("parallel portfolio should solve");
    let repeated = backend
        .solve(&instance, &request(73, 4))
        .expect("repeated portfolio should solve");

    assert_eq!(serial.solution(), parallel.solution());
    assert_eq!(parallel.solution(), repeated.solution());
    assert_eq!(serial.metrics().validated_candidates(), 8);
    assert_eq!(parallel.metrics().validated_candidates(), 8);
    validate_solution(&instance, serial.solution())
        .expect("selected portfolio solution should validate");
}

#[test]
fn portfolio_retains_or_improves_the_canonical_incumbent() {
    let instance = instance();
    let canonical = ConstructiveBackend
        .solve(&instance, &request(91, 1))
        .expect("canonical constructor should solve");
    let portfolio = PortfolioBackend::new(NonZeroUsize::new(8).expect("eight is non-zero"))
        .solve(&instance, &request(91, 4))
        .expect("portfolio should solve");

    let canonical_summary = validate_solution(&instance, canonical.solution())
        .expect("canonical solution should validate");
    let portfolio_summary = validate_solution(&instance, portfolio.solution())
        .expect("portfolio solution should validate");

    assert!(
        ObjectiveValue::from_summary(&portfolio_summary)
            >= ObjectiveValue::from_summary(&canonical_summary)
    );
    assert_eq!(portfolio_summary.placed_item_count(), 53);
    assert_eq!(portfolio_summary.placed_volume(), 587_815_524);
}
