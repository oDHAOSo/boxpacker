use boxpacker::compatibility::adapt_saved_solution;
use boxpacker::geometry::{Dimensions, SCALE};
use boxpacker::model::{InputData, OutputData};
use boxpacker::validate::PackingInstance;

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const CURRENT_SAVED_OUTPUT: &str = include_str!("fixtures/current/saved_output.json");
const SCALED_VOLUME_PER_INPUT_VOLUME: u128 = (SCALE as u128).pow(3);

fn scaled_volume(dimensions: Dimensions) -> u128 {
    dimensions
        .checked_volume()
        .expect("validated fixture dimensions should have representable volume")
}

fn rounded_percentage_basis_points(numerator: u128, denominator: u128) -> u128 {
    numerator
        .checked_mul(10_000)
        .and_then(|scaled| scaled.checked_add(denominator / 2))
        .expect("fixture percentage arithmetic should remain representable")
        / denominator
}

#[test]
fn current_saved_solution_baseline_metrics_are_immutable() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");
    let solution = adapt_saved_solution(&instance, output)
        .expect("current saved output should adapt to exact domain geometry");

    let placed_item_ids = solution
        .containers()
        .iter()
        .flat_map(|container| container.placed_items())
        .map(|placement| placement.item_id())
        .collect::<Vec<_>>();
    let packed_volume = placed_item_ids
        .iter()
        .map(|item_id| scaled_volume(instance.items()[item_id.index()].dimensions()))
        .sum::<u128>();
    let unplaced_volume = solution
        .unplaced_items()
        .iter()
        .map(|item_id| scaled_volume(instance.items()[item_id.index()].dimensions()))
        .sum::<u128>();
    let total_item_volume = instance
        .items()
        .iter()
        .map(|item| scaled_volume(item.dimensions()))
        .sum::<u128>();
    let total_container_capacity = instance
        .containers()
        .iter()
        .map(|container| scaled_volume(container.dimensions()))
        .sum::<u128>();

    assert_eq!(instance.items().len(), 57);
    assert_eq!(placed_item_ids.len(), 49);
    assert_eq!(solution.unplaced_items().len(), 8);

    assert_eq!(packed_volume, 582_885_612);
    assert_eq!(unplaced_volume, 26_095_930);
    assert_eq!(total_item_volume, 608_981_542);
    assert_eq!(total_container_capacity, 735_033_290);
    assert_eq!(packed_volume + unplaced_volume, total_item_volume);

    assert_eq!(packed_volume / SCALED_VOLUME_PER_INPUT_VOLUME, 582_885);
    assert_eq!(
        packed_volume % SCALED_VOLUME_PER_INPUT_VOLUME,
        612,
        "packed volume should be exactly 582,885.612 input cubic units"
    );
    assert_eq!(
        rounded_percentage_basis_points(packed_volume, total_container_capacity),
        7_930,
        "saved-solution utilization should display as 79.30%"
    );
    assert_eq!(
        rounded_percentage_basis_points(total_item_volume, total_container_capacity),
        8_285,
        "theoretical utilization should display as 82.85%"
    );
}
