use std::time::Instant;

use bin_packing::three_d::{
    Bin3D, BoxDemand3D, MAX_DIMENSION_3D, RotationMask3D, ThreeDAlgorithm, ThreeDOptions,
    ThreeDProblem, solve_3d,
};

use crate::geometry::{Aabb, Coordinate, Dimensions, Length, Point};
use crate::solution::{Placement, Solution};
use crate::solver::{
    OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics, SolverOutcome,
};
use crate::validate::{ContainerId, ItemId, PackingInstance, validate_solution};

const CONTAINER_PREFIX: &str = "boxpacker-container-";
const ITEM_PREFIX: &str = "boxpacker-item-";

/// Candidate algorithms exposed without leaking `bin-packing` crate types.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BinPackingStrategy {
    ExtremePoints,
    #[default]
    ExtremePointsContactPoint,
    Auto,
}

/// Adapter for the `bin-packing` 0.3 candidate.
#[derive(Clone, Copy, Debug, Default)]
pub struct BinPackingBackend {
    strategy: BinPackingStrategy,
}

impl BinPackingBackend {
    #[must_use]
    pub const fn new(strategy: BinPackingStrategy) -> Self {
        Self { strategy }
    }
}

impl SolverBackend for BinPackingBackend {
    fn name(&self) -> &str {
        match self.strategy {
            BinPackingStrategy::ExtremePoints => "bin-packing/extreme-points",
            BinPackingStrategy::ExtremePointsContactPoint => {
                "bin-packing/extreme-points-contact-point"
            }
            BinPackingStrategy::Auto => "bin-packing/auto",
        }
    }

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError> {
        let started_at = Instant::now();
        if request.deadline().is_expired() || instance.containers().is_empty() {
            return all_unplaced(instance, started_at);
        }
        ensure_supported_dimensions(instance)?;

        let bins = instance
            .containers()
            .iter()
            .map(|container| {
                Ok(Bin3D {
                    name: container_name(container.id()),
                    width: to_u32(container.dimensions().width().get())?,
                    height: to_u32(container.dimensions().height().get())?,
                    depth: to_u32(container.dimensions().length().get())?,
                    cost: 1.0,
                    quantity: Some(1),
                })
            })
            .collect::<Result<Vec<_>, SolverError>>()?;
        let (demands, mut forced_unplaced) = partition_demands(instance)?;
        if demands.is_empty() {
            forced_unplaced.sort_unstable();
            let solution = Solution::new(Vec::new(), forced_unplaced);
            validate_solution(instance, &solution).map_err(invalid_candidate)?;
            return Ok(SolverOutcome::new(
                solution,
                SolverMetrics::new(0, 1, 0, started_at.elapsed()),
                OptimalityStatus::Heuristic,
            ));
        }

        let external = solve_3d(
            ThreeDProblem { bins, demands },
            ThreeDOptions {
                algorithm: external_algorithm(self.strategy),
                seed: Some(request.seed()),
                ..ThreeDOptions::default()
            },
        )
        .map_err(|error| SolverError::new(format!("bin-packing solve failed: {error}")))?;

        let mut seen_items = vec![false; instance.items().len()];
        let mut placements = Vec::new();
        for layout in &external.layouts {
            let container_id = parse_container_id(&layout.bin_name, instance)?;
            for external_placement in &layout.placements {
                let item_id = parse_item_id(&external_placement.name, instance)?;
                if seen_items[item_id.index()] {
                    return Err(SolverError::new(format!(
                        "bin-packing returned item index {} more than once",
                        item_id.index()
                    )));
                }
                seen_items[item_id.index()] = true;
                placements.push(Placement::new(
                    container_id,
                    item_id,
                    Aabb::new(
                        Point::new(
                            Coordinate::from_scaled_units(u64::from(external_placement.x)),
                            Coordinate::from_scaled_units(u64::from(external_placement.z)),
                            Coordinate::from_scaled_units(u64::from(external_placement.y)),
                        ),
                        dimensions_from_scaled(
                            u64::from(external_placement.width),
                            u64::from(external_placement.depth),
                            u64::from(external_placement.height),
                        )?,
                    ),
                ));
            }
        }

        forced_unplaced.extend(
            instance
                .items()
                .iter()
                .filter(|item| !seen_items[item.id().index()])
                .map(|item| item.id()),
        );
        forced_unplaced.sort_unstable();
        forced_unplaced.dedup();
        let solution = Solution::new(placements, forced_unplaced);
        validate_solution(instance, &solution).map_err(invalid_candidate)?;

        let explored_candidates = u64::try_from(external.metrics.explored_states)
            .map_err(|_| SolverError::new("bin-packing explored-state metric overflowed"))?;
        let improvements = u64::try_from(solution.placements().len())
            .map_err(|_| SolverError::new("bin-packing placement metric overflowed"))?;
        Ok(SolverOutcome::new(
            solution,
            SolverMetrics::new(explored_candidates, 1, improvements, started_at.elapsed()),
            if external.exact {
                OptimalityStatus::ProvenOptimal
            } else {
                OptimalityStatus::Heuristic
            },
        ))
    }
}

