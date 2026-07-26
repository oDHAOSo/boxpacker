use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::compatibility::adapt_saved_solution;
use boxpacker::geometry::{Aabb, Coordinate, Dimensions, Length, Point};
use boxpacker::model::{InputContainer, InputData, Item, OutputData};
use boxpacker::objective::ObjectiveValue;
use boxpacker::solution::{Placement, Solution};
use boxpacker::solver::{
    OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics, SolverOutcome,
};
use boxpacker::validate::{
    Axis, ItemLocation, PackingInstance, SolutionValidationError, validate_solution,
};

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

fn exact_dimensions(width: f64, length: f64, height: f64) -> Dimensions {
    Dimensions::new(
        Length::from_input_units(width).expect("test width should be exact"),
        Length::from_input_units(length).expect("test length should be exact"),
        Length::from_input_units(height).expect("test height should be exact"),
    )
}

fn exact_bounds(x: f64, y: f64, z: f64, width: f64, length: f64, height: f64) -> Aabb {
    Aabb::new(
        Point::new(
            Coordinate::from_input_units(x).expect("test x should be exact"),
            Coordinate::from_input_units(y).expect("test y should be exact"),
            Coordinate::from_input_units(z).expect("test z should be exact"),
        ),
        exact_dimensions(width, length, height),
    )
}

fn placement(
    instance: &PackingInstance,
    container_index: usize,
    item_index: usize,
    bounds: Aabb,
) -> Placement {
    Placement::new(
        instance.containers()[container_index].id(),
        instance.items()[item_index].id(),
        bounds,
    )
}

#[test]
fn current_saved_fixture_passes_the_independent_validator() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");
    let saved = adapt_saved_solution(&instance, output)
        .expect("current saved output should adapt to exact geometry");

    let summary = validate_solution(&instance, &saved.to_solution())
        .expect("current saved output should be independently valid");
    let objective = ObjectiveValue::from_summary(&summary);

    assert_eq!(summary.placed_item_count(), 49);
    assert_eq!(summary.unplaced_item_count(), 8);
    assert_eq!(summary.placed_volume(), 582_885_612);
    assert_eq!(summary.unplaced_volume(), 26_095_930);
    assert_eq!(summary.used_container_count(), 6);
    assert_eq!(objective.unplaced_volume(), 26_095_930);
    assert_eq!(objective.unplaced_item_count(), 8);
}

#[test]
fn face_contact_is_allowed_but_positive_volume_overlap_is_rejected() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[2.0, 2.0, 2.0], [2.0, 2.0, 2.0]]);
    let touching = Solution::new(
        vec![
            placement(&instance, 0, 0, exact_bounds(0.0, 0.0, 0.0, 2.0, 2.0, 2.0)),
            placement(&instance, 0, 1, exact_bounds(2.0, 0.0, 0.0, 2.0, 2.0, 2.0)),
        ],
        Vec::new(),
    );
    validate_solution(&instance, &touching).expect("face contact should not overlap");

    let overlapping = Solution::new(
        vec![
            touching.placements()[0],
            placement(&instance, 0, 1, exact_bounds(1.0, 0.0, 0.0, 2.0, 2.0, 2.0)),
        ],
        Vec::new(),
    );
    let errors =
        validate_solution(&instance, &overlapping).expect_err("positive overlap must fail");

    assert!(errors.errors().contains(&SolutionValidationError::Overlap {
        container_id: instance.containers()[0].id(),
        first_placement: 0,
        second_placement: 1,
    }));
}

#[test]
fn unsupported_area_is_an_objective_metric_not_a_geometry_failure() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0]]);
    let floating = Solution::new(
        vec![placement(
            &instance,
            0,
            0,
            exact_bounds(0.0, 0.0, 1.0, 1.0, 1.0, 1.0),
        )],
        Vec::new(),
    );

    let summary =
        validate_solution(&instance, &floating).expect("support is an objective, not validity");
    let objective = ObjectiveValue::from_summary(&summary);

    assert_eq!(summary.unsupported_area(), 100);
    assert_eq!(objective.unsupported_area(), 100);
}

