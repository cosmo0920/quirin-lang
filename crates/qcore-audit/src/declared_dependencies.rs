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
//! A declaration states which other crates' declarations its contract relies
//! on as `linkage_contract_v1` links (`quant #link NS::NAME;`, card
//! CONNECT-ADOPT-DECLARED-DEPENDENCIES, `qcore_contract::contract_dependency`)
//! or by a `contract inherit NS::NAME;` directive, itself the link. The
//! load-path manifest records every one with the target's identity (its lock
//! digest and crate version, written by `providers/contracts.py sync`).
//! After the pins and the library version ranges are evaluated (so a target
//! already withheld counts as held), [`evaluate`] resolves every dependency
//! of a loaded declaration against the loaded environment and its load path
//! (`qcore_schemas::connect::resolve`) and withholds every declaration it
//! holds, through the pin-mismatch table the legacy-C gateway already holds
//! call sites for:
//!
//! * a changed target digest, or a target held `Pending`: `dependency-changed`,
//!   held `Pending` (`contract-pin-mismatch`: a review and re-pin resolve it);
//! * a crate missing from the load path, a dependent off every load path, a
//!   declaration unloaded, a version out of range, a libc/POSIX target named
//!   elsewhere, a cycle, or a target held `Unknown`: `dependency-unresolved`,
//!   held `Unknown` (`missing-premise`).
//!
//! A link the loader read that the manifest does not record (a stale
//! manifest is refused at load, so this is a module read outside its load
//! path's copy) carries no identity and is held `Pending`: an unrecorded
//! digest never matches.
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

/// What the load path's manifest gives a resolution (card
/// CONNECT-ADOPT-DECLARED-DEPENDENCIES).
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    /// Every load-path crate's version and declared (Cargo) dependencies.
    pub crates: LoadPathFacts,
    /// Every recorded dependency, by dependent crate, with its identity.
    pub dependencies: BTreeMap<String, Vec<DeclaredDependency>>,
    /// Namespace to load-path crate.
    pub namespaces: BTreeMap<String, String>,
}

