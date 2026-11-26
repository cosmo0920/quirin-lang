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

//! Card CALLBACK-TABLE-MEMBER-PATH-STATES (gateway rules 13v and 30
//! amended; contract specification C04.19.6): a table dispatch
//! (`resource_callback_table`) composes its candidates' member-path states
//! at the paths the caller linked below a parameter actual exactly as a
//! synchronous invocation (`resource_callback_invoked`) does: joined over
//! the candidates, so candidates that disagree refuse. Every row below is
//! read in both forms over the same body, the table form dispatching
//! `c_op_table` over an automatic table and the synchronous form calling
//! `c_op_call`; both borrow the op. The fixture callees are trusted test
//! assumptions; a `Safe` here promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// Beside rule 30's `member_paths.qv`: a borrowing op constructor, the
/// table dispatcher, a synchronous dispatcher, the releases, and a
/// contracted identity candidate that states no member path.
const TABLE: &str = "module contract;

extern fn c_op_new(type: i32) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(type);
    ensures normal resource_return_owned(nonnull);
};

extern fn c_op_table(table: ptr<mut, u8>, rko: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(table);
    ensures normal resource_borrowed(rko);
    ensures normal resource_callback_table(table, _, _, rko);
    ensures normal resource_return_none();
};

extern fn c_op_call(rko: ptr<mut, u8>, cb: fnptr) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rko);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, _, _, rko);
    ensures normal resource_return_none();
};

extern fn c_op_drop(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_return_none();
};

extern fn c_item_free(it: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(it, nonnull);
    ensures normal resource_return_none();
};

