//! Diagnostic inventory only: never changes analysis verdicts or certificates.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qcore_compiler::{
    LegacyCError, LegacyDiagnostic, LegacyDiagnosticKind, UnrestrictedLegacyResult,
};
use serde_json::{Value, json};

pub struct AuditReport {
    revision: String,
    units: Vec<Value>,
    diagnostic_counts: BTreeMap<String, usize>,
}

impl AuditReport {
    pub fn new(revision: &str) -> Self {
        Self {
            revision: revision.into(),
            units: Vec::new(),
            diagnostic_counts: BTreeMap::new(),
        }
    }

    pub fn record_data_only(&mut self, library: &str, file: &str, error: Option<&LegacyCError>) {
        self.units.push(json!({
            "library": library, "file": file,
            "status": if error.is_some() { "gateway_failure" } else { "data_only" },
            "reason": error.map(ToString::to_string),
            "source_functions": [], "functions": [], "not_projected": [],
            "diagnostic_counts": {},
        }));
    }

    pub fn record(
        &mut self,
        library: &str,
        file: &str,
        source_functions: &[String],
        result: &Result<UnrestrictedLegacyResult, LegacyCError>,
    ) -> Vec<String> {
        let (status, summary, reason) = match result {
            Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
                ("Safe", Some(&certificate.summary), None)
            }
            Ok(UnrestrictedLegacyResult::Unsafe(summary)) => ("Unsafe", Some(summary), None),
            Ok(UnrestrictedLegacyResult::Unknown {
                partial, reason, ..
            }) => ("Unknown", partial.as_ref(), Some(reason.clone())),
            Err(error) => ("gateway_failure", None, Some(error.to_string())),
        };
        let mut functions = Vec::new();
        let mut counts = BTreeMap::<String, usize>::new();
        let mut examples = Vec::new();
        if let Some(summary) = summary {
            for function in summary.functions.values() {
                let mut diagnostics = Vec::new();
                for diagnostic in &function.diagnostics {
                    let (category, description) = diagnostic_description(diagnostic);
                    *counts.entry(category.into()).or_default() += 1;
                    *self.diagnostic_counts.entry(category.into()).or_default() += 1;
                    if examples.len() < 3 {
                        examples.push(format!(
                            "{}: preprocessed line {}: {category}: {description}",
                            diagnostic.function, diagnostic.line,
                        ));
                    }
                    diagnostics.push(json!({
                        "line": diagnostic.line, "category": category,
                        "description": description,
                    }));
                }
                functions.push(json!({
                    "function": function.function,
                    "status": format!("{:?}", function.status),
                    "diagnostics": diagnostics,
                }));
            }
        }
        let not_projected = source_functions
            .iter()
            .filter(|name| summary.is_none_or(|summary| !summary.functions.contains_key(*name)))
            .collect::<Vec<_>>();
        self.units.push(json!({
            "library": library, "file": file, "status": status, "reason": reason,
            "source_functions": source_functions, "functions": functions,
            "not_projected": not_projected, "diagnostic_counts": counts,
        }));
        if counts.is_empty() {
            return Vec::new();
        }
        let total: usize = counts.values().sum();
        let mut lines = vec![format!(
            "resource diagnostics: {}",
            counts
                .iter()
                .map(|(category, count)| format!("{category}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        )];
        lines.extend(examples);
        if total > 3 {
            lines.push(format!(
                "{} more diagnostic(s); full inventory in optional resource report",
                total - 3
            ));
        }
        lines
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let report = json!({
            "schema_version": 1, "fluent_bit_revision": self.revision,
            "scope": "generic resource projection; separate chunk/callback source profiles are not certificates here",
            "line_coordinates": "Clang -E -P preprocessed translation unit, not original file; line 0 is synthetic",
            "not_projected_meaning": "raw source definitions absent from the projection, including build-excluded definitions; see unit reason for parser failures",
            "diagnostic_counts": self.diagnostic_counts, "units": self.units,
        });
        let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        fs::write(path, bytes)
            .map_err(|error| format!("could not write {}: {error}", path.display()))
    }
}

fn diagnostic_description(diagnostic: &LegacyDiagnostic) -> (&'static str, String) {
    match &diagnostic.kind {
        LegacyDiagnosticKind::Unsupported { description } => ("unsupported", description.clone()),
        LegacyDiagnosticKind::UnknownAlias { variable } => (
            "unknown_alias",
            format!("unresolved resource identity `{variable}`"),
        ),
        LegacyDiagnosticKind::Leak { acquisition_line } => (
            "leak",
            format!("live obligation acquired at preprocessed line {acquisition_line}"),
        ),
        LegacyDiagnosticKind::DoubleRelease { acquisition_line } => (
            "double_release",
            format!("repeated release of acquisition at preprocessed line {acquisition_line}"),
        ),
        LegacyDiagnosticKind::ReleaseAfterEscape { acquisition_line } => (
            "release_after_escape",
            format!(
                "release after transfer of acquisition at preprocessed line {acquisition_line}"
            ),
        ),
        LegacyDiagnosticKind::CustodyConflict { acquisition_line } => (
            "custody_conflict",
            format!("conflicting custody of acquisition at preprocessed line {acquisition_line}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_compiler::{ResourceApiKind, ResourceContracts, analyze_legacy_c_translation_unit};

    #[test]
    fn inventory_keeps_unknown_diagnostics_inside_unsafe_units_and_unprojected_names() {
        let contracts = ResourceContracts::new([("alloc", ResourceApiKind::Acquire)]);
        let summary = analyze_legacy_c_translation_unit(
            "void a(void) { void *p = alloc(); } void b(void) { opaque(); }",
            "a",
            &contracts,
        )
        .unwrap();
        let result = Ok(UnrestrictedLegacyResult::Unsafe(summary));
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["a".into(), "b".into(), "excluded".into()],
            &result,
        );
        let unit = &report.units[0];
        assert_eq!(unit["status"], "Unsafe");
        assert_eq!(unit["not_projected"], json!(["excluded"]));
        assert_eq!(unit["functions"][1]["status"], "Unknown");
        assert_eq!(
            unit["functions"][1]["diagnostics"][0]["description"],
            "call to uncontracted function `opaque`"
        );
        assert_eq!(report.diagnostic_counts["unsupported"], 1);
        assert_eq!(report.diagnostic_counts["leak"], 1);
    }

    #[test]
    fn inventory_retains_failure_reason_without_a_partial_summary() {
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["missing".into()],
            &Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source: String::new(),
                reason: "projection failure".into(),
                partial: None,
            }),
        );
        assert_eq!(report.units[0]["reason"], "projection failure");
        assert_eq!(report.units[0]["functions"], json!([]));
        assert_eq!(report.units[0]["not_projected"], json!(["missing"]));
    }
}
