//! Provider-independent projection; Unknown stays in the report denominator.
use crate::certificate_context::{
    AuditContractTrustPolicy, check_audit_legacy_safety_certificate_in_context,
    expected_audit_certificate_context,
};
use crate::compile_db::CompileTaskIdentity;
use crate::report::AuditReport;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCError, LegacyProgramSummary,
    PreparedTranslationUnit, SourceDefinitionInventory, SourceLineMap, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit, analyze_prepared_translation_unit_with_cross_unit,
    analyze_unrestricted_c11_with_inventory,
    analyze_unrestricted_c11_with_inventory_and_cross_unit, map_preprocessed_lines,
    validate_unrestricted_c11,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::verdict::Verdict;

/// Card CG16-INDEX-AND-OFFSET-EXTENTS (gateway rule 11s): the environment
/// variable that selects the bounds property and names its JSON-lines output.
/// Unset, no bounds analysis runs and every artifact is byte-identical.
pub const BOUNDS_SELECTION: &str = "QCORE_BOUNDS_PROPERTY";

/// The custody status as the four values the composition joins.
fn custody_value(status: AnalysisStatus) -> qcore_c_gateway::index_extent::BoundsVerdict {
    use qcore_c_gateway::index_extent::BoundsVerdict;
    match status {
        AnalysisStatus::Safe => BoundsVerdict::Safe,
        AnalysisStatus::Pending => BoundsVerdict::Pending,
        AnalysisStatus::Unknown => BoundsVerdict::Unknown,
        AnalysisStatus::Unsafe => BoundsVerdict::Unsafe,
    }
}

/// Card CG16-INDEX-AND-OFFSET-EXTENTS: when the bounds property is selected,
/// record each function's bounds row beside its custody status and the
/// combined verdict of a request naming both. Reads the custody result; never
/// changes it. The environment is the one the route installed (card
/// CHUNKIO-META-LEN-BOUNDS-CONTRACTS, [`crate::bounds_selection`]); a route
/// that installed none runs with the empty environment.
fn record_bounds(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    preprocessed: Option<&str>,
    functions: &[String],
    result: &Result<UnrestrictedLegacyResult, LegacyCError>,
    report: &mut AuditReport,
) {
    use qcore_c_gateway::index_extent::{BoundsVerdict, Request, compose};
    use qcore_c_gateway::index_extent::{
        analyze_bounds_c11, analyze_bounds_preprocessed, row_verdict,
    };
    if std::env::var_os(BOUNDS_SELECTION).is_none() {
        return;
    }
    let contracts = crate::bounds_selection::environment();
    let contracts = contracts.as_ref();
    let analysed = match preprocessed {
        Some(text) => analyze_bounds_preprocessed(text, unit, functions, contracts),
        None => analyze_bounds_c11(unit, functions, contracts),
    };
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14f): the body
    // evidence of every declaration this unit defines, header definitions
    // included; evidence for the declaration, never a row.
    if let Ok(analysed) = &analysed
        && let Some(unit) = crate::bounds_evidence::unit_record(library, file, analysed)
    {
        report.record_bounds_evidence(unit);
    }
    let custody = summary_of(result);
    for name in functions {
        let own = custody.and_then(|summary| summary.functions.get(name));
        let custody = own.map(|function| custody_value(function.status));
        let (row, identity, bounds) = match &analysed {
            Ok(report) => match report.row(name) {
                Some(row) => (
                    serde_json::to_value(row).unwrap_or(serde_json::Value::Null),
                    serde_json::json!({
                        "model_version": report.model_version,
                        "contract_version": report.contract_version,
                        "source_sha256": report.source_sha256,
                        "target": report.target,
                        "contracts_sha256": report.contracts_sha256,
                    }),
                    Some(row_verdict(row)),
                ),
                None => (serde_json::Value::Null, serde_json::Value::Null, None),
            },
            Err(error) => (
                serde_json::json!({
                    "function": name,
                    "verdict": "Unknown",
                    "reasons": [format!("bounds-import-failure: {error}")],
                    "accesses": [],
                }),
                serde_json::Value::Null,
                Some(BoundsVerdict::Unknown),
            ),
        };
        let combined = compose(
            Request {
                custody: true,
                bounds: true,
            },
            custody,
            bounds,
        );
        let mut row = serde_json::json!({
            "library": library, "file": file, "function": name,
            "custody": custody.map(BoundsVerdict::as_str), "bounds": row,
            "identity": identity, "combined": combined.as_str(),
        });
        if let Some(scope) = own.and_then(qcore_c_gateway::loop_lifetime_summary::body_scope) {
            row["lifetime_scope"] = scope.into();
        }
        report.record_bounds(row);
    }
}

/// Card NULL-SAFETY-PROPERTY (gateway rules 14k-14p): the environment
/// variable that selects the null-dereference property and names its
/// JSON-lines output. Unset, no null analysis runs and every artifact is
/// byte-identical.
pub const NULL_SELECTION: &str = "QCORE_NULL_PROPERTY";

/// The custody status as the null property's four values.
fn custody_null_value(status: AnalysisStatus) -> qcore_c_gateway::null_deref::NullVerdict {
    use qcore_c_gateway::null_deref::NullVerdict;
    match status {
        AnalysisStatus::Safe => NullVerdict::Safe,
        AnalysisStatus::Pending => NullVerdict::Pending,
        AnalysisStatus::Unknown => NullVerdict::Unknown,
        AnalysisStatus::Unsafe => NullVerdict::Unsafe,
    }
}

