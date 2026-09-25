//! Diagnostic inventory only: never changes analysis verdicts or certificates.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use qcore_c_gateway::legacy_c::{
    LegacyCError, LegacyDiagnostic, LegacyDiagnosticKind, SourceDefinitionInventory, SourceLineMap,
    UnrestrictedLegacyResult, unlocated_site,
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
    reason_code_value(reason.map(ReasonCategory::classify).and_then(Result::ok))
}

/// The report value for a category a producer already classified.
///
/// Rows whose producer carries a typed category — `ChunkSourceResult::Unknown`
/// since REASON-CODES-B2 — go through here instead of being re-classified from
/// their own free text, so the producer and the report cannot disagree. The
/// fail-closed shape is identical: `None` serializes as `null` and asserts
/// nothing.
pub(crate) fn reason_code_value(category: Option<ReasonCategory>) -> Value {
    match category {
        Some(category) => json!(category.as_str()),
        None => Value::Null,
    }
}

/// A refusal of one selected configuration, carrying its closed §L21.1
/// category beside its text.
///
/// Card REASON-CODES-B1. Most refusal texts on this route classify under
/// `ReasonCategory::classify`, but a few are raw OS or tool text -
/// `std::io::Error::to_string()` from reading the selected source, and the
/// `canonicalize`/`git show` failures inside `verify_pinned_bytes`. Those carry
/// no stable marker, so classifying from prose returns `null`: honest (no
/// claim) but incomplete (no code). This type lets such a site name its
/// category where the text cannot, without changing one byte of the text
/// (§L21.1 rule 2).
///
/// Naming a category here can only narrow what a row claims: the sites that use
/// [`Self::environment`] are OS and tool failures, and
/// `ReasonCategory::EnvironmentUnavailable` is the one category that says
/// nothing about the analysed source and can never contribute to a provisional
/// `Unsafe` (§L21.1 rule 1). There is deliberately no constructor that names a
/// source-subject category for opaque text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRefusal {
    reason: String,
    category: Option<ReasonCategory>,
}

impl UnitRefusal {
    /// A refusal whose text the closed vocabulary is expected to recognise.
    /// Fail-closed exactly as before: an unrecognised text carries no category.
    pub fn classified(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let category = ReasonCategory::classify(&reason).ok();
        Self { reason, category }
    }

    /// A refusal minted from opaque OS or tool text, whose category is named at
    /// the site because the text cannot carry it.
    pub fn environment(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            category: Some(ReasonCategory::EnvironmentUnavailable),
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn category(&self) -> Option<ReasonCategory> {
        self.category
    }

    /// Drop the category for a caller that only has a `String` channel. This is
    /// a narrowing, never a relabeling: the text is unchanged.
    pub fn into_reason(self) -> String {
        self.reason
    }
}

impl From<String> for UnitRefusal {
    fn from(reason: String) -> Self {
        Self::classified(reason)
    }
}

impl From<&str> for UnitRefusal {
    fn from(reason: &str) -> Self {
        Self::classified(reason)
    }
}

pub struct AuditReport {
    revision: String,
    units: Vec<Value>,
    diagnostic_counts: BTreeMap<String, usize>,
    source_contracts: Option<Value>,
    /// Card W44-QUIRIN-AUDIT-PROVIDERS: a contracts-only run's evidence mode
    /// and the contract modules its verdicts rest on. Absent otherwise, so
    /// every other report keeps its shape.
    evidence: Option<Value>,
}

impl AuditReport {
    pub fn new(revision: &str) -> Self {
        Self {
            revision: revision.into(),
            units: Vec::new(),
            diagnostic_counts: BTreeMap::new(),
            source_contracts: None,
            evidence: None,
        }
    }

    /// An empty report over the same revision, into which one unit's rows are
    /// recorded apart from every other unit's (card
    /// **W44-AUDIT-PARALLEL-UNITS**). Fold it back with [`Self::absorb`].
    pub fn fragment(&self) -> Self {
        Self::new(&self.revision)
    }

