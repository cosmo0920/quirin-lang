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

//! Contract bundles compose by supersession (card **CONTRACT-SUPERSESSION**,
//! contract spec §C06.6).
//!
//! The resource loader used to refuse a function declared by two modules
//! ("duplicate function contract"). It now decides the function from the pin
//! stamps of its declarations ([`qcore_schemas::contract_supersession::decide`]):
//!
//! * the declaration with the greatest stamp stays in its module and is
//!   loaded as usual; every other declaration of the function is **cut out of
//!   its module text** and retained, text and digests, as superseded;
//! * when the stamps do not decide, every declaration of the function is cut
//!   out and retained as a [`ContractConflict`]: none is in force, and the
//!   gateway holds every call relying on it (`contract-conflict`, `Unknown`).
//!
//! A module that declares one function twice is still refused ("duplicate
//! function contract"): supersession composes bundles, it does not repair a
//! malformed module.
//!
//! A cut replaces the declaration's bytes with spaces and keeps its line
//! breaks, so every other declaration keeps its byte offset, line and text,
//! and therefore its digest. A module left with no declaration is dropped
//! from the retained sources. The retained sources are exactly what the
//! environment then holds, so the source recheck (`check_source_assumptions`)
//! reloads the same environment from them.
//!
//! The stamps come from the caller ([`DeclarationStamps`]): the audit reads
//! them from the locks (`qcore_audit::contract_revision`); every other caller
//! has none ([`Unstamped`]), so every duplicate it loads is a conflict, never
//! an optimistic choice.

use std::collections::BTreeMap;

use qcore_schemas::contract_supersession::{
    Candidate, ConflictedDeclaration, ContractConflict, Decision, StampStanding,
    SupersededDeclaration, decide,
};

use crate::contract_revision::{ModuleDeclaration, module_declaration_digests};

/// Where a loader finds each declaration's pin stamp.
pub trait DeclarationStamps {
    /// The stamp standing of the declaration of `function` in `module`
    /// (a source-assumption key) whose normalised text digest is
    /// `contract_sha256`.
    fn standing(&self, module: &str, function: &str, contract_sha256: &str) -> StampStanding;
}

/// No stamps: every declaration is unpinned, so no duplicate is decided.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unstamped;

impl DeclarationStamps for Unstamped {
    fn standing(&self, _: &str, _: &str, _: &str) -> StampStanding {
        StampStanding::Unpinned
    }
}

/// A fixed table of stamps, keyed by (module, function); a declaration whose
/// digest differs from the recorded one is stale, an absent one unpinned.
#[derive(Clone, Debug, Default)]
pub struct StampTable(pub BTreeMap<(String, String), (String, StampStanding)>);

impl DeclarationStamps for StampTable {
    fn standing(&self, module: &str, function: &str, contract_sha256: &str) -> StampStanding {
        match self.0.get(&(module.to_owned(), function.to_owned())) {
            None => StampStanding::Unpinned,
            Some((digest, _)) if digest != contract_sha256 => StampStanding::Stale,
            Some((_, standing)) => standing.clone(),
        }
    }
}

/// The result of the source transform.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Supersession {
    /// The sources with every superseded or conflicted declaration cut out.
    pub sources: BTreeMap<String, String>,
    /// Per function, the declarations superseded.
    pub superseded: BTreeMap<String, Vec<SupersededDeclaration>>,
    /// Per function, the undecided declarations.
    pub conflicts: BTreeMap<String, ContractConflict>,
    /// The modules whose text a cut changed.
    pub changed_modules: Vec<String>,
}

/// Decide every function more than one declaration of `sources` declares and
/// cut the declarations that are not in force out of their modules.
///
/// # Errors
///
/// A module that does not parse as a contract module.
pub fn supersede_sources(
    sources: &BTreeMap<String, String>,
    stamps: &dyn DeclarationStamps,
) -> Result<Supersession, String> {
    let mut by_function: BTreeMap<String, Vec<(String, ModuleDeclaration)>> = BTreeMap::new();
    for (key, source) in sources {
        for declaration in module_declaration_digests(key, source)? {
            by_function
                .entry(declaration.name.clone())
                .or_default()
                .push((key.clone(), declaration));
        }
    }
    let mut cuts: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut result = Supersession::default();
    for (function, declarations) in by_function {
        if declarations.len() < 2 {
            continue;
        }
        // A module that declares one function twice is malformed, not a
        // composition of bundles: it is refused as before.
        let mut modules = declarations.iter().map(|(key, _)| key).collect::<Vec<_>>();
        modules.sort();
        if let Some(pair) = modules.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(format!(
                "{}: `{function}` is declared twice in one module: duplicate function contract",
                pair[0]
            ));
        }
        let text = |key: &str, declaration: &ModuleDeclaration| {
            sources[key][declaration.start..declaration.end].to_owned()
        };
        let candidates = declarations
            .iter()
            .map(|(key, declaration)| Candidate {
                module: key.clone(),
                contract_sha256: declaration.contract_sha256.clone(),
                stamp: stamps.standing(key, &function, &declaration.contract_sha256),
            })
            .collect::<Vec<_>>();
        match decide(&candidates) {
            Decision::InForce(winner) => {
                let by_pinned_at = winner.stamp.stamp().unwrap_or_default().to_owned();
                // The winner is the first declaration equal to the decided
                // candidate; every other one is cut, a byte-identical twin
                // included.
                let kept = candidates
                    .iter()
                    .position(|candidate| *candidate == winner)
                    .expect("the decided candidate is one of them");
                let mut retained = Vec::new();
                for (index, ((key, declaration), candidate)) in
                    declarations.iter().zip(&candidates).enumerate()
                {
                    if index == kept {
                        continue;
                    }
                    retained.push(SupersededDeclaration {
                        module: key.clone(),
                        contract_sha256: declaration.contract_sha256.clone(),
                        pinned_at: candidate.stamp.stamp().unwrap_or_default().to_owned(),
                        text: text(key, declaration),
                        by_module: winner.module.clone(),
                        by_contract_sha256: winner.contract_sha256.clone(),
                        by_pinned_at: by_pinned_at.clone(),
                    });
                    cuts.entry(key.clone())
                        .or_default()
                        .push((declaration.start, declaration.end));
                }
                retained.sort();
                result.superseded.insert(function, retained);
            }
            Decision::Conflict(reason) => {
                let mut recorded = declarations
                    .iter()
                    .zip(candidates)
                    .map(|((key, declaration), candidate)| {
                        cuts.entry(key.clone())
                            .or_default()
                            .push((declaration.start, declaration.end));
                        ConflictedDeclaration {
                            candidate,
                            text: text(key, declaration),
                        }
                    })
                    .collect::<Vec<_>>();
                recorded.sort();
                result.conflicts.insert(
                    function,
                    ContractConflict {
                        reason,
                        declarations: recorded,
                    },
                );
            }
        }
    }
    for (key, source) in sources {
        let Some(ranges) = cuts.get(key) else {
            result.sources.insert(key.clone(), source.clone());
            continue;
        };
        let mut bytes = source.clone().into_bytes();
        for (start, end) in ranges {
            for byte in &mut bytes[*start..*end] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        }
        let cut = String::from_utf8(bytes).map_err(|error| format!("{key}: {error}"))?;
        result.changed_modules.push(key.clone());
        if !module_declaration_digests(key, &cut)?.is_empty() {
            result.sources.insert(key.clone(), cut);
        }
    }
    Ok(result)
}