/// What the null row says of the checks of `variable` at `line` (a check of
/// `variable` itself or of an interior address `&variable->…`): `proved` when
/// every one is proved or moved to a proved check, otherwise the worst class
/// met. A custody statement's line is its first; when the row has no such
/// check on that line, the checks within three lines decide, reported as
/// `nearby:<class>` (never counted as proved); `unmatched` when there is none.
fn premise_standing(
    row: &qcore_c_gateway::null_deref::NullFunctionRow,
    line: usize,
    variable: &str,
) -> String {
    use qcore_c_gateway::null_deref::NullCheck;
    use std::collections::BTreeMap;
    let by_id = row
        .checks
        .iter()
        .map(|check| (check.id, check))
        .collect::<BTreeMap<_, _>>();
    fn resolve(
        check: &NullCheck,
        by_id: &BTreeMap<usize, &NullCheck>,
        depth: usize,
    ) -> &'static str {
        match check.outcome {
            "proved" => "proved",
            "moved" if depth < 64 => {
                let mut worst = "proved";
                for proof in &check.proofs {
                    let resolved = match proof
                        .strip_prefix("moved:")
                        .and_then(|id| id.parse::<usize>().ok())
                    {
                        Some(origin) => match by_id.get(&origin) {
                            Some(origin) if origin.id != check.id => {
                                resolve(origin, by_id, depth + 1)
                            }
                            _ => "unmatched",
                        },
                        None => "proved",
                    };
                    if resolved != "proved" {
                        worst = resolved;
                    }
                }
                worst
            }
            other => other,
        }
    }
    let names_it = |check: &NullCheck| {
        check.subject == variable
            || check
                .subject
                .strip_prefix('&')
                .and_then(|rest| rest.strip_prefix(variable))
                .is_some_and(|rest| rest.starts_with("->") || rest.starts_with('['))
    };
    let judge = |checks: Vec<&NullCheck>| {
        let mut standing = "proved";
        for check in checks {
            let resolved = resolve(check, &by_id, 0);
            if resolved != "proved" {
                standing = resolved;
            }
        }
        standing
    };
    let exact = row
        .checks
        .iter()
        .filter(|check| check.line == line && names_it(check))
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return judge(exact).to_owned();
    }
    let nearby = row
        .checks
        .iter()
        .filter(|check| check.line.abs_diff(line) <= 3 && names_it(check))
        .collect::<Vec<_>>();
    if nearby.is_empty() {
        "unmatched".to_owned()
    } else {
        format!("nearby:{}", judge(nearby))
    }
}

