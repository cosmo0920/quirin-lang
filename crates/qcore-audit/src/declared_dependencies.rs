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

//! Declared cross-crate contract dependencies at audit time (card
//! **QUIRIN-AUDIT-CONNECT** Part C; contract spec C15.2).
//!
//! A declaration may state which other crates' declarations its contract
//! relies on (`requires resource_contract_dependency("CRATE", NS::NAME, ...)`,
//! `qcore_contract::contract_dependency`). After the pins and the library
//! version ranges are evaluated (so a target already withheld counts as
//! held), [`evaluate`] resolves every stated dependency against the loaded
//! environment and its load path (`qcore_schemas::connect::resolve`) and
//! withholds every declaration it holds, through the pin-mismatch table the
//! legacy-C gateway already holds call sites for:
//!
//! * a changed target digest, or a target held `Pending`: `dependency-changed`,
//!   held `Pending` (`contract-pin-mismatch`: a review and re-pin resolve it);
//! * a crate missing from the load path or not a declared dependency, a
//!   declaration unloaded, a version out of range, a libc/POSIX target named
//!   elsewhere, a cycle, or a target held `Unknown`: `dependency-unresolved`,
//!   held `Unknown` (`missing-premise`).
//!
//! A declaration whose module lies on no load path has no owning crate, so
//! every dependency it states is unresolved: fail-closed, never in force.
//! A held dependency holds its dependents; the resolution reads the set of
//! dependencies only. The evaluation's record (the dependency graph with
//! every identity, and every hold) is the report's
//! `contract_revision.contract_dependencies`, present only when some
//! declaration states a dependency, so every other report is unchanged.

use std::collections::BTreeMap;

use qcore_schemas::connect::{self, CrateFacts, DeclarationFacts, DeclaredDependency, HoldClass};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::PinMismatchReason;
use qcore_schemas::library_version::LibraryVersion;
use serde_json::{Value, json};

/// What the evaluation found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Evaluation {
    /// The record, `None` when no declaration states a dependency.
    pub record: Option<Value>,
}

/// The facts a resolution reads of a load path: crate to version and
/// declared dependencies.
pub type LoadPathFacts = BTreeMap<String, (String, std::collections::BTreeSet<String>)>;

