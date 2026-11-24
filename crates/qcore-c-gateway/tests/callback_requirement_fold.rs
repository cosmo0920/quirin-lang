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

//! Card CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD (gateway rule 74j;
//! contract specification C04.19): a candidate's `resource_owned(n)`
//! requirement on a parameter actual of the call is folded into that
//! position's custody mode for every invocation clause form, not only at a
//! borrowed position or a released slot object. A plain member invocation
//! through a `resource_released(OBJ, nullable)` object, a synchronous
//! invocation handing a `resource_owned(nullable)` or
//! `resource_consumed(nullable)` actual and a member invocation handing an
//! owned-nullable actual each made a possibly null object reach a candidate
//! requiring it live and non-null before this card (the requirement was
//! dropped); now the position is non-null. A requirement at an out or
//! removed position, which no fold states, refuses the call
//! (`callback-requirement-unfolded`; the out position here, before this card
//! `Safe`). The controls are the shipped shapes,
//! `rd_kafka_op_destroy` (a non-null release handed to the candidate) and
//! `mk_thread_destroy` (a non-null release, the candidate handed `_`), which
//! read as before.

use std::collections::BTreeMap;
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::{MemberFacts, read_unit};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{RunMemberFacts, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::callback::{CallbackActual, CallbackClause, MemberObject};
use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent requirement fold test: {reason}");
            false
        }
    }
}

const MODULE: &str = "module contract;

extern fn mobj_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn mobj_drop(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nullable);
    ensures normal resource_return_none();
};

extern fn mobj_free(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nullable);
    ensures normal resource_callback_member_invoked(o, mobj.on_free, o);
    ensures normal resource_return_none();
};

extern fn mobj_free_nonnull(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nonnull);
    ensures normal resource_callback_member_invoked(o, mobj.on_free, o);
    ensures normal resource_return_none();
};

extern fn mobj_free_supplied(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nonnull);
    ensures normal resource_callback_member_invoked(o, mobj.on_free, _);
    ensures normal resource_return_none();
};

extern fn mobj_visit(h: ptr<mut, u8>, o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(h);
    ensures normal resource_owned(o, nullable);
    ensures normal resource_callback_member_invoked(h, mobj.on_free, o);
    ensures normal resource_return_none();
};

extern fn mobj_each(o: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_owned(o, nullable);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, o);
    ensures normal resource_return_none();
};

extern fn mobj_give(o: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_consumed(o, nullable);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, o);
    ensures normal resource_return_none();
};

extern fn mobj_make(p: ptr<mut, ptr<mut, u8>>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_out(p, nonnull);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, p);
    ensures normal resource_return_none();
};

extern fn on_ok(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_return_none();
};

extern fn on_live(o: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_owned(o, nonnull);
    ensures normal resource_return_none();
};
";

const HEADERS: &str = "\
struct mobj { int x; int (*on_free)(struct mobj *); };
struct mobj *mobj_new(void);
void mobj_drop(struct mobj *o);
void mobj_free(struct mobj *o);
void mobj_free_nonnull(struct mobj *o);
void mobj_free_supplied(struct mobj *o);
void mobj_visit(struct mobj *h, struct mobj *o);
void mobj_each(struct mobj *o, int (*cb)(struct mobj *));
void mobj_give(struct mobj *o, int (*cb)(struct mobj *));
void mobj_make(struct mobj **p, int (*cb)(struct mobj *));
int on_ok(struct mobj *o);
int on_live(struct mobj *o);
";

/// Each pair: the object checked non-null before the call, and not.
const BODY: &str = "\
void free_checked(void) { struct mobj *o = mobj_new(); if (!o) return; mobj_free(o); }
void free_unchecked(void) { struct mobj *o = mobj_new(); mobj_free(o); }
void nonnull_checked(void) { struct mobj *o = mobj_new(); if (!o) return; mobj_free_nonnull(o); }
void nonnull_unchecked(void) { struct mobj *o = mobj_new(); mobj_free_nonnull(o); }
void supplied_checked(void) { struct mobj *o = mobj_new(); if (!o) return; mobj_free_supplied(o); }
void visit_checked(struct mobj *h) { struct mobj *o = mobj_new(); if (!o) return; \
mobj_visit(h, o); mobj_drop(o); }
void visit_unchecked(struct mobj *h) { struct mobj *o = mobj_new(); mobj_visit(h, o); \
mobj_drop(o); }
";

