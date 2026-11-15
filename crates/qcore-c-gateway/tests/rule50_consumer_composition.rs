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

//! Card RULE50-CONSUMER-COMPOSITION (gateway rule 51; contract spec C04.19.6
//! amended), folding in MEMBER-PATH-HELPER-AT-A-CALL, over synthetic
//! Monkey-shaped units (trusted test assumptions: nothing here is a fact
//! about Monkey):
//!
//! * the loader's placement of `resource_member_helper(P, TAG.PATH, H)`:
//!   one clause, on an unguarded `#stale` state of a released formal, on a
//!   declaration with no callback clause and no hand-off;
//! * a consumer's direct call of a destroyer naming its helper composes the
//!   helper's member invocations (each a closed empty set over the run's
//!   facts) and then its states at the object linked at the stale path;
//!   every other case refuses (no run facts, an open or non-empty set, a
//!   missing helper declaration);
//! * a direct call of the helper destroyer itself reads its invocation and
//!   then its states, instead of refusing `member-path-direct`;
//! * rule 50's body reading establishes the take only through the named
//!   helper.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::read_unit;
use qcore_c_gateway::contract_dependencies::Recording;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_c_gateway::member_run_facts::{RunMemberFacts, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
void free(void *p);
struct coro { int x; };
void co_delete(struct coro *co);
struct coro *co_new(void);
struct thr { struct coro *caller; struct coro *callee; void *data; void (*cb_destroy)(void *); };
struct hthread { int close; void *session; void *request; struct thr *parent; };
struct hthread *hthread_new(void);
struct thr *thr_new(void);
";

const HELPER_BODY: &str = "\
static inline void thr_destroy(struct thr *th)
{
    if (th->cb_destroy) {
        th->cb_destroy((char *) th + sizeof(struct thr));
    }
    co_delete(th->callee);
    free(th);
}
";

const DESTROYER_BODY: &str = "\
int hthread_destroy(struct hthread *mth)
{
    struct thr *th;
    th = mth->parent;
    thr_destroy(th);
    return 0;
}
";

const BASE: &str = "\
module contract;

extern fn free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn co_delete(co: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(co, nullable);
    ensures normal resource_return_none();
};

extern fn co_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};

extern fn hthread_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};

extern fn thr_new() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nonnull);
};
";

const DESTROYER: &str = "\
extern fn hthread_destroy(mth: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(mth, nonnull);
    ensures normal resource_return_none();
    ensures normal resource_member_helper(mth, hthread.parent, thr_destroy);
    quant live #mth: hthread.parent |-> #stale;
    quant live #mth: hthread.session |-> #kept;
};
";

const HELPER: &str = "\
extern fn thr_destroy(th: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_member_invoked(th, thr.cb_destroy, _);
    ensures normal resource_released(th, nonnull);
    ensures normal resource_return_none();
    quant live #th: thr.callee |-> #stale;
    quant live #th: thr.caller |-> #kept;
    quant live #th: thr.data |-> #kept;
};
";

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("helper.qv".to_owned(), text.to_owned())]))
}

fn both() -> ResourceContracts {
    load(&format!("{BASE}\n{DESTROYER}\n{HELPER}")).expect("loads")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent RULE50-CONSUMER-COMPOSITION test: {reason}");
            false
        }
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The run's member facts of `text` alone (every store the run sees).
fn run_of(text: &str) -> Arc<RunMemberFacts> {
    let facts = read_unit(
        text,
        &C11TranslationUnit {
            source: text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"));
    Arc::new(RunMemberFacts::new(vec![facts], BTreeMap::new()))
}

/// The status and refusal texts of `function` in `PRELUDE + body`, with the
/// run's facts read from the same unit when `with_run`, and whether the row
/// consulted `thr_destroy`.
fn row(
    body: &str,
    function: &str,
    contracts: &ResourceContracts,
    with_run: bool,
) -> (AnalysisStatus, Vec<String>, bool) {
    let text = format!("{PRELUDE}\n{body}");
    let run = with_run.then(|| run_of(&text));
    let recording = Recording::start();
    let outcome = scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            function,
            contracts,
        )
    })
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let record = recording.finish();
    let consulted = record
        .functions
        .get(function)
        .is_some_and(|consulted| format!("{consulted:?}").contains("thr_destroy"));
    let summary = summary_of(&outcome).expect("a summary");
    let row = &summary.functions[function];
    let leaves = row
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    (row.status, leaves, consulted)
}

const CONSUMER: &str = "\
int hthread_destroy(struct hthread *mth);
void consumer(void)
{
    struct hthread *m = hthread_new();
    hthread_destroy(m);
}
";

