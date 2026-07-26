use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use u_nesting_d3::geometry::OrientationConstraint;
use u_nesting_d3::{
    Boundary3D, Config, Geometry3D, Packer3D, Solver, Strategy as ExternalStrategy,
};

use crate::geometry::{Aabb, Coordinate, Dimensions, MAX_EXACT_SCALED_LENGTH, Point};
use crate::solution::{Placement, Solution};
use crate::solver::{
    OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics, SolverOutcome,
};
use crate::validate::{ContainerId, ItemId, PackingInstance, validate_solution};

const ITEM_PREFIX: &str = "boxpacker-item-";

/// Deterministic `u-nesting-d3` strategies admitted to the bake-off adapter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UNestingStrategy {
    #[default]
    ExtremePoint,
    BottomLeftFill,
}

/// Sequential heterogeneous-container adapter for `u-nesting-d3` 0.6.
///
/// The dependency accepts one boundary per solve, so this wrapper visits each
/// inventory container once and passes only the remaining item instances.
#[derive(Clone, Copy, Debug, Default)]
pub struct UNestingBackend {
    strategy: UNestingStrategy,
}

impl UNestingBackend {
    #[must_use]
    pub const fn new(strategy: UNestingStrategy) -> Self {
        Self { strategy }
    }
}

impl SolverBackend for UNestingBackend {
    fn name(&self) -> &str {
        match self.strategy {
            UNestingStrategy::ExtremePoint => "u-nesting-d3/extreme-point-sequential",
            UNestingStrategy::BottomLeftFill => "u-nesting-d3/bottom-left-fill-sequential",
        }
    }

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError> {
        let started_at = Instant::now();
        let mut remaining = instance
            .items()
            .iter()
            .map(|item| item.id())
            .collect::<BTreeSet<_>>();
        let mut container_ids = instance
            .containers()
            .iter()
            .map(|container| container.id())
            .collect::<Vec<_>>();
        container_ids.sort_unstable_by(|left, right| compare_containers(instance, *left, *right));

        let mut placements = Vec::new();
        let mut solve_count = 0_u64;
        for container_id in container_ids {
            if remaining.is_empty() || request.deadline().is_expired() {
                break;
            }
            let remaining_time = request.deadline().remaining();
            let time_limit_ms = duration_millis(remaining_time);
            if time_limit_ms == 0 {
                break;
            }

            let geometries = remaining
                .iter()
                .map(|item_id| {
                    let dimensions = instance.items()[item_id.index()].dimensions();
                    Geometry3D::new(
                        item_name(*item_id),
                        scaled_f64(dimensions.width().get()),
                        scaled_f64(dimensions.length().get()),
                        scaled_f64(dimensions.height().get()),
                    )
                    .with_orientation(OrientationConstraint::Any)
                })
                .collect::<Vec<_>>();
            let container = &instance.containers()[container_id.index()];
            let boundary = Boundary3D::new(
                scaled_f64(container.dimensions().width().get()),
                scaled_f64(container.dimensions().length().get()),
                scaled_f64(container.dimensions().height().get()),
            );
            let mut config = Config::new()
                .with_strategy(external_strategy(self.strategy))
                .with_spacing(0.0)
                .with_margin(0.0)
                .with_time_limit(time_limit_ms);
            config.threads = request.threads().get();
            let result = Packer3D::new(config)
                .solve(&geometries, &boundary)
                .map_err(|error| SolverError::new(format!("u-nesting-d3 solve failed: {error}")))?;
            solve_count = solve_count
                .checked_add(1)
                .ok_or_else(|| SolverError::new("u-nesting solve metric overflowed"))?;

            let mut placed_this_container = BTreeSet::new();
            for external in &result.placements {
                let item_id = parse_item_id(external.geometry_id.as_str(), instance)?;
                if !remaining.contains(&item_id) || !placed_this_container.insert(item_id) {
                    return Err(SolverError::new(format!(
                        "u-nesting-d3 returned unexpected duplicate item index {}",
                        item_id.index()
                    )));
                }
                let position = external.position.as_slice();
                if position.len() != 3 {
                    return Err(SolverError::new(format!(
                        "u-nesting-d3 returned {} coordinates for item index {}",
                        position.len(),
                        item_id.index()
                    )));
                }
                let dimensions = oriented_dimensions(
                    instance.items()[item_id.index()].dimensions(),
                    external.rotation_index.unwrap_or(0),
                )?;
                placements.push(Placement::new(
                    container_id,
                    item_id,
                    Aabb::new(
                        Point::new(
                            scaled_coordinate(position[0])?,
                            scaled_coordinate(position[1])?,
                            scaled_coordinate(position[2])?,
                        ),
                        dimensions,
                    ),
                ));
            }
            for item_id in placed_this_container {
                remaining.remove(&item_id);
            }
        }

        let solution = Solution::new(placements, remaining.into_iter().collect());
        validate_solution(instance, &solution).map_err(|errors| {
            SolverError::new(format!(
                "u-nesting-d3 adapter produced an invalid candidate:\n{errors}"
            ))
        })?;
        let improvements = u64::try_from(solution.placements().len())
            .map_err(|_| SolverError::new("u-nesting placement metric overflowed"))?;
        Ok(SolverOutcome::new(
            solution,
            SolverMetrics::new(0, solve_count, improvements, started_at.elapsed()),
            OptimalityStatus::Heuristic,
        ))
    }
}

