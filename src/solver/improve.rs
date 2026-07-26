use crate::solver::constructive::ConstructionPlan;
use crate::validate::PackingInstance;

/// Deterministic reconstruction neighborhoods used by the anytime portfolio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeighborhoodKind {
    Move,
    Swap,
    Rotation,
    EjectionChain,
    RuinRecreate,
}

impl NeighborhoodKind {
    pub(super) const ALL: [Self; 5] = [
        Self::Move,
        Self::Swap,
        Self::Rotation,
        Self::EjectionChain,
        Self::RuinRecreate,
    ];
}

pub(super) fn neighborhood_plan(
    instance: &PackingInstance,
    kind: NeighborhoodKind,
    seed: u64,
) -> ConstructionPlan {
    let mut plan = ConstructionPlan::canonical(instance);
    let mut generator = SplitMix64::new(seed);

    match kind {
        NeighborhoodKind::Move => move_item(&mut plan, &mut generator),
        NeighborhoodKind::Swap => swap_items(&mut plan, &mut generator),
        NeighborhoodKind::Rotation => rotate_item(&mut plan, &mut generator),
        NeighborhoodKind::EjectionChain => ejection_chain(&mut plan, &mut generator),
        NeighborhoodKind::RuinRecreate => ruin_recreate(&mut plan, &mut generator),
    }
    plan
}

fn move_item(plan: &mut ConstructionPlan, generator: &mut SplitMix64) {
    let item_count = plan.item_ids.len();
    if item_count < 2 {
        return;
    }

    let from = generator.index(item_count);
    let mut to = generator.index(item_count - 1);
    if to >= from {
        to += 1;
    }
    let item = plan.item_ids.remove(from);
    plan.item_ids.insert(to, item);
}

fn swap_items(plan: &mut ConstructionPlan, generator: &mut SplitMix64) {
    let item_count = plan.item_ids.len();
    if item_count < 2 {
        return;
    }

    let first = generator.index(item_count);
    let mut second = generator.index(item_count - 1);
    if second >= first {
        second += 1;
    }
    plan.item_ids.swap(first, second);
}

fn rotate_item(plan: &mut ConstructionPlan, generator: &mut SplitMix64) {
    if plan.item_ids.is_empty() {
        return;
    }

    let item_id = plan.item_ids[generator.index(plan.item_ids.len())];
    let rotation_index = 1 + generator.index(5);
    plan.forced_rotations.insert(item_id, rotation_index);
}

fn ejection_chain(plan: &mut ConstructionPlan, generator: &mut SplitMix64) {
    let item_count = plan.item_ids.len();
    if item_count < 3 {
        move_item(plan, generator);
        return;
    }

    let first = generator.index(item_count);
    let mut second = generator.index(item_count);
    while second == first {
        second = generator.index(item_count);
    }
    let mut third = generator.index(item_count);
    while third == first || third == second {
        third = generator.index(item_count);
    }

    let first_item = plan.item_ids[first];
    plan.item_ids[first] = plan.item_ids[second];
    plan.item_ids[second] = plan.item_ids[third];
    plan.item_ids[third] = first_item;
}

fn ruin_recreate(plan: &mut ConstructionPlan, generator: &mut SplitMix64) {
    let item_count = plan.item_ids.len();
    if item_count < 2 {
        return;
    }

    let ruin_count = (1 + item_count / 4).clamp(2, item_count);
    let mut indices = (0..item_count).collect::<Vec<_>>();
    seeded_shuffle(&mut indices, generator);
    indices.truncate(ruin_count);
    indices.sort_unstable();

    let mut ruined = indices
        .iter()
        .map(|index| plan.item_ids[*index])
        .collect::<Vec<_>>();
    for index in indices.into_iter().rev() {
        plan.item_ids.remove(index);
    }
    seeded_shuffle(&mut ruined, generator);

    let insertion = generator.index(plan.item_ids.len() + 1);
    plan.item_ids.splice(insertion..insertion, ruined);
}