#[test]
fn the_helper_clause_attaches_to_its_stale_state() {
    let contracts = both();
    let declared = contracts
        .retaining_contract("hthread_destroy")
        .expect("a retaining declaration");
    let parent = declared
        .member_paths
        .iter()
        .find(|state| state.member.path == "parent")
        .expect("the stale state");
    assert_eq!(parent.helper.as_deref(), Some("thr_destroy"));
    assert!(
        declared
            .member_paths
            .iter()
            .filter(|state| state.member.path != "parent")
            .all(|state| state.helper.is_none())
    );
}

#[test]
fn a_misplaced_helper_clause_rejects_the_module() {
    let case = |clauses: &str, binder_mode: &str| {
        format!(
            "{BASE}\nextern fn d(mth: ptr<mut, u8>) -> i32\ncontract {{\n    requires \
             resource_contract_v1;\n    ensures normal {binder_mode};\n    ensures normal \
             resource_return_none();\n{clauses}}};\n"
        )
    };
    let released = "resource_released(mth, nonnull)";
    let stale = "    quant live #mth: hthread.parent |-> #stale;\n";
    // A kept path, a set, an unstated path, alone, twice, itself, a
    // borrowed binder, an unknown formal, a malformed clause.
    for (clauses, mode) in [
        (
            "    ensures normal resource_member_helper(mth, hthread.parent, h);\n    quant live \
             #mth: hthread.parent |-> #kept;\n",
            released,
        ),
        (
            "    ensures normal resource_member_helper(mth, hthread.parent, h);\n    quant live \
             #mth: hthread.parent |-> {#stale, #kept};\n",
            released,
        ),
        (
            &format!(
                "    ensures normal resource_member_helper(mth, hthread.session, h);\n{stale}"
            ),
            released,
        ),
        (
            "    ensures normal resource_member_helper(mth, hthread.parent, h);\n",
            released,
        ),
        (
            &format!(
                "    ensures normal resource_member_helper(mth, hthread.parent, h);\n    ensures \
                 normal resource_member_helper(mth, hthread.parent, g);\n{stale}"
            ),
            released,
        ),
        (
            &format!("    ensures normal resource_member_helper(mth, hthread.parent, d);\n{stale}"),
            released,
        ),
        (
            &format!("    ensures normal resource_member_helper(mth, hthread.parent, h);\n{stale}"),
            "resource_borrowed(mth)",
        ),
        (
            &format!("    ensures normal resource_member_helper(x, hthread.parent, h);\n{stale}"),
            released,
        ),
        (
            &format!("    ensures normal resource_member_helper(mth, parent, h);\n{stale}"),
            released,
        ),
    ] {
        assert!(load(&case(clauses, mode)).is_err(), "{clauses}");
    }
    // The stand-alone form loads.
    assert!(
        load(&case(
            &format!("    ensures normal resource_member_helper(mth, hthread.parent, h);\n{stale}"),
            released
        ))
        .is_ok()
    );
    // A helper destroyer (member invocations beside its states) never names a
    // helper: a deeper frame is not composed.
    let nested = "\
extern fn n(th: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_member_invoked(th, thr.cb_destroy, _);
    ensures normal resource_released(th, nonnull);
    ensures normal resource_return_none();
    ensures normal resource_member_helper(th, thr.callee, thr_destroy);
    quant live #th: thr.callee |-> #stale;
};
";
    assert!(load(&format!("{BASE}\n{nested}")).is_err());
}

#[test]
fn a_consumer_composes_the_named_helper_over_the_runs_closed_empty_set() {
    if !clang_available() {
        return;
    }
    let (status, leaves, consulted) = row(CONSUMER, "consumer", &both(), true);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        consulted,
        "the row rests on the helper, so the fold checks its member"
    );
}

#[test]
fn a_consumer_refuses_without_the_runs_facts_or_the_helper() {
    if !clang_available() {
        return;
    }
    let (status, leaves, _) = row(CONSUMER, "consumer", &both(), false);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("member-path-helper")),
        "{leaves:?}"
    );
    // No declaration of the helper in force.
    let without = load(&format!("{BASE}\n{DESTROYER}")).expect("loads");
    let (status, leaves, _) = row(CONSUMER, "consumer", &without, true);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("member-path-helper")),
        "{leaves:?}"
    );
}

#[test]
fn a_consumer_refuses_an_open_or_non_empty_run_set() {
    if !clang_available() {
        return;
    }
    let non_empty = format!(
        "static void on_destroy(void *data) {{ (void) data; }}\n\
         void setup(struct thr *t) {{ t->cb_destroy = on_destroy; }}\n{CONSUMER}"
    );
    let (status, leaves, _) = row(&non_empty, "consumer", &both(), true);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("member-path-helper")),
        "{leaves:?}"
    );
    let open = format!(
        "void (*hook)(void *);\n\
         void setup(struct thr *t) {{ t->cb_destroy = hook; }}\n{CONSUMER}"
    );
    let (status, leaves, _) = row(&open, "consumer", &both(), true);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
}