/// Resolve every declared dependency of `contracts` and withhold every
/// declaration the resolution holds.
///
/// `modules` and `digests` give, per function, the module key that declares
/// it and the SHA-256 of its normalised text (`contract_revision`'s
/// evaluation); `owner` the crate that owns a module key; `crates` the load
/// path's crates.
///
/// # Errors
///
/// A withholding the environment refuses.
pub fn evaluate(
    contracts: &mut ResourceContracts,
    modules: &BTreeMap<String, String>,
    digests: &BTreeMap<String, String>,
    owner: &dyn Fn(&str) -> Option<String>,
    crates: &LoadPathFacts,
) -> Result<Evaluation, String> {
    let declared = contracts.contract_dependencies().clone();
    if declared.is_empty() {
        return Ok(Evaluation::default());
    }
    let in_force = |contracts: &ResourceContracts, name: &str| {
        contracts.contract(name).is_some()
            || contracts.relational_contract(name).is_some()
            || contracts.retaining_contract(name).is_some()
    };
    // The environment: every load-path crate, and every declaration with
    // its digest and whether it is already held.
    let mut environment: BTreeMap<String, CrateFacts> = crates
        .iter()
        .map(|(name, (version, dependencies))| {
            (
                name.clone(),
                CrateFacts {
                    version: LibraryVersion::parse(version).ok(),
                    dependencies: dependencies.clone(),
                    declarations: BTreeMap::new(),
                },
            )
        })
        .collect();
    for (name, digest) in digests {
        let Some(crate_name) = modules.get(name).and_then(|module| owner(module)) else {
            continue;
        };
        let held = if in_force(contracts, name) {
            None
        } else if let Some(withheld) = contracts.pin_mismatch().get(name) {
            Some(if withheld.reason.is_human_resolvable() {
                HoldClass::Pending
            } else {
                HoldClass::Unknown
            })
        } else if contracts.pending_review().contains_key(name) {
            Some(HoldClass::Pending)
        } else {
            Some(HoldClass::Unknown)
        };
        if let Some(facts) = environment.get_mut(&crate_name) {
            facts.declarations.insert(
                name.clone(),
                DeclarationFacts {
                    sha256: digest.clone(),
                    held,
                },
            );
        }
    }
    // Every stated dependency, with its dependent's crate (none off a load
    // path, which no crate declares as a dependency: unresolved).
    let mut dependencies: Vec<DeclaredDependency> = Vec::new();
    for (name, stated) in &declared {
        let module = modules.get(name).cloned().unwrap_or_default();
        let crate_name = owner(&module).unwrap_or_default();
        for dependency in stated {
            let mut dependency = dependency.clone();
            dependency.dependent_crate.clone_from(&crate_name);
            dependency.module.clone_from(&module);
            dependencies.push(dependency);
        }
    }
    let resolution = connect::resolve(&dependencies, &environment, None);
    let mut held_records = Vec::new();
    for standing in resolution.held() {
        let reason = match standing.held {
            Some(HoldClass::Pending) => PinMismatchReason::DependencyChanged,
            _ => PinMismatchReason::DependencyUnresolved,
        };
        let detail = standing
            .causes
            .iter()
            .map(|(id, failure)| format!("{id}: {} ({})", failure.as_str(), failure.formula()))
            .collect::<Vec<_>>()
            .join("; ");
        let withheld = in_force(contracts, &standing.function);
        if withheld {
            contracts.withhold_for_pin_mismatch(
                &standing.function,
                modules.get(&standing.function).cloned().unwrap_or_default(),
                reason,
                None,
                detail.clone(),
            )?;
        }
        held_records.push(json!({
            "crate": standing.crate_name,
            "function": standing.function,
            "class": standing.held,
            "reason": reason.as_str(),
            "withheld_here": withheld,
            "causes": standing.causes.iter().map(|(id, failure)| json!({
                "dependency": id, "failure": failure.as_str(), "formula": failure.formula(),
                "detail": failure,
            })).collect::<Vec<_>>(),
        }));
    }
    let graph: Vec<Value> = dependencies
        .iter()
        .map(|dependency| {
            let target = environment.get(&dependency.target_crate);
            json!({
                "id": dependency.id(),
                "dependent": dependency.dependent,
                "dependent_crate": dependency.dependent_crate,
                "module": dependency.module,
                "target_crate": dependency.target_crate,
                "target_function": dependency.target_function,
                "clause": dependency.clause(),
                "target_identity": {
                    "version": target.and_then(|facts| facts.version).map(|v| v.to_string()),
                    "sha256": target
                        .and_then(|facts| facts.declarations.get(&dependency.target_function))
                        .map(|facts| facts.sha256.clone()),
                },
            })
        })
        .collect();
    let identity = qcore_schemas::sha256_hex(json!(graph).to_string().as_bytes());
    Ok(Evaluation {
        record: Some(json!({
            "meaning": MEANING,
            "graph_sha256": identity,
            "dependencies": graph,
            "cycles": resolution.cycles.iter().map(|(c, f)| format!("{c}:{f}"))
                .collect::<Vec<_>>(),
            "held": held_records,
        })),
    })
}