fn ensure_supported_dimensions(instance: &PackingInstance) -> Result<(), SolverError> {
    let maximum = u64::from(MAX_DIMENSION_3D);
    for container in instance.containers() {
        for value in [
            container.dimensions().width().get(),
            container.dimensions().length().get(),
            container.dimensions().height().get(),
        ] {
            if value > maximum {
                return Err(SolverError::new(format!(
                    "bin-packing cannot represent container index {} dimension {value}; maximum is {maximum}",
                    container.id().index()
                )));
            }
        }
    }
    for item in instance.items() {
        for value in [
            item.dimensions().width().get(),
            item.dimensions().length().get(),
            item.dimensions().height().get(),
        ] {
            if value > maximum {
                return Err(SolverError::new(format!(
                    "bin-packing cannot represent item index {} dimension {value}; maximum is {maximum}",
                    item.id().index()
                )));
            }
        }
    }
    Ok(())
}

fn partition_demands(
    instance: &PackingInstance,
) -> Result<(Vec<BoxDemand3D>, Vec<ItemId>), SolverError> {
    let mut demands = Vec::new();
    let mut forced_unplaced = Vec::new();
    for item in instance.items() {
        if item_fits_any_container(instance, item.dimensions()) {
            demands.push(BoxDemand3D {
                name: item_name(item.id()),
                width: to_u32(item.dimensions().width().get())?,
                height: to_u32(item.dimensions().height().get())?,
                depth: to_u32(item.dimensions().length().get())?,
                quantity: 1,
                allowed_rotations: RotationMask3D::ALL,
            });
        } else {
            forced_unplaced.push(item.id());
        }
    }
    Ok((demands, forced_unplaced))
}

fn item_fits_any_container(instance: &PackingInstance, dimensions: Dimensions) -> bool {
    dimensions.unique_rotations().iter().any(|rotation| {
        instance.containers().iter().any(|container| {
            rotation.width() <= container.dimensions().width()
                && rotation.length() <= container.dimensions().length()
                && rotation.height() <= container.dimensions().height()
        })
    })
}

const fn external_algorithm(strategy: BinPackingStrategy) -> ThreeDAlgorithm {
    match strategy {
        BinPackingStrategy::ExtremePoints => ThreeDAlgorithm::ExtremePoints,
        BinPackingStrategy::ExtremePointsContactPoint => ThreeDAlgorithm::ExtremePointsContactPoint,
        BinPackingStrategy::Auto => ThreeDAlgorithm::Auto,
    }
}

fn all_unplaced(
    instance: &PackingInstance,
    started_at: Instant,
) -> Result<SolverOutcome, SolverError> {
    let solution = Solution::new(
        Vec::new(),
        instance.items().iter().map(|item| item.id()).collect(),
    );
    validate_solution(instance, &solution).map_err(invalid_candidate)?;
    Ok(SolverOutcome::new(
        solution,
        SolverMetrics::new(0, 1, 0, started_at.elapsed()),
        OptimalityStatus::Heuristic,
    ))
}

fn invalid_candidate(errors: impl std::fmt::Display) -> SolverError {
    SolverError::new(format!(
        "bin-packing adapter produced an invalid candidate:\n{errors}"
    ))
}

fn to_u32(value: u64) -> Result<u32, SolverError> {
    u32::try_from(value)
        .map_err(|_| SolverError::new(format!("scaled dimension {value} does not fit u32")))
}

fn dimensions_from_scaled(width: u64, length: u64, height: u64) -> Result<Dimensions, SolverError> {
    Ok(Dimensions::new(
        Length::from_scaled_units(width)
            .ok_or_else(|| SolverError::new("bin-packing returned zero width"))?,
        Length::from_scaled_units(length)
            .ok_or_else(|| SolverError::new("bin-packing returned zero length"))?,
        Length::from_scaled_units(height)
            .ok_or_else(|| SolverError::new("bin-packing returned zero height"))?,
    ))
}

fn container_name(container_id: ContainerId) -> String {
    format!("{CONTAINER_PREFIX}{}", container_id.index())
}

fn item_name(item_id: ItemId) -> String {
    format!("{ITEM_PREFIX}{}", item_id.index())
}

fn parse_container_id(name: &str, instance: &PackingInstance) -> Result<ContainerId, SolverError> {
    let index = parse_index(name, CONTAINER_PREFIX, "container")?;
    instance
        .containers()
        .get(index)
        .map(|container| container.id())
        .ok_or_else(|| SolverError::new(format!("bin-packing returned unknown container {name:?}")))
}

fn parse_item_id(name: &str, instance: &PackingInstance) -> Result<ItemId, SolverError> {
    let index = parse_index(name, ITEM_PREFIX, "item")?;
    instance
        .items()
        .get(index)
        .map(|item| item.id())
        .ok_or_else(|| SolverError::new(format!("bin-packing returned unknown item {name:?}")))
}

fn parse_index(name: &str, prefix: &str, kind: &str) -> Result<usize, SolverError> {
    name.strip_prefix(prefix)
        .ok_or_else(|| SolverError::new(format!("bin-packing returned malformed {kind} {name:?}")))?
        .parse()
        .map_err(|_| SolverError::new(format!("bin-packing returned malformed {kind} {name:?}")))
}
