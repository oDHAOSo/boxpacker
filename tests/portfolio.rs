use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use boxpacker::model::InputData;
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::portfolio::PortfolioBackend;
use boxpacker::solver::{CancellationToken, SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const SCALE_INPUT: &str = include_str!("fixtures/generated/scale_8x77.json");

fn instance(json: &str) -> PackingInstance {
    let input: InputData = serde_json::from_str(json).expect("fixture should deserialize");
    PackingInstance::try_from(&input).expect("fixture should validate")
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
    let instance = instance(CURRENT_INPUT);
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
    let instance = instance(CURRENT_INPUT);
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

#[test]
fn neighborhood_portfolio_retains_or_improves_the_scale_incumbent() {
    let instance = instance(SCALE_INPUT);
    let canonical = ConstructiveBackend
        .solve(&instance, &request(97, 1))
        .expect("scale canonical constructor should solve");
    let portfolio = PortfolioBackend::default()
        .solve(&instance, &request(97, 4))
        .expect("scale neighborhood portfolio should solve");

    let canonical_summary = validate_solution(&instance, canonical.solution())
        .expect("scale canonical solution should validate");
    let portfolio_summary = validate_solution(&instance, portfolio.solution())
        .expect("scale portfolio solution should validate");

    assert!(
        ObjectiveValue::from_summary(&portfolio_summary)
            >= ObjectiveValue::from_summary(&canonical_summary)
    );
    assert_eq!(
        portfolio_summary.placed_item_count() + portfolio_summary.unplaced_item_count(),
        77
    );
    assert_eq!(portfolio.metrics().validated_candidates(), 8);
}

#[test]
fn cancelled_request_returns_a_valid_canonical_incumbent() {
    let instance = instance(CURRENT_INPUT);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let request = SolveRequest::with_cancellation(
        Duration::from_secs(10),
        11,
        NonZeroUsize::new(4).expect("four is non-zero"),
        cancellation,
    );

    let outcome = PortfolioBackend::default()
        .solve(&instance, &request)
        .expect("cancelled portfolio should return its valid canonical incumbent");
    let summary = validate_solution(&instance, outcome.solution())
        .expect("cancelled incumbent should validate");

    assert_eq!(outcome.metrics().validated_candidates(), 1);
    assert_eq!(summary.placed_item_count(), 0);
    assert_eq!(summary.unplaced_item_count(), 57);
}

#[test]
fn deadline_stops_large_portfolio_with_shutdown_allowance() {
    let instance = instance(SCALE_INPUT);
    let time_limit = Duration::from_millis(5);
    let allowance = Duration::from_millis(250);
    let request = SolveRequest::new(
        time_limit,
        19,
        NonZeroUsize::new(4).expect("four is non-zero"),
    );
    let backend =
        PortfolioBackend::new(NonZeroUsize::new(10_000).expect("ten thousand is non-zero"));

    let started_at = Instant::now();
    let outcome = backend
        .solve(&instance, &request)
        .expect("deadline-bounded portfolio should return an incumbent");
    let wall_time = started_at.elapsed();

    validate_solution(&instance, outcome.solution())
        .expect("deadline-bounded incumbent should validate");
    assert!(
        wall_time <= time_limit + allowance,
        "portfolio took {wall_time:?} for a {time_limit:?} budget"
    );
    assert!(outcome.metrics().validated_candidates() >= 1);
    assert!(outcome.metrics().validated_candidates() < 10_000);
}
