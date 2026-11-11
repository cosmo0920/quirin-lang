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

//! Filename compatibility is an input guard, never a semantic route selector.
use crate::frontend::SourceModuleMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionWarning {
    pub kind: &'static str,
    pub path: String,
    pub mode: String,
    pub recommended_extension: &'static str,
}

impl std::fmt::Display for ExtensionWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "warning[{}]: {}: outdated extension ({} mode); use {}",
            self.kind, self.path, self.mode, self.recommended_extension
        )
    }
}

#[must_use]
pub fn mode_name(mode: SourceModuleMode) -> &'static str {
    match mode {
        SourceModuleMode::Contract => "contract",
        SourceModuleMode::Library => "library",
        SourceModuleMode::Executable => "executable",
    }
}

#[must_use]
pub fn contract_module_stem(path: &str) -> Option<&str> {
    path.strip_suffix(".qcore")
        .or_else(|| path.strip_suffix(".qv"))
}

/// Legacy/unrecognized names keep existing dispatch. New names constrain the
/// parsed mode; they never override it. No path canonicalization or lookup.
pub fn validate_source_extension(path: &str, mode: SourceModuleMode) -> Result<(), String> {
    let valid = if path.ends_with(".qv") {
        mode == SourceModuleMode::Contract
    } else if path.ends_with(".qr") {
        mode != SourceModuleMode::Contract
    } else {
        ![".qrlib", ".qcorelib", ".qo"]
            .iter()
            .any(|ext| path.ends_with(ext))
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{path}: extension is incompatible with {} source mode; use {}",
            mode_name(mode),
            if mode == SourceModuleMode::Contract {
                ".qv"
            } else {
                ".qr"
            }
        ))
    }
}

#[must_use]
pub fn legacy_source_warning(path: &str, mode: SourceModuleMode) -> Option<ExtensionWarning> {
    path.ends_with(".qcore").then(|| ExtensionWarning {
        kind: "legacy-source-extension",
        path: path.to_owned(),
        mode: mode_name(mode).to_owned(),
        recommended_extension: if mode == SourceModuleMode::Contract {
            ".qv"
        } else {
            ".qr"
        },
    })
}

#[must_use]
pub fn legacy_manifest_warning(path: &str) -> Option<ExtensionWarning> {
    path.ends_with(".qcorelib").then(|| ExtensionWarning {
        kind: "legacy-source-extension",
        path: path.to_owned(),
        mode: "native-abi".to_owned(),
        recommended_extension: ".qrlib",
    })
}

pub fn validate_manifest_extension(path: &str) -> Result<(), String> {
    if [".qcore", ".qv", ".qr", ".qo"]
        .iter()
        .any(|ext| path.ends_with(ext))
    {
        Err(format!(
            "{path}: source/checkpoint extension cannot name a native ABI manifest; use .qrlib"
        ))
    } else {
        Ok(())
    }
}

/// Reject selecting two suffix spellings of one logical input. Raw parent
/// paths stay raw: this is not filesystem identity or a pin rewrite.
pub fn reject_alias_collisions<'a>(paths: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = std::collections::BTreeMap::<_, Vec<&str>>::new();
    for path in paths {
        let Some((stem, family)) = [".qcore", ".qv", ".qr", ".qcorelib", ".qrlib"]
            .iter()
            .find_map(|suffix| {
                path.strip_suffix(suffix).map(|stem| {
                    (
                        stem,
                        if suffix.ends_with("lib") {
                            "abi"
                        } else {
                            "source"
                        },
                    )
                })
            })
        else {
            continue;
        };
        let entries = seen.entry((stem, family)).or_default();
        for previous in entries.iter() {
            if *previous != path
                && (family == "abi" || previous.ends_with(".qcore") || path.ends_with(".qcore"))
            {
                return Err(format!(
                    "ambiguous extension aliases: `{previous}` and `{path}`"
                ));
            }
        }
        entries.push(path);
    }
    Ok(())
}