fn compare_containers(
    instance: &PackingInstance,
    left: ContainerId,
    right: ContainerId,
) -> Ordering {
    let left_container = &instance.containers()[left.index()];
    let right_container = &instance.containers()[right.index()];
    left_container
        .dimensions()
        .checked_volume()
        .cmp(&right_container.dimensions().checked_volume())
        .then_with(|| {
            dimensions_key(left_container.dimensions())
                .cmp(&dimensions_key(right_container.dimensions()))
        })
        .then_with(|| left_container.name().cmp(right_container.name()))
        .then_with(|| left.cmp(&right))
}

fn dimensions_key(dimensions: Dimensions) -> [u64; 3] {
    [
        dimensions.width().get(),
        dimensions.length().get(),
        dimensions.height().get(),
    ]
}

const fn external_strategy(strategy: UNestingStrategy) -> ExternalStrategy {
    match strategy {
        UNestingStrategy::ExtremePoint => ExternalStrategy::ExtremePoint,
        UNestingStrategy::BottomLeftFill => ExternalStrategy::BottomLeftFill,
    }
}

fn duration_millis(duration: Duration) -> u64 {
    if duration.is_zero() {
        return 0;
    }
    u64::try_from(duration.as_millis().max(1)).unwrap_or(u64::MAX)
}

fn scaled_f64(value: u64) -> f64 {
    value as f64
}

fn scaled_coordinate(value: f64) -> Result<Coordinate, SolverError> {
    if !value.is_finite() || value < 0.0 {
        return Err(SolverError::new(format!(
            "u-nesting-d3 returned invalid coordinate {value}"
        )));
    }
    if value.fract() != 0.0 || value > MAX_EXACT_SCALED_LENGTH as f64 {
        return Err(SolverError::new(format!(
            "u-nesting-d3 returned inexact coordinate {value}"
        )));
    }
    Ok(Coordinate::from_scaled_units(value as u64))
}

fn oriented_dimensions(
    dimensions: Dimensions,
    orientation_index: usize,
) -> Result<Dimensions, SolverError> {
    let width = dimensions.width();
    let length = dimensions.length();
    let height = dimensions.height();
    match orientation_index {
        0 => Ok(Dimensions::new(width, length, height)),
        1 => Ok(Dimensions::new(width, height, length)),
        2 => Ok(Dimensions::new(length, width, height)),
        3 => Ok(Dimensions::new(length, height, width)),
        4 => Ok(Dimensions::new(height, width, length)),
        5 => Ok(Dimensions::new(height, length, width)),
        _ => Err(SolverError::new(format!(
            "u-nesting-d3 returned unknown orientation index {orientation_index}"
        ))),
    }
}

fn item_name(item_id: ItemId) -> String {
    format!("{ITEM_PREFIX}{}", item_id.index())
}

fn parse_item_id(name: &str, instance: &PackingInstance) -> Result<ItemId, SolverError> {
    let index = name
        .strip_prefix(ITEM_PREFIX)
        .ok_or_else(|| SolverError::new(format!("u-nesting-d3 returned malformed item {name:?}")))?
        .parse::<usize>()
        .map_err(|_| SolverError::new(format!("u-nesting-d3 returned malformed item {name:?}")))?;
    instance
        .items()
        .get(index)
        .map(|item| item.id())
        .ok_or_else(|| SolverError::new(format!("u-nesting-d3 returned unknown item {name:?}")))
}
