//! Filename compatibility at contract-reader boundaries. Module keys remain
//! exactly as written; a different suffix never satisfies a missing pin.

use std::collections::BTreeMap;

use qcore_frontend::source_extension::contract_module_stem;

/// Refuse two admitted spellings of the same contract module in one scope.
pub(crate) fn reject_alias_collisions<'a>(
    modules: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let mut stems = BTreeMap::new();
    for module in modules {
        if let Some(stem) = contract_module_stem(module)
            && let Some(first) = stems.insert(stem, module)
            && first != module
        {
            return Err(format!(
                "ambiguous contract module aliases `{first}` and `{module}`"
            ));
        }
    }
    Ok(())
}
