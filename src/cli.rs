use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Parser, ValueEnum};

pub const DEFAULT_INPUT_PATH: &str = "input.json";
pub const DEFAULT_OUTPUT_PATH: &str = "output.json";

/// Command-line arguments for the BoxPacker compatibility shell.
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(
    name = "boxpacker",
    about = "Packs rectangular items into rectangular containers"
)]
pub struct Cli {
    /// Read container and item definitions from this JSON file.
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = DEFAULT_INPUT_PATH
    )]
    pub input: PathBuf,

    /// Write the packing result to this JSON file.
    #[arg(
        short,
        long,
        value_name = "FILE",
        default_value = DEFAULT_OUTPUT_PATH
    )]
    pub output: PathBuf,

    /// Select the search-effort preset.
    #[arg(long, value_enum, default_value = "balanced")]
    pub preset: SearchPreset,

    /// Override the preset's search time limit, in positive seconds.
    #[arg(long, value_name = "SECONDS", value_parser = parse_positive_duration)]
    pub time_limit: Option<Duration>,

    /// Seed used for reproducible randomized search.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,

    /// Maximum number of solver threads.
    #[arg(long, value_name = "COUNT", default_value = "1")]
    pub threads: NonZeroUsize,
}

impl Cli {
    /// Return the HTML report path produced beside the JSON output.
    #[must_use]
    pub fn html_output_path(&self) -> PathBuf {
        html_output_path(&self.output)
    }
}

/// Search effort only; presets must never alter geometry correctness.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum SearchPreset {
    Fast,
    #[default]
    Balanced,
    Thorough,
}

/// Derive the report path using the legacy output-extension behavior.
#[must_use]
pub fn html_output_path(output_path: &Path) -> PathBuf {
    output_path.with_extension("html")
}

fn parse_positive_duration(value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<f64>()
        .map_err(|_| "time limit must be a number of seconds".to_owned())?;

    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("time limit must be a finite number greater than zero".to_owned());
    }

    Duration::try_from_secs_f64(seconds)
        .map_err(|_| "time limit is too large to represent".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_html_path_from_json_output() {
        assert_eq!(
            html_output_path(Path::new("results/packing.json")),
            PathBuf::from("results/packing.html")
        );
    }

    #[test]
    fn derives_html_path_when_output_has_no_extension() {
        assert_eq!(
            html_output_path(Path::new("results/packing")),
            PathBuf::from("results/packing.html")
        );
    }
}
