//! Provider-independent projection; Unknown stays in the report denominator.
use crate::certificate_context::{
    AuditContractTrustPolicy, bind_audit_legacy_safety_certificate,
    check_audit_legacy_safety_certificate_for, expected_audit_certificate_context,
};
use crate::compile_db::CompileTaskIdentity;
use crate::report::AuditReport;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyCError, LegacyProgramSummary, PreparedTranslationUnit,
    SourceDefinitionInventory, SourceLineMap, UnrestrictedLegacyResult,
    analyze_prepared_translation_unit, analyze_unrestricted_c11_with_inventory,
    map_preprocessed_lines, validate_unrestricted_c11,
};
use qcore_schemas::contract::ResourceContracts;

#[derive(Default)]
pub struct AuditCounts {
    pub units: usize,
    pub data_only_units: usize,
    pub source_functions: usize,
    pub projected_functions: usize,
    pub safe_units: usize,
    pub unsafe_units: usize,
    pub unknown_units: usize,
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
    record_inventory(inventory, counts);
    let source_functions = inventory.definitions();
    if source_functions.is_empty() {
        counts.data_only_units += 1;
        let result = validate_unrestricted_c11(unit);
        report.record_data_only(library, file, result.as_ref().err());
        report.record_enumeration(inventory);
        match result {
            Ok(_) => println!("  {library}/{file}: data-only gateway pass"),
            Err(error) => {
                counts.gateway_failures += 1;
                println!("  {library}/{file}: data-only gateway failure: {error}");
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
        },
        result,
        locations.as_ref(),
        report,
        counts,
    )
}

/// Project a unit from the preparation already used for compile-task identity,
/// denominator inventory and diagnostic coordinates.
pub fn project_prepared_unit(
    library: &str,
    file: &str,
    prepared: &PreparedTranslationUnit,
    contracts: &ResourceContracts,
    compile_task_identity: &CompileTaskIdentity,
    source_profile_sha256: &str,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
) -> Result<(), String> {
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
            println!("  {library}/{file}: data-only gateway failure: {error}");
        } else {
            println!("  {library}/{file}: data-only gateway pass");
        }
        return Ok(());
    }
    counts.source_functions += source_functions.len();
    let result = analyze_prepared_translation_unit(prepared, &source_functions[0], contracts);
    let result = match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            let policy = AuditContractTrustPolicy::DeclaredSourcesAreTrustedAssumptions;
            let expected = expected_audit_certificate_context(
                prepared,
                compile_task_identity.clone(),
                contracts,
                source_profile_sha256,
                policy,
            )?;
            let bound = bind_audit_legacy_safety_certificate(
                certificate,
                compile_task_identity.clone(),
                source_profile_sha256,
                policy,
            )?;
            check_audit_legacy_safety_certificate_for(&bound, &expected)?;
            Ok(UnrestrictedLegacyResult::Safe(bound.certificate))
        }
        other => other,
    };
    record_projection(
        ProjectionContext {
            library,
            file,
            inventory,
            source_functions,
        },
        result,
        Some(prepared.locations()),
        report,
        counts,
    )
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
}

fn record_projection(
    context: ProjectionContext<'_>,
    result: Result<UnrestrictedLegacyResult, LegacyCError>,
    prepared_locations: Option<&SourceLineMap>,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
) -> Result<(), String> {
    let ProjectionContext {
        library,
        file,
        inventory,
        source_functions,
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
    report.record_enumeration(inventory);
    match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            counts.safe_units += 1;
            counts.projected_functions += certificate.checked_functions.len();
            println!(
                "  {library}/{file}: Safe ({}/{})",
                certificate.checked_functions.len(),
                source_functions.len()
            );
        }
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => {
            counts.unsafe_units += 1;
            counts.projected_functions += summary.functions.len();
            println!(
                "  {library}/{file}: Unsafe ({}/{})",
                summary.functions.len(),
                source_functions.len()
            );
        }
        Ok(UnrestrictedLegacyResult::Unknown {
            reason, partial, ..
        }) => {
            counts.unknown_units += 1;
            let projected = partial
                .as_ref()
                .map_or(0, |summary| summary.functions.len());
            counts.projected_functions += projected;
            println!(
                "  {library}/{file}: Unknown ({projected}/{}): {reason}",
                source_functions.len()
            );
        }
        Err(error) => {
            counts.gateway_failures += 1;
            println!("  {library}/{file}: gateway failure: {error}");
        }
    }
    for line in detail {
        println!("    {line}");
    }
    Ok(())
}