/// Card NULL-SAFETY-PROPERTY: when the null property is selected, record each
/// function's null row beside its custody status, the custody row's own null
/// premises with what the null row says of each, and the combined verdict of
/// a request naming both. Reads the custody result; never changes it.
#[allow(clippy::too_many_arguments)]
fn record_null(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    preprocessed: Option<&str>,
    functions: &[String],
    result: &Result<UnrestrictedLegacyResult, LegacyCError>,
    contracts: &ResourceContracts,
    locations: Option<&SourceLineMap>,
    report: &mut AuditReport,
) {
    use qcore_c_gateway::null_deref::{
        NullVerdict, Request, analyze_null_c11, analyze_null_preprocessed, compose, row_verdict,
    };
    if std::env::var_os(NULL_SELECTION).is_none() {
        return;
    }
    let bounds = crate::bounds_selection::null_environment();
    let analysed = match preprocessed {
        Some(text) => analyze_null_preprocessed(text, unit, functions, contracts, &bounds),
        None => analyze_null_c11(unit, functions, contracts, &bounds),
    };
    let mapped;
    let locations = match (locations, preprocessed) {
        (Some(map), _) => Some(map),
        (None, Some(text)) => {
            mapped = map_preprocessed_lines(unit, text);
            Some(&mapped)
        }
        (None, None) => None,
    };
    let origin = |line: usize| {
        locations
            .and_then(|map| map.present(line, file))
            .map_or(serde_json::Value::Null, serde_json::Value::String)
    };
    // Card NULL-CANDIDATE-TRIAGE (rules 14t-14v): the unit's evidence for the
    // run's fold (member stores, definition exports, external calls).
    if let Ok(analysed) = &analysed {
        let with_origin = |value: serde_json::Value| {
            let mut value = value;
            for item in value.as_array_mut().into_iter().flatten() {
                let line = item["line"].as_u64().unwrap_or_default() as usize;
                item["origin"] = origin(line);
            }
            value
        };
        report.record_null_evidence(serde_json::json!({
            "library": library,
            "file": file,
            "member_stores": with_origin(serde_json::to_value(&analysed.member_stores).unwrap_or_default()),
            "stable_stores": with_origin(serde_json::to_value(&analysed.stable_stores).unwrap_or_default()),
            "definitions": serde_json::to_value(&analysed.definitions).unwrap_or_default(),
            "external_calls": with_origin(serde_json::to_value(&analysed.external_calls).unwrap_or_default()),
        }));
    }
    let custody = summary_of(result);
    for name in functions {
        let function = custody.and_then(|summary| summary.functions.get(name));
        let custody_value = function.map(|function| custody_null_value(function.status));
        let (row_value, identity, null, own) = match &analysed {
            Ok(report) => match report.row(name) {
                Some(row) => {
                    let mut value = serde_json::to_value(row).unwrap_or(serde_json::Value::Null);
                    for check in value["checks"].as_array_mut().into_iter().flatten() {
                        let line = check["line"].as_u64().unwrap_or_default() as usize;
                        check["origin"] = origin(line);
                    }
                    // Card NULL-CANDIDATE-TRIAGE: every witness step's
                    // original line, for the reviewer.
                    for triage in value["triage"].as_array_mut().into_iter().flatten() {
                        for step in triage["witness"]["path"]
                            .as_array_mut()
                            .into_iter()
                            .flatten()
                        {
                            let line = step["line"].as_u64().unwrap_or_default() as usize;
                            step["origin"] = origin(line);
                        }
                    }
                    (
                        value,
                        serde_json::json!({
                            "model_version": report.model_version,
                            "source_sha256": report.source_sha256,
                            "contracts_sha256": report.contracts_sha256,
                            "resource_contracts_sha256": report.resource_contracts_sha256,
                        }),
                        Some(row_verdict(row)),
                        Some(row),
                    )
                }
                None => (serde_json::Value::Null, serde_json::Value::Null, None, None),
            },
            Err(error) => (
                serde_json::json!({
                    "function": name,
                    "verdict": "Unknown",
                    "reasons": [format!("null-import-failure: {error}")],
                    "checks": [],
                }),
                serde_json::Value::Null,
                Some(NullVerdict::Unknown),
                None,
            ),
        };
        let premises = function
            .map(|function| {
                function
                    .null_premises
                    .iter()
                    .map(|premise| {
                        serde_json::json!({
                            "name": premise.name(name),
                            "line": premise.line,
                            "origin": origin(premise.line),
                            "variable": premise.variable,
                            "null_property": own.map_or_else(
                                || "unmatched".to_owned(),
                                |row| premise_standing(row, premise.line, &premise.variable),
                            ),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let combined = compose(
            Request {
                custody: true,
                null: true,
            },
            custody_value,
            null,
        );
        let mut row = serde_json::json!({
            "library": library,
            "file": file,
            "function": name,
            "custody": custody_value.map(NullVerdict::as_str),
            "null": row_value,
            "identity": identity,
            "combined": combined.as_str(),
        });
        if let Some(scope) = function.and_then(qcore_c_gateway::loop_lifetime_summary::body_scope) {
            row["lifetime_scope"] = scope.into();
        }
        if !premises.is_empty() {
            row["custody_null_premises"] = serde_json::Value::Array(premises);
        }
        report.record_null(row);
    }
}

/// One line of a unit's console account, written to that unit's buffer. A
/// `String` sink cannot fail.
macro_rules! say {
    ($out:expr, $($argument:tt)*) => {{
        use std::fmt::Write as _;
        let _ = writeln!($out, $($argument)*);
    }};
}

#[derive(Default)]
pub struct AuditCounts {
    pub units: usize,
    pub data_only_units: usize,
    pub source_functions: usize,
    pub projected_functions: usize,
    pub safe_units: usize,
    pub unsafe_units: usize,
    pub unknown_units: usize,
    /// Card W45-USER-CONTRACT-PLUGINS: units whose verdict is `Pending` —
    /// not decided, awaiting a named human act. Never counted as `Safe`.
    pub pending_units: usize,
    pub gateway_failures: usize,
    /// Definitions the enumeration counted that exist only after preprocessing
    /// (unit E01-F1). They are ordinary definitions once expanded, so the
    /// unmodified analyzer reads them.
    pub absent_from_raw_scan_definitions: usize,
    /// Definitions the enumeration counted that no implemented analysis reads.
    /// Each carries an explicit `Unknown` row with a classified reason code.
    pub unanalyzable_definitions: usize,
    /// Units whose enumeration *completeness claim* was refused on a named
    /// ground. A refused claim is a scoped `Unknown`, never a downgrade of the
    /// analysed functions' own results.
    pub enumeration_refused_units: usize,
    /// Units whose analysis returned `Safe` but whose definition set was not
    /// established, or carried a definition nothing read. The unit is published
    /// `Unknown`; the per-function rows are unchanged.
    pub withheld_safe_units: usize,
    /// Per-run preparation measurements. Durations are observed wall time for
    /// the three shared stages, summed across units; they are not speedup claims.
    pub prepared_units: usize,
    pub preparation_clang_invocations: usize,
    pub plain_preprocessing_elapsed_us: u64,
    pub syntax_validation_elapsed_us: u64,
    pub line_marked_preprocessing_elapsed_us: u64,
}

impl AuditCounts {
    /// Add another tally to this one (card **W44-AUDIT-PARALLEL-UNITS**).
    ///
    /// Every field is a sum, so absorbing per-unit tallies in unit order gives
    /// the tally a single sequential pass would. The destructuring is
    /// exhaustive on purpose: a new field cannot be added without deciding how
    /// it folds.
    pub fn absorb(&mut self, other: AuditCounts) {
        let AuditCounts {
            units,
            data_only_units,
            source_functions,
            projected_functions,
            safe_units,
            unsafe_units,
            unknown_units,
            pending_units,
            gateway_failures,
            absent_from_raw_scan_definitions,
            unanalyzable_definitions,
            enumeration_refused_units,
            withheld_safe_units,
            prepared_units,
            preparation_clang_invocations,
            plain_preprocessing_elapsed_us,
            syntax_validation_elapsed_us,
            line_marked_preprocessing_elapsed_us,
        } = other;
        self.units += units;
        self.data_only_units += data_only_units;
        self.source_functions += source_functions;
        self.projected_functions += projected_functions;
        self.safe_units += safe_units;
        self.unsafe_units += unsafe_units;
        self.unknown_units += unknown_units;
        self.pending_units += pending_units;
        self.gateway_failures += gateway_failures;
        self.absent_from_raw_scan_definitions += absent_from_raw_scan_definitions;
        self.unanalyzable_definitions += unanalyzable_definitions;
        self.enumeration_refused_units += enumeration_refused_units;
        self.withheld_safe_units += withheld_safe_units;
        self.prepared_units += prepared_units;
        self.preparation_clang_invocations += preparation_clang_invocations;
        self.plain_preprocessing_elapsed_us = self
            .plain_preprocessing_elapsed_us
            .saturating_add(plain_preprocessing_elapsed_us);
        self.syntax_validation_elapsed_us = self
            .syntax_validation_elapsed_us
            .saturating_add(syntax_validation_elapsed_us);
        self.line_marked_preprocessing_elapsed_us = self
            .line_marked_preprocessing_elapsed_us
            .saturating_add(line_marked_preprocessing_elapsed_us);
    }
}

/// Audit-owned context supplied beside one immutable prepared unit.
pub struct PreparedAuditCertificateContext<'a> {
    pub compile_task_identity: &'a CompileTaskIdentity,
    pub source_profile_sha256: &'a str,
}

/// The summary a result carries, whatever its verdict.
fn summary_of(
    result: &Result<UnrestrictedLegacyResult, LegacyCError>,
) -> Option<&LegacyProgramSummary> {
    match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => Some(&certificate.summary),
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => Some(summary),
        Ok(UnrestrictedLegacyResult::Unknown { partial, .. }) => partial.as_ref(),
        Err(_) => None,
    }
}

/// Derive this unit's preprocessed-line → original-coordinate index, or `None`
/// when there is nothing to attribute (card **GATEWAY-C-LOCATION**).
///
/// The `Safe` and `Unknown` arms already carry the preprocessed text the
/// analyzer read; only the `Unsafe` arm does not, and that arm re-preprocesses
/// rather than accept a text derived some other way — a map built against
/// different bytes would name the wrong lines.
fn origin_coordinates(
    unit: &C11TranslationUnit<'_>,
    result: &Result<UnrestrictedLegacyResult, LegacyCError>,
) -> Option<SourceLineMap> {
    let summary = summary_of(result)?;
    if summary
        .functions
        .values()
        .all(|function| function.diagnostics.is_empty())
    {
        return None;
    }
    let recovered;
    let preprocessed = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => certificate.preprocessed_source.as_str(),
        Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            ..
        }) => preprocessed_source.as_str(),
        Ok(UnrestrictedLegacyResult::Unsafe(_)) => {
            recovered = validate_unrestricted_c11(unit).ok()?;
            recovered.as_str()
        }
        Err(_) => return None,
    };
    Some(map_preprocessed_lines(unit, preprocessed))
}

pub fn project_unit(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    inventory: &SourceDefinitionInventory,
    contracts: &ResourceContracts,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
) -> Result<(), String> {
    let mut out = String::new();
    let result = project_unit_into(
        library, file, unit, inventory, contracts, report, counts, &mut out,
    );
    print!("{out}");
    result
}

/// [`project_unit`], with the unit's console lines appended to `out` instead
/// of printed, so a caller computing units concurrently can print each unit's
/// account in unit order (card **W44-AUDIT-PARALLEL-UNITS**).
#[allow(clippy::too_many_arguments)]
pub fn project_unit_into(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    inventory: &SourceDefinitionInventory,
    contracts: &ResourceContracts,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    project_unit_into_with_cross_unit(
        library,
        file,
        unit,
        inventory,
        contracts,
        &[],
        report,
        counts,
        out,
    )
}

/// Pinned legacy projection with opt-in, replayable cross-unit homes.
#[allow(clippy::too_many_arguments)]
pub fn project_unit_into_with_cross_unit(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    inventory: &SourceDefinitionInventory,
    contracts: &ResourceContracts,
    run_homes: &[qcore_c_gateway::legacy_c::CrossUnitHomeEvidence],
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    let scoped = if run_homes.is_empty() {
        None
    } else {
        let source = validate_unrestricted_c11(unit).ok();
        Some(match source {
            Some(source) => {
                qcore_c_gateway::legacy_c::cross_unit_for_caller(&source, contracts, run_homes)
            }
            None => {
                let mut environment = contracts.clone();
                environment.set_inferred_imports(Default::default());
                environment.set_lifetime_imports(Default::default());
                qcore_c_gateway::legacy_c::CrossUnitCallerImports {
                    environment,
                    homes: Vec::new(),
                    reachable: 0,
                    named_only: 0,
                    withheld: None,
                }
            }
        })
    };
    let contracts = scoped
        .as_ref()
        .map_or(contracts, |scoped| &scoped.environment);
    let homes = scoped
        .as_ref()
        .map_or(&[][..], |scoped| scoped.homes.as_slice());
    crate::route_progress::cross_unit_scope(library, file, scoped.as_ref());
    record_inventory(inventory, counts);
    let source_functions = inventory.definitions();
    if source_functions.is_empty() {
        counts.data_only_units += 1;
        let result = validate_unrestricted_c11(unit);
        report.record_data_only(library, file, result.as_ref().err());
        // Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): the unit's member
        // facts, read from its AST, only when a declaration invokes a member.
        // Card OP-CONSTRUCTOR-DECLARATIONS: and its classified stores into
        // the stated members, when a declaration states a result state.
        if !crate::callback_member_fold::member_declarations(contracts).is_empty()
            || !crate::callback_member_fold::constructed_declarations(contracts).is_empty()
        {
            let facts = crate::callback_member_fold::data_only_facts(result.as_deref().ok(), unit);
            let state = crate::callback_member_fold::data_only_state(
                result.as_deref().ok(),
                unit,
                contracts,
            );
            report.record_callback_members(library, file, facts, state);
        }
        report.record_enumeration(inventory);
        match result {
            Ok(_) => say!(out, "  {library}/{file}: data-only gateway pass"),
            Err(error) => {
                counts.gateway_failures += 1;
                say!(
                    out,
                    "  {library}/{file}: data-only gateway failure: {error}"
                );
            }
        }
        return Ok(());
    }
    counts.source_functions += source_functions.len();
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f): the unit's consultations,
    // recorded around its analysis alone (evidence; nothing reads it back).
    let recording = qcore_c_gateway::contract_dependencies::Recording::start();
    let result = if homes.is_empty() {
        analyze_unrestricted_c11_with_inventory(
            unit,
            &source_functions[0],
            contracts,
            Some(inventory),
        )
    } else {
        analyze_unrestricted_c11_with_inventory_and_cross_unit(
            unit,
            &source_functions[0],
            contracts,
            Some(inventory),
            homes,
        )
    };
    let result = state_build_exclusion(inventory, &source_functions[0], result);
    let dependencies = recording.finish();
    // Card GATEWAY-C-LOCATION: the original `file:line` every diagnostic of
    // this unit is presented at. Derived only where there is a diagnostic to
    // attribute, from the *authoritative* preprocessed text — the bytes the
    // analyzer's own line numbers index — so a coordinate can only ever
    // describe the text it was built against. This publishes no verdict and is
    // never read by the analysis: `result` above is already settled.
    let locations = origin_coordinates(unit, &result);
    let preprocessed = match &result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            Some(certificate.preprocessed_source.as_str())
        }
        Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            ..
        }) => Some(preprocessed_source.as_str()),
        _ => None,
    };
    record_bounds(
        library,
        file,
        unit,
        preprocessed,
        source_functions,
        &result,
        report,
    );
    record_null(
        library,
        file,
        unit,
        preprocessed,
        source_functions,
        &result,
        contracts,
        locations.as_ref(),
        report,
    );
    record_projection(
        ProjectionContext {
            library,
            file,
            inventory,
            source_functions,
            contracts,
            dependencies: &dependencies,
            hypothesis_context: hypothesis_context(
                library,
                file,
                preprocessed.unwrap_or(unit.source),
                contracts,
                &format!(
                    "{:?}",
                    (
                        unit,
                        std::env::current_dir().map_err(|e| e.to_string())?,
                        qcore_c_gateway::ClangIdentity::capture()?
                    )
                ),
                "legacy-pinned-resource-profile",
            ),
        },
        result,
        locations.as_ref(),
        report,
        counts,
        out,
    )?;
    record_withheld(scoped.as_ref(), report, out);
    Ok(())
}

/// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): a caller whose
/// reachable homes exceeded the per-unit replay budget held no import; its
/// unit row and console line name why.
fn record_withheld(
    scoped: Option<&qcore_c_gateway::legacy_c::CrossUnitCallerImports>,
    report: &mut AuditReport,
    out: &mut String,
) {
    if let Some(reason) = scoped.and_then(|scoped| scoped.withheld.as_deref()) {
        report.record_cross_unit_withheld(reason);
        say!(out, "    {reason}");
    }
}

/// Project a unit from the preparation already used for compile-task identity,
/// denominator inventory and diagnostic coordinates.
pub fn project_prepared_unit(
    library: &str,
    file: &str,
    prepared: &PreparedTranslationUnit,
    contracts: &ResourceContracts,
    certificate_context: PreparedAuditCertificateContext<'_>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
) -> Result<(), String> {
    let mut out = String::new();
    let result = project_prepared_unit_into(
        library,
        file,
        prepared,
        contracts,
        certificate_context,
        report,
        counts,
        &mut out,
    );
    print!("{out}");
    result
}

/// [`project_prepared_unit`], with the unit's console lines appended to `out`
/// instead of printed (card **W44-AUDIT-PARALLEL-UNITS**).
#[allow(clippy::too_many_arguments)]
pub fn project_prepared_unit_into(
    library: &str,
    file: &str,
    prepared: &PreparedTranslationUnit,
    contracts: &ResourceContracts,
    certificate_context: PreparedAuditCertificateContext<'_>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    project_prepared_unit_into_with_cross_unit(
        library,
        file,
        prepared,
        contracts,
        &[],
        certificate_context,
        report,
        counts,
        out,
    )
}

