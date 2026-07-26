use std::error::Error;
use std::fmt;

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
pub fn render_html(output: &OutputData) -> Result<String, ReportRenderError> {
    render_template(TEMPLATE, output)
}

fn render_template(template: &str, output: &OutputData) -> Result<String, ReportRenderError> {
    let placeholder_count = template.matches(DATA_PLACEHOLDER).count();
    if placeholder_count != 1 {
        return Err(ReportRenderError::InvalidTemplate { placeholder_count });
    }
    validate_numbers(output)?;

    let view_model = ReportViewModel {
        containers: &output.containers,
        unplaced_items: &output.unplaced_items,
    };
    let json = serde_json::to_string(&view_model).map_err(ReportRenderError::Serialize)?;
    let escaped_json = escape_json_for_script(&json);

    Ok(template.replacen(DATA_PLACEHOLDER, &escaped_json, 1))
}

fn validate_numbers(output: &OutputData) -> Result<(), ReportRenderError> {
    for (container_index, container) in output.containers.iter().enumerate() {
        validate_positive(
            &format!("containers[{container_index}].width"),
            container.width,
        )?;
        validate_positive(
            &format!("containers[{container_index}].length"),
            container.length,
        )?;
        validate_positive(
            &format!("containers[{container_index}].height"),
            container.height,
        )?;

        for (item_index, item) in container.placed_items.iter().enumerate() {
            let path = format!("containers[{container_index}].placed_items[{item_index}].coords");
            validate_non_negative(&format!("{path}.x"), item.coords.x)?;
            validate_non_negative(&format!("{path}.y"), item.coords.y)?;
            validate_non_negative(&format!("{path}.z"), item.coords.z)?;
            validate_positive(&format!("{path}.w"), item.coords.w)?;
            validate_positive(&format!("{path}.l"), item.coords.l)?;
            validate_positive(&format!("{path}.h"), item.coords.h)?;
        }
    }

    for (item_index, item) in output.unplaced_items.iter().enumerate() {
        validate_positive(&format!("unplaced_items[{item_index}].width"), item.width)?;
        validate_positive(&format!("unplaced_items[{item_index}].length"), item.length)?;
        validate_positive(&format!("unplaced_items[{item_index}].height"), item.height)?;
    }
    Ok(())
}

fn validate_positive(path: &str, value: f64) -> Result<(), ReportRenderError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(ReportRenderError::InvalidNumber {
            path: path.to_owned(),
            requirement: "must be finite and greater than zero",
        })
    }
}

fn validate_non_negative(path: &str, value: f64) -> Result<(), ReportRenderError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ReportRenderError::InvalidNumber {
            path: path.to_owned(),
            requirement: "must be finite and non-negative",
        })
    }
}

/// A report failure that cannot silently produce an unsafe or incomplete page.
#[derive(Debug)]
pub enum ReportRenderError {
    InvalidTemplate {
        placeholder_count: usize,
    },
    InvalidNumber {
        path: String,
        requirement: &'static str,
    },
    Serialize(serde_json::Error),
}

impl fmt::Display for ReportRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTemplate { placeholder_count } => write!(
                formatter,
                "report template must contain exactly one data placeholder (found {placeholder_count})"
            ),
            Self::InvalidNumber { path, requirement } => {
                write!(formatter, "{path} {requirement}")
            }
            Self::Serialize(_) => formatter.write_str("report data could not be serialized"),
        }
    }
}

impl Error for ReportRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(source) => Some(source),
            Self::InvalidTemplate { .. } | Self::InvalidNumber { .. } => None,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_output() -> OutputData {
        OutputData {
            containers: Vec::new(),
            unplaced_items: Vec::new(),
        }
    }

    #[test]
    fn template_requires_exactly_one_data_placeholder() {
        let output = empty_output();

        assert!(matches!(
            render_template("<html></html>", &output),
            Err(ReportRenderError::InvalidTemplate {
                placeholder_count: 0
            })
        ));
        assert!(matches!(
            render_template(
                "__BOXPACKER_REPORT_DATA____BOXPACKER_REPORT_DATA__",
                &output
            ),
            Err(ReportRenderError::InvalidTemplate {
                placeholder_count: 2
            })
        ));
    }
}
