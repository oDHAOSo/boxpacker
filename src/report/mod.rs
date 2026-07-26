use serde::Serialize;

use crate::model::{Item, OutputContainer, OutputData};

const TEMPLATE: &str = include_str!("template.html");
const DATA_PLACEHOLDER: &str = "__BOXPACKER_REPORT_DATA__";

#[derive(Serialize)]
struct ReportViewModel<'a> {
    containers: &'a [OutputContainer],
    unplaced_items: &'a [Item],
}

/// Render the compatibility HTML report for a packing result.
///
/// Report data is serialized as one JSON document and escaped for safe
/// inclusion in an HTML `script` element. The template parses that document
/// and inserts all user-provided text through DOM text nodes.
pub fn render_html(output: &OutputData) -> Result<String, serde_json::Error> {
    let view_model = ReportViewModel {
        containers: &output.containers,
        unplaced_items: &output.unplaced_items,
    };
    let json = serde_json::to_string(&view_model)?;
    let escaped_json = escape_json_for_script(&json);

    debug_assert_eq!(TEMPLATE.matches(DATA_PLACEHOLDER).count(), 1);
    Ok(TEMPLATE.replacen(DATA_PLACEHOLDER, &escaped_json, 1))
}

fn escape_json_for_script(json: &str) -> String {
    let mut escaped = String::with_capacity(json.len());

    for character in json.chars() {
        match character {
            '<' => escaped.push_str(r"\u003c"),
            '>' => escaped.push_str(r"\u003e"),
            '&' => escaped.push_str(r"\u0026"),
            '\u{2028}' => escaped.push_str(r"\u2028"),
            '\u{2029}' => escaped.push_str(r"\u2029"),
            _ => escaped.push(character),
        }
    }

    escaped
}
