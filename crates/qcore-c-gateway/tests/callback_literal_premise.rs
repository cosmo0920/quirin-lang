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

//! Card OPENSSL-PEM-CALLBACK-ARM (gateway rule 57): a literal-argument
//! premise beside a synchronous invocation, on route G, over the synthetic
//! `p_*` callees below (trusted test assumptions: a `Safe` here is `Safe`
//! over them and promotes nothing). `p_read` is the PEM readers' shape: the
//! declaration applies only where `x` is a null pointer constant, and there
//! its password callback is decided as rule 13u decides every invocation.
//!
//! Card CALLBACK-CANDIDATE-LITERAL-PREMISE (rule 57 amended, this target so
//! the read census needs no new unit): the same premise at a callback
//! *candidate*, over the synthetic `q_*` callees. A premise-carrying
//! candidate has no contract in force on each candidate path (an invocation
//! of a function-map candidate, a composed candidate, a registration and a
//! constructed member registration), and the premise-free twin of each still
//! decides.

use std::collections::BTreeMap;
use std::sync::Arc;

use qcore_c_gateway::callback_member_closure::read_unit;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_c_gateway::member_run_facts::{DeclaredStore, RunMemberFacts, declarations_unit, scoped};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::callback::CallbackClause;

const CONTRACTS: &str = "module contract;

extern fn p_alloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};

extern fn p_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn p_read(out: ptr<mut, u8>, x: ptr<mut, ptr<mut, u8>>, cb: fnptr, u: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(x, 0);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(x);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(u);
    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);
    ensures normal resource_return_owned(nullable);
};

extern fn p_read_plain(out: ptr<mut, u8>, x: ptr<mut, ptr<mut, u8>>, cb: fnptr, u: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(x, 0);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(x);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(u);
    ensures normal resource_callback_invoked(cb, _, _, _, u);
    ensures normal resource_return_owned(nullable);
};
";

const PRELUDE: &str = "\
#include <stddef.h>
typedef int pass_cb(char *buf, int size, int rwflag, void *u);
void *p_alloc(__SIZE_TYPE__ n); void p_free(void *p);
void *p_read(void *out, void **x, pass_cb *cb, void *u);
void *p_read_plain(void *out, void **x, pass_cb *cb, void *u);
void unknown_effect(void *p);
pass_cb *pick(void);
";

const UNIT: &str = "\
static int quiet(char *buf, int size, int rwflag, void *u) { return size; }
static int reads(char *buf, int size, int rwflag, void *u) { return *(int *)u + size; }
static int frees(char *buf, int size, int rwflag, void *u) { p_free(u); return 0; }
static int opaque(char *buf, int size, int rwflag, void *u) { unknown_effect(u); return 0; }
void in_premise(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); p_free(r); }
void in_premise_zero(void *bio, void *u) { void *r = p_read(bio, 0, quiet, u); p_free(r); }
void in_premise_leak(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); }
void u_is_not_kept(void *bio) { void *u = p_alloc(8); void *r = p_read(bio, NULL, quiet, u); p_free(r); p_free(u); }
void u_is_not_released(void *bio) { void *u = p_alloc(8); void *r = p_read(bio, NULL, quiet, u); p_free(r); }
void reuse_arm(void *bio, void *u) { void *slot = NULL; void *r = p_read(bio, &slot, quiet, u); p_free(r); }
void reuse_formal(void *bio, void **x, void *u) { void *r = p_read(bio, x, quiet, u); p_free(r); }
void effectful(void *bio, void *u) { void *r = p_read(bio, NULL, frees, u); p_free(r); }
void uncontracted(void *bio, void *u) { void *r = p_read(bio, NULL, opaque, u); p_free(r); }
void null_callback(void *bio, void *u) { void *r = p_read(bio, NULL, NULL, u); p_free(r); }
void null_callback_zero(void *bio, void *u) { void *r = p_read(bio, 0, 0, u); p_free(r); }
void null_callback_cast(void *bio, void *u) { void *r = p_read(bio, NULL, (pass_cb *)0, u); p_free(r); }
void null_callback_leak(void *bio, void *u) { void *r = p_read(bio, NULL, NULL, u); }
void null_callback_reuse(void *bio, void *u) { void *slot = NULL; void *r = p_read(bio, &slot, NULL, u); p_free(r); }
void null_callback_plain(void *bio, void *u) { void *r = p_read_plain(bio, NULL, NULL, u); p_free(r); }
void plain_identity(void *bio, void *u) { void *r = p_read_plain(bio, NULL, quiet, u); p_free(r); }
void result_callback(void *bio, void *u) { void *r = p_read(bio, NULL, pick(), u); p_free(r); }
void local_callback(void *bio, void *u, int which) { pass_cb *cb = which ? quiet : NULL; void *r = p_read(bio, NULL, cb, u); p_free(r); }
void formal_callback(void *bio, pass_cb *cb, void *u) { void *r = p_read(bio, NULL, cb, u); p_free(r); }
void requires_u(void *bio, void *u) { void *r = p_read(bio, NULL, reads, u); p_free(r); }
void requires_u_direct(void *bio, void *u) { int n = reads(NULL, 0, 0, u); (void)n; }
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "premise_callbacks.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
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
            eprintln!("skipping Clang-dependent OPENSSL-PEM-CALLBACK-ARM test: {reason}");
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
        self.status == AnalysisStatus::Safe
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

