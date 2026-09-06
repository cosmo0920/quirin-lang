//! Provider-independent projection; Unknown stays in the report denominator.
use crate::report::AuditReport;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    validate_unrestricted_c11,
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
}

pub fn project_unit(
    library: &str,
    file: &str,
    unit: &C11TranslationUnit<'_>,
    source_functions: &[String],
    contracts: &ResourceContracts,
    report: &mut AuditReport,
    counts: &mut AuditCounts,
) -> Result<(), String> {
    counts.units += 1;
    if source_functions.is_empty() {
        counts.data_only_units += 1;
        let result = validate_unrestricted_c11(unit);
        report.record_data_only(library, file, result.as_ref().err());
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
    let result = analyze_unrestricted_c11(unit, &source_functions[0], contracts);
    let detail = report.record(library, file, source_functions, &result);
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
