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

//! Card W45-CONTRACT-DEFERRED-REVIEW: a contract, or one clause of it, marked
//! as awaiting human review (contract spec §C06).
//!
//! The loader validates a marked declaration in full and then **withholds**
//! it: it is recorded in `ResourceContracts::pending_review` and never enters
//! the function map, so a consumer that knows nothing of the marker sees an
//! uncontracted callee. These tests pin the round trip, the refusals that keep
//! loading total-or-reject, the propagation through inheritance (§C15) and the
//! byte identity of every environment without a marker. No fixture file is
//! added: every module is an inline text, so the corpus pins do not move.

use std::collections::BTreeMap;

use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
};
use qcore_contract::contract_loader::{
    PublishedModuleRef, check_source_assumptions, load_published_resource_contracts,
    load_resource_contract_sources,
};
use qcore_contract::extern_bridge::pending_review_refusal;
use qcore_schemas::contract::{
    ContractTrust, ResourceContracts, ResourceParameterMode, ReviewDeferral, ReviewDisposition,
    ReviewScope,
};

const BARE: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), text.to_owned())]))
}

/// `BARE` with `k_close`'s contract deferred as a whole.
fn contract_deferred(marker: &str) -> String {
    BARE.replace(
        "    ensures normal resource_released(h, nonnull);",
        &format!("    {marker}\n    ensures normal resource_released(h, nonnull);"),
    )
}

/// `BARE` with one `k_close` clause replaced by `replacement`.
fn clause_replaced(clause: &str, replacement: &str) -> String {
    assert!(BARE.contains(clause), "{clause}");
    BARE.replace(clause, replacement)
}

fn refused(text: &str) -> String {
    load(text).expect_err("the whole module must be refused")
}

#[test]
fn a_contract_level_marker_withholds_the_validated_contract() {
    let bare = load(BARE).unwrap();
    let deferred = load(&contract_deferred(
        "requires resource_review_pending(\"the size argument is unverified\", \"cosmo\");",
    ))
    .unwrap();
    // Withheld: no consumer can apply it by name.
    assert!(deferred.contract("k_close").is_none());
    assert_eq!(deferred.contract("k_open"), bare.contract("k_open"));
    let pending = &deferred.pending_review()["k_close"];
    assert_eq!(pending.trust, ContractTrust::PendingHumanReview);
    assert_eq!(pending.module, "k.qcore");
    // Validated in full: the withheld contract is exactly the bare one.
    assert_eq!(&pending.contract, bare.contract("k_close").unwrap());
    assert_eq!(
        pending.deferrals,
        vec![ReviewDeferral {
            scope: ReviewScope::Contract,
            reason: "the size argument is unverified".into(),
            owner: Some("cosmo".into()),
            disposition: ReviewDisposition::AcceptableAsWrittenAfterReview,
        }]
    );
    // The environment reproduces itself from its retained text, and a
    // tampered pending map is refused.
    check_source_assumptions(&deferred).unwrap();
    let mut forged = deferred.clone();
    forged.set_pending_review(BTreeMap::new());
    assert!(check_source_assumptions(&forged).is_err());
    // And it round-trips through its serialized form.
    let json = serde_json::to_string(&deferred).unwrap();
    assert_eq!(
        serde_json::from_str::<ResourceContracts>(&json).unwrap(),
        deferred
    );
    assert!(json.contains("\"pending_review\""));
    assert!(json.contains("\"trust\":\"pending_human_review\""));
}

#[test]
fn a_clause_level_marker_withholds_the_whole_contract_and_names_the_clause() {
    let bare = load(BARE).unwrap();
    let deferred = load(&clause_replaced(
        "ensures normal resource_released(h, nonnull);",
        "ensures normal resource_review_pending(resource_released(h,   nonnull), \
         \"is the handle really freed on the error path?\");",
    ))
    .unwrap();
    assert!(
        deferred.contract("k_close").is_none(),
        "never partially applied"
    );
    let pending = &deferred.pending_review()["k_close"];
    assert_eq!(&pending.contract, bare.contract("k_close").unwrap());
    assert_eq!(
        pending.contract.parameters[0],
        ResourceParameterMode::Released { nullable: false },
        "the wrapped clause is read exactly as if written bare"
    );
    assert_eq!(
        pending.deferrals,
        vec![ReviewDeferral {
            scope: ReviewScope::Clause {
                clause: "ensures normal resource_released(h, nonnull)".into(),
            },
            reason: "is the handle really freed on the error path?".into(),
            owner: None,
            disposition: ReviewDisposition::AcceptableAsWrittenAfterReview,
        }]
    );
    check_source_assumptions(&deferred).unwrap();
}