fn rows() -> BTreeMap<String, Row> {
    rows_of(UNIT, "requires_u_direct")
}

fn rows_of(unit: &str, entry: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{unit}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

const OUTSIDE: &str = "outside its declared literal-argument premise";

#[test]
fn within_the_premise_an_identity_callback_applies_the_declaration() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["in_premise", "in_premise_zero", "u_is_not_kept"] {
        assert!(rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    // The declaration really applies: its fresh result is owed.
    assert_eq!(
        rows["in_premise_leak"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["in_premise_leak"]
    );
    // `u` is read during the call, not kept and not released: the caller
    // still owes the object it handed.
    assert_eq!(
        rows["u_is_not_released"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["u_is_not_released"]
    );
}

#[test]
fn a_non_null_x_stays_outside_the_premise() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["reuse_arm", "reuse_formal"] {
        assert!(rows[name].refused_with(OUTSIDE), "{name}: {:?}", rows[name]);
        // Withdrawn before the callback route reads the call.
        assert!(
            !rows[name]
                .unsupported
                .iter()
                .any(|d| d.contains("callback-")),
            "{name}: {:?}",
            rows[name]
        );
    }
}

#[test]
fn no_callback_is_assumed_benign_within_the_premise() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    assert!(
        rows["uncontracted"].refused_with("callback-uncontracted"),
        "{:?}",
        rows["uncontracted"]
    );
    assert!(
        rows["effectful"].refused_with("callback-effectful"),
        "{:?}",
        rows["effectful"]
    );
    // A formal and a call's result are open candidate sets: the null
    // default admits only a null pointer constant.
    for name in ["formal_callback", "result_callback"] {
        assert!(
            rows[name].refused_with("callback-undecided"),
            "{name}: {:?}",
            rows[name]
        );
    }
}

#[test]
fn a_callback_reading_u_puts_its_requirement_on_the_call() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    // As rule 13u: the call is decided as a direct call of the callback with
    // the same `u` is.
    assert_eq!(
        rows["requires_u"].status, rows["requires_u_direct"].status,
        "{:?} {:?}",
        rows["requires_u"], rows["requires_u_direct"]
    );
    assert_eq!(
        rows["requires_u"].unsupported.is_empty(),
        rows["requires_u_direct"].unsupported.is_empty()
    );
}

#[test]
fn a_null_callback_under_the_null_default_invokes_nothing() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    for name in ["null_callback", "null_callback_zero", "null_callback_cast"] {
        assert!(rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    // The declaration still applies: the result is owed.
    assert_eq!(
        rows["null_callback_leak"].status,
        AnalysisStatus::Unsafe,
        "{:?}",
        rows["null_callback_leak"]
    );
    // The null default changes nothing outside the `x` premise.
    assert!(
        rows["null_callback_reuse"].refused_with(OUTSIDE),
        "{:?}",
        rows["null_callback_reuse"]
    );
}

#[test]
fn without_the_null_default_a_null_callback_still_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows();
    assert!(
        rows["null_callback_plain"].refused_with("callback-undecided"),
        "{:?}",
        rows["null_callback_plain"]
    );
    // A non-null callback reads the same under either clause.
    assert!(
        rows["plain_identity"].safe(),
        "{:?}",
        rows["plain_identity"]
    );
    // A local that may hold a function or null is not a null pointer
    // constant: decided as rule 13u decides it, never read as no invocation.
    assert!(
        rows["local_callback"].refused_with("callback-undecided"),
        "{:?}",
        rows["local_callback"]
    );
}

/// No structural check establishes a null default: a unit defining the
/// declared function withdraws the declaration there (rule 13y), whatever
/// its body does with the callback.
const DEFINING: &str = "\
static int quiet(char *buf, int size, int rwflag, void *u) { return size; }
void *p_read(void *out, void **x, pass_cb *cb, void *u) { if (cb) cb(0, 0, 0, u); return p_alloc(1); }
void caller(void *bio, void *u) { void *r = p_read(bio, NULL, quiet, u); p_free(r); }
";

#[test]
fn a_defining_body_never_validates_a_null_default() {
    if !clang_available() {
        return;
    }
    let rows = rows_of(DEFINING, "caller");
    assert!(
        rows["caller"].refused_with("callback-withdrawn"),
        "{:?}",
        rows["caller"]
    );
}

// ---------------------------------------------------------------------------
// Card CALLBACK-CANDIDATE-LITERAL-PREMISE (rule 57 amended): a declaration
// carrying a literal-argument premise, met as a callback candidate, has no
// contract in force there. Each candidate path has its pair: the premise
// candidate refuses, and the same declaration without the premise still
// decides. Before the card each premise row was `Safe`.
// ---------------------------------------------------------------------------

const CANDIDATE_CONTRACTS: &str = "module contract;

extern fn q_alloc(n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_owned(nullable);
};

extern fn q_each(t: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_invoked(cb, t, _, _);
    ensures normal resource_return_none();
};

extern fn q_put(t: ptr<mut, u8>, v: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(size, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(v);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};

extern fn q_put_any(t: ptr<mut, u8>, v: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(v);
    ensures normal resource_borrowed(size);
    ensures normal resource_return_none();
};

extern fn q_set_hook(o: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(o);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_slot_registered(o, q_obj.hook, cb);
    ensures normal resource_return_none();
};

extern fn q_put_slot(t: ptr<mut, u8>, v: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(size, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(v);
    ensures normal resource_borrowed(size);
    ensures normal resource_callback_slot_invoked(t, q_obj.hook, _);
    ensures normal resource_return_none();
};

extern fn q_put_slot_any(t: ptr<mut, u8>, v: ptr<mut, u8>, size: isize) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(v);
    ensures normal resource_borrowed(size);
    ensures normal resource_callback_slot_invoked(t, q_obj.hook, _);
    ensures normal resource_return_none();
};

extern fn q_drop(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(p, 0);
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn q_drop_any(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn q_list_new(free_cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(free_cb);
    ensures normal resource_callback_registered(free_cb, result);
    ensures normal resource_return_owned(nonnull);
};

extern fn q_list_add(rl: ptr<mut, u8>, elem: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_borrowed(elem);
    ensures normal resource_element_added(elem, rl);
    ensures normal resource_return_none();
};

extern fn q_list_destroy(rl: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_callback_registry_invoked(rl, element);
    ensures normal resource_elements_dropped(rl);
    ensures normal resource_callback_unregistered(rl);
    ensures normal resource_released_if_constructed(rl);
    ensures normal resource_return_none();
};

extern fn q_op_new(cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_member_registered(result, q_op.op_cb, cb);
    ensures normal resource_return_owned(nonnull);
};

extern fn q_op_destroy(o: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(o, nonnull);
    ensures normal resource_callback_member_invoked(o, q_op.op_cb, o, _, _);
    ensures normal resource_return_none();
};
";

const CANDIDATE_PRELUDE: &str = "\
#include <stddef.h>
typedef int q_cb(void *t, void *v, long size);
typedef void q_hook(void *t);
typedef void q_free_cb(void *p);
struct q_obj;
struct q_op { q_cb *op_cb; };
void *q_alloc(__SIZE_TYPE__ n);
void q_each(void *t, q_cb *cb);
int q_put(void *t, void *v, long size);
int q_put_any(void *t, void *v, long size);
void q_set_hook(struct q_obj *o, q_hook *cb);
int q_put_slot(void *t, void *v, long size);
int q_put_slot_any(void *t, void *v, long size);
void q_drop(void *p);
void q_drop_any(void *p);
void *q_list_new(q_free_cb *free_cb);
void q_list_add(void *rl, void *elem);
void q_list_destroy(void *rl);
struct q_op *q_op_new(q_cb *cb);
void q_op_destroy(struct q_op *o);
";

const CANDIDATE_UNIT: &str = "\
void each_premise(void *t) { q_each(t, q_put); }
void each_plain(void *t) { q_each(t, q_put_any); }
void each_slot_premise(void *t) { q_each(t, q_put_slot); }
void each_slot_plain(void *t) { q_each(t, q_put_slot_any); }
void list_premise(void) { void *l = q_list_new(q_drop); void *p = q_alloc(8); if (!p) { q_list_destroy(l); return; } q_list_add(l, p); q_list_destroy(l); }
void list_plain(void) { void *l = q_list_new(q_drop_any); void *p = q_alloc(8); if (!p) { q_list_destroy(l); return; } q_list_add(l, p); q_list_destroy(l); }
void op_premise(void) { struct q_op *o = q_op_new(q_put); q_op_destroy(o); }
void op_plain(void) { struct q_op *o = q_op_new(q_put_any); q_op_destroy(o); }
void direct_in_premise(void *t, void *v) { int n = q_put(t, v, 0); (void)n; }
";

fn candidate_contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "premise_candidates.qv".to_owned(),
        CANDIDATE_CONTRACTS.to_owned(),
    )]))
    .expect("the candidate fixture contracts load")
}

