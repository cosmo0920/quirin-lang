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

//! W45-REFUTED-CONTRACT-WITHDRAWAL — a declared contract its own analysed
//! body refutes serves no caller
//! (`docs/reports/w45-refuted-contract-withdrawal/README.md`).
//!
//! A held-out generic family, `w_*`, unrelated to Fluent Bit and to the
//! families of earlier cards:
//!
//! * `w_malloc`/`w_free`/`w_ref`/`w_unref`: externals, declared with no body;
//! * `w_ok`: a counted-user specification its body establishes;
//! * `w_bad`: the same specification, which its body refutes (the N2 shape: a
//!   rolled-back addition reported as success);
//! * `w_keep`: declared `resource_borrowed`, while its body releases;
//! * `w_peek`: declared `resource_borrowed`, which its body satisfies;
//! * `w_rel`: declared `resource_released(p, nullable)` over a body W4 reads
//!   as `nonnull` (a refinement, never a refutation).
//!
//! Each caller is analysed twice where the question is cross-unit: once in a
//! unit without the callee's body and no table (the gap: the declaration is
//! trusted), and once with the defining unit's refutation table recorded.
//!
//! `Safe` here is `Safe` for a synthetic declared projection under the
//! premises the report names, never a statement about Fluent Bit.

#[path = "support/host_abi.rs"]
mod host_abi;

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit, refuted_contract,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract_refutation::{
    ContractRefutation, RefutationKind, RefutationTable, insert_refutation, table_digest,
};
use qcore_schemas::contract_revision::PinMismatchReason;

const CONTRACTS: &str = r#"module contract;

extern fn w_malloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn w_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn w_free_nn(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nonnull);
    ensures normal resource_return_none();
};

extern fn w_ref(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_added(t, refs);
    ensures normal resource_return_none();
};

extern fn w_unref(t: ptr<mut, u8>, check: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(check, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(check);
    ensures normal resource_user_dropped(t, refs);
    ensures normal resource_return_none();
};

extern fn w_ok(t: ptr<mut, u8>, fail: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(fail);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn w_bad(t: ptr<mut, u8>, fail: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(fail);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn w_keep(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};

extern fn w_peek(p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};

extern fn w_rel(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
"#;

const PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    r#"

struct w_task { int refs; };
struct w_obj { int value; };
void *w_malloc(size_t size);
void w_free(void *p);
void w_free_nn(void *p);
void w_ref(struct w_task *t);
void w_unref(struct w_task *t, int check);
int w_ok(struct w_task *t, int fail);
int w_bad(struct w_task *t, int fail);
void w_keep(struct w_obj *p);
int w_peek(struct w_obj *p);
void w_rel(struct w_obj *p);
"#
);

/// The defining unit: every declared body.
const BODIES: &str = r#"
int w_ok(struct w_task *t, int fail)
{
    if (fail) {
        return -1;
    }
    w_ref(t);
    return 0;
}

int w_bad(struct w_task *t, int fail)
{
    w_ref(t);
    if (fail) {
        w_unref(t, 0);
    }
    return 0;
}

void w_keep(struct w_obj *p)
{
    w_free(p);
}

int w_peek(struct w_obj *p)
{
    return 0;
}

void w_rel(struct w_obj *p)
{
    w_free_nn(p);
}
"#;

/// Callers, analysed with or without the bodies above.
const CALLERS: &str = r#"
int c_ok(struct w_task *t)
{
    int r;

    r = w_ok(t, 0);
    if (r == 0) {
        w_unref(t, 0);
    }
    return r;
}

int c_bad(struct w_task *t)
{
    int r;

    r = w_bad(t, 0);
    if (r == 0) {
        w_unref(t, 0);
    }
    return r;
}

void c_keep(void)
{
    struct w_obj *o;

    o = w_malloc(sizeof(struct w_obj));
    if (!o) {
        return;
    }
    w_keep(o);
    w_free(o);
}

void c_peek(void)
{
    struct w_obj *o;

    o = w_malloc(sizeof(struct w_obj));
    if (!o) {
        return;
    }
    w_peek(o);
    w_free(o);
}

void c_extern(void)
{
    struct w_obj *o;

    o = w_malloc(sizeof(struct w_obj));
    if (!o) {
        return;
    }
    w_free(o);
}

void c_rel(void)
{
    struct w_obj *o;

    o = w_malloc(sizeof(struct w_obj));
    if (!o) {
        return;
    }
    w_rel(o);
}
"#;

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "refuted_contract.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn analyse(source: &str, entry: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}\n{source}"), entry, contracts)
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn descriptions(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description }
            | LegacyDiagnosticKind::CountedUser { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn held(summary: &LegacyFunctionSummary, callee: &str) -> bool {
    descriptions(summary)
        .iter()
        .any(|description| refuted_contract::held_call(description) == Some(callee))
}

/// The table the defining unit's rows yield: phase one.
fn defining_table() -> RefutationTable {
    let summary = analyse(BODIES, "w_ok", &contracts());
    let mut table = RefutationTable::new();
    for function in summary.functions.values() {
        for refutation in &function.refutations {
            insert_refutation(
                &mut table,
                &function.function,
                ContractRefutation {
                    unit: "bodies.c".into(),
                    kind: refutation.kind,
                    clause: refutation.clause.clone(),
                    origin: None,
                    reason: refutation.reason.clone(),
                },
            );
        }
    }
    table
}

fn with_table(table: &RefutationTable) -> ResourceContracts {
    let mut contracts = contracts();
    contracts.set_refuted_by_body(table);
    contracts
}

// ---- phase one: what the defining unit refutes ----

#[test]
fn the_defining_unit_refutes_exactly_the_contradicted_declarations() {
    let table = defining_table();
    assert_eq!(
        table.keys().cloned().collect::<Vec<_>>(),
        vec!["w_bad".to_owned(), "w_keep".to_owned()],
        "{table:#?}"
    );
    assert!(table["w_bad"].iter().any(|refutation| {
        refutation.kind == RefutationKind::CountedUserClause
            && refutation.clause == "resource_user_exit(t, 0, added)"
    }));
    assert!(table["w_keep"].iter().any(|refutation| {
        refutation.kind == RefutationKind::CustodyDuty
            && refutation.clause == "mode-parameter-contradiction"
    }));
}

#[test]
fn a_refuted_body_keeps_its_own_refutation() {
    let summary = analyse(BODIES, "w_bad", &with_table(&defining_table()));
    let bad = function(&summary, "w_bad");
    // The declaration stays in force for its own body: the row still carries
    // the refutation that convicts it, a provisional `Unsafe` candidate.
    assert_eq!(
        bad.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        descriptions(bad)
    );
    assert!(!bad.refutations.is_empty());
}

#[test]
fn a_refinement_conflict_is_not_a_refutation() {
    let summary = analyse(BODIES, "w_rel", &contracts());
    let rel = function(&summary, "w_rel");
    assert!(
        rel.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            LegacyDiagnosticKind::SummaryConflict { .. }
        )),
        "{:?}",
        descriptions(rel)
    );
    assert!(rel.refutations.is_empty(), "{:?}", rel.refutations);
}