    /// Append a [`Self::fragment`]'s rows after this report's rows and add its
    /// diagnostic counts.
    ///
    /// Recording units into fragments and absorbing the fragments in unit order
    /// yields exactly the report that recording every unit here in that order
    /// would: rows are appended in order, the only cross-unit aggregate is a
    /// per-category sum, and every attachment (`record_enumeration`,
    /// `record_configuration`) targets the row its own unit just recorded,
    /// which lives in the same fragment. A fragment of another revision, or one
    /// carrying source-contract evidence (which is whole-report data, never
    /// per-unit), is refused rather than merged.
    pub fn absorb(&mut self, fragment: Self) -> Result<(), String> {
        let Self {
            revision,
            units,
            diagnostic_counts,
            source_contracts,
            evidence,
        } = fragment;
        if revision != self.revision {
            return Err(format!(
                "report fragment for revision {revision} cannot join a report for {}",
                self.revision
            ));
        }
        if source_contracts.is_some() {
            return Err("a per-unit report fragment carries no source-contract evidence".into());
        }
        if evidence.is_some() {
            return Err("a per-unit report fragment carries no evidence-mode record".into());
        }
        self.units.extend(units);
        for (category, count) in diagnostic_counts {
            *self.diagnostic_counts.entry(category).or_default() += count;
        }
        Ok(())
    }

    pub(crate) fn set_source_contracts(&mut self, evidence: Value) {
        self.source_contracts = Some(evidence);
    }

    /// Record the evidence mode of the whole run (card W44-QUIRIN-AUDIT-PROVIDERS).
    pub(crate) fn set_evidence(&mut self, evidence: Value) {
        self.evidence = Some(evidence);
    }

