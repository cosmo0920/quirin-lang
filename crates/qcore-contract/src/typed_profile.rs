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

//! Structural validation for declared typed-source bounds and callback effects.
//! Loading these bounds establishes no source behavior or custody facts.
use qcore_schemas::profile::{ImportRole, LibraryProfile};

pub fn validate_typed_profile(library: &LibraryProfile) -> Result<(), String> {
    let Some(profile) = &library.typed_import else {
        return if library.callback_summaries.is_empty() {
            Ok(())
        } else {
            Err("callback summaries require a typed import profile".into())
        };
    };
    if profile.functions.is_empty() {
        return Err("typed import profile declares no functions".into());
    }
    let identifier = |name: &str| {
        !name.is_empty()
            && name
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    };
    let type_name = |name: &str| !name.is_empty() && name.trim() == name;
    for name in profile
        .functions
        .keys()
        .chain(profile.records.iter())
        .chain(profile.restricted_calls.iter())
    {
        if !identifier(name) {
            return Err(format!("invalid or empty typed declaration `{name}`"));
        }
    }
    if profile.sizeof_types.iter().any(|name| !type_name(name)) {
        return Err("empty or untrimmed sizeof type".into());
    }
    for (name, signature) in &profile.signature_constraints {
        if !profile.functions.contains_key(name) {
            return Err(format!("signature for undeclared function `{name}`"));
        }
        if !type_name(&signature.result) || signature.parameters.iter().any(|name| !type_name(name))
        {
            return Err(format!("empty or untrimmed signature type for `{name}`"));
        }
    }
    for (name, role) in &profile.functions {
        // Signature presence activates the importer's restricted-call policy.
        // A callback must never lose its allowlist by dropping this constraint.
        if *role == ImportRole::Callback && !profile.signature_constraints.contains_key(name) {
            return Err(format!("callback `{name}` requires an exact signature"));
        }
    }
    for name in library.callback_summaries.keys() {
        if profile.functions.get(name) != Some(&ImportRole::Callback) {
            return Err(format!(
                "callback summary `{name}` does not name a callback"
            ));
        }
    }
    Ok(())
}
