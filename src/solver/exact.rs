use crate::objective::ObjectiveValue;
use crate::solution::{Placement, Solution};
use crate::solver::constructive::event_placements;
use crate::solver::{SolveRequest, SolverError};
use crate::validate::{ItemId, PackingInstance, validate_solution};

/// Bounds for exact-event repair of one validated incumbent's residual items.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepairConfig {
    max_items: usize,
    max_nodes: u64,
}

impl RepairConfig {
    #[must_use]
    pub const fn new(max_items: usize, max_nodes: u64) -> Self {
        Self {
            max_items,
            max_nodes,
        }
    }

    #[must_use]
    pub const fn max_items(self) -> usize {
        self.max_items
    }

    #[must_use]
    pub const fn max_nodes(self) -> u64 {
        self.max_nodes
    }
}

impl Default for RepairConfig {
    fn default() -> Self {
        Self::new(6, 512)
    }
}

/// Best independently valid incumbent found by a bounded residual repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairOutcome {
    solution: Solution,
    explored_nodes: u64,
    explored_candidates: u64,
    exhaustive: bool,
}

impl RepairOutcome {
    #[must_use]
    pub const fn solution(&self) -> &Solution {
        &self.solution
    }

    #[must_use]
    pub const fn explored_nodes(&self) -> u64 {
        self.explored_nodes
    }

    #[must_use]
    pub const fn explored_candidates(&self) -> u64 {
        self.explored_candidates
    }

    /// True only for the frozen-placement event subproblem, never the global packing problem.
    #[must_use]
    pub const fn exhaustive(&self) -> bool {
        self.exhaustive
    }
}

pub fn repair_residual(
    instance: &PackingInstance,
    incumbent: &Solution,
    request: &SolveRequest,
    config: RepairConfig,
) -> Result<Option<RepairOutcome>, SolverError> {
    let incumbent_summary = validate_solution(instance, incumbent).map_err(|errors| {
        SolverError::new(format!(
            "exact repair requires a valid incumbent:\n{errors}"
        ))
    })?;
    let mut residual = incumbent.unplaced_items().to_vec();
    if residual.is_empty() || residual.len() > config.max_items || config.max_nodes == 0 {
        return Ok(None);
    }
    residual.sort_unstable_by(|left, right| compare_residual_items(instance, *left, *right));

    let mut search = Search {
        instance,
        request,
        config,
        residual,
        placements: incumbent.placements().to_vec(),
        unplaced: Vec::new(),
        best_solution: incumbent.clone(),
        best_objective: ObjectiveValue::from_summary(&incumbent_summary),
        best_added_volume: 0,
        best_added_count: 0,
        explored_nodes: 0,
        explored_candidates: 0,
        exhaustive: true,
    };
    search.visit(0, 0, 0)?;

    Ok(Some(RepairOutcome {
        solution: search.best_solution,
        explored_nodes: search.explored_nodes,
        explored_candidates: search.explored_candidates,
        exhaustive: search.exhaustive,
    }))
}

fn compare_residual_items(
    instance: &PackingInstance,
    left: ItemId,
    right: ItemId,
) -> std::cmp::Ordering {
    let left_item = &instance.items()[left.index()];
    let right_item = &instance.items()[right.index()];
    right_item
        .dimensions()
        .checked_volume()
        .cmp(&left_item.dimensions().checked_volume())
        .then_with(|| left_item.name().cmp(right_item.name()))
        .then_with(|| left.cmp(&right))
}

struct Search<'search> {
    instance: &'search PackingInstance,
    request: &'search SolveRequest,
    config: RepairConfig,
    residual: Vec<ItemId>,
    placements: Vec<Placement>,
    unplaced: Vec<ItemId>,
    best_solution: Solution,
    best_objective: ObjectiveValue,
    best_added_volume: u128,
    best_added_count: usize,
    explored_nodes: u64,
    explored_candidates: u64,
    exhaustive: bool,
}