fn seeded_shuffle<T>(values: &mut [T], generator: &mut SplitMix64) {
    for upper in (1..values.len()).rev() {
        let selected = generator.index(upper + 1);
        values.swap(upper, selected);
    }
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
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

    fn index(&mut self, length: usize) -> usize {
        let range = u64::try_from(length).expect("collection length must fit u64");
        usize::try_from(self.next() % range).expect("selected index must fit usize")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use crate::model::{InputContainer, InputData, Item};
    use crate::solver::SolveRequest;
    use crate::solver::constructive::solve_with_plan;
    use crate::validate::ItemId;

    use super::*;

    fn instance() -> PackingInstance {
        let input = InputData {
            containers: vec![InputContainer {
                name: "container".to_owned(),
                width: 10.0,
                length: 10.0,
                height: 10.0,
            }],
            contents: (0..12)
                .map(|index| Item {
                    name: format!("item {index}"),
                    width: 1.0 + f64::from(index % 3),
                    length: 1.0,
                    height: 1.0,
                })
                .collect(),
        };
        PackingInstance::try_from(&input).expect("neighborhood fixture should validate")
    }

    #[test]
    fn every_neighborhood_is_reproducible_and_preserves_the_item_permutation() {
        let instance = instance();
        let expected = ConstructionPlan::canonical(&instance)
            .item_ids
            .into_iter()
            .collect::<BTreeSet<ItemId>>();

        for kind in NeighborhoodKind::ALL {
            let first = neighborhood_plan(&instance, kind, 37);
            let second = neighborhood_plan(&instance, kind, 37);
            let actual = first.item_ids.iter().copied().collect::<BTreeSet<ItemId>>();

            assert_eq!(first, second, "{kind:?}");
            assert_eq!(actual, expected, "{kind:?}");
            assert_eq!(first.item_ids.len(), expected.len(), "{kind:?}");
        }
    }

    #[test]
    fn order_neighborhoods_change_order_and_rotation_changes_preference() {
        let instance = instance();
        let canonical = ConstructionPlan::canonical(&instance);

        for kind in [
            NeighborhoodKind::Move,
            NeighborhoodKind::Swap,
            NeighborhoodKind::EjectionChain,
            NeighborhoodKind::RuinRecreate,
        ] {
            let candidate = neighborhood_plan(&instance, kind, 53);
            assert_ne!(candidate.item_ids, canonical.item_ids, "{kind:?}");
            assert!(candidate.forced_rotations.is_empty(), "{kind:?}");
        }

        let rotation = neighborhood_plan(&instance, NeighborhoodKind::Rotation, 53);
        assert_eq!(rotation.item_ids, canonical.item_ids);
        assert_eq!(rotation.forced_rotations.len(), 1);
        assert!(
            rotation
                .forced_rotations
                .values()
                .all(|index| (1..=5).contains(index))
        );
    }

    #[test]
    fn rotation_neighborhood_forces_the_selected_exact_orientation() {
        let input = InputData {
            containers: vec![InputContainer {
                name: "container".to_owned(),
                width: 10.0,
                length: 10.0,
                height: 10.0,
            }],
            contents: vec![Item {
                name: "rotatable".to_owned(),
                width: 1.0,
                length: 2.0,
                height: 3.0,
            }],
        };
        let instance = PackingInstance::try_from(&input).expect("rotation fixture should validate");
        let plan = neighborhood_plan(&instance, NeighborhoodKind::Rotation, 71);
        let item_id = instance.items()[0].id();
        let rotation_index = plan.forced_rotations[&item_id];
        let rotations = instance.items()[0].dimensions().unique_rotations();
        let request = SolveRequest::new(
            Duration::from_secs(1),
            71,
            NonZeroUsize::new(1).expect("one is non-zero"),
        );

        let outcome =
            solve_with_plan(&instance, &request, &plan).expect("rotation plan should solve");
        let placement = outcome
            .solution()
            .placements()
            .first()
            .expect("rotated item should be placed");

        assert_eq!(
            placement.bounds().dimensions(),
            rotations[rotation_index % rotations.len()]
        );
    }
}