/// Prepared projection under a run-wide, opt-in cross-unit home table.
#[allow(clippy::too_many_arguments)]
pub fn project_prepared_unit_into_with_cross_unit(
    library: &str,
    file: &str,
    prepared: &PreparedTranslationUnit,
    contracts: &ResourceContracts,
    run_homes: &[qcore_c_gateway::legacy_c::CrossUnitHomeEvidence],
    certificate_context: PreparedAuditCertificateContext<'_>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    project_prepared_unit_into_with_link_standing(
        library,
        file,
        prepared,
        contracts,
        run_homes,
        None,
        certificate_context,
        report,
        counts,
        out,
    )
}

/// Card **LINK-STANDING** (rule 40): a caller's admission of the homes rule
/// 4c's scope holds; returns the kept imports and each refused home with
/// its reason.
pub type LinkAdmission<'a> = &'a dyn Fn(
    qcore_c_gateway::legacy_c::CrossUnitCallerImports,
) -> (
    qcore_c_gateway::legacy_c::CrossUnitCallerImports,
    Vec<(String, String)>,
);

/// [`project_prepared_unit_into_with_cross_unit`], with rule 40's admission
/// applied to the held homes after rule 4c's scope and budget.
#[allow(clippy::too_many_arguments)]
pub fn project_prepared_unit_into_with_link_standing(
    library: &str,
    file: &str,
    prepared: &PreparedTranslationUnit,
    contracts: &ResourceContracts,
    run_homes: &[qcore_c_gateway::legacy_c::CrossUnitHomeEvidence],
    admission: Option<LinkAdmission<'_>>,
    certificate_context: PreparedAuditCertificateContext<'_>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    let scoped = if run_homes.is_empty() {
        None
    } else {
        prepared.preprocessed_source().ok().map(|source| {
            qcore_c_gateway::legacy_c::cross_unit_for_caller(source, contracts, run_homes)
        })
    };
    let (scoped, link_refusals) = match (scoped, admission) {
        (Some(scoped), Some(admission)) => {
            let (scoped, refusals) = admission(scoped);
            (Some(scoped), refusals)
        }
        (scoped, _) => (scoped, Vec::new()),
    };
    let contracts = scoped
        .as_ref()
        .map_or(contracts, |scoped| &scoped.environment);
    let homes = scoped
        .as_ref()
        .map_or(&[][..], |scoped| scoped.homes.as_slice());
    crate::route_progress::cross_unit_scope(library, file, scoped.as_ref());
    let PreparedAuditCertificateContext {
        compile_task_identity,
        source_profile_sha256,
    } = certificate_context;
    let metrics = prepared.metrics();
    counts.prepared_units += 1;
    counts.preparation_clang_invocations += usize::from(metrics.clang_invocations());
    counts.plain_preprocessing_elapsed_us = counts
        .plain_preprocessing_elapsed_us
        .saturating_add(metrics.plain_preprocessing.elapsed_us);
    counts.syntax_validation_elapsed_us = counts
        .syntax_validation_elapsed_us
        .saturating_add(metrics.syntax_validation.elapsed_us);
    counts.line_marked_preprocessing_elapsed_us = counts
        .line_marked_preprocessing_elapsed_us
        .saturating_add(metrics.line_marked_preprocessing.elapsed_us);
    let inventory = prepared.inventory();
    record_inventory(inventory, counts);
    let source_functions = inventory.definitions();
    if source_functions.is_empty() {
        counts.data_only_units += 1;
        report.record_data_only(library, file, prepared.validation_error());
        // Card CALLBACK-MEMBER-DATA-ONLY-UNITS (rule 12n): the unit's member
        // facts, read from its AST, only when a declaration invokes a member.
        if !crate::callback_member_fold::member_declarations(contracts).is_empty()
            || !crate::callback_member_fold::constructed_declarations(contracts).is_empty()
        {
            let view = C11TranslationUnit {
                source: prepared.raw_source(),
                include_paths: prepared.include_paths().to_vec(),
                definitions: prepared.definitions().to_vec(),
                preprocessor_options: prepared.preprocessor_options().to_vec(),
                dialect: prepared.dialect(),
                ..Default::default()
            };
            let facts = crate::callback_member_fold::data_only_facts(
                prepared.preprocessed_source().ok(),
                &view,
            );
            let state = crate::callback_member_fold::data_only_state(
                prepared.preprocessed_source().ok(),
                &view,
                contracts,
            );
            report.record_callback_members(library, file, facts, state);
        }
        report.record_enumeration(inventory);
        if let Some(error) = prepared.validation_error() {
            counts.gateway_failures += 1;
            say!(
                out,
                "  {library}/{file}: data-only gateway failure: {error}"
            );
        } else {
            say!(out, "  {library}/{file}: data-only gateway pass");
        }
        return Ok(());
    }
    counts.source_functions += source_functions.len();
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f): recorded around the
    // analysis alone; the certificate recheck below runs outside it.
    let recording = qcore_c_gateway::contract_dependencies::Recording::start();
    let result = if homes.is_empty() {
        analyze_prepared_translation_unit(prepared, &source_functions[0], contracts)
    } else {
        analyze_prepared_translation_unit_with_cross_unit(
            prepared,
            &source_functions[0],
            contracts,
            homes,
        )
    };
    let result = state_build_exclusion(inventory, &source_functions[0], result);
    let dependencies = recording.finish();
    let result = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            let policy = AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions;
            let admission = (|| {
                let expected = expected_audit_certificate_context(
                    prepared,
                    compile_task_identity.clone(),
                    contracts,
                    source_profile_sha256,
                    policy,
                )?;
                check_audit_legacy_safety_certificate_in_context(
                    &certificate,
                    compile_task_identity,
                    source_profile_sha256,
                    policy,
                    &expected,
                )
            })();
            match admission {
                Ok(()) => Ok(UnrestrictedLegacyResult::Safe(certificate)),
                Err(reason) => {
                    let mut summary = certificate.summary;
                    summary.status = AnalysisStatus::Unknown;
                    for function in summary.functions.values_mut() {
                        function.status = AnalysisStatus::Unknown;
                    }
                    Ok(UnrestrictedLegacyResult::Unknown {
                        preprocessed_source: certificate.preprocessed_source,
                        reason: format!("certificate context binding refused: {reason}"),
                        partial: Some(summary),
                    })
                }
            }
        }
        other => other,
    };
    if std::env::var_os(BOUNDS_SELECTION).is_some() || std::env::var_os(NULL_SELECTION).is_some() {
        let view = C11TranslationUnit {
            source: prepared.raw_source(),
            include_paths: prepared.include_paths().to_vec(),
            definitions: prepared.definitions().to_vec(),
            preprocessor_options: prepared.preprocessor_options().to_vec(),
            dialect: prepared.dialect(),
            ..Default::default()
        };
        record_bounds(
            library,
            file,
            &view,
            prepared.preprocessed_source().ok(),
            source_functions,
            &result,
            report,
        );
        // Card NULL-SAFETY-PROPERTY (rules 14k-14p).
        record_null(
            library,
            file,
            &view,
            prepared.preprocessed_source().ok(),
            source_functions,
            &result,
            contracts,
            Some(prepared.locations()),
            report,
        );
    }
    record_projection(
        ProjectionContext {
            library,
            file,
            inventory,
            source_functions,
            contracts,
            dependencies: &dependencies,
            hypothesis_context: hypothesis_context(
                library,
                file,
                prepared
                    .preprocessed_source()
                    .unwrap_or(prepared.raw_source()),
                contracts,
                &serde_json::to_string(compile_task_identity).map_err(|e| e.to_string())?,
                source_profile_sha256,
            ),
        },
        result,
        Some(prepared.locations()),
        report,
        counts,
        out,
    )?;
    record_withheld(scoped.as_ref(), report, out);
    record_link_refusals(&link_refusals, report, out);
    Ok(())
}