/// The run's member facts: the unit's own stores and the declared slot
/// registrations (as `callback_member_fold::declared_stores` derives them),
/// so a composed candidate's own slot invocation decides over the run's set.
fn candidate_run(contracts: &ResourceContracts) -> Arc<RunMemberFacts> {
    let text = format!("{CANDIDATE_PRELUDE}\n{CANDIDATE_UNIT}\n");
    let unit = read_unit(
        &text,
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("read: {error}\n{text}"));
    let mut stores = Vec::new();
    for (name, declaration) in contracts.retaining() {
        for clause in &declaration.callbacks {
            if let CallbackClause::MemberRegistered {
                member, callback, ..
            } = clause
            {
                stores.push(DeclaredStore::Parameter {
                    member: member.spelling(),
                    function: name.clone(),
                    index: *callback,
                });
            }
        }
    }
    stores.sort();
    Arc::new(RunMemberFacts::new(
        vec![unit, declarations_unit(&stores)],
        BTreeMap::new(),
    ))
}

/// Every row of the candidate unit, under the run's member facts or (for
/// the constructed member path, which the route reads where the unit's own
/// set does not decide) without them.
fn candidate_rows(with_run: bool) -> BTreeMap<String, Row> {
    let contracts = candidate_contracts();
    let run = with_run.then(|| candidate_run(&contracts));
    let text = format!("{CANDIDATE_PRELUDE}\n{CANDIDATE_UNIT}\n");
    // The certificate is rechecked over the same run facts it was read under.
    let outcome = scoped(run, || {
        let outcome = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source: &text,
                ..Default::default()
            },
            "direct_in_premise",
            &contracts,
        )
        .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
        if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
            check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
        }
        outcome
    });
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .filter_map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                Some(description.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

/// Whether a refusal names `candidate` as having no contract in force.
fn uncontracted(row: &Row, candidate: &str) -> bool {
    row.status != AnalysisStatus::Safe
        && row
            .unsupported
            .iter()
            .any(|d| d.contains("callback-uncontracted") && d.contains(&format!("`{candidate}`")))
}

/// The function-map path (`callback_contracts::classify`): `q_put` has the
/// identity's shape, so it was applied at the invocation whatever the
/// library hands `size`; now it has no contract in force there. A direct
/// call within its premise is unchanged.
#[test]
fn a_premise_carrying_function_map_candidate_has_no_contract_in_force() {
    if !clang_available() {
        return;
    }
    let rows = candidate_rows(true);
    assert!(
        uncontracted(&rows["each_premise"], "q_put"),
        "{:?}",
        rows["each_premise"]
    );
    assert!(rows["each_plain"].safe(), "{:?}", rows["each_plain"]);
    assert!(
        rows["direct_in_premise"].safe(),
        "{:?}",
        rows["direct_in_premise"]
    );
}

/// The composition path (`candidate_composition`): `q_put_slot` is withheld
/// and its own slot invocation decides over the run's set, so it was
/// composed and applied; now the premise refuses it before the composition.
#[test]
fn a_premise_carrying_composed_candidate_has_no_contract_in_force() {
    if !clang_available() {
        return;
    }
    let rows = candidate_rows(true);
    let row = &rows["each_slot_premise"];
    assert!(uncontracted(row, "q_put_slot"), "{row:?}");
    assert!(
        !row.unsupported
            .iter()
            .any(|d| d.contains("callback-own-invocation-refused")),
        "{row:?}"
    );
    assert!(
        rows["each_slot_plain"].safe(),
        "{:?}",
        rows["each_slot_plain"]
    );
}

/// The registration path (rule 13w): a registration stores no contract for
/// a premise-carrying candidate, so the registry is not a releasing one;
/// without the premise the registry takes and releases the element.
#[test]
fn a_premise_carrying_registered_candidate_has_no_contract_in_force() {
    if !clang_available() {
        return;
    }
    let rows = candidate_rows(true);
    let row = &rows["list_premise"];
    assert!(
        row.status != AnalysisStatus::Safe
            && row.unsupported.iter().any(|d| d.contains("callback-")),
        "{row:?}"
    );
    assert!(rows["list_plain"].safe(), "{:?}", rows["list_plain"]);
}

/// The constructed member path (rule 12o): a member registration stores no
/// contract for a premise-carrying candidate, so its invocation refuses. Read
/// without the run's facts, where the unit's own set does not decide the
/// member and the route reads the constructor's registration instead.
#[test]
fn a_premise_carrying_constructed_candidate_has_no_contract_in_force() {
    if !clang_available() {
        return;
    }
    let rows = candidate_rows(false);
    assert!(
        uncontracted(&rows["op_premise"], "q_put"),
        "{:?}",
        rows["op_premise"]
    );
    assert!(rows["op_plain"].safe(), "{:?}", rows["op_plain"]);
}