#[test]
fn several_clauses_may_be_deferred_each_once() {
    let text = clause_replaced(
        "ensures normal resource_released(h, nonnull);\n    ensures normal resource_borrowed(n);",
        "ensures normal resource_review_pending(resource_released(h, nonnull), \"one\");\n    \
         ensures normal resource_review_pending(resource_borrowed(n), \"two\");",
    );
    let deferred = load(&text).unwrap();
    let reasons = deferred.pending_review()["k_close"]
        .deferrals
        .iter()
        .map(|deferral| deferral.reason.as_str())
        .collect::<Vec<_>>();
    assert_eq!(reasons, ["one", "two"]);
}

#[test]
fn removing_the_marker_restores_the_original_environment() {
    let bare = load(BARE).unwrap();
    for text in [
        contract_deferred("requires resource_review_pending(\"why\");"),
        clause_replaced(
            "ensures normal resource_return_none();",
            "ensures normal resource_review_pending(resource_return_none(), \"why\");",
        ),
    ] {
        let deferred = load(&text).unwrap();
        // Unwrapping reproduces the bare text, hence the bare environment.
        let unwrapped = text
            .replace("    requires resource_review_pending(\"why\");\n", "")
            .replace(
                "resource_review_pending(resource_return_none(), \"why\")",
                "resource_return_none()",
            );
        assert_eq!(unwrapped, BARE);
        let restored = load(&unwrapped).unwrap();
        assert_eq!(restored, bare);
        assert!(restored.pending_review().is_empty());
        // The pending contract is the one the restored environment applies.
        assert_eq!(
            &deferred.pending_review()["k_close"].contract,
            restored.contract("k_close").unwrap()
        );
    }
}

#[test]
fn an_environment_without_a_marker_serializes_as_before() {
    let json = serde_json::to_string(&load(BARE).unwrap()).unwrap();
    assert!(!json.contains("pending_review"), "{json}");
}

#[test]
fn a_missing_or_empty_reason_refuses_the_module() {
    for marker in [
        "requires resource_review_pending();",
        "requires resource_review_pending(\"\");",
        "requires resource_review_pending(\"   \");",
    ] {
        let error = refused(&contract_deferred(marker));
        assert!(
            error.contains("no reason") || error.contains("empty reason"),
            "{marker}: {error}"
        );
    }
    let error = refused(&clause_replaced(
        "ensures normal resource_released(h, nonnull);",
        "ensures normal resource_review_pending(resource_released(h, nonnull));",
    ));
    assert!(error.contains("no reason"), "{error}");
}

#[test]
fn a_malformed_reason_or_owner_refuses_the_module() {
    for marker in [
        // Not a string literal.
        "requires resource_review_pending(unverified);",
        "requires resource_review_pending(\"why\", cosmo);",
        // An escape: one plain spelling per reason.
        "requires resource_review_pending(\"why \\\" not\");",
        // A third string.
        "requires resource_review_pending(\"why\", \"cosmo\", \"extra\");",
        // A trailing comma.
        "requires resource_review_pending(\"why\",);",
        // An empty owner.
        "requires resource_review_pending(\"why\", \"\");",
    ] {
        let error = refused(&contract_deferred(marker));
        assert!(
            error.contains("review marker"),
            "{marker}: the refusal names the marker: {error}"
        );
    }
}

#[test]
fn a_duplicate_marker_refuses_the_module() {
    let error = refused(&contract_deferred(
        "requires resource_review_pending(\"one\");\n    \
         requires resource_review_pending(\"two\");",
    ));
    assert!(error.contains("duplicate review marker"), "{error}");
    // A contract-level marker beside a clause-level one, in either order.
    let text = contract_deferred("requires resource_review_pending(\"whole\");").replace(
        "ensures normal resource_borrowed(n);",
        "ensures normal resource_review_pending(resource_borrowed(n), \"clause\");",
    );
    assert!(refused(&text).contains("duplicate review marker"));
    let text = BARE
        .replace(
            "    ensures normal resource_released(h, nonnull);",
            "    ensures normal resource_review_pending(resource_released(h, nonnull), \"c\");",
        )
        .replace(
            "    ensures normal resource_return_none();\n};\n",
            "    ensures normal resource_return_none();\n    \
             requires resource_review_pending(\"whole\");\n};\n",
        );
    assert!(refused(&text).contains("duplicate review marker"));
    // The same clause twice is the loader's own duplicate-mode refusal.
    let text = clause_replaced(
        "ensures normal resource_borrowed(n);",
        "ensures normal resource_borrowed(n);\n    \
         ensures normal resource_review_pending(resource_borrowed(n), \"again\");",
    );
    assert!(refused(&text).contains("duplicate mode"));
}