/// Card **LINK-STANDING** (rule 40): the homes rule 40 refused for this
/// caller, on its unit row and console lines.
fn record_link_refusals(refusals: &[(String, String)], report: &mut AuditReport, out: &mut String) {
    if refusals.is_empty() {
        return;
    }
    report.record_cross_unit_link_refusals(refusals);
    for (function, reason) in refusals {
        say!(out, "    cross-unit import `{function}` refused: {reason}");
    }
}

/// Card **CURRENT-RUN-DEFECTS**: a unit whose every raw-scan definition is
/// build-excluded (librdkafka's `tinycthread.c` under `WITH_C11THREADS`) has
/// no definition for the analysis to enter, so its entry, the first raw name,
/// is not found. The unit stays `Unknown` with no partial summary, as Cycle 19
/// decided (fail-closed, never data-only or `Safe`), and its rows stay in
/// `not_projected`; only the reason changes, from that first name reported as
/// a missing function to the stated cause. Any other result is returned as it
/// is.
fn state_build_exclusion(
    inventory: &SourceDefinitionInventory,
    entry: &str,
    result: Result<UnrestrictedLegacyResult, LegacyCError>,
) -> Result<UnrestrictedLegacyResult, LegacyCError> {
    let Some(stated) = inventory.build_excluded_reason() else {
        return result;
    };
    let missing = LegacyCError::FunctionNotFound(entry.to_owned()).to_string();
    match result {
        Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason,
            partial: None,
        }) if reason == missing => Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason: stated,
            partial: None,
        }),
        other => other,
    }
}

