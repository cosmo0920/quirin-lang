// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! One immutable preparation of a C translation unit for an audit run.
//!
//! The two preprocessing forms are deliberately produced separately. The
//! line-marked stream is never reconstructed by stripping or inserting text
//! in the authoritative `-E -P` stream.

use std::io::Write;
use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;

use super::enumeration::{inventory_from_line_marked_result, preprocess_with_line_markers_counted};
use super::location::{SourceLineMap, map_marked_preprocessing};
use super::{
    C11TranslationUnit, CCompilerDialect, CPreprocessorOption, CSourceInput, LegacyCError,
    LegacyFunctionScope, SourceDefinitionInventory, reject_filename_sensitive_constructs,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationStageOutcome {
    Succeeded,
    Failed,
    NotAttempted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PreparationStageMetrics {
    pub elapsed_us: u64,
    pub clang_invocations: u8,
    pub outcome: PreparationStageOutcome,
}

impl PreparationStageMetrics {
    const NOT_ATTEMPTED: Self = Self {
        elapsed_us: 0,
        clang_invocations: 0,
        outcome: PreparationStageOutcome::NotAttempted,
    };
}

/// Measured stages and actual successfully spawned Clang process count.
/// Timings are observations of this run and make no throughput or speedup claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparationMetrics {
    pub plain_preprocessing: PreparationStageMetrics,
    pub syntax_validation: PreparationStageMetrics,
    pub line_marked_preprocessing: PreparationStageMetrics,
}

impl PreparationMetrics {
    #[must_use]
    pub fn clang_invocations(&self) -> u8 {
        self.plain_preprocessing
            .clang_invocations
            .saturating_add(self.syntax_validation.clang_invocations)
            .saturating_add(self.line_marked_preprocessing.clang_invocations)
    }
}

/// Exact preprocessing products and derived read-only indexes for one unit.
///
/// Construction validates the authoritative `-E -P` output with a separate
/// syntax pass. The line-marked form is retained independently and drives both
/// the main-source inventory and diagnostic coordinate map. Failure to obtain
/// that auxiliary form is represented by the existing fail-closed inventory
/// and location refusals; it never substitutes an approximation.
#[derive(Debug)]
pub struct PreparedTranslationUnit {
    raw_source: String,
    preprocessed_source: Option<String>,
    validation_error: Option<LegacyCError>,
    line_marked_source: Option<String>,
    line_marked_error: Option<LegacyCError>,
    inventory: SourceDefinitionInventory,
    locations: SourceLineMap,
    dialect: CCompilerDialect,
    function_scope: LegacyFunctionScope,
    include_paths: Vec<String>,
    definitions: Vec<String>,
    preprocessor_options: Vec<CPreprocessorOption>,
    input: CSourceInput,
    metrics: PreparationMetrics,
}

impl PreparedTranslationUnit {
    #[must_use]
    pub fn raw_source(&self) -> &str {
        &self.raw_source
    }

    pub fn preprocessed_source(&self) -> Result<&str, &LegacyCError> {
        self.preprocessed_source.as_deref().ok_or_else(|| {
            self.validation_error
                .as_ref()
                .expect("failed preparation has an error")
        })
    }

    #[must_use]
    pub fn validation_error(&self) -> Option<&LegacyCError> {
        self.validation_error.as_ref()
    }

    #[must_use]
    pub fn line_marked_source(&self) -> Option<&str> {
        self.line_marked_source.as_deref()
    }

    #[must_use]
    pub fn line_marked_error(&self) -> Option<&LegacyCError> {
        self.line_marked_error.as_ref()
    }

    #[must_use]
    pub fn inventory(&self) -> &SourceDefinitionInventory {
        &self.inventory
    }

    #[must_use]
    pub fn locations(&self) -> &SourceLineMap {
        &self.locations
    }

    #[must_use]
    pub const fn dialect(&self) -> CCompilerDialect {
        self.dialect
    }

    #[must_use]
    pub const fn function_scope(&self) -> LegacyFunctionScope {
        self.function_scope
    }

    #[must_use]
    pub fn include_paths(&self) -> &[String] {
        &self.include_paths
    }

    #[must_use]
    pub fn definitions(&self) -> &[String] {
        &self.definitions
    }

    #[must_use]
    pub fn preprocessor_options(&self) -> &[CPreprocessorOption] {
        &self.preprocessor_options
    }

    #[must_use]
    pub const fn input(&self) -> &CSourceInput {
        &self.input
    }

    #[must_use]
    pub const fn metrics(&self) -> &PreparationMetrics {
        &self.metrics
    }
}

fn elapsed_us(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn preprocess_plain(
    unit: &C11TranslationUnit<'_>,
) -> (Result<String, LegacyCError>, PreparationStageMetrics) {
    let start = Instant::now();
    let mut invocations = 0;
    let result = (|| {
        let mut command =
            crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
        let dialect = unit
            .dialect
            .clang_invocation_argument()
            .map_err(LegacyCError::ProductionParserUnavailable)?;
        command.args(["-x", "c", dialect, "-E", "-P"]);
        for include in &unit.include_paths {
            command.arg(format!("-I{include}"));
        }
        for definition in &unit.definitions {
            command.arg(format!("-D{definition}"));
        }
        for option in &unit.preprocessor_options {
            option.append_to(&mut command);
        }
        let piped = unit.append_primary_input(&mut command)?;
        let mut child = command
            .stdin(if piped { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        invocations = 1;
        if piped {
            child
                .stdin
                .take()
                .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing stdin".into()))?
                .write_all(unit.source.as_bytes())
                .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        if !output.status.success() {
            return Err(LegacyCError::ProductionParserRejected(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let preprocessed_source = String::from_utf8_lossy(&output.stdout).into_owned();
        reject_filename_sensitive_constructs(&preprocessed_source, &unit.input)?;
        Ok(preprocessed_source)
    })();
    let metrics = PreparationStageMetrics {
        elapsed_us: elapsed_us(start),
        clang_invocations: invocations,
        outcome: if result.is_ok() {
            PreparationStageOutcome::Succeeded
        } else {
            PreparationStageOutcome::Failed
        },
    };
    (result, metrics)
}

fn validate_syntax(
    unit: &C11TranslationUnit<'_>,
    preprocessed_source: &str,
) -> (Result<(), LegacyCError>, PreparationStageMetrics) {
    let start = Instant::now();
    let mut invocations = 0;
    let result = (|| {
        let mut command =
            crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
        unit.append_frontend_flags(&mut command);
        let dialect = unit
            .dialect
            .clang_invocation_argument()
            .map_err(LegacyCError::ProductionParserUnavailable)?;
        let mut child = command
            .args(["-x", "c", dialect, "-fsyntax-only", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        invocations = 1;
        child
            .stdin
            .take()
            .ok_or_else(|| {
                LegacyCError::ProductionParserUnavailable("missing syntax stdin".into())
            })?
            .write_all(preprocessed_source.as_bytes())
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        let output = child
            .wait_with_output()
            .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
        if !output.status.success() {
            return Err(LegacyCError::ProductionParserRejected(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        Ok(())
    })();
    let metrics = PreparationStageMetrics {
        elapsed_us: elapsed_us(start),
        clang_invocations: invocations,
        outcome: if result.is_ok() {
            PreparationStageOutcome::Succeeded
        } else {
            PreparationStageOutcome::Failed
        },
    };
    (result, metrics)
}

/// The plain preprocessing alone (card OP-APP-RK-CALLBACKS-VIA-POLL-CB).
pub(super) fn preprocess_plain_only(unit: &C11TranslationUnit<'_>) -> Result<String, LegacyCError> {
    preprocess_plain(unit).0
}

pub(super) fn validate_plain_and_syntax(
    unit: &C11TranslationUnit<'_>,
) -> Result<(String, u64, u64), LegacyCError> {
    let (preprocessed_source, plain) = preprocess_plain(unit);
    let preprocessed_source = preprocessed_source?;
    let (syntax, syntax_metrics) = validate_syntax(unit, &preprocessed_source);
    syntax?;
    Ok((
        preprocessed_source,
        plain.elapsed_us,
        syntax_metrics.elapsed_us,
    ))
}

/// Prepare one translation unit exactly once for identity, enumeration,
/// analysis and diagnostic reporting within the current run.
pub fn prepare_translation_unit(unit: &C11TranslationUnit<'_>) -> PreparedTranslationUnit {
    let (plain, plain_metrics) = preprocess_plain(unit);
    let (validated, syntax_metrics) = match plain {
        Ok(source) => {
            let (syntax, metrics) = validate_syntax(unit, &source);
            (syntax.map(|()| source), metrics)
        }
        Err(error) => (Err(error), PreparationStageMetrics::NOT_ATTEMPTED),
    };

    let marked_start = Instant::now();
    let (marked, marked_invoked) = preprocess_with_line_markers_counted(unit);
    let marked_elapsed = elapsed_us(marked_start);
    let marked_metrics = PreparationStageMetrics {
        elapsed_us: marked_elapsed,
        clang_invocations: u8::from(marked_invoked),
        outcome: if marked.is_ok() {
            PreparationStageOutcome::Succeeded
        } else {
            PreparationStageOutcome::Failed
        },
    };
    let inventory = inventory_from_line_marked_result(unit, marked.as_deref());
    let locations = match (marked.as_deref(), validated.as_deref()) {
        (Ok(source), Ok(preprocessed_source)) => {
            map_marked_preprocessing(source, preprocessed_source)
        }
        _ => SourceLineMap::expansion_unavailable(),
    };
    let (line_marked_source, line_marked_error) = match marked {
        Ok(source) => (Some(source), None),
        Err(error) => (None, Some(error)),
    };
    let (preprocessed_source, validation_error) = match validated {
        Ok(source) => (Some(source), None),
        Err(error) => (None, Some(error)),
    };
    PreparedTranslationUnit {
        raw_source: unit.source.to_owned(),
        preprocessed_source,
        validation_error,
        line_marked_source,
        line_marked_error,
        inventory,
        locations,
        dialect: unit.dialect,
        function_scope: unit.function_scope,
        include_paths: unit.include_paths.clone(),
        definitions: unit.definitions.clone(),
        preprocessor_options: unit.preprocessor_options.clone(),
        input: unit.input.clone(),
        metrics: PreparationMetrics {
            plain_preprocessing: plain_metrics,
            syntax_validation: syntax_metrics,
            line_marked_preprocessing: marked_metrics,
        },
    }
}