// ---- accepting ----

#[test]
fn an_established_contract_still_serves_its_callers() {
    let table = defining_table();
    // Cross unit, under the table.
    let summary = analyse(CALLERS, "c_ok", &with_table(&table));
    // A caller of an exit-clause declaration is never fully decided (a value
    // no clause covers leaves its count undecided), but the declaration is
    // applied: the call is not held.
    let ok = function(&summary, "c_ok");
    assert!(!held(ok, "w_ok"), "{:?}", descriptions(ok));
    assert!(
        descriptions(ok).iter().any(|description| description
            .contains("`w_ok` returned a value its exit clauses do not cover")),
        "{:?}",
        descriptions(ok)
    );
    let peek = function(&summary, "c_peek");
    assert_eq!(
        peek.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptions(peek)
    );
    // In the defining unit.
    let summary = analyse(&format!("{BODIES}\n{CALLERS}"), "c_peek", &contracts());
    let peek = function(&summary, "c_peek");
    assert_eq!(
        peek.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptions(peek)
    );
    assert!(!held(peek, "w_peek"));
}

// ---- refusing ----

#[test]
fn a_cross_unit_caller_relied_on_the_refuted_declarations_before_the_table() {
    // The gap: without the table a unit that does not define the bodies
    // trusts both declarations and certifies both callers.
    let summary = analyse(CALLERS, "c_bad", &contracts());
    let keep = function(&summary, "c_keep");
    assert_eq!(
        keep.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptions(keep)
    );
    // The counted half of `w_bad` is applied too (its exit clauses fork the
    // caller's count), although its body refutes one of them.
    let bad = function(&summary, "c_bad");
    assert!(!held(bad, "w_bad"));
    assert!(
        descriptions(bad).iter().any(|description| description
            .contains("`w_bad` returned a value its exit clauses do not cover")),
        "{:?}",
        descriptions(bad)
    );
}

