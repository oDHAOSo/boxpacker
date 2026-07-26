use boxpacker::compatibility::{SavedSolutionAdapterError, adapt_saved_solution};
use boxpacker::geometry::CoordinateConversionError;
use boxpacker::model::{
    Cuboid, InputContainer, InputData, Item, OutputContainer, OutputData, PlacedItem,
};
use boxpacker::report::render_html;
use boxpacker::validate::PackingInstance;

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");
const CURRENT_SAVED_OUTPUT: &str = include_str!("fixtures/current/saved_output.json");

fn current_fixture() -> (PackingInstance, OutputData) {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current input fixture should deserialize");
    let instance =
        PackingInstance::try_from(&input).expect("current input fixture should validate");
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");
    (instance, output)
}

#[test]
fn current_saved_solution_maps_to_stable_ids_and_exact_geometry() {
    let (instance, output) = current_fixture();

    let saved = adapt_saved_solution(&instance, output)
        .expect("current saved solution should adapt to exact domain data");

    assert_eq!(saved.containers().len(), 6);
    assert_eq!(
        saved
            .containers()
            .iter()
            .map(|container| container.placed_items().len())
            .sum::<usize>(),
        49
    );
    assert_eq!(saved.unplaced_items().len(), 8);

    let first_container = &saved.containers()[0];
    assert_eq!(first_container.container_id().index(), 5);
    let first_placement = first_container.placed_items()[0];
    assert_eq!(first_placement.item_id().index(), 3);
    assert_eq!(first_placement.bounds().origin().x().get(), 0);
    assert_eq!(first_placement.bounds().origin().y().get(), 0);
    assert_eq!(first_placement.bounds().origin().z().get(), 0);
    assert_eq!(first_placement.bounds().dimensions().width().get(), 275);
    assert_eq!(first_placement.bounds().dimensions().length().get(), 310);
    assert_eq!(first_placement.bounds().dimensions().height().get(), 180);

    let ryougi_knife = first_container.placed_items()[1];
    assert_eq!(ryougi_knife.bounds().origin().x().get(), 275);
    assert_eq!(ryougi_knife.bounds().dimensions().length().get(), 292);
}

#[test]
fn adapted_saved_solution_remains_the_report_source_of_truth() {
    let (instance, output) = current_fixture();
    let expected = output.clone();
    let saved = adapt_saved_solution(&instance, output)
        .expect("current saved solution should adapt to exact domain data");

    assert_eq!(saved.output(), &expected);
    let html = render_html(saved.output()).expect("adapted output should render");
    assert!(html.contains(r#"<script id="boxpacker-report-data""#));
}

#[test]
fn saved_coordinates_are_converted_with_field_specific_errors() {
    let (instance, mut output) = current_fixture();
    output.containers[0].placed_items[0].coords.x = 0.25;

    let error = adapt_saved_solution(&instance, output)
        .expect_err("over-precision saved coordinates should be rejected");

    assert_eq!(
        error,
        SavedSolutionAdapterError::InvalidCoordinate {
            path: "containers[0].placed_items[0].coords.x".to_owned(),
            value: 0.25,
            reason: CoordinateConversionError::OverPrecision,
        }
    );
    assert_eq!(
        error.to_string(),
        "containers[0].placed_items[0].coords.x must use no more than one decimal place (got 0.25)"
    );
}

#[test]
fn saved_solution_must_account_for_every_input_item_once() {
    let (instance, mut output) = current_fixture();
    output.unplaced_items.pop();

    let error = adapt_saved_solution(&instance, output)
        .expect_err("a saved solution that omits an item should not adapt");

    assert!(matches!(
        error,
        SavedSolutionAdapterError::MissingItems(ref ids)
            if ids.len() == 1 && ids[0].index() == 55
    ));
}

#[test]
fn duplicate_names_are_resolved_by_dimensions_and_stable_input_order() {
    let input = InputData {
        containers: vec![
            InputContainer {
                name: "duplicate container".to_owned(),
                width: 10.0,
                length: 10.0,
                height: 10.0,
            },
            InputContainer {
                name: "duplicate container".to_owned(),
                width: 20.0,
                length: 20.0,
                height: 20.0,
            },
        ],
        contents: vec![
            Item {
                name: "duplicate item".to_owned(),
                width: 1.0,
                length: 2.0,
                height: 3.0,
            },
            Item {
                name: "duplicate item".to_owned(),
                width: 4.0,
                length: 5.0,
                height: 6.0,
            },
        ],
    };
    let instance = PackingInstance::try_from(&input).expect("duplicate names should validate");
    let output = OutputData {
        containers: vec![
            OutputContainer {
                name: "duplicate container".to_owned(),
                width: 20.0,
                length: 20.0,
                height: 20.0,
                placed_items: vec![PlacedItem {
                    name: "duplicate item".to_owned(),
                    coords: Cuboid {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 6.0,
                        l: 4.0,
                        h: 5.0,
                    },
                    color: "#123456".to_owned(),
                }],
            },
            OutputContainer {
                name: "duplicate container".to_owned(),
                width: 10.0,
                length: 10.0,
                height: 10.0,
                placed_items: Vec::new(),
            },
        ],
        unplaced_items: vec![Item {
            name: "duplicate item".to_owned(),
            width: 1.0,
            length: 2.0,
            height: 3.0,
        }],
    };

    let saved = adapt_saved_solution(&instance, output)
        .expect("dimensions should disambiguate duplicate display names");

    assert_eq!(saved.containers()[0].container_id().index(), 1);
    assert_eq!(saved.containers()[0].placed_items()[0].item_id().index(), 1);
    assert_eq!(saved.containers()[1].container_id().index(), 0);
    assert_eq!(saved.unplaced_items()[0].index(), 0);
}
