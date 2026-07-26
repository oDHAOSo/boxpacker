use std::cmp::Ordering;

use crate::validate::SolutionSummary;

/// Inspectable components of the provisional lexicographic objective.
///
/// `Ord` treats a better objective as greater, making `max` select the best
/// validated incumbent. The volume/count ordering is intentionally localized
/// here while D-004 awaits product confirmation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectiveValue {
    unplaced_volume: u128,
    unplaced_item_count: usize,
    used_container_count: usize,
    unsupported_area: u128,
    bounding_volume: u128,
    deterministic_key: Vec<u64>,
}

impl ObjectiveValue {
    #[must_use]
    pub fn from_summary(summary: &SolutionSummary) -> Self {
        Self {
            unplaced_volume: summary.unplaced_volume(),
            unplaced_item_count: summary.unplaced_item_count(),
            used_container_count: summary.used_container_count(),
            unsupported_area: summary.unsupported_area(),
            bounding_volume: summary.bounding_volume(),
            deterministic_key: summary.deterministic_key().to_vec(),
        }
    }

    #[must_use]
    pub const fn unplaced_volume(&self) -> u128 {
        self.unplaced_volume
    }

    #[must_use]
    pub const fn unplaced_item_count(&self) -> usize {
        self.unplaced_item_count
    }

    #[must_use]
    pub const fn used_container_count(&self) -> usize {
        self.used_container_count
    }

    #[must_use]
    pub const fn bounding_volume(&self) -> u128 {
        self.bounding_volume
    }

    #[must_use]
    pub const fn unsupported_area(&self) -> u128 {
        self.unsupported_area
    }
}

impl Ord for ObjectiveValue {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .unplaced_volume
            .cmp(&self.unplaced_volume)
            .then_with(|| other.unplaced_item_count.cmp(&self.unplaced_item_count))
            .then_with(|| other.used_container_count.cmp(&self.used_container_count))
            .then_with(|| other.unsupported_area.cmp(&self.unsupported_area))
            .then_with(|| other.bounding_volume.cmp(&self.bounding_volume))
            .then_with(|| other.deterministic_key.cmp(&self.deterministic_key))
    }
}

impl PartialOrd for ObjectiveValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
