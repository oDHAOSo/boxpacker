use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::solution::Solution;
use crate::validate::PackingInstance;

pub mod constructive;
pub mod exact;
pub mod improve;
pub mod portfolio;

/// Backend-independent solver entry point used by the bake-off and portfolio.
pub trait SolverBackend: Send + Sync {
    fn name(&self) -> &str;

    fn solve(
        &self,
        instance: &PackingInstance,
        request: &SolveRequest,
    ) -> Result<SolverOutcome, SolverError>;
}

/// Cloneable cooperative-cancellation signal shared by a solve and its owner.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Reproducible effort controls supplied to a backend.
#[derive(Clone, Debug)]
pub struct SolveRequest {
    deadline: Deadline,
    seed: u64,
    threads: NonZeroUsize,
    cancellation: CancellationToken,
}

impl SolveRequest {
    #[must_use]
    pub fn new(time_limit: Duration, seed: u64, threads: NonZeroUsize) -> Self {
        Self::with_cancellation(time_limit, seed, threads, CancellationToken::new())
    }

    #[must_use]
    pub fn with_cancellation(
        time_limit: Duration,
        seed: u64,
        threads: NonZeroUsize,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            deadline: Deadline::new(time_limit),
            seed,
            threads,
            cancellation,
        }
    }

    #[must_use]
    pub const fn deadline(&self) -> &Deadline {
        &self.deadline
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub const fn threads(&self) -> NonZeroUsize {
        self.threads
    }

    #[must_use]
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    #[must_use]
    pub fn should_stop(&self) -> bool {
        self.cancellation.is_cancelled() || self.deadline.is_expired()
    }
}

/// Monotonic wall-clock budget. Geometry correctness must not depend on it.
#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    started_at: Instant,
    time_limit: Duration,
}

impl Deadline {
    #[must_use]
    pub fn new(time_limit: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            time_limit,
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed_at(Instant::now())
    }

    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.remaining_at(Instant::now())
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    fn elapsed_at(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }

    fn remaining_at(&self, now: Instant) -> Duration {
        self.time_limit.saturating_sub(self.elapsed_at(now))
    }

    fn is_expired_at(&self, now: Instant) -> bool {
        self.elapsed_at(now) >= self.time_limit
    }
}

/// Honest status of the returned incumbent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimalityStatus {
    Heuristic,
    BoundMatched,
    ProvenOptimal,
}

/// Common metrics used to compare bake-off backends.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SolverMetrics {
    explored_candidates: u64,
    validated_candidates: u64,
    improvements: u64,
    elapsed: Duration,
}

impl SolverMetrics {
    #[must_use]
    pub const fn new(
        explored_candidates: u64,
        validated_candidates: u64,
        improvements: u64,
        elapsed: Duration,
    ) -> Self {
        Self {
            explored_candidates,
            validated_candidates,
            improvements,
            elapsed,
        }
    }

    #[must_use]
    pub const fn explored_candidates(self) -> u64 {
        self.explored_candidates
    }

    #[must_use]
    pub const fn validated_candidates(self) -> u64 {
        self.validated_candidates
    }

    #[must_use]
    pub const fn improvements(self) -> u64 {
        self.improvements
    }

    #[must_use]
    pub const fn elapsed(self) -> Duration {
        self.elapsed
    }
}

/// Backend result; its solution remains untrusted until independently checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverOutcome {
    solution: Solution,
    metrics: SolverMetrics,
    optimality: OptimalityStatus,
}

impl SolverOutcome {
    #[must_use]
    pub const fn new(
        solution: Solution,
        metrics: SolverMetrics,
        optimality: OptimalityStatus,
    ) -> Self {
        Self {
            solution,
            metrics,
            optimality,
        }
    }

    #[must_use]
    pub const fn solution(&self) -> &Solution {
        &self.solution
    }

    #[must_use]
    pub const fn metrics(&self) -> SolverMetrics {
        self.metrics
    }

    #[must_use]
    pub const fn optimality(&self) -> OptimalityStatus {
        self.optimality
    }
}

/// Backend failure that does not masquerade as a packing result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverError(String);

impl SolverError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for SolverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SolverError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_uses_a_monotonic_bounded_budget() {
        let started_at = Instant::now();
        let deadline = Deadline {
            started_at,
            time_limit: Duration::from_secs(5),
        };

        assert_eq!(
            deadline.remaining_at(started_at + Duration::from_secs(2)),
            Duration::from_secs(3)
        );
        assert!(!deadline.is_expired_at(started_at + Duration::from_millis(4_999)));
        assert!(deadline.is_expired_at(started_at + Duration::from_secs(5)));
        assert_eq!(
            deadline.remaining_at(started_at + Duration::from_secs(8)),
            Duration::ZERO
        );
    }

    #[test]
    fn cancellation_is_shared_across_token_clones_and_requests() {
        let cancellation = CancellationToken::new();
        let request = SolveRequest::with_cancellation(
            Duration::from_secs(1),
            0,
            NonZeroUsize::new(1).expect("one is non-zero"),
            cancellation.clone(),
        );

        assert!(!request.should_stop());
        cancellation.cancel();
        assert!(request.should_stop());
        assert!(request.cancellation().is_cancelled());
    }
}