    /// Retain a selected configuration whose source/build premises could not
    /// be established. Missing evidence is never a data-only or Safe unit.
    pub fn record_unknown(
        &mut self,
        library: &str,
        file: &str,
        functions: &[String],
        refusal: &UnitRefusal,
    ) {
        let reason = refusal.reason();
        self.units.push(json!({
            "library": library, "file": file, "status": "Unknown", "reason": reason,
            "reason_code": reason_code_value(refusal.category()),
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

    /// Card **GATEWAY-C-LOCATION**: render one diagnostic's site.
    ///
    /// The presented coordinate is the **original** pre-preprocessing
    /// `file:line`, because the preprocessed translation unit is a text nobody
    /// reads. The preprocessed position stays in the row's `line` key, which is
    /// the leaf identity every pinned corpus comparison is keyed on; it reaches
    /// presented text only as the explicit fallback when no original coordinate
    /// was established, where it names itself as internal.
    fn site(locations: Option<&SourceLineMap>, file: &str, line: usize) -> String {
        locations
            .and_then(|map| map.present(line, file))
            .unwrap_or_else(|| unlocated_site(line))
    }

    /// The row value for an original coordinate: the presented `file:line`, or
    /// `null` where none is established. `null` asserts nothing.
    fn origin(locations: Option<&SourceLineMap>, file: &str, line: usize) -> Value {
        match locations.and_then(|map| map.present(line, file)) {
            Some(origin) => json!(origin),
            None => Value::Null,
        }
    }

    pub fn record(
        &mut self,
        library: &str,
        file: &str,
        source_functions: &[String],
        result: &Result<UnrestrictedLegacyResult, LegacyCError>,
        locations: Option<&SourceLineMap>,
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
                    let (category, description) = diagnostic_description(diagnostic, &|line| {
                        Self::site(locations, file, line)
                    });
                    *counts.entry(category.into()).or_default() += 1;
                    *self.diagnostic_counts.entry(category.into()).or_default() += 1;
                    if examples.len() < 3 {
                        examples.push(format!(
                            "{}: {}: {category}: {description}",
                            diagnostic.function,
                            Self::site(locations, file, diagnostic.line),
                        ));
                    }
                    diagnostics.push(json!({
                        "line": diagnostic.line,
                        "origin": Self::origin(locations, file, diagnostic.line),
                        "category": category,
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
            "origin_coordinates": origin_coordinates(locations),
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
            "scope": "generic resource projection; separate chunk/callback source profiles are not certificates here; an admitted scalar access through a pointer decides custody only, never bounds, alignment, or the liveness or nullness of an object the projection does not track; an index or pointer traversal (a stepped pointer, a moved index, a typed read in a loop condition) decides custody, use after release and invalid release only: no bounds claim; an indexed collection's element ranges decide which indices owe a release, never that an index is inside the allocation; a typed read handed to a borrowed or data-tail call actual (a scalar, a pointer read, a sub-object address) decides its base's custody only, never the liveness of an object a read pointer names",
            "line_coordinates": "`line` is the Clang -E -P preprocessed translation unit's line, not the original file; line 0 is synthetic. It is retained as the internal leaf identity every pinned comparison is keyed on, and is not a coordinate anyone can open; `origin` beside it is the original pre-preprocessing coordinate and is what diagnostic text presents",
            "origin_meaning": "the original `file:line` of the C a person wrote (card GATEWAY-C-LOCATION), derived from the line markers of the same unit's marked preprocessing and paired content line by content line with the analysed -E -P text; a diagnostic inside a macro expansion carries the expansion site's line. No presented file spelling is ever read from a flagless marker: an included file is named by its own entering (flag 1) marker and the primary input by this report's own `file`, so a `#line` directive in analysed source can neither move the audited scope (standing constraint 12) nor name a file here. A flagless marker that renames its frame withdraws that frame's coordinates instead of adopting the name, and `null` asserts no coordinate at all; `origin_coordinates` carries the per-unit account",
            "not_projected_meaning": "raw source definitions absent from the projection, including build-excluded definitions; see unit reason for parser failures",
            "enumeration_meaning": "the enumerated main-source definition denominator (E01-F1): `absent_from_raw_scan` names main-source definitions the historical raw token scan did not see — produced by macro expansion, or lost by that scan for another reason — each an ordinary definition in the preprocessed unit that the unmodified analyzer reads; `unanalyzable` definitions are counted but not read by the resource analyzer and carry their own classified Unknown row; `completeness: refused` means the definition set itself could not be established on the named ground, so no whole-unit Safe claim is published over it",
            "reason_code_meaning": "closed reason-code vocabulary of QCORE_SPEC_r0_3 L21.1 classifying the sibling reason; null asserts no category, either because there is no reason or because the closed vocabulary does not recognise it; `environment-unavailable` says nothing about the analysed source and never contributes to a provisional Unsafe",
            "diagnostic_counts": self.diagnostic_counts, "units": self.units,
        });
        if let Some(evidence) = &self.source_contracts {
            report["source_contracts"] = evidence.clone();
        }
        if let Some(evidence) = &self.evidence {
            report["evidence"] = evidence.clone();
        }
        let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        fs::write(path, bytes)
            .map_err(|error| format!("could not write {}: {error}", path.display()))
    }
}

/// The per-unit account of the original-coordinate derivation.
///
/// `null` means the derivation was not run for this unit, which happens exactly
/// when the unit publishes no diagnostic to attribute. A refusal is published
/// with its ground rather than left silent, and it is always an
/// `environment-unavailable` fact about this analysis machinery — never a
/// statement about the analysed source (§L21.1 rule 1).
fn origin_coordinates(locations: Option<&SourceLineMap>) -> Value {
    let Some(map) = locations else {
        return Value::Null;
    };
    if map.is_established() {
        return json!({
            "status": "established",
            "withdrawn_lines": map.relabelled_lines(),
        });
    }
    let refusals = map
        .refusals()
        .iter()
        .map(|refusal| {
            json!({
                "ground": refusal.as_str(),
                "reason": refusal.reason(),
                "reason_code": reason_code_value(Some(refusal.category())),
            })
        })
        .collect::<Vec<_>>();
    json!({ "status": "refused", "refusals": refusals })
}

/// Card **GATEWAY-C-LOCATION**: every site this text names is the original
/// pre-preprocessing coordinate, supplied by `site`. The internal preprocessed
/// position appears only through `site`'s explicit fallback.
fn diagnostic_description(
    diagnostic: &LegacyDiagnostic,
    site: &impl Fn(usize) -> String,
) -> (&'static str, String) {
    match &diagnostic.kind {
        LegacyDiagnosticKind::Unsupported { description } => ("unsupported", description.clone()),
        LegacyDiagnosticKind::UnknownAlias { variable } => (
            "unknown_alias",
            format!("unresolved resource identity `{variable}`"),
        ),
        LegacyDiagnosticKind::Leak { acquisition_line } => (
            "leak",
            format!("live obligation acquired at {}", site(*acquisition_line)),
        ),
        LegacyDiagnosticKind::DoubleRelease { acquisition_line } => (
            "double_release",
            format!(
                "repeated release of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::ReleaseAfterEscape { acquisition_line } => (
            "release_after_escape",
            format!(
                "release after transfer of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::CustodyConflict { acquisition_line } => (
            "custody_conflict",
            format!(
                "conflicting custody of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        // Card W43-LIBC-RESULT-ALIAS: the two derived-pointer violation
        // candidates. Provisional, like every other defect category here.
        LegacyDiagnosticKind::InvalidRelease { acquisition_line } => (
            "invalid_release",
            format!(
                "release of a pointer into, not the identity of, the acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::UseAfterRelease { acquisition_line } => (
            "use_after_release",
            format!(
                "use of a pointer derived from the acquisition at {} after its release",
                site(*acquisition_line)
            ),
        ),
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kind candidates, in the
        // same two categories, with their own wording.
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquisition_line,
            acquired,
            released,
        } => (
            "invalid_release",
            format!(
                "{} release of the {} acquired at {}",
                released.spelling(),
                acquired.spelling(),
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::UseAfterClose {
            acquisition_line,
            kind,
        } => (
            "use_after_release",
            format!(
                "use of the {} acquired at {} after its release",
                kind.spelling(),
                site(*acquisition_line)
            ),
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

    /// One acquisition with no release, and one uncontracted call: a leak row
    /// whose site is an acquisition line, and an unsupported row whose site is
    /// its own line.
    const TWO_FUNCTIONS: &str = "void a(void) { void *p = alloc(); } void b(void) { opaque(); }";

    fn two_function_result() -> Result<UnrestrictedLegacyResult, LegacyCError> {
        // W1 literal-site migration: loaded from a `.qcore` fixture module.
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "alloc_acquire.qcore".to_owned(),
            include_str!("../tests/fixtures/alloc_acquire.qcore").to_owned(),
        )]))
        .unwrap();
        Ok(UnrestrictedLegacyResult::Unsafe(
            analyze_legacy_c_translation_unit(TWO_FUNCTIONS, "a", &contracts).unwrap(),
        ))
    }

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
            None,
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

    // -----------------------------------------------------------------------
    // Card GATEWAY-C-LOCATION: the presented coordinate, and the internal
    // position it never replaces.
    // -----------------------------------------------------------------------

    /// A unit with no established coordinate keeps the internal position, and
    /// says it is internal. `origin` is `null`, which asserts nothing.
    #[test]
    fn a_unit_without_coordinates_names_its_internal_position_as_internal() {
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["a".into()],
            &two_function_result(),
            None,
        );
        let unit = &report.units[0];
        let unsupported = &unit["functions"][1]["diagnostics"][0];
        assert_eq!(unsupported["category"], "unsupported");
        assert_eq!(unsupported["line"], 1);
        assert_eq!(unsupported["origin"], Value::Null);
        let leak = &unit["functions"][0]["diagnostics"][0];
        assert_eq!(leak["category"], "leak");
        assert_eq!(
            leak["description"],
            "live obligation acquired at preprocessed line 1"
        );
        assert_eq!(unit["origin_coordinates"], Value::Null);
    }

    /// With a map in force the same rows present the original `file:line` in
    /// both the `origin` key and the diagnostic text, and **still** carry the
    /// unchanged preprocessed `line`.
    #[test]
    fn a_presented_coordinate_replaces_the_text_and_not_the_leaf_identity() {
        // A one-line unit that is line 42 of the file it was written in.
        let marked = format!("# 42 \"<stdin>\"\n{TWO_FUNCTIONS}\n");
        let map = qcore_c_gateway::legacy_c::map_marked_preprocessing(
            &marked,
            &format!("{TWO_FUNCTIONS}\n"),
        );
        assert!(map.is_established(), "{:?}", map.refusals());
        let mut report = AuditReport::new("revision");
        let plain = {
            let mut plain = AuditReport::new("revision");
            plain.record(
                "test",
                "src/unit.c",
                &["a".into()],
                &two_function_result(),
                None,
            );
            plain
        };
        report.record(
            "test",
            "src/unit.c",
            &["a".into()],
            &two_function_result(),
            Some(&map),
        );
        let unit = &report.units[0];
        let unsupported = &unit["functions"][1]["diagnostics"][0];
        assert_eq!(
            unsupported["line"], plain.units[0]["functions"][1]["diagnostics"][0]["line"],
            "the internal leaf identity is untouched by the presentation"
        );
        assert_eq!(unsupported["origin"], "src/unit.c:42");
        let leak = &unit["functions"][0]["diagnostics"][0];
        assert_eq!(
            leak["description"], "live obligation acquired at src/unit.c:42",
            "presented text names the original coordinate, never the preprocessed one"
        );
        assert_eq!(
            unit["origin_coordinates"],
            json!({"status": "established", "withdrawn_lines": 0})
        );
        // Only the presentation moved: category, status and counts are equal.
        assert_eq!(unit["status"], plain.units[0]["status"]);
        assert_eq!(
            unit["diagnostic_counts"],
            plain.units[0]["diagnostic_counts"]
        );
        assert_eq!(unsupported["category"], "unsupported");
    }

    #[test]
    fn a_refused_derivation_is_published_as_an_environment_ground() {
        let map = qcore_c_gateway::legacy_c::map_marked_preprocessing("no marker here\n", "x\n");
        assert!(!map.is_established());
        let value = origin_coordinates(Some(&map));
        assert_eq!(value["status"], "refused");
        assert_eq!(
            value["refusals"][0]["reason_code"],
            "environment-unavailable"
        );
        // L21.1 rule 1: a ground about this machinery can never support Unsafe.
        assert!(!ReasonCategory::EnvironmentUnavailable.may_support_unsafe());
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
            None,
        );
        assert_eq!(report.units[0]["reason"], "projection failure");
        assert_eq!(report.units[0]["functions"], json!([]));
        assert_eq!(report.units[0]["not_projected"], json!(["missing"]));
    }

    // -----------------------------------------------------------------------
    // Card REASON-CODES: the audit report's per-unit reason category.
    // -----------------------------------------------------------------------

    #[test]
    fn opaque_os_text_names_its_category_at_the_site_without_changing_its_text() {
        // Card REASON-CODES-B1. `std::io::Error::to_string()` carries no marker
        // the closed vocabulary recognises, so classifying from prose yields
        // null. The three reachable OS/tool sites on the resource-projection
        // route now name their category instead, and the text is byte-identical.
        let text = "No such file or directory (os error 2)";
        assert_eq!(reason_code(Some(text)), Value::Null);
        let refusal = UnitRefusal::environment(text);
        assert_eq!(refusal.reason(), text);
        assert_eq!(
            refusal.category(),
            Some(ReasonCategory::EnvironmentUnavailable)
        );
        let mut report = AuditReport::new("revision");
        report.record_unknown("test", "unit.c", &[], &refusal);
        let unit = &report.units[0];
        assert_eq!(unit["reason"], text);
        assert_eq!(unit["reason_code"], "environment-unavailable");
        // The named category is the one subject that can never be read as a
        // statement about the analysed source (L21.1 rule 1).
        assert!(!ReasonCategory::EnvironmentUnavailable.describes_analysed_source());
    }

    #[test]
    fn a_classified_refusal_still_fails_closed_on_text_nobody_recognises() {
        // `classified` is the ordinary constructor and must not acquire a
        // category from nowhere: there is no fallback, and in particular no
        // fallback onto a source-subject category.
        let refusal = UnitRefusal::classified("something nobody has ever emitted");
        assert_eq!(refusal.category(), None);
        let mut report = AuditReport::new("revision");
        report.record_unknown("test", "unit.c", &[], &refusal);
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn a_producer_supplied_category_is_published_without_being_reclassified() {
        // Card REASON-CODES-B2/B3: rows whose producer already classified their
        // reason (ChunkSourceResult::Unknown) publish that category rather than
        // re-deriving it here, so the two cannot disagree. The fail-closed shape
        // is unchanged: an unclassified row is null, never a defaulted category.
        assert_eq!(
            reason_code_value(Some(ReasonCategory::UnsupportedSyntax)),
            json!("unsupported-syntax")
        );
        assert_eq!(
            reason_code_value(Some(ReasonCategory::EnvironmentUnavailable)),
            json!("environment-unavailable")
        );
        assert_eq!(reason_code_value(None), Value::Null);
        // The two routes agree on a reason both can see.
        let reason = "outside single borrowed chunk / int-or-void-result source profile";
        assert_eq!(
            reason_code(Some(reason)),
            reason_code_value(ReasonCategory::classify(reason).ok())
        );
    }

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
            &UnitRefusal::classified("No such file or directory (os error 2)"),
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
                      compilation database selects /usr/bin/cc. Prepare an explicit \
                      Clang build (scripts/prepare_cmetrics_clang_audit.py, \
                      scripts/prepare_cycle19_monkey.py) and run quirin-audit current \
                      over it; GCC compiler equivalence is unproven";
        let mut report = AuditReport::new("revision");
        report.record_unknown(
            "test",
            "unit.c",
            &["f".into()],
            &UnitRefusal::classified(reason),
        );
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
                None,
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
        report.record_unknown(
            "lib",
            "unit.c",
            &["a".into(), "b".into()],
            &UnitRefusal::classified(reason),
        );
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

    /// Card W44-AUDIT-PARALLEL-UNITS: units recorded into fragments that are
    /// computed on several threads and completed in reverse order, then
    /// absorbed in unit order, write exactly the bytes of one report recorded
    /// sequentially.
    #[test]
    fn fragments_absorbed_in_unit_order_write_the_sequential_report() {
        fn record_unit(report: &mut AuditReport, index: usize) {
            let file = format!("unit{index}.c");
            match index % 3 {
                0 => {
                    report.record(
                        "test",
                        &file,
                        &["a".into(), "b".into()],
                        &two_function_result(),
                        None,
                    );
                }
                1 => report.record_data_only("test", &file, None),
                _ => report.record_unknown(
                    "test",
                    &file,
                    &["c".into()],
                    &UnitRefusal::environment(format!("unit {index} unreadable")),
                ),
            }
            report.record_configuration(json!({ "unit": index }));
        }
        let directory = std::env::temp_dir().join(format!(
            "qcore-report-fragments-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut sequential = AuditReport::new("revision");
        for index in 0..12 {
            record_unit(&mut sequential, index);
        }
        sequential
            .write(&directory.join("sequential.json"))
            .unwrap();
        for jobs in [1, 4, 12] {
            let mut folded = AuditReport::new("revision");
            let template = AuditReport::new("revision");
            crate::parallel::run_ordered(
                (0..12).collect::<Vec<usize>>(),
                jobs,
                Some((0..12).rev().collect()),
                |index| {
                    std::thread::sleep(std::time::Duration::from_millis((12 - index as u64) * 3));
                    let mut fragment = template.fragment();
                    record_unit(&mut fragment, index);
                    fragment
                },
                |fragment| folded.absorb(fragment),
            )
            .unwrap();
            let path = directory.join(format!("folded-{jobs}.json"));
            folded.write(&path).unwrap();
            assert_eq!(
                fs::read(&path).unwrap(),
                fs::read(directory.join("sequential.json")).unwrap(),
                "jobs = {jobs}"
            );
        }
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_fragment_of_another_revision_is_refused() {
        let mut report = AuditReport::new("revision");
        let mut other = AuditReport::new("other");
        other.record_data_only("test", "unit.c", None);
        assert!(report.absorb(other).is_err());
        assert!(report.units.is_empty());
        let mut evidence = report.fragment();
        evidence.set_source_contracts(json!({}));
        assert!(report.absorb(evidence).is_err());
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