#[test]
fn a_cross_unit_caller_is_held_through_the_table() {
    let summary = analyse(CALLERS, "c_bad", &with_table(&defining_table()));
    for (caller, callee) in [("c_bad", "w_bad"), ("c_keep", "w_keep")] {
        let row = function(&summary, caller);
        assert_eq!(
            row.status,
            AnalysisStatus::Unknown,
            "{:?}",
            descriptions(row)
        );
        assert!(held(row, callee), "{caller}: {:?}", descriptions(row));
    }
}

#[test]
fn a_caller_in_the_defining_unit_is_held_without_any_table() {
    let summary = analyse(&format!("{BODIES}\n{CALLERS}"), "c_bad", &contracts());
    let bad = function(&summary, "c_bad");
    assert_eq!(
        bad.status,
        AnalysisStatus::Unknown,
        "{:?}",
        descriptions(bad)
    );
    assert!(held(bad, "w_bad"), "{:?}", descriptions(bad));
    // A duty contradiction keeps constraint 8's own key in its unit.
    let keep = function(&summary, "c_keep");
    assert_eq!(
        keep.status,
        AnalysisStatus::Unknown,
        "{:?}",
        descriptions(keep)
    );
    assert!(!held(keep, "w_keep"));
    assert!(
        descriptions(keep)
            .iter()
            .any(|description| description.contains("mode-parameter-contradiction")),
        "{:?}",
        descriptions(keep)
    );
}

#[test]
fn a_held_call_is_unknown_never_pending_or_unsafe() {
    let summary = analyse(CALLERS, "c_bad", &with_table(&defining_table()));
    let bad = function(&summary, "c_bad");
    assert_eq!(bad.status.verdict().as_str(), "Unknown");
    assert!(!qcore_c_gateway::legacy_c::verdict::is_held_call(
        &refuted_contract::hold_description("w_bad")
    ));
}

// ---- externals ----

#[test]
fn an_external_contract_with_no_body_is_unaffected() {
    let table = defining_table();
    let before = analyse(CALLERS, "c_extern", &contracts());
    let after = analyse(CALLERS, "c_extern", &with_table(&table));
    let (before, after) = (function(&before, "c_extern"), function(&after, "c_extern"));
    assert_eq!(
        after.status,
        AnalysisStatus::Safe,
        "{:?}",
        descriptions(after)
    );
    assert_eq!(before.diagnostics, after.diagnostics);
    // A table entry for a name with no declaration in force is dropped.
    let mut foreign = RefutationTable::new();
    insert_refutation(
        &mut foreign,
        "w_undeclared",
        ContractRefutation {
            unit: "other.c".into(),
            kind: RefutationKind::CountedUserClause,
            clause: "c".into(),
            origin: None,
            reason: "r".into(),
        },
    );
    assert!(with_table(&foreign).refuted_by_body().is_empty());
}

// ---- pinned together with refuted ----

#[test]
fn a_pin_mismatch_is_pending_a_refutation_unknown_and_their_join_unknown() {
    let table = defining_table();
    let source = format!(
        "{CALLERS}\nint c_pinned(struct w_obj *o)\n{{\n    return w_peek(o);\n}}\n\nint c_both(struct w_task *t, struct w_obj *o)\n{{\n    int r;\n\n    r = w_peek(o);\n    r = w_bad(t, 0);\n    if (r == 0) {{\n        w_unref(t, 0);\n    }}\n    return r;\n}}\n"
    );
    let mut contracts = with_table(&table);
    contracts
        .withhold_for_pin_mismatch(
            "w_peek",
            "refuted_contract.qcore".into(),
            PinMismatchReason::SourceChanged,
            None,
            String::new(),
        )
        .unwrap();
    let summary = analyse(&source, "c_pinned", &contracts);
    let pinned = function(&summary, "c_pinned");
    assert_eq!(
        pinned.status,
        AnalysisStatus::Pending,
        "{:?}",
        descriptions(pinned)
    );
    let both = function(&summary, "c_both");
    assert_eq!(
        both.status,
        AnalysisStatus::Unknown,
        "{:?}",
        descriptions(both)
    );
    assert!(held(both, "w_bad"));
}

// ---- the table as an input ----

#[test]
fn the_table_travels_in_the_environment_and_is_digested() {
    let table = defining_table();
    let contracts = with_table(&table);
    let serialized = serde_json::to_string(&contracts).unwrap();
    assert!(serialized.contains("refuted_by_body"));
    // Without a refutation the environment serializes exactly as before.
    assert!(
        !serde_json::to_string(&self::contracts())
            .unwrap()
            .contains("refuted_by_body")
    );
    assert_ne!(table_digest(&table), table_digest(&RefutationTable::new()));
}