/// Count one analysed unit under its published verdict (card
/// W45-USER-CONTRACT-PLUGINS).
fn tally(counts: &mut AuditCounts, verdict: Verdict) {
    match verdict {
        Verdict::Safe => counts.safe_units += 1,
        Verdict::Pending => counts.pending_units += 1,
        Verdict::Unknown => counts.unknown_units += 1,
        Verdict::Unsafe => counts.unsafe_units += 1,
    }
}

fn record_inventory(inventory: &SourceDefinitionInventory, counts: &mut AuditCounts) {
    counts.units += 1;
    counts.absent_from_raw_scan_definitions += inventory.absent_from_raw_scan().len();
    counts.unanalyzable_definitions += inventory.unanalyzable().len();
    if !inventory.is_complete() {
        counts.enumeration_refused_units += 1;
    }
}

struct ProjectionContext<'a> {
    library: &'a str,
    file: &'a str,
    inventory: &'a SourceDefinitionInventory,
    source_functions: &'a [String],
    /// The environment the unit was analysed under (card
    /// W47-REVIEW-HOLD-CONCRETE-REASON: it names each held call's reasons).
    contracts: &'a ResourceContracts,
    /// Card VERDICT-CONTRACT-DEPENDENCIES: what the unit's analysis recorded.
    dependencies: &'a qcore_c_gateway::contract_dependencies::UnitRecord,
    hypothesis_context: qcore_schemas::hypothesis::Context,
}

enum ProjectionPublication {
    Safe(usize),
    Unsafe(usize),
    Unknown { reason: String, projected: usize },
    Error(LegacyCError),
}

