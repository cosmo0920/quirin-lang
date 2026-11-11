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

//! Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (gateway rules 12m and 13l): the
//! caller branch check on route G, over the synthetic serve loops and
//! callbacks of `fixtures/contracts/return_code_handoff.qv` (trusted test
//! assumptions: a `Safe` here is `Safe` over them and promotes nothing).
//!
//! A conditional hand-off is admitted only where the invoker's body branches
//! on the callback's result with the guard's own constant and never names
//! the handed object where the guard holds; a mismatched branch, a typo, a
//! constant of another type and a direct call refuse.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
#include <stddef.h>
typedef enum { H_PASS, H_HANDLED, H_KEEP, H_YIELD } h_res_t;
enum other { OTHER_A, OTHER_B };
struct hop { int x; };
typedef h_res_t(h_cb_t)(void *q, struct hop *op, void *opaque);
struct hop *h_dequeue(void *q);
void h_destroy(struct hop *op);

/* Keeps or disposes of the op on PASS and never names it otherwise. */
int h_serve(void *q, h_cb_t *cb, void *opaque) {
    struct hop *op;
    while ((op = h_dequeue(q))) {
        h_res_t res = cb(q, op, opaque);
        if (res == H_PASS)
            h_destroy(op);
    }
    return 0;
}

/* Branches on KEEP: the op is untouched there. */
int h_keep_serve(void *q, h_cb_t *cb, void *opaque) {
    struct hop *op = h_dequeue(q);
    if (!op)
        return 0;
    h_res_t res = cb(q, op, opaque);
    if (res == H_KEEP)
        return 1;
    h_destroy(op);
    return 0;
}

/* Branches on HANDLED and KEEP only: every other value, PASS and any value
 * outside the enumeration, destroys the op. */
int h_pop_serve(void *q, h_cb_t *cb, void *opaque) {
    struct hop *op = h_dequeue(q);
    if (!op)
        return 0;
    h_res_t res = cb(q, op, opaque);
    if (res == H_HANDLED || res == H_KEEP)
        return 1;
    h_destroy(op);
    return 0;
}

/* The op stays the loop's (`_`): it compares the call itself with KEEP. */
int h_op_serve(void *q, h_cb_t *cb, void *opaque) {
    struct hop *op = h_dequeue(q);
    if (!op)
        return 0;
    if (cb(q, op, opaque) != H_KEEP)
        h_destroy(op);
    return 0;
}

/* Stores the callback's result into a narrower variable before comparing
 * with PASS: the split classes could disagree with the guard. */
int h_converted_serve(void *q, h_cb_t *cb, void *opaque) {
    struct hop *op;
    while ((op = h_dequeue(q))) {
        unsigned char res = cb(q, op, opaque);
        if (res == H_PASS)
            h_destroy(op);
    }
    return 0;
}

h_res_t not_pass_cb(void *q, struct hop *op, void *opaque) {
    if (op->x)
        return H_PASS;
    h_destroy(op);
    return H_HANDLED;
}
h_res_t not_pass_named_cb(void *q, struct hop *op, void *opaque) {
    if (op->x)
        return H_PASS;
    h_destroy(op);
    return H_HANDLED;
}
h_res_t keep_cb(void *q, struct hop *op, void *opaque) { return op->x ? H_KEEP : H_HANDLED; }
h_res_t typo_cb(void *q, struct hop *op, void *opaque) { return H_HANDLED; }
h_res_t other_type_cb(void *q, struct hop *op, void *opaque) { return H_HANDLED; }
";