#[test]
fn a_marker_on_an_unsupported_item_refuses_the_module() {
    for (clause, replacement, expected) in [
        // The resource-v1 selector cannot be deferred on its own ...
        (
            "requires resource_contract_v1;",
            "requires resource_review_pending(resource_contract_v1(), \"why\");",
            "cannot be deferred on their own",
        ),
        // ... under either introducer.
        (
            "ensures normal resource_borrowed(n);",
            "ensures normal resource_borrowed(n);\n    \
             ensures normal resource_review_pending(resource_contract_v1(), \"why\");",
            "only an `ensures normal` resource-v1 clause",
        ),
        // A literal-argument premise cannot be deferred on its own.
        (
            "ensures normal resource_borrowed(n);",
            "ensures normal resource_borrowed(n);\n    \
             requires resource_review_pending(resource_literal_argument(n, 0), \"why\");",
            "cannot be deferred on their own",
        ),
        // A contract-level marker is a requirement, not a postcondition.
        (
            "ensures normal resource_borrowed(n);",
            "ensures normal resource_borrowed(n);\n    \
             ensures normal resource_review_pending(\"why\");",
            "is a requirement",
        ),
        // A marker cannot wrap a marker.
        (
            "ensures normal resource_borrowed(n);",
            "ensures normal resource_review_pending(resource_review_pending(resource_borrowed(n), \
             \"a\"), \"b\");",
            "cannot wrap another review marker",
        ),
        // A predicate outside the vocabulary stays refused when wrapped.
        (
            "ensures normal resource_borrowed(n);",
            "ensures normal resource_review_pending(resource_frobnicated(n), \"why\");",
            "unsupported predicate",
        ),
        // So does an error-exit clause: only `ensures normal` is read at all.
        (
            "ensures normal resource_borrowed(n);",
            "ensures error(E) resource_review_pending(resource_borrowed(n), \"why\");",
            "unsupported error",
        ),
    ] {
        let error = refused(&clause_replaced(clause, replacement));
        assert!(error.contains(expected), "{replacement}: {error}");
    }
}

// ---------------------------------------------------------------------------
// Inheritance (contract spec §C15)
// ---------------------------------------------------------------------------

const LIBC: &str = "quirin-provider-libc";
const APP: &str = "quirin-provider-app";

fn krate(name: &str, dependencies: &[&str], modules: &[(&str, &str)]) -> PublishedContractCrate {
    PublishedContractCrate {
        name: name.to_owned(),
        namespace: derive_contract_namespace(name).expect("fixture crate names derive"),
        version: "0.1.0".to_owned(),
        dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
        modules: modules
            .iter()
            .map(|(module, text)| ((*module).to_owned(), (*text).to_owned()))
            .collect(),
        published: modules
            .iter()
            .map(|(module, _)| (*module).to_owned())
            .collect(),
    }
}

const PARENT_WHOLE: &str = "module contract;