#[test]
fn invalid_orientation_and_out_of_bounds_are_reported_exactly() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 2.0, 3.0]]);
    let invalid = Solution::new(
        vec![placement(
            &instance,
            0,
            0,
            exact_bounds(9.0, 0.0, 0.0, 4.0, 2.0, 1.0),
        )],
        Vec::new(),
    );

    let errors = validate_solution(&instance, &invalid)
        .expect_err("wrong dimensions and bounds should both fail");

    assert!(errors.errors().iter().any(|error| matches!(
        error,
        SolutionValidationError::InvalidOrientation {
            placement_index: 0,
            item_id,
            ..
        } if *item_id == instance.items()[0].id()
    )));
    assert!(
        errors
            .errors()
            .contains(&SolutionValidationError::OutOfBounds {
                placement_index: 0,
                container_id: instance.containers()[0].id(),
                axis: Axis::X,
                end: 130,
                limit: 100,
            })
    );
}

#[test]
fn duplicate_and_missing_items_are_reported_together() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0], [1.0, 1.0, 1.0]]);
    let item_zero = instance.items()[0].id();
    let item_one = instance.items()[1].id();
    let invalid = Solution::new(
        vec![placement(
            &instance,
            0,
            0,
            exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
        )],
        vec![item_zero],
    );

    let errors =
        validate_solution(&instance, &invalid).expect_err("coverage violations should fail");

    assert!(
        errors
            .errors()
            .contains(&SolutionValidationError::DuplicateItem {
                item_id: item_zero,
                first: ItemLocation::Placed(0),
                duplicate: ItemLocation::Unplaced(0),
            })
    );
    assert!(
        errors
            .errors()
            .contains(&SolutionValidationError::MissingItem { item_id: item_one })
    );
}

#[test]
fn coordinate_end_overflow_is_rejected_without_wrapping() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0]]);
    let bounds = Aabb::new(
        Point::new(
            Coordinate::from_scaled_units(u64::MAX),
            Coordinate::from_scaled_units(0),
            Coordinate::from_scaled_units(0),
        ),
        exact_dimensions(1.0, 1.0, 1.0),
    );
    let invalid = Solution::new(vec![placement(&instance, 0, 0, bounds)], Vec::new());

    let errors =
        validate_solution(&instance, &invalid).expect_err("coordinate overflow should fail");

    assert!(
        errors
            .errors()
            .contains(&SolutionValidationError::CoordinateOverflow {
                placement_index: 0,
                axis: Axis::X,
                origin: u64::MAX,
            })
    );
}

#[test]
fn ids_from_a_different_instance_cannot_escape_range_checks() {
    let larger = instance(
        &[[10.0, 10.0, 10.0], [10.0, 10.0, 10.0]],
        &[[1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
    );
    let smaller = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0]]);
    let invalid = Solution::new(
        vec![Placement::new(
            larger.containers()[1].id(),
            larger.items()[1].id(),
            exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
        )],
        vec![smaller.items()[0].id()],
    );

    let errors =
        validate_solution(&smaller, &invalid).expect_err("foreign out-of-range IDs should fail");

    assert!(matches!(
        errors.errors()[0],
        SolutionValidationError::UnknownContainer {
            placement_index: 0,
            ..
        }
    ));
    assert!(matches!(
        errors.errors()[1],
        SolutionValidationError::UnknownPlacedItem {
            placement_index: 0,
            ..
        }
    ));
}

#[test]
fn objective_is_volume_first_then_count_and_compactness() {
    let instance = instance(
        &[[10.0, 10.0, 10.0]],
        &[[2.0, 2.0, 2.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
    );
    let leave_large = Solution::new(
        vec![
            placement(&instance, 0, 1, exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)),
            placement(&instance, 0, 2, exact_bounds(1.0, 0.0, 0.0, 1.0, 1.0, 1.0)),
        ],
        vec![instance.items()[0].id()],
    );
    let leave_two_small = Solution::new(
        vec![placement(
            &instance,
            0,
            0,
            exact_bounds(0.0, 0.0, 0.0, 2.0, 2.0, 2.0),
        )],
        vec![instance.items()[1].id(), instance.items()[2].id()],
    );
    let volume_first = ObjectiveValue::from_summary(
        &validate_solution(&instance, &leave_two_small).expect("solution should validate"),
    );
    let count_first = ObjectiveValue::from_summary(
        &validate_solution(&instance, &leave_large).expect("solution should validate"),
    );

    assert!(volume_first > count_first);
    assert_eq!(volume_first.unplaced_item_count(), 2);
    assert_eq!(count_first.unplaced_item_count(), 1);

    let compact = Solution::new(
        vec![
            placement(&instance, 0, 1, exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)),
            placement(&instance, 0, 2, exact_bounds(1.0, 0.0, 0.0, 1.0, 1.0, 1.0)),
        ],
        vec![instance.items()[0].id()],
    );
    let spread = Solution::new(
        vec![
            compact.placements()[0],
            placement(&instance, 0, 2, exact_bounds(9.0, 0.0, 0.0, 1.0, 1.0, 1.0)),
        ],
        vec![instance.items()[0].id()],
    );
    let compact_objective = ObjectiveValue::from_summary(
        &validate_solution(&instance, &compact).expect("compact solution should validate"),
    );
    let spread_objective = ObjectiveValue::from_summary(
        &validate_solution(&instance, &spread).expect("spread solution should validate"),
    );

    assert!(compact_objective > spread_objective);
    assert!(compact_objective.bounding_volume() < spread_objective.bounding_volume());
}