extern fn c_cb_plain(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "\
#include <stddef.h>
struct item { int x; };
struct op;
typedef int (*op_cb_t)(void *rk, void *rkq, struct op *rko);
struct op { int type; op_cb_t op_cb; struct item *payload; struct item *aux; };
struct entry { const char *name; op_cb_t handler; };
struct item *c_item_new(int n);
struct op *c_op_new(int type);
int c_op_table(struct entry *table, struct op *rko);
int c_op_call(struct op *rko, op_cb_t cb);
void c_op_drop(struct op *rko);
void c_item_free(struct item *it);
int c_cb_releases(void *rk, void *rkq, struct op *rko);
int c_cb_moves(void *rk, void *rkq, struct op *rko);
int c_cb_stale(void *rk, void *rkq, struct op *rko);
int c_cb_maybe(void *rk, void *rkq, struct op *rko);
int c_cb_other(void *rk, void *rkq, struct op *rko);
int c_cb_plain(void *rk, void *rkq, struct op *rko);
";

/// Card MEMBER-PATH-INVOCATION-PREMISE: the invokers' bodies, each calling
/// its candidate on every path, so their candidates' states read the
/// invocation as run (rule 30 amended); `member_path_invocation_premise.rs`
/// pins the bodies that may not call.
const INVOKER_BODIES: &str = "\
int c_op_table(struct entry *table, struct op *rko) { return table[0].handler(NULL, NULL, rko); }
int c_op_call(struct op *rko, op_cb_t cb) { return cb(NULL, NULL, rko); }
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "member_paths.qv".to_owned(),
            include_str!("fixtures/contracts/member_paths.qv").to_owned(),
        ),
        ("table_states.qv".to_owned(), TABLE.to_owned()),
    ]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent table member-path test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe && self.unsupported.is_empty()
    }
    fn refused_with(&self, key: &str) -> bool {
        self.status != AnalysisStatus::Safe && self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The row of `f` in the unit `PRELUDE + source`.
fn row(source: &str) -> Row {
    let text = format!("{PRELUDE}\n{INVOKER_BODIES}\n{source}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        "f",
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    let row = &summary.functions["f"];
    Row {
        status: row.status,
        unsupported: row
            .diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect(),
    }
}

/// The table form: a fresh item linked at the op's payload, a table of
/// `candidates` dispatched over the op, then `after`.
fn table(candidates: &[&str], after: &str) -> String {
    let entries = candidates
        .iter()
        .map(|cb| format!("{{\"{cb}\", {cb}}}, "))
        .collect::<String>();
    format!(
        "void f(int n) {{ struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) {{ return; }} o = c_op_new(2); o->payload = it; \
         struct entry table[] = {{ {entries}{{NULL, NULL}} }}; \
         c_op_table(table, o); {after} }}"
    )
}

/// The synchronous form of the same body over the one candidate `cb`.
fn synchronous(cb: &str, after: &str) -> String {
    format!(
        "void f(int n) {{ struct item *it; struct op *o; it = c_item_new(n); \
         if (!it) {{ return; }} o = c_op_new(2); o->payload = it; \
         c_op_call(o, {cb}); {after} }}"
    )
}

/// The card's probe (CALLBACK-MEMBER-RELEASED-EFFECT-ORDER's
/// `evidence/probe-table.rs.txt`, over a borrowing dispatcher): the
/// candidate releases the payload, the caller frees it again. A double
/// release, never `Safe`; the table form reads as the synchronous one.
#[test]
fn a_table_candidate_releasing_the_linked_payload_then_a_free_is_never_safe() {
    if !clang_available() {
        return;
    }
    let after = "o->payload = NULL; c_item_free(it); c_op_drop(o);";
    let sync = row(&synchronous("c_cb_releases", after));
    assert_eq!(sync.status, AnalysisStatus::Unsafe, "{sync:?}");
    let dispatched = row(&table(&["c_cb_releases"], after));
    assert!(!dispatched.safe(), "{dispatched:?}");
    assert_eq!(dispatched.status, sync.status, "{dispatched:?}");
}

/// The candidate's `#released` payload is taken by the dispatch: no free
/// follows, nothing leaks. As the synchronous form.
#[test]
fn a_table_candidate_releasing_the_linked_payload_takes_it() {
    if !clang_available() {
        return;
    }
    let after = "c_op_drop(o);";
    let sync = row(&synchronous("c_cb_releases", after));
    assert!(sync.safe(), "{sync:?}");
    let dispatched = row(&table(&["c_cb_releases"], after));
    assert!(dispatched.safe(), "{dispatched:?}");
}

/// Candidates that agree (`#released`, `#moved`: the payload leaves the
/// caller's custody once, whichever entries run, since each clears the
/// member) compose; candidates that disagree (`#released` beside a kept
/// payload) refuse `member-path-undecided`, in either order of entries.
#[test]
fn table_candidates_join_and_disagreeing_ones_refuse() {
    if !clang_available() {
        return;
    }
    let agreed = row(&table(&["c_cb_releases", "c_cb_moves"], "c_op_drop(o);"));
    assert!(agreed.safe(), "{agreed:?}");
    let doubled = row(&table(
        &["c_cb_releases", "c_cb_moves"],
        "o->payload = NULL; c_item_free(it); c_op_drop(o);",
    ));
    assert!(!doubled.safe(), "{doubled:?}");
    for candidates in [
        ["c_cb_releases", "c_cb_other"],
        ["c_cb_other", "c_cb_releases"],
        ["c_cb_plain", "c_cb_releases"],
    ] {
        for after in [
            "c_op_drop(o);",
            "o->payload = NULL; c_item_free(it); c_op_drop(o);",
        ] {
            let disagreed = row(&table(&candidates, after));
            assert!(
                disagreed.refused_with("member-path-undecided"),
                "{candidates:?} {after}: {disagreed:?}"
            );
        }
    }
}

/// The other refusals of rule 30 read at a table dispatch exactly as at a
/// synchronous one: a stale payload (`member-path-stale`), a may-set at a
/// path no arm releases (`member-path-undecided`), a path below a stated one
/// (`member-path-unstated`, never through a `Safe` row).
#[test]
fn rule_30_refusals_read_at_a_table_dispatch() {
    if !clang_available() {
        return;
    }
    let after = "o->payload = NULL; c_item_free(it); c_op_drop(o);";
    for (cb, key) in [
        ("c_cb_stale", "member-path-stale"),
        ("c_cb_maybe", "member-path-undecided"),
    ] {
        let sync = row(&synchronous(cb, after));
        assert!(sync.refused_with(key), "{cb}: {sync:?}");
        let dispatched = row(&table(&[cb], after));
        assert!(dispatched.refused_with(key), "{cb}: {dispatched:?}");
    }
}

/// Controls: a table candidate stating nothing (a contracted identity) or
/// stating only another path (`c_cb_other`: `op.aux`) leaves the payload
/// linked and live, as before this card: the caller's own free is the one
/// release, and dropping the op without it loses the item.
#[test]
fn a_table_candidate_stating_nothing_at_the_path_keeps_the_payload() {
    if !clang_available() {
        return;
    }
    for candidates in [
        &["c_cb_plain"][..],
        &["c_cb_other"],
        &["c_cb_plain", "c_cb_other"],
    ] {
        let freed = row(&table(
            candidates,
            "o->payload = NULL; c_item_free(it); c_op_drop(o);",
        ));
        assert!(freed.safe(), "{candidates:?}: {freed:?}");
        let lost = row(&table(candidates, "c_op_drop(o);"));
        assert_eq!(
            lost.status,
            AnalysisStatus::Unsafe,
            "{candidates:?}: {lost:?}"
        );
    }
    for cb in ["c_cb_plain", "c_cb_other"] {
        let sync = row(&synchronous(
            cb,
            "o->payload = NULL; c_item_free(it); c_op_drop(o);",
        ));
        assert!(sync.safe(), "{cb}: {sync:?}");
    }
}

const CMT_MPACK: &str =
    include_str!("../../../providers/qcore-provider-cmetrics/contracts/cmetrics/mpack_utils.qv");
const CTR_CONTEXT: &str =
    include_str!("../../../providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv");

/// The shipped table clauses: the cmetrics and ctraces map unpackers, each
/// the one table declaration of its module, and neither module states a
/// member path, so no candidate those declarations govern carries a state
/// this card now composes (their handlers are read from in-unit summaries,
/// which state none). That these two modules are the only shipped `.qv`
/// naming `resource_callback_table`, and that neither provider states a
/// member path anywhere, is the report's census
/// (`evidence/shipped-table-census.txt`); their readings are measured by its
/// opt-in `current` run.
#[test]
fn the_shipped_table_clauses_are_the_map_unpackers() {
    use qcore_schemas::callback::CallbackClause;
    for (module, text, expected) in [
        ("cmetrics/mpack_utils.qv", CMT_MPACK, "cmt_mpack_unpack_map"),
        (
            "ctraces/context_lifetime.qv",
            CTR_CONTEXT,
            "ctr_mpack_unpack_map",
        ),
    ] {
        assert!(!text.contains("|->"), "{module} states a member path");
        let loaded =
            load_resource_contract_sources(&BTreeMap::from([(module.to_owned(), text.to_owned())]))
                .unwrap_or_else(|error| panic!("{module}: {error}"));
        let tables = loaded
            .retaining()
            .iter()
            .filter(|(_, declaration)| {
                declaration
                    .callbacks
                    .iter()
                    .any(|clause| matches!(clause, CallbackClause::Table { .. }))
            })
            .map(|(name, declaration)| {
                assert!(declaration.member_paths.is_empty(), "{name}");
                name.clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(tables, [expected], "{module}");
    }
}
