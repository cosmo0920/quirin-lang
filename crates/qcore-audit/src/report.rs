//! Diagnostic inventory only: never changes analysis verdicts or certificates.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qcore_c_gateway::legacy_c::{
    LegacyCError, LegacyDiagnostic, LegacyDiagnosticKind, SourceDefinitionInventory,
    UnrestrictedLegacyResult,
};
use qcore_schemas::reason::ReasonCategory;
use serde_json::{Value, json};

/// The closed L21.1 category of a per-unit reason, as a report value.
///
/// This is the second carrier of the reason-code vocabulary
/// (`docs/QCORE_SPEC_r0_3.md` §L21.1); the first is
/// `qcore_schemas::obligation::DerivationIncomplete::category`. It *refines*
/// the row: `status`, `reason`, `source_functions`, `functions`,
/// `not_projected` and `diagnostic_counts` keep their exact meaning and bytes.
///
/// Fail-closed in two directions:
///
/// * A unit with no reason carries no category — `null`, not a guess.
/// * A reason the closed vocabulary does not recognise also carries `null`.
///   `null` asserts nothing, so an unrecognised row can never be read as a
///   statement about the analysed source (§L21.1 rule 1). It is never
///   defaulted to a category.
///   `qcore_schemas`'s `reason_codes` test enumerates the reason families this
///   workspace emits and pins that each one classifies.
pub(crate) fn reason_code(reason: Option<&str>) -> Value {
    match reason.map(ReasonCategory::classify) {
        Some(Ok(category)) => json!(category.as_str()),
        Some(Err(_)) | None => Value::Null,
    }
}

pub struct AuditReport {
    revision: String,
    units: Vec<Value>,
    diagnostic_counts: BTreeMap<String, usize>,
    source_contracts: Option<Value>,
}

impl AuditReport {
    pub fn new(revision: &str) -> Self {
        Self {
            revision: revision.into(),
            units: Vec::new(),
            diagnostic_counts: BTreeMap::new(),
            source_contracts: None,
        }
    }

    pub(crate) fn set_source_contracts(&mut self, evidence: Value) {
        self.source_contracts = Some(evidence);
    }

    /// Retain a selected configuration whose source/build premises could not
    /// be established. Missing evidence is never a data-only or Safe unit.
    pub fn record_unknown(
        &mut self,
        library: &str,
        file: &str,
        functions: &[String],
        reason: &str,
    ) {
        self.units.push(json!({
            "library": library, "file": file, "status": "Unknown", "reason": reason,
            "reason_code": reason_code(Some(reason)),
            "source_functions": functions, "functions": [], "not_projected": functions,
            "diagnostic_counts": {},
        }));
    }

    /// Attach the enumerated main-source definition inventory to the
    /// immediately preceding unit (unit **E01-F1**).
    ///
    /// This block *refines* the row and replaces nothing: `source_functions`
    /// is the same denominator the analyzer selected over, and
    /// `not_projected` keeps its meaning. What it adds is the honest account
    /// of that denominator — which definitions exist only after preprocessing,
    /// which ones no implemented analysis reads and why, and whether the
    /// definition set was established at all. A refused completeness claim is
    /// a scoped `Unknown` about the *inventory*, never a claim about the
    /// analysed functions.
    pub fn record_enumeration(&mut self, inventory: &SourceDefinitionInventory) {
        let unanalyzable = inventory
            .unanalyzable()
            .iter()
            .map(|definition| {
                let reason = definition.form().reason();
                json!({
                    "function": definition.function(),
                    "status": "unknown",
                    "form": definition.form().as_str(),
                    "reason": reason,
                    "reason_code": reason_code(Some(reason)),
                })
            })
            .collect::<Vec<_>>();
        let refusals = inventory
            .refusals()
            .iter()
            .map(|refusal| {
                let reason = refusal.reason();
                json!({
                    "ground": refusal.as_str(),
                    "reason": reason,
                    "reason_code": reason_code(Some(reason)),
                })
            })
            .collect::<Vec<_>>();
        let Some(unit) = self.units.last_mut() else {
            return;
        };
        unit["enumeration"] = json!({
            "completeness": if inventory.is_complete() { "established" } else { "refused" },
            "refusals": refusals,
            "absent_from_raw_scan": inventory.absent_from_raw_scan(),
            "unanalyzable": unanalyzable,
        });
    }