extern fn free(ptr: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending(\"parent under review\", \"libc-owner\");
    ensures normal resource_released(ptr, nullable);
    ensures normal resource_return_none();
};
";

const PARENT_CLAUSE: &str = "module contract;

extern fn free(ptr: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_review_pending(resource_released(ptr, nullable), \"clause\");
    ensures normal resource_return_none();
};
";

const CHILD: &str = "module contract;
#include \"quirin-provider-libc:libc/heap.qcore\";

extern fn app_free(p: ptr<mut, u8>) -> unit
contract inherit libc::free;
";

fn inherited(parent: &str) -> ResourceContracts {
    let load_path = ContractLoadPath::new(vec![
        krate(LIBC, &[], &[("libc/heap.qcore", parent)]),
        krate(APP, &[LIBC], &[("app/heap.qcore", CHILD)]),
    ])
    .unwrap();
    load_published_resource_contracts(
        &load_path,
        &[
            PublishedModuleRef::new(LIBC, "libc/heap.qcore"),
            PublishedModuleRef::new(APP, "app/heap.qcore"),
        ],
    )
    .unwrap()
}

#[test]
fn a_child_inheriting_a_deferred_contract_is_deferred() {
    let contracts = inherited(PARENT_WHOLE);
    assert!(contracts.contract("free").is_none());
    assert!(
        contracts.contract("app_free").is_none(),
        "never inherited as trusted"
    );
    let parent = &contracts.pending_review()["free"];
    let child = &contracts.pending_review()["app_free"];
    assert_eq!(child.deferrals, parent.deferrals);
    assert_eq!(child.contract, parent.contract);
    assert_eq!(child.module, "quirin-provider-app:app/heap.qcore");
    check_source_assumptions(&contracts).unwrap();
}

#[test]
fn a_child_inheriting_a_deferred_clause_is_deferred_for_that_clause() {
    let contracts = inherited(PARENT_CLAUSE);
    let child = &contracts.pending_review()["app_free"];
    assert_eq!(
        child.deferrals,
        vec![ReviewDeferral {
            // The parameter is renamed by position, as every inherited clause is.
            scope: ReviewScope::Clause {
                clause: "ensures normal resource_released(p, nullable)".into(),
            },
            reason: "clause".into(),
            owner: None,
            disposition: ReviewDisposition::AcceptableAsWrittenAfterReview,
        }]
    );
    check_source_assumptions(&contracts).unwrap();
}

/// Card W47-REVIEW-HOLD-CONCRETE-REASON: a child inherits a parent's
/// disposition with its marker, so it can never shed "must be rewritten".
#[test]
fn a_child_inherits_the_rewrite_disposition() {
    let parent = PARENT_CLAUSE.replace("\"clause\")", "\"clause\", rewrite_before_acceptance)");
    let contracts = inherited(&parent);
    let child = &contracts.pending_review()["app_free"];
    assert_eq!(
        child.deferrals[0].disposition,
        ReviewDisposition::MustBeRewrittenBeforeAcceptance
    );
    assert_eq!(
        contracts.pending_review()["free"].deferrals[0].disposition,
        ReviewDisposition::MustBeRewrittenBeforeAcceptance
    );
    check_source_assumptions(&contracts).unwrap();
}

// ---------------------------------------------------------------------------
// The rewrite disposition (card W47-REVIEW-HOLD-CONCRETE-REASON)
// ---------------------------------------------------------------------------

#[test]
fn a_trailing_rewrite_word_marks_the_marker_and_nothing_else() {
    let bare = load(BARE).unwrap();
    for (text, scope, owner) in [
        (
            contract_deferred(
                "requires resource_review_pending(\"retained, not freed\", rewrite_before_acceptance);",
            ),
            ReviewScope::Contract,
            None,
        ),
        (
            contract_deferred(
                "requires resource_review_pending(\"retained, not freed\", \"cosmo\", \
                 rewrite_before_acceptance);",
            ),
            ReviewScope::Contract,
            Some("cosmo".to_owned()),
        ),
        (
            clause_replaced(
                "ensures normal resource_released(h, nonnull);",
                "ensures normal resource_review_pending(resource_released(h, nonnull), \
                 \"retained, not freed\", rewrite_before_acceptance);",
            ),
            ReviewScope::Clause {
                clause: "ensures normal resource_released(h, nonnull)".into(),
            },
            None,
        ),
    ] {
        let deferred = load(&text).unwrap();
        let pending = &deferred.pending_review()["k_close"];
        // Withheld exactly as a plain marker withholds: the category changes
        // no contract and no consumer's view.
        assert!(deferred.contract("k_close").is_none());
        assert_eq!(&pending.contract, bare.contract("k_close").unwrap());
        assert_eq!(
            pending.deferrals,
            vec![ReviewDeferral {
                scope,
                reason: "retained, not freed".into(),
                owner,
                disposition: ReviewDisposition::MustBeRewrittenBeforeAcceptance,
            }]
        );
        check_source_assumptions(&deferred).unwrap();
        let json = serde_json::to_string(&deferred).unwrap();
        assert!(
            json.contains("\"disposition\":\"must-be-rewritten-before-acceptance\""),
            "{json}"
        );
        assert_eq!(
            serde_json::from_str::<ResourceContracts>(&json).unwrap(),
            deferred
        );
    }
    // A plain marker serializes exactly as before the card.
    let plain = load(&contract_deferred(
        "requires resource_review_pending(\"why\");",
    ))
    .unwrap();
    let json = serde_json::to_string(&plain).unwrap();
    assert!(!json.contains("disposition"), "{json}");
    assert_eq!(
        plain.pending_review()["k_close"].deferrals[0].disposition,
        ReviewDisposition::AcceptableAsWrittenAfterReview
    );
}

#[test]
fn a_misplaced_or_unknown_disposition_refuses_the_module() {
    for marker in [
        // The word stands last, after the reason and the owner.
        "requires resource_review_pending(rewrite_before_acceptance, \"why\");",
        "requires resource_review_pending(\"why\", rewrite_before_acceptance, \"cosmo\");",
        // Twice.
        "requires resource_review_pending(\"why\", rewrite_before_acceptance, \
         rewrite_before_acceptance);",
        // Another word.
        "requires resource_review_pending(\"why\", accept_as_written);",
        "requires resource_review_pending(\"why\", rewrite);",
        // Not a word: a string in the owner position is an owner, so a third
        // string is refused as ever.
        "requires resource_review_pending(\"why\", \"cosmo\", \"rewrite_before_acceptance\");",
        // Trailing comma.
        "requires resource_review_pending(\"why\", rewrite_before_acceptance,);",
    ] {
        let error = refused(&contract_deferred(marker));
        assert!(error.contains("review marker"), "{marker}: {error}");
    }
}

#[test]
fn a_disposition_is_no_reason() {
    // Fail-closed: the word never stands in for the mandatory reason.
    for marker in [
        "requires resource_review_pending(, rewrite_before_acceptance);",
        "requires resource_review_pending(\"\", rewrite_before_acceptance);",
        "requires resource_review_pending(\"  \", rewrite_before_acceptance);",
    ] {
        let error = refused(&contract_deferred(marker));
        assert!(
            error.contains("no reason") || error.contains("empty reason"),
            "{marker}: {error}"
        );
    }
    for replacement in [
        "ensures normal resource_review_pending(resource_released(h, nonnull), \
         rewrite_before_acceptance);",
        "ensures normal resource_review_pending(resource_released(h, nonnull), \"\", \
         rewrite_before_acceptance);",
    ] {
        let error = refused(&clause_replaced(
            "ensures normal resource_released(h, nonnull);",
            replacement,
        ));
        assert!(error.contains("review marker"), "{replacement}: {error}");
    }
}

// ---------------------------------------------------------------------------
// The checked extern-call route
// ---------------------------------------------------------------------------

#[test]
fn the_extern_call_route_names_the_review_and_joins_nothing() {
    let contracts = load(&contract_deferred(
        "requires resource_review_pending(\"the size is unverified\");",
    ))
    .unwrap();
    let refusal = pending_review_refusal("k_close", &contracts).unwrap();
    assert!(
        refusal.contains("awaits human review (awaiting-human-review)")
            && refusal.contains("the size is unverified")
            && refusal.contains("k.qcore"),
        "{refusal}"
    );
    assert!(pending_review_refusal("k_open", &contracts).is_none());
    // The signatures are still read (the text is validated and visible), but
    // there is no loaded contract to join them with.
    let signatures = qcore_contract::extern_bridge::declared_signatures(&contracts).unwrap();
    assert!(signatures.contains_key("k_close"));
    assert!(contracts.contract("k_close").is_none());
}

// ---------------------------------------------------------------------------
// Other interpreters
// ---------------------------------------------------------------------------

/// The observation interpreter does not recognise the marker: a module that
/// carries one is refused there, never read as a satisfied requirement.
#[test]
fn the_observation_interpreter_refuses_the_marker() {
    let source = "module contract;\n\
                  extern fn observe(result: i32) -> unit\n\
                  contract {\n    requires resource_review_pending(\"why\");\n    \
                  ensures normal result >= 0;\n};\n";
    let values = BTreeMap::from([("result".to_owned(), 0_i128)]);
    assert!(
        qcore_contract::contract_observation::check_contract_observation(
            source, "observe", &values
        )
        .is_err()
    );
}