fn environment() -> ResourceContracts {
    let source = include_str!("fixtures/contracts/return_code_handoff.qv");
    let source = if cfg!(windows) {
        source.replace("-> u32", "-> i32")
    } else {
        source.to_owned()
    };
    load_resource_contract_sources(&BTreeMap::from([(
        "return_code_handoff.qv".to_owned(),
        source,
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
            eprintln!("skipping Clang-dependent SUPPLIED-OWNED-RETURN-CODE-HANDOFF test: {reason}");
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

/// The row of `entry`, the last definition of `PRELUDE + source`.
fn row(source: &str, entry: &str) -> Row {
    let text = format!("{PRELUDE}\n{source}\n");
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
    let row = &summary.functions[entry];
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

fn call(entry: &str, serve: &str, callback: &str) -> Row {
    row(
        &format!("int {entry}(void *q, void *opaque) {{ return {serve}(q, {callback}, opaque); }}"),
        entry,
    )
}

/// The not-PASS form (`supplied_owned`): the loop disposes of the op on PASS
/// and never names it otherwise. The alias and the `@` name read alike.
#[test]
fn the_not_pass_form_is_admitted_where_the_loop_branches_on_pass() {
    if !clang_available() {
        return;
    }
    let aliased = call("use_not_pass", "h_serve", "not_pass_cb");
    assert!(aliased.safe(), "{aliased:?}");
    let named = call("use_named", "h_serve", "not_pass_named_cb");
    assert!(named.safe(), "{named:?}");
}

/// The KEEP-only form, at a `supplied_owned` position and at `_` (the call
/// compared directly).
#[test]
fn the_keep_only_form_is_admitted_where_the_loop_branches_on_keep() {
    if !clang_available() {
        return;
    }
    let supplied = call("use_keep", "h_keep_serve", "keep_cb");
    assert!(supplied.safe(), "{supplied:?}");
    let own = call("use_op_keep", "h_op_serve", "keep_cb");
    assert!(own.safe(), "{own:?}");
}

/// A mismatched caller branch refuses: the pop loop never compares with
/// PASS and destroys the op for every value outside HANDLED and KEEP; the
/// KEEP loop destroys it on HANDLED, a value `!= PASS`.
#[test]
fn a_mismatched_caller_branch_refuses() {
    if !clang_available() {
        return;
    }
    for (entry, serve, callback) in [
        ("use_pop", "h_pop_serve", "not_pass_cb"),
        ("use_keep_loop", "h_keep_serve", "not_pass_cb"),
    ] {
        let refused = call(entry, serve, callback);
        assert!(
            refused.refused_with("callback-handoff-unbranched"),
            "{entry}: {refused:?}"
        );
    }
}

/// A guard the invoker never compares with is refused even where the object
/// happens to stay untouched (the user's decision: the same constant).
#[test]
fn a_guard_the_caller_does_not_branch_on_never_admits() {
    if !clang_available() {
        return;
    }
    let refused = call("use_keep_at_pass_loop", "h_serve", "keep_cb");
    assert!(
        refused.refused_with("callback-handoff-unbranched"),
        "{refused:?}"
    );
}

/// An invoker that stores the callback's result through a conversion before
/// comparing refuses (`callback-handoff-unbranched`), even when the compared
/// constant and per-class handling otherwise match.
#[test]
fn an_invoker_that_converts_the_result_refuses() {
    if !clang_available() {
        return;
    }
    let refused = call("use_converted", "h_converted_serve", "not_pass_cb");
    assert!(
        refused.refused_with("callback-handoff-unbranched"),
        "{refused:?}"
    );
}

/// A typo and a constant of another enumeration resolve to nothing in the
/// candidate's own result type, and refuse.
#[test]
fn an_unresolved_or_mistyped_constant_refuses() {
    if !clang_available() {
        return;
    }
    for (entry, callback) in [("use_typo", "typo_cb"), ("use_other", "other_type_cb")] {
        let refused = call(entry, "h_serve", callback);
        assert!(
            refused.refused_with("callback-handoff-unbranched"),
            "{entry}: {refused:?}"
        );
    }
}

/// A direct call does not read the result's branch: it refuses.
#[test]
fn a_direct_call_of_a_hand_off_declaration_refuses() {
    if !clang_available() {
        return;
    }
    let direct = row(
        "int direct(void *q, struct hop *op) { h_res_t res = not_pass_cb(q, op, NULL); \
         return res == H_PASS; }",
        "direct",
    );
    assert!(direct.refused_with("callback-handoff-direct"), "{direct:?}");
}
