//! Declared Monkey syntax bounds for the shared typed C path importer.
//! Calls remain opaque; source projection does not establish custody or safety.
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::import_source_functions_from_profile;
pub use crate::typed_source::{SourceExpression, SourceOperation, SourceRegion, SourceValue};
pub use crate::typed_source::{
    SourcePath as MonkeySourcePath, SourceResult as MonkeySourceResult,
    SourceSummary as MonkeySourceSummary,
};
use qcore_schemas::profile::ImportRole;

// Q5: source-contract text remains an interpreter input. Its declaration order
// preserves the historical wrapper report ordering; selection comes from data.
const WRAPPER_SOURCE_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv"
);

/// Import default profile callbacks. Revision is a caller assertion; the pinned
/// external audit separately checks Git/source/build identity. Each successful
/// summary carries the exact declared profile assumptions, never verified facts.
pub fn import_monkey_callbacks(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    import_source_functions_from_profile(
        unit,
        source_revision,
        &profile,
        "monkey",
        ImportRole::Callback,
    )
}

/// Preserve the bounded wrapper grammar and opaque-call behavior. No missing
/// profile or malformed selection can fall back to a Rust table.
pub fn import_monkey_wrappers(
    unit: &C11TranslationUnit<'_>,
    source_revision: &str,
) -> Result<Vec<MonkeySourceResult>, String> {
    let profile =
        crate::source_profiles::load_profile(crate::source_profiles::FLUENT_BIT_SOURCE_SET)
            .map_err(|error| error.to_string())?;
    let mut results = import_source_functions_from_profile(
        unit,
        source_revision,
        &profile,
        "monkey",
        ImportRole::Wrapper,
    )?;
    let names = wrapper_source_order()?;
    let result_name = |result: &MonkeySourceResult| match result {
        MonkeySourceResult::Summary(summary) => summary.function().to_owned(),
        MonkeySourceResult::Unknown { function, .. } => function.clone(),
    };
    if results.len() != names.len()
        || results
            .iter()
            .any(|result| !names.contains(&result_name(result)))
    {
        return Err("Monkey wrapper profile does not match source-contract declarations".into());
    }
    results.sort_by_key(|result| names.iter().position(|name| *name == result_name(result)));
    Ok(results)
}

pub(crate) fn wrapper_source_order() -> Result<Vec<String>, String> {
    let parsed = crate::frontend::parse_source_program(WRAPPER_SOURCE_CONTRACT)
        .map_err(|error| format!("invalid wrapper source contract: {error:?}"))?;
    Ok(parsed
        .declarations
        .into_iter()
        .map(|decl| decl.value.name)
        .collect())
}