/// Resolve every declared dependency of `contracts` and withhold every
/// declaration the resolution holds.
///
/// `modules` and `digests` give, per function, the module key that declares
/// it and the SHA-256 of its normalised text (`contract_revision`'s
/// evaluation); `owner` the crate that owns a module key; `manifest` the load
/// path's crates, recorded dependencies and namespaces.
///
/// # Errors
///
/// A withholding the environment refuses.
pub fn evaluate(
    contracts: &mut ResourceContracts,
    modules: &BTreeMap<String, String>,
    digests: &BTreeMap<String, String>,
    owner: &dyn Fn(&str) -> Option<String>,
    manifest: &Manifest,
) -> Result<Evaluation, String> {
    let crates = &manifest.crates;
    let declared = contracts.contract_dependencies().clone();
    // Every dependency of a loaded declaration: the manifest's (identity
    // recorded), then every link the loader read that the manifest does not
    // record (none on a load path whose manifest reads; off a load path the
    // dependent has no crate and is unresolved).
    let mut dependencies: Vec<DeclaredDependency> = Vec::new();
    for (name, module) in modules {
        if !digests.contains_key(name) {
            continue;
        }
        let Some(crate_name) = owner(module) else {
            continue;
        };
        dependencies.extend(
            manifest
                .dependencies
                .get(&crate_name)
                .into_iter()
                .flatten()
                .filter(|dependency| dependency.dependent == *name)
                .cloned(),
        );
    }
    for (name, stated) in &declared {
        let module = modules.get(name).cloned().unwrap_or_default();
        let crate_name = owner(&module).unwrap_or_default();
        for written in stated {
            let recorded = dependencies.iter().any(|dependency| {
                dependency.dependent_crate == crate_name
                    && dependency.dependent == *name
                    && dependency.target_namespace == written.target_namespace
                    && dependency.target_function == written.target_function
            });
            if recorded {
                continue;
            }
            let mut dependency = written.clone();
            dependency.dependent_crate.clone_from(&crate_name);
            dependency.module.clone_from(&module);
            if dependency.target_crate.is_empty() {
                dependency.target_crate = manifest
                    .namespaces
                    .get(&dependency.target_namespace)
                    .cloned()
                    .unwrap_or_default();
            }
            dependencies.push(dependency);
        }
    }
    if dependencies.is_empty() {
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
                "form": dependency.form.as_str(),
                "vocabulary": dependency.vocabulary,
                "link": dependency.link(),
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
pub const MEANING: &str = "declared contract dependencies (cards QUIRIN-AUDIT-CONNECT and CONNECT-ADOPT-DECLARED-DEPENDENCIES, contract spec C15.2, vocabulary linkage_contract_v1): a declaration's contract relies on the named crates' declarations, each a link or an inherit directive, with the identities the load-path manifest records (the target crate's version, the target declaration's SHA-256); a dependency that does not resolve holds its declaration (dependency-changed: Pending, a review and re-pin resolve it; dependency-unresolved: Unknown), a held declaration holds every declaration that depends on it, and a cycle is held Unknown; nothing here makes a declaration Safe";

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use qcore_schemas::connect::{DependencyForm, LINKAGE_VOCABULARY};

    const LIBC: &str = "module contract;\n\nextern fn malloc(size: usize) -> ptr<mut, u8>\n\
        contract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(size);\n    \
        ensures normal resource_return_owned(nullable);\n};\n";

    const LINK: &str = "requires linkage_contract_v1;\n    quant #link libc::malloc;";

    fn user(clause: &str) -> String {
        format!(
            "module contract;\n\nextern fn acme_open(size: usize) -> ptr<mut, u8>\ncontract {{\n    \
             requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
             {clause}\n    ensures normal resource_borrowed(size);\n    \
             ensures normal resource_return_owned(nullable);\n}};\n"
        )
    }

    fn run(clause: &str, manifest: &Manifest, owned: bool) -> (ResourceContracts, Evaluation) {
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
        let owner = |module: &str| {
            module
                .split_once(':')
                .map(|(c, _)| c.to_owned())
                .filter(|c| owned || c != "quirin-provider-acme")
        };
        let evaluation = evaluate(&mut contracts, &modules, &digests, &owner, manifest).unwrap();
        (contracts, evaluation)
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

    fn recorded(
        target: &str,
        form: DependencyForm,
        since: &str,
        sha256: &str,
    ) -> DeclaredDependency {
        DeclaredDependency {
            dependent_crate: "quirin-provider-acme".into(),
            dependent: "acme_open".into(),
            module: "acme/open.qv".into(),
            target_crate: "quirin-provider-libc".into(),
            target_namespace: "libc".into(),
            target_function: target.into(),
            since: Some(LibraryVersion::parse(since).unwrap()),
            removed: None,
            sha256: Some(sha256.into()),
            form,
            vocabulary: LINKAGE_VOCABULARY.into(),
        }
    }

    /// A load path of the two crates (no Cargo edge: D1 (a)) recording
    /// `dependencies`.
    fn manifest(dependencies: Vec<DeclaredDependency>, libc: bool) -> Manifest {
        let mut crates = LoadPathFacts::new();
        if libc {
            crates.insert(
                "quirin-provider-libc".into(),
                ("0.1.0".into(), std::collections::BTreeSet::new()),
            );
        }
        crates.insert(
            "quirin-provider-acme".into(),
            ("0.1.0".into(), std::collections::BTreeSet::new()),
        );
        Manifest {
            crates,
            dependencies: if dependencies.is_empty() {
                BTreeMap::new()
            } else {
                BTreeMap::from([("quirin-provider-acme".to_owned(), dependencies)])
            },
            namespaces: BTreeMap::from([
                ("libc".to_owned(), "quirin-provider-libc".to_owned()),
                ("acme".to_owned(), "quirin-provider-acme".to_owned()),
            ]),
        }
    }

    #[test]
    fn a_resolved_link_keeps_the_declaration_in_force() {
        let entry = recorded("malloc", DependencyForm::Link, "0.1.0", &malloc_digest());
        let (contracts, evaluation) = run(LINK, &manifest(vec![entry], true), true);
        assert!(contracts.contract("acme_open").is_some());
        let record = evaluation.record.unwrap();
        assert_eq!(record["held"], json!([]));
        assert_eq!(record["dependencies"][0]["target_function"], "malloc");
        assert_eq!(
            record["dependencies"][0]["vocabulary"],
            "linkage_contract_v1"
        );
        assert_eq!(
            record["dependencies"][0]["link"],
            "quant #link libc::malloc;"
        );
    }

    #[test]
    fn every_unresolved_link_withholds_with_its_class() {
        let digest = malloc_digest();
        for (entry, libc, reason) in [
            // A stale digest after a target re-pin: Pending.
            (
                recorded("malloc", DependencyForm::Link, "0.1.0", &"0".repeat(64)),
                true,
                PinMismatchReason::DependencyChanged,
            ),
            // Out of range: Unknown.
            (
                recorded("malloc", DependencyForm::Link, "0.2.0", &digest),
                true,
                PinMismatchReason::DependencyUnresolved,
            ),
            // The libc crate missing from the load path: Unknown.
            (
                recorded("malloc", DependencyForm::Link, "0.1.0", &digest),
                false,
                PinMismatchReason::DependencyUnresolved,
            ),
        ] {
            let (contracts, evaluation) = run(LINK, &manifest(vec![entry], libc), true);
            assert!(contracts.contract("acme_open").is_none());
            assert_eq!(contracts.pin_mismatch()["acme_open"].reason, reason);
            assert_eq!(evaluation.record.unwrap()["held"][0]["withheld_here"], true);
        }
        // A declaration the target crate does not load: Unknown.
        let calloc = "requires linkage_contract_v1;\n    quant #link libc::calloc;";
        let entry = recorded("calloc", DependencyForm::Link, "0.1.0", &digest);
        let (contracts, _) = run(calloc, &manifest(vec![entry], true), true);
        assert_eq!(
            contracts.pin_mismatch()["acme_open"].reason,
            PinMismatchReason::DependencyUnresolved
        );
    }

    #[test]
    fn a_link_the_manifest_does_not_record_is_held_pending() {
        // No identity recorded: an unrecorded digest matches nothing.
        let (contracts, _) = run(LINK, &manifest(Vec::new(), true), true);
        assert_eq!(
            contracts.pin_mismatch()["acme_open"].reason,
            PinMismatchReason::DependencyChanged
        );
    }

    #[test]
    fn a_dependent_off_every_load_path_is_unresolved() {
        let (contracts, _) = run(LINK, &manifest(Vec::new(), true), false);
        assert_eq!(
            contracts.pin_mismatch()["acme_open"].reason,
            PinMismatchReason::DependencyUnresolved
        );
    }

    #[test]
    fn a_recorded_inherit_directive_is_resolved_like_a_link() {
        // The manifest records the directive; the declaration states no link.
        let digest = malloc_digest();
        let held = recorded("malloc", DependencyForm::Inherit, "0.1.0", &"1".repeat(64));
        let (contracts, evaluation) = run("", &manifest(vec![held], true), true);
        assert_eq!(
            contracts.pin_mismatch()["acme_open"].reason,
            PinMismatchReason::DependencyChanged
        );
        assert_eq!(
            evaluation.record.unwrap()["dependencies"][0]["form"],
            "inherit"
        );
        let resolved = recorded("malloc", DependencyForm::Inherit, "0.1.0", &digest);
        let (contracts, _) = run("", &manifest(vec![resolved], true), true);
        assert!(contracts.contract("acme_open").is_some());
    }

    #[test]
    fn no_dependency_no_record() {
        let (contracts, evaluation) = run("", &manifest(Vec::new(), true), true);
        assert!(evaluation.record.is_none());
        assert!(contracts.pin_mismatch().is_empty());
    }

    /// Adopting a link changes the declaration's normalised digest (the
    /// clauses are part of its text), so its lock entry no longer covers it
    /// (`contract-changed`) until a person re-pins it.
    #[test]
    fn adopting_a_link_changes_the_declaration_digest() {
        let digest = |clause: &str| {
            let sources =
                BTreeMap::from([("quirin-provider-acme:acme/open.qv".to_owned(), user(clause))]);
            qcore_contract::contract_revision::declaration_digests(&sources).unwrap()["acme_open"]
                .1
                .clone()
        };
        assert_ne!(digest(""), digest(LINK));
    }

    #[test]
    fn the_retired_written_form_is_refused() {
        let sources = BTreeMap::from([(
            "quirin-provider-acme:acme/open.qv".to_owned(),
            user(
                "requires resource_contract_dependency(\"quirin-provider-libc\", libc::malloc, \
                 since \"0.1.0\");",
            ),
        )]);
        let error = load_resource_contract_sources(&sources).unwrap_err();
        assert!(error.contains("QL005"), "{error}");
    }
}
