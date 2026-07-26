use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, MutexGuard};
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
        let incumbent = Arc::new(SharedIncumbent::new(instance));

        let canonical = solve_with_item_order(instance, request, plan.canonical.item_order)?;
        incumbent.publish(plan.canonical.index, canonical)?;
        execute_seeded_partitions(instance, request, plan.partitions, Arc::clone(&incumbent))?;

        let snapshot = incumbent.snapshot()?;
        Ok(SolverOutcome::new(
            snapshot.solution,
            SolverMetrics::new(
                snapshot.explored_candidates,
                snapshot.validated_candidates,
                snapshot.improvements,
                started_at.elapsed(),
            ),
            OptimalityStatus::Heuristic,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorkUnit {
    index: usize,
    item_order: ItemOrder,
}

#[derive(Debug, Eq, PartialEq)]
struct WorkPlan {
    canonical: WorkUnit,
    partitions: Vec<Vec<WorkUnit>>,
}

impl WorkPlan {
    fn new(seed: u64, work_units: NonZeroUsize, threads: NonZeroUsize) -> Self {
        let canonical = WorkUnit {
            index: 0,
            item_order: ItemOrder::Canonical,
        };
        let seeded_count = work_units.get().saturating_sub(1);
        let worker_count = threads.get().min(seeded_count);
        let mut partitions = vec![Vec::new(); worker_count];

        for index in 1..work_units.get() {
            let worker_index = (index - 1) % worker_count;
            partitions[worker_index].push(WorkUnit {
                index,
                item_order: ItemOrder::Seeded(derive_seed(seed, index)),
            });
        }

        Self {
            canonical,
            partitions,
        }
    }
}

fn derive_seed(seed: u64, work_index: usize) -> u64 {
    let index = u64::try_from(work_index).expect("work index must fit u64");
    let mut value = seed ^ index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn execute_seeded_partitions(
    instance: &PackingInstance,
    request: &SolveRequest,
    partitions: Vec<Vec<WorkUnit>>,
    incumbent: Arc<SharedIncumbent<'_>>,
) -> Result<(), SolverError> {
    thread::scope(|scope| {
        let handles = partitions
            .into_iter()
            .map(|partition| {
                let incumbent = Arc::clone(&incumbent);
                scope.spawn(move || {
                    for work in partition {
                        if request.should_stop() {
                            break;
                        }
                        let outcome = solve_with_item_order(instance, request, work.item_order)
                            .inspect_err(|_| {
                                request.cancellation().cancel();
                            })?;
                        incumbent.publish(work.index, outcome).inspect_err(|_| {
                            request.cancellation().cancel();
                        })?;
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();

        let mut first_error = None;
        for handle in handles {
            let worker_result = handle
                .join()
                .map_err(|_| SolverError::new("portfolio worker panicked"))
                .and_then(|result| result);
            if first_error.is_none()
                && let Err(error) = worker_result
            {
                request.cancellation().cancel();
                first_error = Some(error);
            }
        }

        first_error.map_or(Ok(()), Err)
    })
}

#[derive(Debug)]
struct ValidatedCandidate {
    objective: ObjectiveValue,
    solution: Solution,
    explored_candidates: u64,
}

#[derive(Debug, Default)]
struct IncumbentState {
    candidates: BTreeMap<usize, ValidatedCandidate>,
    best_work_index: Option<usize>,
}

#[derive(Debug)]
struct SharedIncumbent<'instance> {
    instance: &'instance PackingInstance,
    state: Mutex<IncumbentState>,
}

impl<'instance> SharedIncumbent<'instance> {
    const fn new(instance: &'instance PackingInstance) -> Self {
        Self {
            instance,
            state: Mutex::new(IncumbentState {
                candidates: BTreeMap::new(),
                best_work_index: None,
            }),
        }
    }

    fn publish(&self, work_index: usize, outcome: SolverOutcome) -> Result<(), SolverError> {
        let summary = validate_solution(self.instance, outcome.solution()).map_err(|errors| {
            SolverError::new(format!(
                "portfolio work unit {work_index} produced an invalid candidate:\n{errors}"
            ))
        })?;
        let candidate = ValidatedCandidate {
            objective: ObjectiveValue::from_summary(&summary),
            solution: outcome.solution().clone(),
            explored_candidates: outcome.metrics().explored_candidates(),
        };
        let mut state = self.lock()?;
        if state.candidates.contains_key(&work_index) {
            return Err(SolverError::new(format!(
                "portfolio work unit {work_index} published twice"
            )));
        }

        let replaces_best = state.best_work_index.is_none_or(|best_index| {
            let best = &state.candidates[&best_index];
            candidate.objective > best.objective
                || (candidate.objective == best.objective && work_index < best_index)
        });
        state.candidates.insert(work_index, candidate);
        if replaces_best {
            state.best_work_index = Some(work_index);
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<IncumbentSnapshot, SolverError> {
        let state = self.lock()?;
        let best_work_index = state
            .best_work_index
            .ok_or_else(|| SolverError::new("portfolio produced no validated incumbent"))?;
        let solution = state.candidates[&best_work_index].solution.clone();
        let mut explored_candidates = 0_u64;
        let mut improvements = 0_u64;
        let mut best: Option<&ObjectiveValue> = None;

        for candidate in state.candidates.values() {
            explored_candidates = checked_metric_add(
                explored_candidates,
                candidate.explored_candidates,
                "explored-candidate",
            )?;
            if best.is_none_or(|current| candidate.objective > *current) {
                improvements = checked_metric_add(improvements, 1, "improvement")?;
                best = Some(&candidate.objective);
            }
        }

        let validated_candidates = u64::try_from(state.candidates.len())
            .map_err(|_| SolverError::new("validated-candidate metric overflowed"))?;
        Ok(IncumbentSnapshot {
            solution,
            explored_candidates,
            validated_candidates,
            improvements,
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, IncumbentState>, SolverError> {
        self.state
            .lock()
            .map_err(|_| SolverError::new("shared incumbent lock was poisoned"))
    }
}

fn checked_metric_add(current: u64, addition: u64, name: &str) -> Result<u64, SolverError> {
    current
        .checked_add(addition)
        .ok_or_else(|| SolverError::new(format!("{name} metric overflowed")))
}

#[derive(Debug)]
struct IncumbentSnapshot {
    solution: Solution,
    explored_candidates: u64,
    validated_candidates: u64,
    improvements: u64,
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

        assert_eq!(plan.canonical.item_order, ItemOrder::Canonical);
        assert_eq!(
            plan.partitions
                .iter()
                .map(|partition| { partition.iter().map(|work| work.index).collect::<Vec<_>>() })
                .collect::<Vec<_>>(),
            vec![vec![1, 4], vec![2, 5], vec![3, 6]]
        );
        assert_eq!(
            plan.partitions[0][0].item_order,
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

    #[test]
    fn one_work_unit_needs_no_seeded_worker_partition() {
        let plan = WorkPlan::new(
            0,
            NonZeroUsize::new(1).expect("one is non-zero"),
            NonZeroUsize::new(8).expect("eight is non-zero"),
        );

        assert!(plan.partitions.is_empty());
        assert_eq!(plan.canonical.index, 0);
    }
}