impl Search<'_> {
    fn visit(
        &mut self,
        depth: usize,
        added_volume: u128,
        added_count: usize,
    ) -> Result<(), SolverError> {
        if self.request.should_stop() || self.explored_nodes >= self.config.max_nodes {
            self.exhaustive = false;
            return Ok(());
        }
        self.explored_nodes = self
            .explored_nodes
            .checked_add(1)
            .ok_or_else(|| SolverError::new("repair node metric overflowed"))?;

        let remaining = &self.residual[depth..];
        let remaining_volume = remaining.iter().try_fold(0_u128, |total, item_id| {
            total
                .checked_add(
                    self.instance.items()[item_id.index()]
                        .dimensions()
                        .checked_volume()
                        .expect("validated item volume must be representable"),
                )
                .ok_or_else(|| SolverError::new("repair volume bound overflowed"))
        })?;
        let volume_bound = added_volume
            .checked_add(remaining_volume)
            .ok_or_else(|| SolverError::new("repair volume bound overflowed"))?;
        if volume_bound < self.best_added_volume
            || (volume_bound == self.best_added_volume
                && added_count + remaining.len() < self.best_added_count)
        {
            return Ok(());
        }

        if depth == self.residual.len() {
            return self.consider_leaf(added_volume, added_count);
        }

        let item_id = self.residual[depth];
        let item_volume = self.instance.items()[item_id.index()]
            .dimensions()
            .checked_volume()
            .expect("validated item volume must be representable");
        let candidates = event_placements(
            self.instance,
            &self.placements,
            item_id,
            self.request,
            &mut self.explored_candidates,
        )?;
        for placement in candidates {
            if self.request.should_stop() || self.explored_nodes >= self.config.max_nodes {
                self.exhaustive = false;
                break;
            }
            self.placements.push(placement);
            self.visit(
                depth + 1,
                added_volume
                    .checked_add(item_volume)
                    .ok_or_else(|| SolverError::new("repair placed volume overflowed"))?,
                added_count + 1,
            )?;
            self.placements.pop();
        }

        self.unplaced.push(item_id);
        self.visit(depth + 1, added_volume, added_count)?;
        self.unplaced.pop();
        Ok(())
    }

    fn consider_leaf(&mut self, added_volume: u128, added_count: usize) -> Result<(), SolverError> {
        let solution = Solution::new(self.placements.clone(), self.unplaced.clone());
        let summary = validate_solution(self.instance, &solution).map_err(|errors| {
            SolverError::new(format!("exact repair generated an invalid leaf:\n{errors}"))
        })?;
        let objective = ObjectiveValue::from_summary(&summary);
        if objective > self.best_objective {
            self.best_solution = solution;
            self.best_objective = objective;
            self.best_added_volume = added_volume;
            self.best_added_count = added_count;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use crate::model::{InputContainer, InputData, Item};
    use crate::validate::validate_solution;

    use super::*;

    fn instance() -> PackingInstance {
        let input = InputData {
            containers: vec![InputContainer {
                name: "container".to_owned(),
                width: 4.0,
                length: 2.0,
                height: 1.0,
            }],
            contents: vec![
                Item {
                    name: "left".to_owned(),
                    width: 2.0,
                    length: 2.0,
                    height: 1.0,
                },
                Item {
                    name: "right".to_owned(),
                    width: 2.0,
                    length: 2.0,
                    height: 1.0,
                },
            ],
        };
        PackingInstance::try_from(&input).expect("repair fixture should validate")
    }

    fn request() -> SolveRequest {
        SolveRequest::new(
            Duration::from_secs(1),
            0,
            NonZeroUsize::new(1).expect("one is non-zero"),
        )
    }

    fn all_unplaced(instance: &PackingInstance) -> Solution {
        Solution::new(
            Vec::new(),
            instance.items().iter().map(|item| item.id()).collect(),
        )
    }

    #[test]
    fn exhaustive_event_repair_places_a_small_residual() {
        let instance = instance();
        let outcome = repair_residual(
            &instance,
            &all_unplaced(&instance),
            &request(),
            RepairConfig::new(4, 1_000),
        )
        .expect("repair should solve")
        .expect("small residual should be attempted");
        let summary = validate_solution(&instance, outcome.solution())
            .expect("repaired solution should validate");

        assert_eq!(summary.placed_item_count(), 2);
        assert_eq!(summary.unplaced_item_count(), 0);
        assert!(outcome.exhaustive());
        assert!(outcome.explored_nodes() > 0);
        assert!(outcome.explored_candidates() > 0);
    }

    #[test]
    fn node_bound_returns_the_original_valid_incumbent_without_overclaiming() {
        let instance = instance();
        let incumbent = all_unplaced(&instance);
        let outcome = repair_residual(&instance, &incumbent, &request(), RepairConfig::new(4, 1))
            .expect("bounded repair should return")
            .expect("small residual should be attempted");

        assert_eq!(outcome.solution(), &incumbent);
        assert!(!outcome.exhaustive());
        assert_eq!(outcome.explored_nodes(), 1);
    }

    #[test]
    fn residual_over_item_bound_is_skipped() {
        let instance = instance();

        assert!(
            repair_residual(
                &instance,
                &all_unplaced(&instance),
                &request(),
                RepairConfig::new(1, 1_000),
            )
            .expect("bounded repair should not fail")
            .is_none()
        );
    }
}