/// The synchronous forms name their candidate at the call.
fn invoked_body(candidate: &str) -> String {
    format!(
        "\
void each_checked(void) {{ struct mobj *o = mobj_new(); if (!o) return; \
mobj_each(o, {candidate}); mobj_drop(o); }}
void each_unchecked(void) {{ struct mobj *o = mobj_new(); mobj_each(o, {candidate}); \
mobj_drop(o); }}
void give_checked(void) {{ struct mobj *o = mobj_new(); if (!o) return; \
mobj_give(o, {candidate}); }}
void give_unchecked(void) {{ struct mobj *o = mobj_new(); mobj_give(o, {candidate}); }}
void make(void) {{ struct mobj *o; mobj_make(&o, {candidate}); mobj_drop(o); }}
"
    )
}

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("mobj.qv".to_owned(), MODULE.to_owned())]))
        .expect("the synthetic module loads")
}

fn unit_facts(body: &str) -> MemberFacts {
    let text = format!("{HEADERS}{body}");
    read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"))
}

/// The run: one unit stores `candidate` into `mobj.on_free`, the other is
/// the calling unit.
fn run(candidate: &str, body: &str) -> Arc<RunMemberFacts> {
    Arc::new(RunMemberFacts::new(
        vec![
            unit_facts(&format!(
                "void reg(struct mobj *o) {{ o->on_free = {candidate}; }}\n"
            )),
            unit_facts(body),
        ],
        BTreeMap::new(),
    ))
}

