//! Provider-independent projection; Unknown stays in the report denominator.
use crate::report::AuditReport;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, SourceDefinitionInventory, UnrestrictedLegacyResult,
    analyze_unrestricted_c11_with_inventory, validate_unrestricted_c11,
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
    counts.units += 1;
    counts.absent_from_raw_scan_definitions += inventory.absent_from_raw_scan().len();
    counts.unanalyzable_definitions += inventory.unanalyzable().len();
    if !inventory.is_complete() {
        counts.enumeration_refused_units += 1;
    }
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
    // Fail-closed (unit E01-F1): a whole-unit `Safe` claim is a claim over the
    // *declared main-source scope*. It cannot stand when that scope was not
    // established, or when it carries a definition no analysis read. The
    // per-function rows below are untouched — nothing is downgraded except the
    // unit-level claim, and only with the ground named in `reason`.
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
    let detail = report.record(library, file, source_functions, &result);
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
