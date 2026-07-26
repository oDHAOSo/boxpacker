use std::num::NonZeroUsize;
use std::thread;
use std::time::Instant;

use crate::objective::ObjectiveValue;
use crate::solution::Solution;
use crate::solver::constructive::{ItemOrder, solve_with_item_order};
use crate::solver::{
    OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics, SolverOutcome,
};
use crate::validate::{PackingInstance, validate_solution};

/// Deterministic fixed-work portfolio over clean-room constructive orders.
#[derive(Clone, Copy, Debug)]
pub struct PortfolioBackend {
    work_units: NonZeroUsize,
}

impl PortfolioBackend {
    #[must_use]
    pub const fn new(work_units: NonZeroUsize) -> Self {
        Self { work_units }
    }

    #[must_use]
    pub const fn work_units(self) -> NonZeroUsize {
        self.work_units
    }
}

impl Default for PortfolioBackend {
    fn default() -> Self {
        Self::new(NonZeroUsize::new(8).expect("eight is non-zero"))
    }
}

impl SolverBackend for PortfolioBackend {
    fn name(&self) -> &str {
        "deterministic-constructive-portfolio"
    }

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError> {
        let started_at = Instant::now();
        let plan = WorkPlan::new(request.seed(), self.work_units, request.threads());
        let mut candidates = execute_plan(instance, request, plan)?;
        candidates.sort_unstable_by_key(|candidate| candidate.work_index);

        let mut explored_candidates = 0_u64;
        let mut validated_candidates = 0_u64;
        let mut improvements = 0_u64;
        let mut best: Option<(ObjectiveValue, Solution)> = None;

        for candidate in candidates {
            explored_candidates = checked_metric_add(
                explored_candidates,
                candidate.outcome.metrics().explored_candidates(),
                "explored-candidate",
            )?;
            validated_candidates =
                checked_metric_add(validated_candidates, 1, "validated-candidate")?;
            let summary =
                validate_solution(instance, candidate.outcome.solution()).map_err(|errors| {
                    SolverError::new(format!(
                        "portfolio work unit {} produced an invalid candidate:\n{errors}",
                        candidate.work_index
                    ))
                })?;
            let objective = ObjectiveValue::from_summary(&summary);
            if best
                .as_ref()
                .is_none_or(|(current, _)| objective > *current)
            {
                improvements = checked_metric_add(improvements, 1, "improvement")?;
                best = Some((objective, candidate.outcome.solution().clone()));
            }
        }

        let (_, solution) =
            best.ok_or_else(|| SolverError::new("portfolio produced no candidate"))?;
        Ok(SolverOutcome::new(
            solution,
            SolverMetrics::new(
                explored_candidates,
                validated_candidates,
                improvements,
                started_at.elapsed(),
            ),
            OptimalityStatus::Heuristic,
        ))
    }
}

fn checked_metric_add(current: u64, addition: u64, name: &str) -> Result<u64, SolverError> {
    current
        .checked_add(addition)
        .ok_or_else(|| SolverError::new(format!("{name} metric overflowed")))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorkUnit {
    index: usize,
    item_order: ItemOrder,
}

#[derive(Debug, Eq, PartialEq)]
struct WorkPlan {
    partitions: Vec<Vec<WorkUnit>>,
}

impl WorkPlan {
    fn new(seed: u64, work_units: NonZeroUsize, threads: NonZeroUsize) -> Self {
        let worker_count = threads.get().min(work_units.get());
        let mut partitions = vec![Vec::new(); worker_count];

        for index in 0..work_units.get() {
            let item_order = if index == 0 {
                ItemOrder::Canonical
            } else {
                ItemOrder::Seeded(derive_seed(seed, index))
            };
            partitions[index % worker_count].push(WorkUnit { index, item_order });
        }

        Self { partitions }
    }
}

fn derive_seed(seed: u64, work_index: usize) -> u64 {
    let index = u64::try_from(work_index).expect("work index must fit u64");
    let mut value = seed ^ index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Debug)]
struct Candidate {
    work_index: usize,
    outcome: SolverOutcome,
}

fn execute_plan(
    instance: &PackingInstance,
    request: &SolveRequest,
    plan: WorkPlan,
) -> Result<Vec<Candidate>, SolverError> {
    thread::scope(|scope| {
        let handles = plan
            .partitions
            .into_iter()
            .map(|partition| {
                scope.spawn(move || {
                    partition
                        .into_iter()
                        .map(|work| {
                            solve_with_item_order(instance, request, work.item_order).map(
                                |outcome| Candidate {
                                    work_index: work.index,
                                    outcome,
                                },
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
            })
            .collect::<Vec<_>>();

        let mut candidates = Vec::new();
        for handle in handles {
            let mut worker_candidates = handle
                .join()
                .map_err(|_| SolverError::new("portfolio worker panicked"))??;
            candidates.append(&mut worker_candidates);
        }
        Ok(candidates)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_plan_is_seeded_and_partitioned_round_robin() {
        let plan = WorkPlan::new(
            17,
            NonZeroUsize::new(7).expect("seven is non-zero"),
            NonZeroUsize::new(3).expect("three is non-zero"),
        );

        assert_eq!(
            plan.partitions
                .iter()
                .map(|partition| { partition.iter().map(|work| work.index).collect::<Vec<_>>() })
                .collect::<Vec<_>>(),
            vec![vec![0, 3, 6], vec![1, 4], vec![2, 5]]
        );
        assert_eq!(plan.partitions[0][0].item_order, ItemOrder::Canonical);
        assert_eq!(
            plan.partitions[1][0].item_order,
            ItemOrder::Seeded(derive_seed(17, 1))
        );
    }

    #[test]
    fn fixed_seed_defines_work_independently_of_thread_count() {
        let work_units = NonZeroUsize::new(8).expect("eight is non-zero");
        let serial = WorkPlan::new(
            41,
            work_units,
            NonZeroUsize::new(1).expect("one is non-zero"),
        );
        let parallel = WorkPlan::new(
            41,
            work_units,
            NonZeroUsize::new(4).expect("four is non-zero"),
        );

        let mut serial_work = serial.partitions.into_iter().flatten().collect::<Vec<_>>();
        let mut parallel_work = parallel
            .partitions
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        serial_work.sort_unstable_by_key(|work| work.index);
        parallel_work.sort_unstable_by_key(|work| work.index);

        assert_eq!(serial_work, parallel_work);
        assert_ne!(derive_seed(41, 1), derive_seed(42, 1));
    }
}
