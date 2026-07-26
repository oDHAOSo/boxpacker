use std::cmp::{Ordering, Reverse};
use std::collections::BTreeSet;
use std::time::Instant;

use crate::geometry::{Aabb, Coordinate, Dimensions, Point};
use crate::solution::{Placement, Solution};
use crate::solver::{
    OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics, SolverOutcome,
};
use crate::validate::{ContainerId, ItemId, PackingInstance, validate_solution};

/// Clean-room maximal-space constructive baseline.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConstructiveBackend;

impl SolverBackend for ConstructiveBackend {
    fn name(&self) -> &str {
        "constructive-maximal-space"
    }

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError> {
        let started_at = Instant::now();
        let mut states = instance
            .containers()
            .iter()
            .map(|container| ContainerState::new(container.id(), container.dimensions()))
            .collect::<Vec<_>>();
        let mut item_ids = instance
            .items()
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>();
        item_ids.sort_unstable_by(|left, right| compare_items(instance, *left, *right));

        let mut explored_candidates = 0_u64;
        let mut placements = Vec::new();
        let mut unplaced_items = Vec::new();

        for (position, item_id) in item_ids.iter().copied().enumerate() {
            if request.deadline().is_expired() {
                unplaced_items.extend(item_ids[position..].iter().copied());
                break;
            }

            let dimensions = instance.items()[item_id.index()].dimensions();
            let candidate = find_best_candidate(
                instance,
                &states,
                item_id,
                dimensions,
                &mut explored_candidates,
                request,
            )?;
            if let Some(candidate) = candidate {
                let placement = Placement::new(candidate.container_id, item_id, candidate.bounds);
                states[candidate.container_id.index()].place(placement)?;
                placements.push(placement);
            } else {
                unplaced_items.push(item_id);
            }
        }

        let improvement_count = u64::try_from(placements.len())
            .map_err(|_| SolverError::new("placement count does not fit solver metrics"))?;
        let solution = Solution::new(placements, unplaced_items);
        validate_solution(instance, &solution).map_err(|errors| {
            SolverError::new(format!(
                "constructive backend produced an invalid candidate:\n{errors}"
            ))
        })?;
        let elapsed = started_at.elapsed();
        Ok(SolverOutcome::new(
            solution,
            SolverMetrics::new(explored_candidates, 1, improvement_count, elapsed),
            OptimalityStatus::Heuristic,
        ))
    }
}

fn compare_items(instance: &PackingInstance, left: ItemId, right: ItemId) -> Ordering {
    let left_item = &instance.items()[left.index()];
    let right_item = &instance.items()[right.index()];
    let left_dimensions = left_item.dimensions();
    let right_dimensions = right_item.dimensions();
    let mut left_sides = [
        left_dimensions.width().get(),
        left_dimensions.length().get(),
        left_dimensions.height().get(),
    ];
    let mut right_sides = [
        right_dimensions.width().get(),
        right_dimensions.length().get(),
        right_dimensions.height().get(),
    ];
    left_sides.sort_unstable_by(|a, b| b.cmp(a));
    right_sides.sort_unstable_by(|a, b| b.cmp(a));

    right_dimensions
        .checked_volume()
        .cmp(&left_dimensions.checked_volume())
        .then_with(|| right_sides.cmp(&left_sides))
        .then_with(|| left_item.name().cmp(right_item.name()))
        .then_with(|| left.cmp(&right))
}

