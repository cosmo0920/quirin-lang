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

//! Card SCHEDULER-DESTROY-MEMBER-STATES (gateway rule 50; contract spec
//! C04.19.6 amended): the loader's placement of a helper destroyer's member
//! invocations beside its member-path states, and rule 30's body reading
//! composing such a helper where a destroyer hands it the value of a stated
//! `#stale` member, over the synthetic Monkey-shaped units below (trusted
//! test assumptions: nothing here is a fact about Monkey).
//!
//! The run's member facts are not installed on this route (no pre-pass), so
//! a helper that states a member invocation is never composed here: its
//! take stays a declared assumption, fail-closed.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::contract_dependencies::Recording;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
void free(void *p);
void co_delete(void *co);
struct mk_list { struct mk_list *prev, *next; };
struct thr { void *caller; void *callee; void *data; void (*cb_destroy)(void *); };
struct hthread { int close; void *session; void *request; struct thr *parent; struct mk_list _head; };
static inline void list_del(struct mk_list *entry)
{
    entry->prev->next = entry->next;
    entry->next->prev = entry->prev;
    entry->prev = NULL;
    entry->next = NULL;
}
";

const HELPER: &str = "\
static inline void thr_destroy(struct thr *th)
{
    co_delete(th->callee);
    free(th);
}
";

const HELPER_INVOKING: &str = "\
static inline void thr_destroy(struct thr *th)
{
    if (th->cb_destroy) {
        th->cb_destroy((char *) th + sizeof(struct thr));
    }
    co_delete(th->callee);
    free(th);
}
";

const CONTRACTS: &str = "\
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
    ensures normal resource_borrowed(co);
    ensures normal resource_return_none();
};

extern fn hthread_destroy(mth: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_unpublished(mth, _head);
    ensures normal resource_released(mth, nonnull);
    ensures normal resource_return_none();
    quant live #mth: hthread.parent |-> #stale;
    quant live #mth: hthread.session |-> #kept;
    quant live #mth: hthread.request |-> #kept;
};
";

const THR_DESTROY: &str = "\
extern fn thr_destroy(th: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(th, nonnull);
    ensures normal resource_return_none();
    quant live #th: thr.callee |-> #stale;
    quant live #th: thr.caller |-> #kept;
    quant live #th: thr.data |-> #kept;
};
";

const THR_DESTROY_INVOKING: &str = "\
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

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent SCHEDULER-DESTROY-MEMBER-STATES test: {reason}");
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

/// The body evidence recorded for `hthread_destroy` and the row of `caller`
/// in the unit `PRELUDE + helper + destroyer + caller`.
fn run(
    helper: &str,
    destroyer: &str,
    contracts: &ResourceContracts,
) -> (BTreeSet<String>, AnalysisStatus) {
    let text = format!(
        "{PRELUDE}\n{helper}\n{destroyer}\n\
         void caller(struct hthread *m) {{ (void) m; }}\n"
    );
    let recording = Recording::start();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "caller",
        contracts,
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    let record = recording.finish();
    let status = summary_of(&outcome)
        .map(|summary| summary.functions["caller"].status)
        .unwrap_or(AnalysisStatus::Unknown);
    (
        record
            .validated
            .get("hthread_destroy")
            .cloned()
            .unwrap_or_default(),
        status,
    )
}

const DESTROYER: &str = "\
int hthread_destroy(struct hthread *mth)
{
    struct thr *th;
    list_del(&mth->_head);
    th = mth->parent;
    thr_destroy(th);
    return 0;
}
";

#[test]
fn the_helpers_member_invocation_stands_beside_its_states_on_its_released_formal() {
    let contracts = load(&format!("{CONTRACTS}\n{THR_DESTROY_INVOKING}")).expect("loads");
    let declared = contracts
        .retaining_contract("thr_destroy")
        .expect("a retaining declaration");
    assert_eq!(declared.member_paths.len(), 3);
    assert_eq!(declared.callbacks.len(), 1);
}

