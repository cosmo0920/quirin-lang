// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

/// Check new suffixes without moving legacy failures ahead of their existing
/// consumer. Header probing is only advisory for a legacy source.
pub(crate) fn validate_source_name(path: &str, source: &str) -> Result<(), String> {
    use qcore_frontend::frontend::parse_source_module_mode;
    use qcore_frontend::source_extension::{legacy_source_warning, validate_source_extension};
    match parse_source_module_mode(source) {
        Ok(mode) => {
            validate_source_extension(path, mode)?;
            if let Some(warning) = legacy_source_warning(path, mode) {
                eprintln!("{warning}");
            }
            Ok(())
        }
        Err(error) if path.ends_with(".qv") => Err(error.to_string()),
        Err(_) => {
            if path.ends_with(".qcore") {
                eprintln!(
                    "warning[legacy-source-extension]: {path}: outdated extension (unresolved mode); contract sources use .qv"
                );
            }
            Ok(())
        }
    }
}