/// What the record means, carried in it.
pub const MEANING: &str = "declared contract dependencies (card QUIRIN-AUDIT-CONNECT, contract spec C15.2): a declaration's contract relies on the named crates' declarations with the stated identities (a version range over the crate's version, the target declaration's SHA-256); a dependency that does not resolve holds its declaration (dependency-changed: Pending, a review and re-pin resolve it; dependency-unresolved: Unknown), a held declaration holds every declaration that depends on it, and a cycle is held Unknown; nothing here makes a declaration Safe";

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_contract::contract_loader::load_resource_contract_sources;

    const LIBC: &str = "module contract;\n\nextern fn malloc(size: usize) -> ptr<mut, u8>\n\
        contract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(size);\n    \
        ensures normal resource_return_owned(nullable);\n};\n";

    fn user(clause: &str) -> String {
        format!(
            "module contract;\n\nextern fn acme_open(size: usize) -> ptr<mut, u8>\ncontract {{\n    \
             requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
             {clause}\n    ensures normal resource_borrowed(size);\n    \
             ensures normal resource_return_owned(nullable);\n}};\n"
        )
    }

    fn run(clause: &str, crates: &LoadPathFacts) -> (ResourceContracts, Evaluation) {
        let sources = BTreeMap::from([
            (
                "quirin-provider-libc:libc/memory.qv".to_owned(),
                LIBC.to_owned(),
            ),
            ("quirin-provider-acme:acme/open.qv".to_owned(), user(clause)),
        ]);
        let mut contracts = load_resource_contract_sources(&sources).expect("loads");
        let digests =
            qcore_contract::contract_revision::declaration_digests(contracts.source_assumptions())
                .unwrap();
        let modules: BTreeMap<String, String> = digests
            .iter()
            .map(|(name, (module, _))| (name.clone(), module.clone()))
            .collect();
        let digests: BTreeMap<String, String> = digests
            .into_iter()
            .map(|(name, (_, digest))| (name, digest))
            .collect();
        let owner = |module: &str| module.split_once(':').map(|(c, _)| c.to_owned());
        let evaluation = evaluate(&mut contracts, &modules, &digests, &owner, crates).unwrap();
        (contracts, evaluation)
    }

    fn crates(acme_depends: bool) -> LoadPathFacts {
        BTreeMap::from([
            (
                "quirin-provider-libc".to_owned(),
                ("0.1.0".to_owned(), std::collections::BTreeSet::new()),
            ),
            (
                "quirin-provider-acme".to_owned(),
                (
                    "0.1.0".to_owned(),
                    if acme_depends {
                        std::collections::BTreeSet::from(["quirin-provider-libc".to_owned()])
                    } else {
                        std::collections::BTreeSet::new()
                    },
                ),
            ),
        ])
    }

    fn malloc_digest() -> String {
        let sources = BTreeMap::from([(
            "quirin-provider-libc:libc/memory.qv".to_owned(),
            LIBC.to_owned(),
        )]);
        qcore_contract::contract_revision::declaration_digests(&sources).unwrap()["malloc"]
            .1
            .clone()
    }

    #[test]
    fn a_satisfied_dependency_keeps_the_declaration_in_force() {
        let clause = format!(
            "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, since \
             \"0.1.0\", sha256 \"{}\");",
            malloc_digest()
        );
        let (contracts, evaluation) = run(&clause, &crates(true));
        assert!(contracts.contract("acme_open").is_some());
        let record = evaluation.record.unwrap();
        assert_eq!(record["held"], json!([]));
        assert_eq!(record["dependencies"][0]["target_function"], "malloc");
    }

    #[test]
    fn every_unresolved_dependency_withholds_with_its_class() {
        for (clause, depends, reason) in [
            // Out of range: Unknown.
            (
                "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, \
                 since \"0.2.0\");"
                    .to_owned(),
                true,
                PinMismatchReason::DependencyUnresolved,
            ),
            // Digest changed: Pending.
            (
                format!(
                    "requires resource_contract_dependency(\"quirin-provider-libc\", \
                     libc::malloc, sha256 \"{}\");",
                    "0".repeat(64)
                ),
                true,
                PinMismatchReason::DependencyChanged,
            ),
            // The crate is not a declared dependency: Unknown.
            (
                "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, \
                 since \"0.1.0\");"
                    .to_owned(),
                false,
                PinMismatchReason::DependencyUnresolved,
            ),
            // A declaration the target crate does not load: Unknown.
            (
                "requires resource_contract_dependency(\"quirin-provider-libc\", libc::calloc, \
                 since \"0.1.0\");"
                    .to_owned(),
                true,
                PinMismatchReason::DependencyUnresolved,
            ),
        ] {
            let (contracts, evaluation) = run(&clause, &crates(depends));
            assert!(contracts.contract("acme_open").is_none(), "{clause}");
            assert_eq!(
                contracts.pin_mismatch()["acme_open"].reason,
                reason,
                "{clause}"
            );
            assert_eq!(
                reason.is_human_resolvable(),
                reason == PinMismatchReason::DependencyChanged
            );
            assert_eq!(evaluation.record.unwrap()["held"][0]["withheld_here"], true);
        }
        // The libc crate missing from the load path: Unknown.
        let mut missing = crates(true);
        missing.remove("quirin-provider-libc");
        let (contracts, _) = run(
            "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, \
             since \"0.1.0\");",
            &missing,
        );
        assert_eq!(
            contracts.pin_mismatch()["acme_open"].reason,
            PinMismatchReason::DependencyUnresolved
        );
    }

    #[test]
    fn no_dependency_no_record() {
        let (contracts, evaluation) = run("", &crates(true));
        assert!(evaluation.record.is_none());
        assert!(contracts.pin_mismatch().is_empty());
    }

    /// Adopting a dependency in an existing declaration changes the
    /// declaration's normalised digest (the clause is part of its text), so
    /// its lock entry no longer covers it (`contract-changed`) until a person
    /// re-pins it: why no shipped contract adopts one in this card.
    #[test]
    fn adopting_a_dependency_changes_the_declaration_digest() {
        let digest = |clause: &str| {
            let sources =
                BTreeMap::from([("quirin-provider-acme:acme/open.qv".to_owned(), user(clause))]);
            qcore_contract::contract_revision::declaration_digests(&sources).unwrap()["acme_open"]
                .1
                .clone()
        };
        assert_ne!(
            digest(""),
            digest(
                "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, \
                 since \"0.1.0\");"
            )
        );
    }
}
