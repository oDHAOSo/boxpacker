//! Application composition for file I/O, solving, validation, and reporting.

use std::error::Error;
use std::fmt;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cli::{Cli, SearchPreset};
use crate::compatibility::output_from_solution;
use crate::model::InputData;
use crate::report::render_html;
use crate::solver::portfolio::PortfolioBackend;
use crate::solver::{OptimalityStatus, SolveRequest, SolverBackend, SolverError, SolverMetrics};
use crate::validate::{
    InputValidationErrors, PackingInstance, SolutionSummary, SolutionValidationErrors,
    validate_solution,
};

/// Fixed effort associated with one CLI preset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchSettings {
    time_limit: Duration,
    work_units: NonZeroUsize,
}

impl SearchSettings {
    /// Return the default budget for a preset.
    #[must_use]
    pub fn for_preset(preset: SearchPreset) -> Self {
        match preset {
            SearchPreset::Fast => Self::new(Duration::from_secs(1), 1),
            SearchPreset::Balanced => Self::new(Duration::from_secs(10), 8),
            SearchPreset::Thorough => Self::new(Duration::from_secs(30), 14),
        }
    }

    fn new(time_limit: Duration, work_units: usize) -> Self {
        Self {
            time_limit,
            work_units: NonZeroUsize::new(work_units).expect("preset work must be non-zero"),
        }
    }

    #[must_use]
    pub const fn time_limit(self) -> Duration {
        self.time_limit
    }

    #[must_use]
    pub const fn work_units(self) -> NonZeroUsize {
        self.work_units
    }
}

/// Successful end-to-end run details for CLI presentation and tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSummary {
    solution: SolutionSummary,
    metrics: SolverMetrics,
    optimality: OptimalityStatus,
    json_output_path: PathBuf,
    html_output_path: PathBuf,
}

impl RunSummary {
    #[must_use]
    pub const fn solution(&self) -> &SolutionSummary {
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

    #[must_use]
    pub fn json_output_path(&self) -> &Path {
        &self.json_output_path
    }

    #[must_use]
    pub fn html_output_path(&self) -> &Path {
        &self.html_output_path
    }
}

/// Run the selected portfolio and write compatible JSON and HTML outputs.
pub fn run(cli: &Cli) -> Result<RunSummary, AppError> {
    let html_output_path = cli.html_output_path();
    if cli.output == html_output_path {
        return Err(AppError::ConflictingOutputPaths(cli.output.clone()));
    }

    let input_json = fs::read_to_string(&cli.input).map_err(|source| AppError::ReadInput {
        path: cli.input.clone(),
        source,
    })?;
    let input: InputData =
        serde_json::from_str(&input_json).map_err(|source| AppError::ParseInput {
            path: cli.input.clone(),
            source,
        })?;
    let instance = PackingInstance::try_from(&input).map_err(AppError::ValidateInput)?;

    let settings = SearchSettings::for_preset(cli.preset);
    let request = SolveRequest::new(
        cli.time_limit.unwrap_or(settings.time_limit()),
        cli.seed,
        cli.threads,
    );
    let outcome = PortfolioBackend::new(settings.work_units())
        .solve(&instance, &request)
        .map_err(AppError::Solve)?;
    let solution =
        validate_solution(&instance, outcome.solution()).map_err(AppError::ValidateSolution)?;

    let output = output_from_solution(&instance, outcome.solution());
    let mut output_json =
        serde_json::to_string_pretty(&output).map_err(AppError::SerializeOutput)?;
    output_json.push('\n');
    let html = render_html(&output).map_err(AppError::RenderReport)?;

    write_output(&cli.output, &output_json)?;
    write_output(&html_output_path, &html)?;

    Ok(RunSummary {
        solution,
        metrics: outcome.metrics(),
        optimality: outcome.optimality(),
        json_output_path: cli.output.clone(),
        html_output_path,
    })
}

fn write_output(path: &Path, contents: &str) -> Result<(), AppError> {
    fs::write(path, contents).map_err(|source| AppError::WriteOutput {
        path: path.to_owned(),
        source,
    })
}

/// A stage-specific application failure with its source preserved.
#[derive(Debug)]
pub enum AppError {
    ConflictingOutputPaths(PathBuf),
    ReadInput {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseInput {
        path: PathBuf,
        source: serde_json::Error,
    },
    ValidateInput(InputValidationErrors),
    Solve(SolverError),
    ValidateSolution(SolutionValidationErrors),
    SerializeOutput(serde_json::Error),
    RenderReport(serde_json::Error),
    WriteOutput {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingOutputPaths(path) => write!(
                formatter,
                "JSON and HTML output paths resolve to the same file: {}",
                path.display()
            ),
            Self::ReadInput { path, .. } => {
                write!(formatter, "failed to read input {}", path.display())
            }
            Self::ParseInput { path, .. } => {
                write!(formatter, "failed to parse input JSON {}", path.display())
            }
            Self::ValidateInput(_) => formatter.write_str("input is invalid"),
            Self::Solve(_) => formatter.write_str("solver failed"),
            Self::ValidateSolution(_) => formatter.write_str("solver returned an invalid solution"),
            Self::SerializeOutput(_) => {
                formatter.write_str("failed to serialize compatible JSON output")
            }
            Self::RenderReport(_) => formatter.write_str("failed to render HTML report"),
            Self::WriteOutput { path, .. } => {
                write!(formatter, "failed to write output {}", path.display())
            }
        }?;

        if let Some(source) = self.source() {
            write!(formatter, ": {source}")?;
        }
        Ok(())
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConflictingOutputPaths(_) => None,
            Self::ReadInput { source, .. } | Self::WriteOutput { source, .. } => Some(source),
            Self::ParseInput { source, .. }
            | Self::SerializeOutput(source)
            | Self::RenderReport(source) => Some(source),
            Self::ValidateInput(source) => Some(source),
            Self::Solve(source) => Some(source),
            Self::ValidateSolution(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_increase_only_time_and_fixed_work() {
        let fast = SearchSettings::for_preset(SearchPreset::Fast);
        let balanced = SearchSettings::for_preset(SearchPreset::Balanced);
        let thorough = SearchSettings::for_preset(SearchPreset::Thorough);

        assert!(fast.time_limit() < balanced.time_limit());
        assert!(balanced.time_limit() < thorough.time_limit());
        assert!(fast.work_units() < balanced.work_units());
        assert!(balanced.work_units() < thorough.work_units());
    }
}