#[test]
fn a_member_invocation_beside_states_elsewhere_rejects_the_module() {
    // The invocation on another formal than the states' binder.
    let other = "\
extern fn two(th: ptr<mut, u8>, o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_member_invoked(o, thr.cb_destroy, _);
    ensures normal resource_released(th, nonnull);
    ensures normal resource_released(o, nonnull);
    ensures normal resource_return_none();
    quant live #th: thr.callee |-> #stale;
};
";
    assert!(load(&format!("module contract;\n{other}")).is_err());
    // A borrowed binder: not a helper destroyer.
    let borrowed = "\
extern fn keep(th: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_member_invoked(th, thr.cb_destroy, _);
    ensures normal resource_borrowed(th);
    ensures normal resource_return_none();
    quant live #th: thr.callee |-> #stale;
};
";
    assert!(load(&format!("module contract;\n{borrowed}")).is_err());
    // A synchronous invocation of a parameter, not a member invocation.
    let synchronous = "\
extern fn sync(th: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_callback_invoked(cb, th);
    ensures normal resource_released(th, nonnull);
    ensures normal resource_borrowed(cb);
    ensures normal resource_return_none();
    quant live #th: thr.callee |-> #stale;
};
";
    assert!(load(&format!("module contract;\n{synchronous}")).is_err());
}

#[test]
fn a_destroyer_handing_a_stale_members_copy_to_a_declared_helper_takes_its_object() {
    if !clang_available() {
        return;
    }
    let contracts = load(&format!("{CONTRACTS}\n{THR_DESTROY}")).expect("loads");
    let (evidence, status) = run(HELPER, DESTROYER, &contracts);
    assert_eq!(
        evidence,
        BTreeSet::from(["30".to_owned(), "50".to_owned()]),
        "the states validate (the link member's stores inside `list_del` are the unlink's) and \
         the stale `parent`'s object is taken by the composed `thr_destroy`"
    );
    assert_eq!(status, AnalysisStatus::Safe);
}

#[test]
fn a_helper_whose_member_invocation_is_not_decided_over_the_run_is_not_composed() {
    if !clang_available() {
        return;
    }
    let contracts = load(&format!("{CONTRACTS}\n{THR_DESTROY_INVOKING}")).expect("loads");
    let (evidence, _) = run(HELPER_INVOKING, DESTROYER, &contracts);
    assert_eq!(evidence, BTreeSet::from(["30".to_owned()]));
}

#[test]
fn a_take_on_one_branch_twice_or_without_a_helper_declaration_is_not_established() {
    if !clang_available() {
        return;
    }
    let with_helper = load(&format!("{CONTRACTS}\n{THR_DESTROY}")).expect("loads");
    let twice = "\
int hthread_destroy(struct hthread *mth)
{
    struct thr *th;
    list_del(&mth->_head);
    th = mth->parent;
    thr_destroy(th);
    thr_destroy(th);
    return 0;
}
";
    assert_eq!(
        run(HELPER, twice, &with_helper).0,
        BTreeSet::from(["30".to_owned()])
    );
    let branch = "\
int flag;
int hthread_destroy(struct hthread *mth)
{
    struct thr *th;
    list_del(&mth->_head);
    th = mth->parent;
    if (flag)
        thr_destroy(th);
    return 0;
}
";
    assert_eq!(
        run(HELPER, branch, &with_helper).0,
        BTreeSet::from(["30".to_owned()])
    );
    let escaped = "\
struct thr *saved;
int hthread_destroy(struct hthread *mth)
{
    struct thr *th;
    list_del(&mth->_head);
    th = mth->parent;
    saved = th;
    thr_destroy(th);
    return 0;
}
";
    assert_eq!(
        run(HELPER, escaped, &with_helper).0,
        BTreeSet::from(["30".to_owned()])
    );
    // No declaration for the helper: its body is not composed.
    let without = load(CONTRACTS).expect("loads");
    assert_eq!(
        run(HELPER, DESTROYER, &without).0,
        BTreeSet::from(["30".to_owned()])
    );
}

#[test]
fn a_direct_call_of_a_helper_with_an_invocation_beside_its_states_refuses() {
    if !clang_available() {
        return;
    }
    let contracts = load(&format!("{CONTRACTS}\n{THR_DESTROY_INVOKING}")).expect("loads");
    let text = format!(
        "{PRELUDE}\n{HELPER_INVOKING}\n\
         void *alloc_thr(void);\n\
         void caller(struct thr *t) {{ thr_destroy(t); }}\n"
    );
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "caller",
        &contracts,
    )
    .expect("the unit analyses");
    let row = &summary_of(&outcome).expect("a summary").functions["caller"];
    assert_ne!(row.status, AnalysisStatus::Safe, "{row:?}");
    assert!(
        format!("{:?}", row.diagnostics).contains("member-path-direct"),
        "{:?}",
        row.diagnostics
    );
}