#[test]
fn objective_uses_count_then_containers_after_volume() {
    let count_instance = instance(
        &[[10.0, 10.0, 10.0]],
        &[[1.0, 1.0, 2.0], [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
    );
    let leave_one = Solution::new(
        vec![
            placement(
                &count_instance,
                0,
                1,
                exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
            placement(
                &count_instance,
                0,
                2,
                exact_bounds(1.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
        ],
        vec![count_instance.items()[0].id()],
    );
    let leave_two = Solution::new(
        vec![placement(
            &count_instance,
            0,
            0,
            exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 2.0),
        )],
        vec![
            count_instance.items()[1].id(),
            count_instance.items()[2].id(),
        ],
    );
    let one_unplaced = ObjectiveValue::from_summary(
        &validate_solution(&count_instance, &leave_one).expect("solution should validate"),
    );
    let two_unplaced = ObjectiveValue::from_summary(
        &validate_solution(&count_instance, &leave_two).expect("solution should validate"),
    );

    assert_eq!(
        one_unplaced.unplaced_volume(),
        two_unplaced.unplaced_volume()
    );
    assert!(one_unplaced > two_unplaced);

    let container_instance = instance(
        &[[10.0, 10.0, 10.0], [10.0, 10.0, 10.0]],
        &[[1.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
    );
    let one_container = Solution::new(
        vec![
            placement(
                &container_instance,
                0,
                0,
                exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
            placement(
                &container_instance,
                0,
                1,
                exact_bounds(1.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
        ],
        Vec::new(),
    );
    let two_containers = Solution::new(
        vec![
            placement(
                &container_instance,
                0,
                0,
                exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
            placement(
                &container_instance,
                1,
                1,
                exact_bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            ),
        ],
        Vec::new(),
    );
    let one_container_objective = ObjectiveValue::from_summary(
        &validate_solution(&container_instance, &one_container)
            .expect("one-container solution should validate"),
    );
    let two_container_objective = ObjectiveValue::from_summary(
        &validate_solution(&container_instance, &two_containers)
            .expect("two-container solution should validate"),
    );

    assert_eq!(one_container_objective.unplaced_volume(), 0);
    assert_eq!(one_container_objective.unplaced_item_count(), 0);
    assert_eq!(one_container_objective.used_container_count(), 1);
    assert_eq!(two_container_objective.used_container_count(), 2);
    assert!(one_container_objective > two_container_objective);
}

struct AllUnplacedBackend;

impl SolverBackend for AllUnplacedBackend {
    fn name(&self) -> &str {
        "all-unplaced-test"
    }

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError> {
        let solution = Solution::new(
            Vec::new(),
            instance.items().iter().map(|item| item.id()).collect(),
        );
        Ok(SolverOutcome::new(
            solution,
            SolverMetrics::new(1, 1, 0, request.deadline().elapsed()),
            OptimalityStatus::Heuristic,
        ))
    }
}

#[test]
fn backend_contract_is_object_safe_and_reports_common_metrics() {
    let instance = instance(&[[10.0, 10.0, 10.0]], &[[1.0, 1.0, 1.0]]);
    let backend: &dyn SolverBackend = &AllUnplacedBackend;
    let request = SolveRequest::new(
        Duration::from_secs(1),
        42,
        NonZeroUsize::new(1).expect("one is non-zero"),
    );

    let outcome = backend
        .solve(&instance, &request)
        .expect("test backend should solve");
    let summary = validate_solution(&instance, outcome.solution())
        .expect("backend output should validate independently");

    assert_eq!(backend.name(), "all-unplaced-test");
    assert_eq!(request.seed(), 42);
    assert_eq!(request.threads().get(), 1);
    assert_eq!(outcome.optimality(), OptimalityStatus::Heuristic);
    assert_eq!(outcome.metrics().explored_candidates(), 1);
    assert_eq!(outcome.metrics().validated_candidates(), 1);
    assert_eq!(outcome.metrics().improvements(), 0);
    assert!(outcome.metrics().elapsed() <= request.deadline().elapsed());
    assert_eq!(summary.unplaced_item_count(), 1);
}

#[test]
fn bounded_randomized_valid_solutions_survive_reordering_and_detect_overlap() {
    let mut generator = TestGenerator::new(0x19b4_56f2_a77c_d301);

    for _case in 0..512 {
        let item_count = 2 + generator.range(5);
        let item_scaled_dimensions = (0..item_count)
            .map(|_| {
                [
                    generator.scaled_length(20),
                    generator.scaled_length(20),
                    generator.scaled_length(20),
                ]
            })
            .collect::<Vec<_>>();
        let container_scaled_width = item_scaled_dimensions
            .iter()
            .map(|dimensions| dimensions[0])
            .sum::<u64>();
        let container_scaled_length = item_scaled_dimensions
            .iter()
            .map(|dimensions| dimensions[1])
            .max()
            .expect("generated item set is non-empty");
        let container_scaled_height = item_scaled_dimensions
            .iter()
            .map(|dimensions| dimensions[2])
            .max()
            .expect("generated item set is non-empty");
        let item_dimensions = item_scaled_dimensions
            .iter()
            .map(|dimensions| {
                [
                    to_input_length(dimensions[0]),
                    to_input_length(dimensions[1]),
                    to_input_length(dimensions[2]),
                ]
            })
            .collect::<Vec<_>>();
        let instance = instance(
            &[[
                to_input_length(container_scaled_width),
                to_input_length(container_scaled_length),
                to_input_length(container_scaled_height),
            ]],
            &item_dimensions,
        );

        let mut scaled_x = 0_u64;
        let placements = item_scaled_dimensions
            .iter()
            .enumerate()
            .map(|(item_index, dimensions)| {
                let placed = placement(
                    &instance,
                    0,
                    item_index,
                    exact_bounds(
                        to_input_length(scaled_x),
                        0.0,
                        0.0,
                        to_input_length(dimensions[0]),
                        to_input_length(dimensions[1]),
                        to_input_length(dimensions[2]),
                    ),
                );
                scaled_x += dimensions[0];
                placed
            })
            .collect::<Vec<_>>();
        let valid = Solution::new(placements.clone(), Vec::new());
        let valid_summary =
            validate_solution(&instance, &valid).expect("generated shelf solution should validate");

        let mut reversed = placements.clone();
        reversed.reverse();
        let reversed_summary = validate_solution(&instance, &Solution::new(reversed, Vec::new()))
            .expect("placement order must not change validity");
        assert_eq!(
            ObjectiveValue::from_summary(&valid_summary),
            ObjectiveValue::from_summary(&reversed_summary)
        );
        assert_eq!(valid_summary.placed_item_count(), item_count);
        assert_eq!(valid_summary.unplaced_item_count(), 0);

        let mut overlapping = placements;
        let second_dimensions = item_dimensions[1];
        overlapping[1] = placement(
            &instance,
            0,
            1,
            exact_bounds(
                0.0,
                0.0,
                0.0,
                second_dimensions[0],
                second_dimensions[1],
                second_dimensions[2],
            ),
        );
        let errors = validate_solution(&instance, &Solution::new(overlapping, Vec::new()))
            .expect_err("generated positive-volume overlap should fail");
        assert!(
            errors
                .errors()
                .iter()
                .any(|error| matches!(error, SolutionValidationError::Overlap { .. }))
        );
    }
}

struct TestGenerator {
    state: u64,
}

impl TestGenerator {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn range(&mut self, upper: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(upper).expect("test range fits u64"))
            .expect("bounded test value fits usize")
    }

    fn scaled_length(&mut self, max_scaled: u64) -> u64 {
        1 + self.next() % max_scaled
    }
}

fn to_input_length(scaled: u64) -> f64 {
    scaled as f64 / 10.0
}