fn find_best_candidate(
    instance: &PackingInstance,
    states: &[ContainerState],
    item_id: ItemId,
    dimensions: Dimensions,
    explored_candidates: &mut u64,
    request: &SolveRequest,
) -> Result<Option<Candidate>, SolverError> {
    let mut best = None;
    let rotations = dimensions.unique_rotations();

    for state in states {
        let mut seen = BTreeSet::new();
        for space in &state.spaces {
            for rotation in &rotations {
                if request.deadline().is_expired() {
                    return Ok(best);
                }
                *explored_candidates = explored_candidates
                    .checked_add(1)
                    .ok_or_else(|| SolverError::new("explored-candidate metric overflowed"))?;
                if !space.fits(*rotation) {
                    continue;
                }

                let signature = (
                    space.x_min,
                    space.y_min,
                    space.z_min,
                    rotation.width().get(),
                    rotation.length().get(),
                    rotation.height().get(),
                );
                if !seen.insert(signature) {
                    continue;
                }
                let bounds = space.bounds_at_origin(*rotation);
                if state
                    .placements
                    .iter()
                    .any(|placement| bounds_overlap(bounds, placement.bounds()))
                {
                    continue;
                }

                let score = candidate_score(instance, state, item_id, bounds, *space)?;
                let candidate = Candidate {
                    container_id: state.container_id,
                    bounds,
                    score,
                };
                if best
                    .as_ref()
                    .is_none_or(|current: &Candidate| candidate.score < current.score)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    Ok(best)
}

fn candidate_score(
    instance: &PackingInstance,
    state: &ContainerState,
    item_id: ItemId,
    bounds: Aabb,
    source_space: FreeSpace,
) -> Result<CandidateScore, SolverError> {
    let extents = Extents::from_bounds(bounds)?;
    let dimensions = bounds.dimensions();
    let item_volume = dimensions
        .checked_volume()
        .ok_or_else(|| SolverError::new("candidate item volume overflowed"))?;
    let source_volume = source_space.checked_volume()?;
    let unsupported_area = unsupported_area(state, extents, dimensions)?;
    let contact_area = contact_area(state, extents)?;
    let bounding_volume = u128::from(state.x_max.max(extents.x_max))
        .checked_mul(u128::from(state.y_max.max(extents.y_max)))
        .and_then(|area| area.checked_mul(u128::from(state.z_max.max(extents.z_max))))
        .ok_or_else(|| SolverError::new("candidate bounding volume overflowed"))?;
    let container = &instance.containers()[state.container_id.index()];
    let container_volume = container
        .dimensions()
        .checked_volume()
        .expect("validated container volume must be representable");
    let fragmentation = state
        .spaces
        .iter()
        .filter(|space| space.intersects(extents))
        .count();

    Ok(CandidateScore {
        opens_container: state.placements.is_empty(),
        unsupported_area,
        contact_area: Reverse(contact_area),
        bounding_volume,
        residual_space_volume: source_volume - item_volume,
        fragmentation,
        container_volume,
        z: extents.z_min,
        y: extents.y_min,
        x: extents.x_min,
        container_dimensions: [
            container.dimensions().width().get(),
            container.dimensions().length().get(),
            container.dimensions().height().get(),
        ],
        container_name: container.name().to_owned(),
        item_name: instance.items()[item_id.index()].name().to_owned(),
        rotation: [
            dimensions.width().get(),
            dimensions.length().get(),
            dimensions.height().get(),
        ],
        container_id: state.container_id,
    })
}

fn unsupported_area(
    state: &ContainerState,
    candidate: Extents,
    dimensions: Dimensions,
) -> Result<u128, SolverError> {
    let bottom_area = u128::from(dimensions.width().get())
        .checked_mul(u128::from(dimensions.length().get()))
        .ok_or_else(|| SolverError::new("candidate support area overflowed"))?;
    if candidate.z_min == 0 {
        return Ok(0);
    }

    let mut supported_area = 0_u128;
    for placement in &state.placements {
        let support = Extents::from_bounds(placement.bounds())?;
        if support.z_max != candidate.z_min {
            continue;
        }
        let overlap_area = u128::from(interval_overlap(
            candidate.x_min,
            candidate.x_max,
            support.x_min,
            support.x_max,
        ))
        .checked_mul(u128::from(interval_overlap(
            candidate.y_min,
            candidate.y_max,
            support.y_min,
            support.y_max,
        )))
        .ok_or_else(|| SolverError::new("candidate support overlap overflowed"))?;
        supported_area = supported_area
            .checked_add(overlap_area)
            .ok_or_else(|| SolverError::new("candidate support total overflowed"))?;
    }
    Ok(bottom_area - supported_area.min(bottom_area))
}

fn contact_area(state: &ContainerState, candidate: Extents) -> Result<u128, SolverError> {
    let mut contact = 0_u128;
    let width = candidate.x_max - candidate.x_min;
    let length = candidate.y_max - candidate.y_min;
    let height = candidate.z_max - candidate.z_min;
    let yz = checked_area(length, height)?;
    let xz = checked_area(width, height)?;
    let xy = checked_area(width, length)?;

    if candidate.x_min == 0 {
        contact = checked_add(contact, yz)?;
    }
    if candidate.x_max == state.dimensions.width().get() {
        contact = checked_add(contact, yz)?;
    }
    if candidate.y_min == 0 {
        contact = checked_add(contact, xz)?;
    }
    if candidate.y_max == state.dimensions.length().get() {
        contact = checked_add(contact, xz)?;
    }
    if candidate.z_min == 0 {
        contact = checked_add(contact, xy)?;
    }
    if candidate.z_max == state.dimensions.height().get() {
        contact = checked_add(contact, xy)?;
    }

    for placement in &state.placements {
        let other = Extents::from_bounds(placement.bounds())?;
        if candidate.x_min == other.x_max || candidate.x_max == other.x_min {
            contact = checked_add(
                contact,
                checked_area(
                    interval_overlap(candidate.y_min, candidate.y_max, other.y_min, other.y_max),
                    interval_overlap(candidate.z_min, candidate.z_max, other.z_min, other.z_max),
                )?,
            )?;
        }
        if candidate.y_min == other.y_max || candidate.y_max == other.y_min {
            contact = checked_add(
                contact,
                checked_area(
                    interval_overlap(candidate.x_min, candidate.x_max, other.x_min, other.x_max),
                    interval_overlap(candidate.z_min, candidate.z_max, other.z_min, other.z_max),
                )?,
            )?;
        }
        if candidate.z_min == other.z_max || candidate.z_max == other.z_min {
            contact = checked_add(
                contact,
                checked_area(
                    interval_overlap(candidate.x_min, candidate.x_max, other.x_min, other.x_max),
                    interval_overlap(candidate.y_min, candidate.y_max, other.y_min, other.y_max),
                )?,
            )?;
        }
    }
    Ok(contact)
}

fn checked_area(first: u64, second: u64) -> Result<u128, SolverError> {
    u128::from(first)
        .checked_mul(u128::from(second))
        .ok_or_else(|| SolverError::new("candidate contact area overflowed"))
}

fn checked_add(left: u128, right: u128) -> Result<u128, SolverError> {
    left.checked_add(right)
        .ok_or_else(|| SolverError::new("candidate contact total overflowed"))
}

fn interval_overlap(first_min: u64, first_max: u64, second_min: u64, second_max: u64) -> u64 {
    first_max
        .min(second_max)
        .saturating_sub(first_min.max(second_min))
}

fn bounds_overlap(left: Aabb, right: Aabb) -> bool {
    match (Extents::from_bounds(left), Extents::from_bounds(right)) {
        (Ok(left), Ok(right)) => left.overlaps(right),
        _ => true,
    }
}

#[derive(Clone, Debug)]
struct Candidate {
    container_id: ContainerId,
    bounds: Aabb,
    score: CandidateScore,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CandidateScore {
    opens_container: bool,
    unsupported_area: u128,
    contact_area: Reverse<u128>,
    bounding_volume: u128,
    residual_space_volume: u128,
    fragmentation: usize,
    container_volume: u128,
    z: u64,
    y: u64,
    x: u64,
    container_dimensions: [u64; 3],
    container_name: String,
    item_name: String,
    rotation: [u64; 3],
    container_id: ContainerId,
}

#[derive(Clone, Debug)]
struct ContainerState {
    container_id: ContainerId,
    dimensions: Dimensions,
    spaces: Vec<FreeSpace>,
    placements: Vec<Placement>,
    x_max: u64,
    y_max: u64,
    z_max: u64,
}

impl ContainerState {
    fn new(container_id: ContainerId, dimensions: Dimensions) -> Self {
        Self {
            container_id,
            dimensions,
            spaces: vec![FreeSpace::container(dimensions)],
            placements: Vec::new(),
            x_max: 0,
            y_max: 0,
            z_max: 0,
        }
    }

    fn place(&mut self, placement: Placement) -> Result<(), SolverError> {
        let placed = Extents::from_bounds(placement.bounds())?;
        let mut next_spaces = Vec::new();
        for space in self.spaces.drain(..) {
            if space.intersects(placed) {
                next_spaces.extend(space.split_around(placed));
            } else {
                next_spaces.push(space);
            }
        }
        prune_contained_spaces(&mut next_spaces);
        self.spaces = next_spaces;
        self.x_max = self.x_max.max(placed.x_max);
        self.y_max = self.y_max.max(placed.y_max);
        self.z_max = self.z_max.max(placed.z_max);
        self.placements.push(placement);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FreeSpace {
    x_min: u64,
    x_max: u64,
    y_min: u64,
    y_max: u64,
    z_min: u64,
    z_max: u64,
}

impl FreeSpace {
    fn container(dimensions: Dimensions) -> Self {
        Self {
            x_min: 0,
            x_max: dimensions.width().get(),
            y_min: 0,
            y_max: dimensions.length().get(),
            z_min: 0,
            z_max: dimensions.height().get(),
        }
    }

    const fn fits(self, dimensions: Dimensions) -> bool {
        dimensions.width().get() <= self.x_max - self.x_min
            && dimensions.length().get() <= self.y_max - self.y_min
            && dimensions.height().get() <= self.z_max - self.z_min
    }

    fn bounds_at_origin(self, dimensions: Dimensions) -> Aabb {
        Aabb::new(
            Point::new(
                Coordinate::from_scaled_units(self.x_min),
                Coordinate::from_scaled_units(self.y_min),
                Coordinate::from_scaled_units(self.z_min),
            ),
            dimensions,
        )
    }

    fn checked_volume(self) -> Result<u128, SolverError> {
        u128::from(self.x_max - self.x_min)
            .checked_mul(u128::from(self.y_max - self.y_min))
            .and_then(|area| area.checked_mul(u128::from(self.z_max - self.z_min)))
            .ok_or_else(|| SolverError::new("maximal free-space volume overflowed"))
    }

    const fn intersects(self, other: Extents) -> bool {
        self.x_min < other.x_max
            && other.x_min < self.x_max
            && self.y_min < other.y_max
            && other.y_min < self.y_max
            && self.z_min < other.z_max
            && other.z_min < self.z_max
    }

    const fn contains(self, other: Self) -> bool {
        self.x_min <= other.x_min
            && self.x_max >= other.x_max
            && self.y_min <= other.y_min
            && self.y_max >= other.y_max
            && self.z_min <= other.z_min
            && self.z_max >= other.z_max
    }

    fn split_around(self, placed: Extents) -> Vec<Self> {
        let intersection = Extents {
            x_min: self.x_min.max(placed.x_min),
            x_max: self.x_max.min(placed.x_max),
            y_min: self.y_min.max(placed.y_min),
            y_max: self.y_max.min(placed.y_max),
            z_min: self.z_min.max(placed.z_min),
            z_max: self.z_max.min(placed.z_max),
        };
        let candidates = [
            Self {
                x_max: intersection.x_min,
                ..self
            },
            Self {
                x_min: intersection.x_max,
                ..self
            },
            Self {
                y_max: intersection.y_min,
                ..self
            },
            Self {
                y_min: intersection.y_max,
                ..self
            },
            Self {
                z_max: intersection.z_min,
                ..self
            },
            Self {
                z_min: intersection.z_max,
                ..self
            },
        ];
        candidates
            .into_iter()
            .filter(|space| {
                space.x_min < space.x_max && space.y_min < space.y_max && space.z_min < space.z_max
            })
            .collect()
    }
}

fn prune_contained_spaces(spaces: &mut Vec<FreeSpace>) {
    spaces.sort_unstable();
    spaces.dedup();
    let retained = spaces
        .iter()
        .enumerate()
        .filter_map(|(index, space)| {
            let contained = spaces
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != index && other.contains(*space));
            (!contained).then_some(*space)
        })
        .collect();
    *spaces = retained;
}

#[derive(Clone, Copy, Debug)]
struct Extents {
    x_min: u64,
    x_max: u64,
    y_min: u64,
    y_max: u64,
    z_min: u64,
    z_max: u64,
}

impl Extents {
    fn from_bounds(bounds: Aabb) -> Result<Self, SolverError> {
        let origin = bounds.origin();
        let dimensions = bounds.dimensions();
        Ok(Self {
            x_min: origin.x().get(),
            x_max: origin
                .x()
                .checked_add(dimensions.width())
                .ok_or_else(|| SolverError::new("candidate x-extent overflowed"))?,
            y_min: origin.y().get(),
            y_max: origin
                .y()
                .checked_add(dimensions.length())
                .ok_or_else(|| SolverError::new("candidate y-extent overflowed"))?,
            z_min: origin.z().get(),
            z_max: origin
                .z()
                .checked_add(dimensions.height())
                .ok_or_else(|| SolverError::new("candidate z-extent overflowed"))?,
        })
    }

    const fn overlaps(self, other: Self) -> bool {
        self.x_min < other.x_max
            && other.x_min < self.x_max
            && self.y_min < other.y_max
            && other.y_min < self.y_max
            && self.z_min < other.z_max
            && other.z_min < self.z_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Length;

    fn dimensions(width: u64, length: u64, height: u64) -> Dimensions {
        Dimensions::new(
            Length::from_scaled_units(width).expect("positive test width"),
            Length::from_scaled_units(length).expect("positive test length"),
            Length::from_scaled_units(height).expect("positive test height"),
        )
    }

    #[test]
    fn maximal_space_split_excludes_the_placed_volume() {
        let space = FreeSpace::container(dimensions(100, 80, 60));
        let placed = Extents {
            x_min: 20,
            x_max: 50,
            y_min: 10,
            y_max: 40,
            z_min: 5,
            z_max: 25,
        };

        let residuals = space.split_around(placed);

        assert_eq!(residuals.len(), 6);
        assert!(residuals.iter().all(|residual| space.contains(*residual)));
        assert!(
            residuals
                .iter()
                .all(|residual| !residual.intersects(placed))
        );
    }

    #[test]
    fn dominated_maximal_spaces_are_removed_deterministically() {
        let outer = FreeSpace {
            x_min: 0,
            x_max: 100,
            y_min: 0,
            y_max: 100,
            z_min: 0,
            z_max: 100,
        };
        let inner = FreeSpace {
            x_min: 10,
            x_max: 20,
            y_min: 10,
            y_max: 20,
            z_min: 10,
            z_max: 20,
        };
        let mut spaces = vec![inner, outer, inner];

        prune_contained_spaces(&mut spaces);

        assert_eq!(spaces, vec![outer]);
    }
}