fn row(
    body: &str,
    name: &str,
    run: Option<Arc<RunMemberFacts>>,
    contracts: &ResourceContracts,
) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{HEADERS}{body}");
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            name,
            contracts,
        )
    })
    .unwrap_or_else(|error| panic!("analysed: {error:?}\n{text}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let row = &summary.functions[name];
    let leaves = row
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect();
    (row.status, leaves)
}

/// The pinned defect: a plain member invocation through a nullable released
/// parameter whose candidate requires the object live and non-null. An
/// identity candidate decides both frees; `on_live` decides the checked free
/// and refuses the possibly null one (before this card the requirement was
/// dropped and `free_unchecked` was `Safe`).
#[test]
fn a_member_invocation_through_a_nullable_release_folds_the_requirement() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    for name in ["free_checked", "free_unchecked"] {
        let (status, leaves) = row(BODY, name, Some(run("on_ok", BODY)), &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
    let (status, leaves) = row(BODY, "free_checked", Some(run("on_live", BODY)), &contracts);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    let (status, leaves) = row(
        BODY,
        "free_unchecked",
        Some(run("on_live", BODY)),
        &contracts,
    );
    assert_ne!(
        status,
        AnalysisStatus::Safe,
        "a possibly null release handed to a candidate requiring it non-null: {leaves:?}"
    );
}

/// An owned-nullable actual of a member invocation (the object a borrowed
/// holder): the same fold.
#[test]
fn an_owned_nullable_member_actual_folds_the_requirement() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    for name in ["visit_checked", "visit_unchecked"] {
        let (status, leaves) = row(BODY, name, Some(run("on_ok", BODY)), &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
    let (status, leaves) = row(
        BODY,
        "visit_checked",
        Some(run("on_live", BODY)),
        &contracts,
    );
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    let (status, leaves) = row(
        BODY,
        "visit_unchecked",
        Some(run("on_live", BODY)),
        &contracts,
    );
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
}

/// The synchronous form (rule 13u) at an owned-nullable and a
/// consumed-nullable position: the same fold.
#[test]
fn a_synchronous_invocation_folds_at_owned_and_consumed_positions() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    let ok = invoked_body("on_ok");
    for name in [
        "each_checked",
        "each_unchecked",
        "give_checked",
        "give_unchecked",
    ] {
        let (status, leaves) = row(&ok, name, None, &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
    let live = invoked_body("on_live");
    for name in ["each_checked", "give_checked"] {
        let (status, leaves) = row(&live, name, None, &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
    for name in ["each_unchecked", "give_unchecked"] {
        let (status, leaves) = row(&live, name, None, &contracts);
        assert_ne!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// An out position holds no object for the requirement: a candidate
/// requiring its actual live refuses the call (`callback-requirement-unfolded`);
/// an identity candidate reads the call as before.
#[test]
fn a_requirement_at_an_out_position_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    let (status, leaves) = row(&invoked_body("on_ok"), "make", None, &contracts);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    let (status, leaves) = row(&invoked_body("on_live"), "make", None, &contracts);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-requirement-unfolded")),
        "{leaves:?}"
    );
}

/// The controls, the shipped shapes: a non-null release handed to the
/// candidate (`rd_kafka_op_destroy`) and a non-null release whose candidate
/// is handed `_` (`mk_thread_destroy`) read exactly as before: the release
/// already requires the object non-null, so the checked call decides and the
/// unchecked one refuses, with either candidate.
#[test]
fn the_shipped_non_null_release_shapes_are_unchanged() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    for candidate in ["on_ok", "on_live"] {
        let (status, leaves) = row(
            BODY,
            "nonnull_checked",
            Some(run(candidate, BODY)),
            &contracts,
        );
        assert_eq!(status, AnalysisStatus::Safe, "{candidate}: {leaves:?}");
        let (status, leaves) = row(
            BODY,
            "nonnull_unchecked",
            Some(run(candidate, BODY)),
            &contracts,
        );
        assert_ne!(status, AnalysisStatus::Safe, "{candidate}: {leaves:?}");
        let (status, leaves) = row(
            BODY,
            "supplied_checked",
            Some(run(candidate, BODY)),
            &contracts,
        );
        assert_eq!(status, AnalysisStatus::Safe, "{candidate}: {leaves:?}");
    }
}

const RK_CLIENT: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv");
const MK_LIST: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv");

/// The shipped declarations themselves: every member invocation through a
/// released object releases it non-null, so every requirement the fold can
/// put on such a position leaves its mode as declared.
#[test]
fn the_shipped_member_releases_are_non_null() {
    let mut seen = Vec::new();
    for (module, text) in [
        ("librdkafka/client.qv", RK_CLIENT),
        ("monkey/intrusive_list.qv", MK_LIST),
    ] {
        let contracts =
            load_resource_contract_sources(&BTreeMap::from([(module.to_owned(), text.to_owned())]))
                .unwrap_or_else(|error| panic!("{module}: {error}"));
        for (name, declaration) in contracts.retaining() {
            for clause in &declaration.callbacks {
                let CallbackClause::MemberInvoked {
                    object: MemberObject::Parameter { index },
                    actuals,
                    ..
                } = clause
                else {
                    continue;
                };
                let mode = declaration.contract.parameters[*index];
                if !matches!(mode, ResourceParameterMode::Released { .. }) {
                    continue;
                }
                assert_eq!(
                    mode,
                    ResourceParameterMode::Released { nullable: false },
                    "{name}"
                );
                for actual in actuals {
                    if let CallbackActual::Parameter { index } = actual {
                        let mode = declaration.contract.parameters[*index];
                        for nullable in [false, true] {
                            assert_eq!(
                                qcore_c_gateway::callback_contracts::fold_requirement(
                                    mode, nullable
                                ),
                                Some(mode),
                                "{name}"
                            );
                        }
                    }
                }
                seen.push(name.clone());
            }
        }
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen, ["mk_thread_destroy", "rd_kafka_op_destroy"]);
}
