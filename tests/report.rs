use boxpacker::model::{Cuboid, Item, OutputContainer, OutputData, PlacedItem};
use boxpacker::report::render_html;

const CURRENT_SAVED_OUTPUT: &str = include_str!("fixtures/current/saved_output.json");
const DATA_ELEMENT_START: &str = r#"<script id="boxpacker-report-data" type="application/json">"#;

fn embedded_report_json(html: &str) -> &str {
    let after_start = html
        .split_once(DATA_ELEMENT_START)
        .expect("report should contain its data element")
        .1;
    after_start
        .split_once("</script>")
        .expect("report data element should be closed")
        .0
}

#[test]
fn current_saved_output_renders_through_the_report_view_model() {
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");

    let html = render_html(&output).expect("current saved output should render");
    let embedded: OutputData = serde_json::from_str(embedded_report_json(&html))
        .expect("embedded report data should remain valid JSON");

    assert_eq!(embedded, output);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("Logistics Overview"));
    assert!(html.contains("Container Vol"));
    assert!(html.contains("Unplaced Vol"));
    assert!(html.contains("THREE.BoxGeometry"));
    assert!(html.contains("function highlightItem(id)"));
    assert!(html.contains(r#"id="wireframe-toggle""#));
    assert!(html.contains("Toggle X-Ray View"));
}

#[test]
fn arbitrary_names_cannot_end_the_data_script_or_inject_markup() {
    let output = OutputData {
        containers: vec![OutputContainer {
            name: r#"</script><img src=x onerror="container-owned"> & container"#.to_owned(),
            width: 10.0,
            length: 10.0,
            height: 10.0,
            placed_items: vec![PlacedItem {
                name: "<svg onload=placed-owned>`'\"\u{2028}\u{2029}".to_owned(),
                coords: Cuboid {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: 1.0,
                    l: 2.0,
                    h: 3.0,
                },
                color: "#123456".to_owned(),
            }],
        }],
        unplaced_items: vec![Item {
            name: "<b>unplaced-owned</b>".to_owned(),
            width: 4.0,
            length: 5.0,
            height: 6.0,
        }],
    };

    let html = render_html(&output).expect("arbitrary names should render");
    let embedded_json = embedded_report_json(&html);
    let embedded: OutputData =
        serde_json::from_str(embedded_json).expect("escaped data should decode exactly");

    assert_eq!(embedded, output);
    assert!(!embedded_json.contains('<'));
    assert!(!embedded_json.contains('>'));
    assert!(!embedded_json.contains('&'));
    assert!(!embedded_json.contains('\u{2028}'));
    assert!(!embedded_json.contains('\u{2029}'));
    assert!(embedded_json.contains(r"\u003c/script\u003e\u003cimg"));
    assert!(embedded_json.contains(r"\u0026 container"));
    assert!(embedded_json.contains(r"\u2028\u2029"));
}

#[test]
fn template_inserts_user_visible_names_as_text() {
    let output: OutputData = serde_json::from_str(CURRENT_SAVED_OUTPUT)
        .expect("current saved output fixture should deserialize");

    let html = render_html(&output).expect("current saved output should render");

    assert!(html.contains("heading.textContent = `${container.name} (${utilization}%)`;"));
    assert!(html.contains("name.textContent = item.name;"));
    assert!(html.contains("tag.textContent = item.name;"));
    assert!(html.contains("context.fillText(text, 256, 80);"));
    assert!(!html.contains("innerHTML"));
    assert!(!html.contains("onclick="));
}
