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

/// Measured stages and external-process count for one preparation.
///
/// Timings are observations of this run and make no throughput or speedup
/// claim. `clang_invocations` counts successfully spawned Clang processes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparationMetrics {
    pub plain_preprocessing_elapsed_us: u64,
    pub syntax_validation_elapsed_us: u64,
    pub line_marked_preprocessing_elapsed_us: u64,
    pub clang_invocations: u8,
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
    preprocessed_source: String,
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

    #[must_use]
    pub fn preprocessed_source(&self) -> &str {
        &self.preprocessed_source
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

fn preprocess_plain(unit: &C11TranslationUnit<'_>) -> Result<(String, u64), LegacyCError> {
    let start = Instant::now();
    let mut command = crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
    command.args(["-x", "c", unit.dialect.clang_argument(), "-E", "-P"]);
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
    Ok((preprocessed_source, elapsed_us(start)))
}

fn validate_syntax(
    unit: &C11TranslationUnit<'_>,
    preprocessed_source: &str,
) -> Result<u64, LegacyCError> {
    let start = Instant::now();
    let mut command = crate::clang::command().map_err(LegacyCError::ProductionParserUnavailable)?;
    unit.append_frontend_flags(&mut command);
    let mut child = command
        .args([
            "-x",
            "c",
            unit.dialect.clang_argument(),
            "-fsyntax-only",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| LegacyCError::ProductionParserUnavailable(error.to_string()))?;
    child
        .stdin
        .take()
        .ok_or_else(|| LegacyCError::ProductionParserUnavailable("missing syntax stdin".into()))?
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
    Ok(elapsed_us(start))
}

pub(super) fn validate_plain_and_syntax(
    unit: &C11TranslationUnit<'_>,
) -> Result<(String, u64, u64), LegacyCError> {
    let (preprocessed_source, plain_elapsed) = preprocess_plain(unit)?;
    let syntax_elapsed = validate_syntax(unit, &preprocessed_source)?;
    Ok((preprocessed_source, plain_elapsed, syntax_elapsed))
}

/// Prepare one translation unit exactly once for identity, enumeration,
/// analysis and diagnostic reporting within the current run.
pub fn prepare_translation_unit(
    unit: &C11TranslationUnit<'_>,
) -> Result<PreparedTranslationUnit, LegacyCError> {
    let (preprocessed_source, plain_preprocessing_elapsed_us, syntax_validation_elapsed_us) =
        validate_plain_and_syntax(unit)?;

    let marked_start = Instant::now();
    let (marked, marked_invoked) = preprocess_with_line_markers_counted(unit);
    let line_marked_preprocessing_elapsed_us = elapsed_us(marked_start);
    let inventory = inventory_from_line_marked_result(unit, marked.as_deref());
    let locations = match marked.as_deref() {
        Ok(source) => map_marked_preprocessing(source, &preprocessed_source),
        Err(_) => SourceLineMap::expansion_unavailable(),
    };
    let (line_marked_source, line_marked_error) = match marked {
        Ok(source) => (Some(source), None),
        Err(error) => (None, Some(error)),
    };
    Ok(PreparedTranslationUnit {
        raw_source: unit.source.to_owned(),
        preprocessed_source,
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
            plain_preprocessing_elapsed_us,
            syntax_validation_elapsed_us,
            line_marked_preprocessing_elapsed_us,
            clang_invocations: 2 + u8::from(marked_invoked),
        },
    })
}