#[test]
fn a_direct_call_of_the_helper_reads_its_invocation_then_its_states() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "{HELPER_BODY}\nvoid caller(void)\n{{\n    struct thr *t = thr_new();\n    \
         thr_destroy(t);\n}}\n"
    );
    let contracts = load(&format!("{BASE}\n{HELPER}")).expect("loads");
    let (status, leaves, _) = row(&body, "caller", &contracts, true);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        !leaves
            .iter()
            .any(|leaf| leaf.contains("member-path-direct")),
        "{leaves:?}"
    );
    // Without the run's facts the unit's own empty set is refused.
    let (status, leaves, _) = row(&body, "caller", &contracts, false);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("callback-member-open")),
        "{leaves:?}"
    );
}

/// Rule 50's evidence of `hthread_destroy` in a unit that defines it.
fn evidence(destroyer: &str, contracts: &ResourceContracts, with_run: bool) -> BTreeSet<String> {
    let text = format!(
        "{PRELUDE}\n{HELPER_BODY}\n{destroyer}\nvoid caller(struct hthread *m) {{ (void) m; }}\n"
    );
    let run = with_run.then(|| run_of(&text));
    let recording = Recording::start();
    scoped(run, || {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            "caller",
            contracts,
        )
    })
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    recording
        .finish()
        .validated
        .get("hthread_destroy")
        .cloned()
        .unwrap_or_default()
}

#[test]
fn rule_50_establishes_the_take_only_through_the_named_helper() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        evidence(DESTROYER_BODY, &both(), true),
        BTreeSet::from(["30".to_owned(), "50".to_owned()]),
        "the take is established through the named helper over the run's closed empty set"
    );
    // A declaration naming another helper: the composition of `thr_destroy`
    // is not admitted, so the take stays a declared assumption.
    let other = DESTROYER.replace(
        "resource_member_helper(mth, hthread.parent, thr_destroy)",
        "resource_member_helper(mth, hthread.parent, other_destroy)",
    );
    let contracts = load(&format!("{BASE}\n{other}\n{HELPER}")).expect("loads");
    assert!(!evidence(DESTROYER_BODY, &contracts, true).contains("50"));
    // Without the run's facts the helper's invocation is not decided.
    assert!(!evidence(DESTROYER_BODY, &both(), false).contains("50"));
}

const LINKED: &str = "\
int hthread_destroy(struct hthread *mth);
void linked(void)
{
    struct hthread *m = hthread_new();
    struct thr *t = thr_new();
    struct coro *co = co_new();
    m->parent = t;
    t->callee = co;
    hthread_destroy(m);
}
void thr_destroy(struct thr *th);
void one(void)
{
    struct thr *t = thr_new();
    struct coro *co = co_new();
    t->callee = co;
    thr_destroy(t);
}
void one_twice(void)
{
    struct thr *t = thr_new();
    struct coro *co = co_new();
    t->callee = co;
    thr_destroy(t);
    co_delete(co);
}
void outer(void)
{
    struct hthread *m = hthread_new();
    struct thr *t = thr_new();
    m->parent = t;
    hthread_destroy(m);
}
void linked_twice(void)
{
    struct hthread *m = hthread_new();
    struct thr *t = thr_new();
    struct coro *co = co_new();
    t->callee = co;
    m->parent = t;
    hthread_destroy(m);
    co_delete(co);
}
";

#[test]
fn the_helpers_states_decide_what_a_caller_links_and_a_deeper_link_is_refused() {
    if !clang_available() {
        return;
    }
    // MEMBER-PATH-HELPER-AT-A-CALL: the helper's `callee |-> #stale` takes
    // the object linked there once; releasing it again refuses.
    let (status, leaves, _) = row(LINKED, "one", &both(), true);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    let (status, leaves, _) = row(LINKED, "one_twice", &both(), true);
    assert_ne!(status, AnalysisStatus::Safe, "{leaves:?}");
    assert!(
        leaves
            .iter()
            .any(|leaf| leaf.contains("member-path-double-release")),
        "{leaves:?}"
    );
    // The destroyer's own stale `parent` takes the object linked there.
    let (status, leaves, _) = row(LINKED, "outer", &both(), true);
    assert_eq!(status, AnalysisStatus::Safe, "{leaves:?}");
    // A link below the linked object (`m->parent->callee`) is not one the
    // tracker holds: the store refuses before the call, with or without the
    // composition, so the helper's states never decide a deeper link here.
    for function in ["linked", "linked_twice"] {
        let (status, leaves, _) = row(LINKED, function, &both(), true);
        assert_ne!(status, AnalysisStatus::Safe, "{function}: {leaves:?}");
    }
}
