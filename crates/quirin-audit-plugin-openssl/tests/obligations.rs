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

//! Card W48-OPENSSL-CONTRACT-PLUGIN: a binding's obligations over a small
//! release-shaped tree pinned by its own digest, and what the host's firewall
//! and join make of the claims: a `Safe` proposal only when every obligation
//! is discharged, published `Pending` and never `Safe`; everything else
//! `Unknown`, with the undischarged obligation as its reason; a pin mismatch
//! makes every binding `Unknown`.

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use qcore_audit::subcommand_plugins::{classify_claim, join};
use qcore_schemas::verdict::{ReasonedVerdict, Verdict};
use quirin_audit_plugin::ProposedVerdict;
use quirin_audit_plugin_openssl::contracts::{Binding, Class, ContractSet, Definition};
use quirin_audit_plugin_openssl::pin::{self, Pin};
use quirin_audit_plugin_openssl::{BindingCheck, check_binding, claim, environment};

const HEADER: &str = "struct mini_st;\n\
void MINI_clear(void);\n\
int MINI_get(const struct mini_st *m);\n\
void MINI_free(struct mini_st *m);\n\
long MINI_wide(void);\n\
int MINI_elsewhere(void);\n";

const SOURCE: &str = "#include <openssl/mini.h>\n\
struct mini_st { int v; };\n\
void MINI_clear(void) { }\n\
int MINI_get(const struct mini_st *m) { return m->v; }\n\
void MINI_free(struct mini_st *m) { (void)m; }\n\
long MINI_wide(void) { return 0; }\n";

fn tree(directory: &Path) -> std::path::PathBuf {
    let tree = directory.join("mini-9.9.9");
    std::fs::create_dir_all(tree.join("include/openssl")).unwrap();
    std::fs::create_dir_all(tree.join("src")).unwrap();
    std::fs::write(
        tree.join("VERSION.dat"),
        "MAJOR=9\nMINOR=9\nPATCH=9\nPRE_RELEASE_TAG=\n",
    )
    .unwrap();
    std::fs::write(tree.join("include/openssl/mini.h"), HEADER).unwrap();
    std::fs::write(tree.join("src/mini.c"), SOURCE).unwrap();
    std::fs::write(tree.join("src/other.c"), "#include <openssl/mini.h>\n").unwrap();
    tree
}

fn pinned(tree: &Path) -> Pin {
    let (digest, _) = pin::tree_digest(tree, &pin::BYPRODUCTS).unwrap();
    Pin {
        version: "9.9.9",
        directory: "mini-9.9.9",
        prepared_tree_sha256: Box::leak(digest.into_boxed_str()),
        ..pin::OPENSSL
    }
}

fn binding(function: &str, prototype: &str, class: &str, file: Option<&str>) -> Binding {
    Binding {
        function: function.to_owned(),
        header: "openssl/mini.h".to_owned(),
        prototype: prototype.to_owned(),
        class: class.to_owned(),
        contract: format!("the contract of {function}"),
        definition: file.map(|file| Definition {
            file: file.to_owned(),
            object: "unused.o".to_owned(),
            flags: "lib".to_owned(),
        }),
        returns: None,
        stores: None,
        escapes: None,
        census_pairs: 1,
    }
}

fn set() -> ContractSet {
    let class = |checked: bool| Class {
        checked,
        meaning: "m".to_owned(),
    };
    let set = ContractSet {
        schema: "quirin-openssl-contracts/1".to_owned(),
        contract_set_version: 1,
        library: "openssl".to_owned(),
        release: "9.9.9".to_owned(),
        flag_sets: BTreeMap::from([("lib".to_owned(), vec!["-Iinclude".to_owned()])]),
        classes: BTreeMap::from([
            ("no-pointer-interface".to_owned(), class(true)),
            ("static-object-return".to_owned(), class(true)),
            ("pure-reader".to_owned(), class(true)),
            ("member-store".to_owned(), class(true)),
            ("refcounted-release".to_owned(), class(false)),
        ]),
        bindings: vec![
            binding("MINI_clear", "void (void)", "no-pointer-interface", Some("src/mini.c")),
            binding("MINI_get", "int (const struct mini_st *)", "pure-reader", Some("src/mini.c")),
            binding("MINI_free", "void (struct mini_st *)", "refcounted-release", None),
            // The contract's prototype is not the header's.
            binding("MINI_wide", "int (void)", "no-pointer-interface", Some("src/mini.c")),
            // Declared, but not defined where the contract says.
            binding("MINI_elsewhere", "int (void)", "no-pointer-interface", Some("src/other.c")),
            // Not declared at all.
            binding("MINI_absent", "int (void)", "no-pointer-interface", Some("src/mini.c")),
        ],
    };
    set.validate("9.9.9").unwrap();
    set
}

fn checks(pin: &Pin, set: &ContractSet, tree: &Path) -> BTreeMap<String, BindingCheck> {
    let environment = environment(pin, set, tree, &support::clang());
    set.bindings
        .iter()
        .map(|binding| {
            (
                binding.function.clone(),
                check_binding(&environment, set, binding),
            )
        })
        .collect()
}

fn failed(check: &BindingCheck) -> Option<&'static str> {
    check
        .obligations
        .iter()
        .find(|o| !o.discharged)
        .map(|o| o.name)
}

