use crate::geometry::Aabb;
use crate::validate::{ContainerId, ItemId};

/// One proposed item placement in exact internal geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placement {
    container_id: ContainerId,
    item_id: ItemId,
    bounds: Aabb,
}

impl Placement {
    #[must_use]
    pub const fn new(container_id: ContainerId, item_id: ItemId, bounds: Aabb) -> Self {
        Self {
            container_id,
            item_id,
            bounds,
        }
    }

    #[must_use]
    pub const fn container_id(self) -> ContainerId {
        self.container_id
    }

    #[must_use]
    pub const fn item_id(self) -> ItemId {
        self.item_id
    }

    #[must_use]
    pub const fn bounds(self) -> Aabb {
        self.bounds
    }
}

/// Backend-neutral candidate solution.
///
/// Constructors intentionally do not enforce coverage or geometry rules. Every
/// backend must pass its candidate through the independent validator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Solution {
    placements: Vec<Placement>,
    unplaced_items: Vec<ItemId>,
}

impl Solution {
    #[must_use]
    pub fn new(placements: Vec<Placement>, unplaced_items: Vec<ItemId>) -> Self {
        Self {
            placements,
            unplaced_items,
        }
    }

    #[must_use]
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    #[must_use]
    pub fn unplaced_items(&self) -> &[ItemId] {
        &self.unplaced_items
    }
}