fn record_projection(
    context: ProjectionContext<'_>,
    result: Result<UnrestrictedLegacyResult, LegacyCError>,
    prepared_locations: Option<&SourceLineMap>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
    out: &mut String,
) -> Result<(), String> {
    let ProjectionContext {
        library,
        file,
        inventory,
        source_functions,
        contracts,
        dependencies,
        hypothesis_context,
    } = context;
    let result = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) if !inventory.supports_unit_safe() => {
            counts.withheld_safe_units += 1;
            Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source: certificate.preprocessed_source,
                reason: inventory
                    .withheld_safe_reason()
                    .unwrap_or_else(|| "enumeration completeness refused".to_owned()),
                partial: Some(certificate.summary),
            })
        }
        // Card W45-SR5-SOUNDNESS (`PENDING-BYPASSES-UNIT-SAFE-GATES`): the
        // inventory withholds `Safe` from this unit, so no human act on its
        // held calls could make it `Safe`: it is `Unknown`, not `Pending`.
        Ok(UnrestrictedLegacyResult::Unknown {
            preprocessed_source,
            reason,
            partial: Some(mut summary),
        }) if summary.status == AnalysisStatus::Pending && !inventory.supports_unit_safe() => {
            summary.status = AnalysisStatus::Unknown;
            Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source,
                reason: format!(
                    "{reason}; {}",
                    inventory
                        .withheld_safe_reason()
                        .unwrap_or_else(|| "enumeration completeness refused".to_owned())
                ),
                partial: Some(summary),
            })
        }
        other => other,
    };
    let locations = prepared_locations.filter(|_| {
        summary_of(&result).is_some_and(|summary| {
            summary
                .functions
                .values()
                .any(|function| !function.diagnostics.is_empty())
        })
    });
    let detail = report.record(library, file, source_functions, &result, locations);
    // Card W47-REVIEW-HOLD-CONCRETE-REASON: every row holding a call names
    // each held declaration's review reasons; additive, and fail-closed on a
    // hold whose reason the environment cannot name.
    if let Some(row) = report.last_unit_mut() {
        crate::review_hold_reasons::annotate_unit(row, contracts)
            .map_err(|error| format!("{library}/{file}: {error}"))?;
        // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13f): the row fields, after
        // the verdict is settled; additive.
        crate::contract_dependencies::annotate_unit(row, dependencies, &result, contracts)?;
    }
    // All full-record consumers are finished; retain the publication account.
    let publication = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            ProjectionPublication::Safe(certificate.checked_functions.len())
        }
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
            ProjectionPublication::Unsafe(summary.functions.len())
        }
        Ok(UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        }) => ProjectionPublication::Unknown {
            reason,
            projected: partial
                .as_ref()
                .map_or(0, |summary| summary.functions.len()),
        },
        Err(error) => ProjectionPublication::Error(error),
    };
    if let Some(row) = report.last_unit_mut() {
        if !dependencies.reading_sites.is_empty() || !dependencies.reading_site_omissions.is_empty()
        {
            row["reading_site_domain"] = serde_json::json!({
                "schema": qcore_c_gateway::reading_site_evidence::SCHEMA,
                "coverage": "selected-observations-only",
                "captured_local_fragments": dependencies.reading_sites.len(),
                "omissions": dependencies.reading_site_omissions,
                "universal_disposition": "open"
            });
        }
        let reading_graphs = if dependencies.reading_sites.is_empty() {
            None
        } else {
            Some(crate::contract_dependencies::replayed_hypothesis_graphs(
                contracts,
            )?)
        };
        let mut reading_binding_omissions = Vec::new();
        let mut bound_readings = Vec::new();
        for function in row["functions"].as_array_mut().into_iter().flatten() {
            let mut context = hypothesis_context.clone();
            if let Some(source) = &dependencies.primary_source_sha256 {
                context.source_identity = source.clone();
            }
            if let Some(name) = function["function"].as_str() {
                context.artifact.push('#');
                context.artifact.push_str(name);
            }
            context.scope = serde_json::to_string(source_functions).map_err(|e| e.to_string())?;
            // Card ARM-OBLIGATIONS: a carried premise is one of the row's
            // assumptions.
            context.assumptions =
                crate::report::assumptions_with_premises(&context.assumptions, function);
            let name = function["function"].as_str().unwrap_or_default();
            let mut readings = Vec::new();
            for observation in dependencies
                .reading_sites
                .iter()
                .filter(|site| site.function == name)
            {
                let consulted = function["contract_dependencies"]["hypothesis_dependencies"]
                    [&observation.declaration]
                    .as_array()
                    .is_some_and(|atoms| !atoms.is_empty());
                let binding = reading_graphs
                    .as_deref()
                    .and_then(|graphs| graphs.get(&observation.declaration))
                    .ok_or_else(|| "selected production graph absent".to_owned())
                    .and_then(|graph| {
                        if !consulted {
                            return Err("no actual row consultation closure".to_owned());
                        }
                        qcore_c_gateway::reading_site_evidence::validate_observation_environment(
                            observation,
                            &dependencies.reading_environments,
                        )?;
                        qcore_c_gateway::reading_site_evidence::bind(observation, &context, graph)
                    });
                match binding {
                    Ok(receipt) => {
                        bound_readings.push(receipt.clone());
                        readings.push(receipt)
                    }
                    Err(reason) => reading_binding_omissions.push(serde_json::json!({
                        "function": name, "declaration": observation.declaration, "reason": reason
                    })),
                }
            }
            if !readings.is_empty() {
                readings.sort_by(|a, b| a.id.cmp(&b.id));
                function["reading_site_evidence"] =
                    serde_json::to_value(readings).map_err(|e| e.to_string())?;
            }
            function["hypothesis_context"] =
                serde_json::to_value(context).map_err(|e| e.to_string())?;
        }
        let captured = qcore_c_gateway::reading_site_evidence::publication_environments(
            &bound_readings,
            &dependencies.reading_environments,
        )?;
        if !captured.is_empty() {
            row["reading_site_environments"] = serde_json::json!({
                "schema": qcore_c_gateway::reading_site_evidence::ENVIRONMENTS_SCHEMA,
                "environments": captured,
                "claim": "exact-canonical-effective-environment-bytes-only",
                "universal_disposition": "open"
            });
        }
        if !reading_binding_omissions.is_empty() {
            // Missing evidence bindings never acquire semantic authority: omit
            // the receipt and retain the settled result, with an exact reason.
            row["reading_site_domain"]["binding_omissions"] =
                serde_json::to_value(reading_binding_omissions).map_err(|e| e.to_string())?;
        }
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the unit's member
    // facts for the run's closure, only when a declaration invokes a member.
    // Card OP-CONSTRUCTOR-DECLARATIONS: and the unit's classified stores into
    // the stated members, when a declaration states a result state.
    if !crate::callback_member_fold::member_declarations(contracts).is_empty()
        || !crate::callback_member_fold::constructed_declarations(contracts).is_empty()
    {
        report.record_callback_members(
            library,
            file,
            dependencies.callback_members.clone(),
            dependencies.constructed_states.clone(),
        );
    }
    report.record_enumeration(inventory);
    // Card W45-USER-CONTRACT-PLUGINS: the unit is tallied and printed under
    // the verdict its row publishes, which is the result's own verdict unless
    // a checker plugin's link joined it upwards.
    let published = report.last_unit_verdict();
    match publication {
        ProjectionPublication::Safe(projected) => {
            let verdict = published.unwrap_or(Verdict::Safe);
            tally(counts, verdict);
            counts.projected_functions += projected;
            say!(
                out,
                "  {library}/{file}: {verdict} ({}/{})",
                projected,
                source_functions.len()
            );
        }
        ProjectionPublication::Unsafe(projected) => {
            tally(counts, Verdict::Unsafe);
            counts.projected_functions += projected;
            say!(
                out,
                "  {library}/{file}: Unsafe ({}/{})",
                projected,
                source_functions.len()
            );
        }
        ProjectionPublication::Unknown { reason, projected } => {
            let verdict = published.unwrap_or(Verdict::Unknown);
            tally(counts, verdict);
            counts.projected_functions += projected;
            say!(
                out,
                "  {library}/{file}: {verdict} ({projected}/{}): {reason}",
                source_functions.len()
            );
        }
        ProjectionPublication::Error(error) => {
            counts.gateway_failures += 1;
            say!(out, "  {library}/{file}: gateway failure: {error}");
        }
    }
    for line in detail {
        say!(out, "    {line}");
    }
    Ok(())
}

fn hypothesis_context(
    library: &str,
    file: &str,
    source: &str,
    contracts: &ResourceContracts,
    build: &str,
    profile: &str,
) -> qcore_schemas::hypothesis::Context {
    qcore_schemas::hypothesis::Context {
        artifact: format!("{library}/{file}"),
        property: "resource-custody.v1".into(),
        semantic_profile: profile.into(),
        scope: "selected-function-inventory; intraprocedural custody with validated summaries"
            .into(),
        bounds: "existing gateway budgets; no new hypothesis discharge".into(),
        assumptions: qcore_schemas::sha256_hex(
            serde_json::to_string(contracts.source_assumptions())
                .expect("string map")
                .as_bytes(),
        ),
        source_identity: qcore_schemas::sha256_hex(source.as_bytes()),
        build_identity: qcore_schemas::sha256_hex(build.as_bytes()),
        evidence_version: format!(
            "contract-hypothesis-graph.v1:{}",
            option_env!("QCORE_AUDIT_BUILD_COMMIT").unwrap_or("unrecorded-build")
        ),
    }
}