#[test]
fn a_binding_is_safe_only_when_every_obligation_is_discharged() {
    let directory = support::scratch("obligations");
    let tree = tree(&directory);
    let pin = pinned(&tree);
    let set = set();
    let checks = checks(&pin, &set, &tree);
    assert_eq!(checks["MINI_clear"].proposed(), ProposedVerdict::Safe);
    assert_eq!(checks["MINI_get"].proposed(), ProposedVerdict::Safe);
    for (function, obligation) in [
        ("MINI_free", "class-rule"),
        ("MINI_wide", "declared"),
        ("MINI_elsewhere", "defined"),
        ("MINI_absent", "declared"),
    ] {
        let check = &checks[function];
        assert_eq!(check.proposed(), ProposedVerdict::Unknown, "{function}");
        assert_eq!(failed(check), Some(obligation), "{function}");
        assert!(check.reason().contains(obligation), "{function}");
    }
    assert!(
        checks["MINI_free"]
            .reason()
            .contains("no rule for contract class `refcounted-release`")
    );
    // The plugin never proposes Unsafe.
    assert!(
        checks
            .values()
            .all(|check| check.proposed() != ProposedVerdict::Unsafe)
    );
}

#[test]
fn a_pin_mismatch_makes_every_binding_unknown() {
    let directory = support::scratch("pin");
    let tree = tree(&directory);
    let pin = pinned(&tree);
    let set = set();
    // One byte of one file: the prepared tree no longer has the pinned digest.
    std::fs::write(tree.join("src/other.c"), "#include <openssl/mini.h> \n").unwrap();
    let checks = checks(&pin, &set, &tree);
    for (function, check) in &checks {
        assert_eq!(check.proposed(), ProposedVerdict::Unknown, "{function}");
        assert_eq!(failed(check), Some("release-pin"), "{function}");
    }
    // An absent tree, and a tree of another version, are mismatches too.
    let absent = checks(&pin, &set, &directory.join("nothing"));
    assert!(absent.values().all(|check| failed(check) == Some("release-pin")));
    std::fs::write(tree.join("src/other.c"), "#include <openssl/mini.h>\n").unwrap();
    let other = Pin {
        version: "9.9.8",
        ..pin.clone()
    };
    let checks = checks(&other, &set, &tree);
    assert!(checks.values().all(|check| failed(check) == Some("release-pin")));
}

#[test]
fn the_host_publishes_a_safe_proposal_pending_and_never_safe() {
    let directory = support::scratch("compose");
    let tree = tree(&directory);
    let pin = pinned(&tree);
    let set = set();
    let checks = checks(&pin, &set, &tree);
    let mut claims = Vec::new();
    for binding in &set.bindings {
        let claim = claim(&pin, binding, &checks[&binding.function], "clang", None);
        let classified = classify_claim(&claim).verdict("quirin-audit-openssl@test");
        let expected = match claim.verdict {
            ProposedVerdict::Safe => Verdict::Pending,
            _ => Verdict::Unknown,
        };
        assert_eq!(classified.verdict(), expected, "{}", binding.function);
        claims.push((claim.artifact.clone(), classified));
    }
    let host = BTreeMap::from([(
        "openssl@9.9.9::MINI_clear".to_owned(),
        ReasonedVerdict::violation("residual-custody", "a host row that is Unsafe"),
    )]);
    let (rows, aggregate) = join(&host, &ReasonedVerdict::safe(), &claims);
    assert_eq!(rows.len(), set.bindings.len());
    // No row is Safe; the host's Unsafe is not lowered by the plugin's Pending.
    assert!(rows.values().all(|row| row.verdict() != Verdict::Safe));
    assert_eq!(rows["openssl@9.9.9::MINI_clear"].verdict(), Verdict::Unsafe);
    assert_eq!(rows["openssl@9.9.9::MINI_get"].verdict(), Verdict::Pending);
    assert_eq!(rows["openssl@9.9.9::MINI_free"].verdict(), Verdict::Unknown);
    // The Unknown row keeps the plugin's own reason.
    assert!(
        rows["openssl@9.9.9::MINI_free"]
            .reasons()
            .iter()
            .any(|reason| reason.detail.contains("refcounted-release"))
    );
    assert_eq!(aggregate.unwrap().verdict(), Verdict::Unsafe);
}

/// W48-PLUGIN-VERDICT-PRESERVATION (discharger): the published row of an
/// uncertified `Safe` proposal carries the firewall's reason only; the
/// plugin's own reason (here, every obligation it discharged) reaches the
/// host's report in its `claims` record but not in the row's reason multiset,
/// so a projection of the row alone cannot recover it. This asserts the
/// preserving behaviour and fails on the base host; it is ignored until that
/// card closes the gap in the host's composition path, which this card does
/// not edit.
#[test]
#[ignore = "W48-PLUGIN-VERDICT-PRESERVATION: the row of an uncertified Safe claim drops the plugin's own reason"]
fn the_row_of_an_uncertified_safe_claim_keeps_the_plugins_reason() {
    let directory = support::scratch("preserve");
    let tree = tree(&directory);
    let pin = pinned(&tree);
    let set = set();
    let checks = checks(&pin, &set, &tree);
    let binding = &set.bindings[0];
    let claim = claim(&pin, binding, &checks[&binding.function], "clang", None);
    assert_eq!(claim.verdict, ProposedVerdict::Safe);
    let classified = classify_claim(&claim).verdict("quirin-audit-openssl@test");
    let (rows, _) = join(
        &BTreeMap::new(),
        &ReasonedVerdict::safe(),
        &[(claim.artifact.clone(), classified)],
    );
    assert!(
        rows[&claim.artifact]
            .reasons()
            .iter()
            .any(|reason| reason.detail.contains(&claim.reason)),
        "the row's reasons {:?} do not carry the plugin's reason `{}`",
        rows[&claim.artifact].reasons(),
        claim.reason
    );
}
