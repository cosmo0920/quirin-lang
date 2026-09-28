//! Provider-independent projection; Unknown stays in the report denominator.
use crate::certificate_context::{
    AuditContractTrustPolicy, bind_audit_legacy_safety_certificate,
    check_audit_legacy_safety_certificate_for, expected_audit_certificate_context,
};
use crate::compile_db::CompileTaskIdentity;
use crate::report::AuditReport;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCError, LegacyProgramSummary,
    PreparedTranslationUnit, SourceDefinitionInventory, SourceLineMap, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit, analyze_unrestricted_c11_with_inventory,
    map_preprocessed_lines, validate_unrestricted_c11,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::verdict::Verdict;

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
    record_inventory(inventory, counts);
    let source_functions = inventory.definitions();
    if source_functions.is_empty() {
        counts.data_only_units += 1;
        let result = validate_unrestricted_c11(unit);
        report.record_data_only(library, file, result.as_ref().err());
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
    let result = analyze_unrestricted_c11_with_inventory(
        unit,
        &source_functions[0],
        contracts,
        Some(inventory),
    );
    // Card GATEWAY-C-LOCATION: the original `file:line` every diagnostic of
    // this unit is presented at. Derived only where there is a diagnostic to
    // attribute, from the *authoritative* preprocessed text — the bytes the
    // analyzer's own line numbers index — so a coordinate can only ever
    // describe the text it was built against. This publishes no verdict and is
    // never read by the analysis: `result` above is already settled.
    let locations = origin_coordinates(unit, &result);
    record_projection(
        ProjectionContext {
            library,
            file,
            inventory,
            source_functions,
            contracts,
        },
        result,
        locations.as_ref(),
        report,
        counts,
        out,
    )
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
    let result = analyze_prepared_translation_unit(prepared, &source_functions[0], contracts);
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
                let bound = bind_audit_legacy_safety_certificate(
                    &certificate,
                    compile_task_identity.clone(),
                    source_profile_sha256,
                    policy,
                )?;
                check_audit_legacy_safety_certificate_for(&bound, &expected)?;
                Ok::<_, String>(bound)
            })();
            match admission {
                Ok(bound) => Ok(UnrestrictedLegacyResult::Safe(bound.certificate)),
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
    record_projection(
        ProjectionContext {
            library,
            file,
            inventory,
            source_functions,
            contracts,
        },
        result,
        Some(prepared.locations()),
        report,
        counts,
        out,
    )
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
    }
    report.record_enumeration(inventory);
    // Card W45-USER-CONTRACT-PLUGINS: the unit is tallied and printed under
    // the verdict its row publishes, which is the result's own verdict unless
    // a checker plugin's link joined it upwards.
    let published = report.last_unit_verdict();
    match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            let verdict = published.unwrap_or(Verdict::Safe);
            tally(counts, verdict);
            counts.projected_functions += certificate.checked_functions.len();
            say!(
                out,
                "  {library}/{file}: {verdict} ({}/{})",
                certificate.checked_functions.len(),
                source_functions.len()
            );
        }
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
            tally(counts, Verdict::Unsafe);
            counts.projected_functions += summary.functions.len();
            say!(
                out,
                "  {library}/{file}: Unsafe ({}/{})",
                summary.functions.len(),
                source_functions.len()
            );
        }
        Ok(UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        }) => {
            let verdict = published.unwrap_or(Verdict::Unknown);
            tally(counts, verdict);
            let projected = partial
                .as_ref()
                .map_or(0, |summary| summary.functions.len());
            counts.projected_functions += projected;
            say!(
                out,
                "  {library}/{file}: {verdict} ({projected}/{}): {reason}",
                source_functions.len()
            );
        }
        Err(error) => {
            counts.gateway_failures += 1;
            say!(out, "  {library}/{file}: gateway failure: {error}");
        }
    }
    for line in detail {
        say!(out, "    {line}");
    }
    Ok(())
}