    /// Attach the exact configuration inventory to the immediately preceding
    /// unit, including rejected commands which cannot mint a compile-task proof.
    pub fn record_configuration(&mut self, configuration: Value) {
        if let Some(unit) = self.units.last_mut() {
            unit["compile_configuration"] = configuration;
        }
    }

    pub fn record_data_only(&mut self, library: &str, file: &str, error: Option<&LegacyCError>) {
        let reason = error.map(ToString::to_string);
        self.units.push(json!({
            "library": library, "file": file,
            "status": if error.is_some() { "gateway_failure" } else { "data_only" },
            "reason": reason,
            "reason_code": reason_code(reason.as_deref()),
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
            "reason_code": reason_code(reason.as_deref()),
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
        self.write_with_revision_key(path, "source_revision")
    }

    /// Compatibility with an existing report schema; verdict contents are unchanged.
    pub fn write_with_revision_key(&self, path: &Path, revision_key: &str) -> Result<(), String> {
        let mut report = json!({
            "schema_version": 1, revision_key: self.revision,
            "scope": "generic resource projection; separate chunk/callback source profiles are not certificates here",
            "line_coordinates": "Clang -E -P preprocessed translation unit, not original file; line 0 is synthetic",
            "not_projected_meaning": "raw source definitions absent from the projection, including build-excluded definitions; see unit reason for parser failures",
            "enumeration_meaning": "the enumerated main-source definition denominator (E01-F1): `absent_from_raw_scan` names main-source definitions the historical raw token scan did not see — produced by macro expansion, or lost by that scan for another reason — each an ordinary definition in the preprocessed unit that the unmodified analyzer reads; `unanalyzable` definitions are counted but not read by the resource analyzer and carry their own classified Unknown row; `completeness: refused` means the definition set itself could not be established on the named ground, so no whole-unit Safe claim is published over it",
            "reason_code_meaning": "closed reason-code vocabulary of QCORE_SPEC_r0_3 L21.1 classifying the sibling reason; null asserts no category, either because there is no reason or because the closed vocabulary does not recognise it; `environment-unavailable` says nothing about the analysed source and never contributes to a provisional Unsafe",
            "diagnostic_counts": self.diagnostic_counts, "units": self.units,
        });
        if let Some(evidence) = &self.source_contracts {
            report["source_contracts"] = evidence.clone();
        }
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
        LegacyDiagnosticKind::SummaryConflict { inferred, declared } => (
            "summary_conflict",
            format!(
                "inferred custody summary {inferred:?} contradicts the declared contract \
                 {declared:?}; the declared contract governs every call site"
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit;
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use std::collections::BTreeMap;

    #[test]
    fn inventory_keeps_unknown_diagnostics_inside_unsafe_units_and_unprojected_names() {
        // W1 literal-site migration: loaded from a `.qcore` fixture module.
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "alloc_acquire.qcore".to_owned(),
            include_str!("../tests/fixtures/alloc_acquire.qcore").to_owned(),
        )]))
        .unwrap();
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

    // -----------------------------------------------------------------------
    // Card REASON-CODES: the audit report's per-unit reason category.
    // -----------------------------------------------------------------------

    #[test]
    fn a_unit_without_a_reason_asserts_no_category() {
        assert_eq!(reason_code(None), Value::Null);
        let mut report = AuditReport::new("revision");
        report.record_data_only("test", "data.c", None);
        assert_eq!(report.units[0]["reason"], Value::Null);
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn an_unrecognised_reason_asserts_no_category_rather_than_a_source_defect() {
        // Fail-closed: `null` makes no claim. It is never a category, and in
        // particular never one whose subject is the analysed source.
        assert_eq!(
            reason_code(Some("No such file or directory (os error 2)")),
            Value::Null
        );
        let mut report = AuditReport::new("revision");
        report.record_unknown(
            "test",
            "unit.c",
            &[],
            "No such file or directory (os error 2)",
        );
        assert_eq!(
            report.units[0]["reason"],
            "No such file or directory (os error 2)"
        );
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn a_missing_clang_is_recorded_as_an_environment_failure_not_a_source_defect() {
        let reason = "audit requires the configured Clang executable (/usr/bin/clang); \
                      compilation database selects /usr/bin/cc. Generate an explicit \
                      Clang build with scripts/prepare_cycle19_clang_audit.py --run; \
                      GCC compiler equivalence is unproven";
        let mut report = AuditReport::new("revision");
        report.record_unknown("test", "unit.c", &["f".into()], reason);
        let unit = &report.units[0];
        assert_eq!(unit["status"], "Unknown");
        assert_eq!(unit["reason_code"], "environment-unavailable");
        let category = ReasonCategory::classify(reason).unwrap();
        assert!(!category.describes_analysed_source());
        assert!(!category.may_support_unsafe());
    }

    #[test]
    fn the_projection_summary_separates_unmodeled_constructs_from_open_custody() {
        let mut report = AuditReport::new("revision");
        for (reason, expected) in [
            (
                "resource projection unresolved: 2 unsupported operation(s), \
                 0 unresolved alias(es), 0 custody summary conflict(s)",
                "unsupported-syntax",
            ),
            (
                "resource projection unresolved: 0 unsupported operation(s), \
                 1 unresolved alias(es), 0 custody summary conflict(s)",
                "residual-custody",
            ),
        ] {
            report.record(
                "test",
                "unit.c",
                &[],
                &Ok(UnrestrictedLegacyResult::Unknown {
                    preprocessed_source: String::new(),
                    reason: reason.into(),
                    partial: None,
                }),
            );
            let unit = report.units.last().unwrap();
            assert_eq!(unit["reason"], reason, "the reason text is untouched");
            assert_eq!(unit["reason_code"], expected);
        }
    }

    #[test]
    fn the_category_refines_the_unit_and_replaces_nothing() {
        // L21.1 rule 2: every pre-existing key keeps its exact value.
        let reason = "resource projection unresolved: 1 unsupported operation(s), \
                      0 unresolved alias(es), 0 custody summary conflict(s)";
        let mut report = AuditReport::new("revision");
        report.record_unknown("lib", "unit.c", &["a".into(), "b".into()], reason);
        let unit = &report.units[0];
        assert_eq!(unit["library"], "lib");
        assert_eq!(unit["file"], "unit.c");
        assert_eq!(unit["status"], "Unknown");
        assert_eq!(unit["reason"], reason);
        assert_eq!(unit["source_functions"], json!(["a", "b"]));
        assert_eq!(unit["not_projected"], json!(["a", "b"]));
        assert_eq!(unit["functions"], json!([]));
        assert_eq!(unit["diagnostic_counts"], json!({}));
        assert_eq!(unit["reason_code"], "unsupported-syntax");
        // Exactly one key was added to the eight this unit shape already had.
        let keys: Vec<_> = unit.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "diagnostic_counts",
                "file",
                "functions",
                "library",
                "not_projected",
                "reason",
                "reason_code",
                "source_functions",
                "status",
            ]
        );
    }

    #[test]
    fn the_envelope_explains_the_vocabulary_it_now_carries() {
        let directory = std::env::temp_dir().join(format!(
            "qcore-reason-envelope-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("report.json");
        AuditReport::new("revision").write(&path).unwrap();
        let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let meaning = written["reason_code_meaning"].as_str().unwrap();
        assert!(meaning.contains("L21.1"));
        assert!(meaning.contains("environment-unavailable"));
        assert!(meaning.contains("null asserts no category"));
        fs::remove_dir_all(&directory).ok();
    }
}
