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

//! Card CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x,
//! `docs/reports/cg27-loops-recursion-budgets/README.md`).
//!
//! Three parts, each with accepting and refusing controls:
//!
//! * **Loop invariants.** Route G's settled loop-head state is the inductive
//!   invariant of its work list: a per-iteration acquire/release is `Safe`, a
//!   body that breaks the invariant (an acquisition it does not release) is
//!   not. The counted-user pass (`legacy_counted_loops.rs`) summarizes a loop
//!   whose every token keeps the counts as it found them, and refuses one with
//!   an operation, an opaque call, a `return` or a `goto` in it.
//! * **Recursion.** The signed threshold rank (`legacy_signed_rank.rs`):
//!   `n <= c` base, `n > c` step, actuals `n - k`; single-rank, multi-rank and
//!   mutual recursion with a valid summary are `Safe` and rechecked by the
//!   certificate; every non-decreasing, narrowing, mixed or base-recursive
//!   shape refuses.
//! * **Budgets.** Every budget's exhaustion is reported and never `Safe`: the
//!   custody alias widening, the counted-user path bound, and route T's path
//!   and operation limits.
//!
//! `Safe` here is `Safe` over synthetic fixture contracts (trusted test
//! assumptions) and promotes nothing.

#[path = "support/clang_gap.rs"]
mod clang_gap;
#[path = "support/host_abi.rs"]
mod host_abi;

use std::collections::BTreeMap;

use clang_gap::*;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, ContractTrust, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, check_legacy_safety_certificate,
    counted_users::{ClauseRecord, ClauseVerdict},
};
use qcore_contract::contract_loader::load_resource_contract_sources;

// ---------------------------------------------------------------------------
// Loop invariants on route G: the settled loop-head state
// ---------------------------------------------------------------------------

#[test]
fn a_per_iteration_acquire_release_loop_is_safe() {
    if !clang_available() {
        return;
    }
    for source in [
        "void subject(int n) { int i; for (i = 0; i < n; i++) { void *p = q_alloc(); q_free(p); } }",
        "void subject(int n) { while (n > 0) { void *p = q_alloc(); q_free(p); n--; } }",
        "void subject(int n) { do { void *p = q_alloc(); q_free(p); n--; } while (n > 0); }",
        "void subject(int n) { for (;;) { void *p = q_alloc(); if (n) { q_free(p); break; } q_free(p); } }",
    ] {
        let run = g(source);
        assert!(run.safe(), "{source}: {run:?}");
    }
}

#[test]
fn a_body_that_breaks_the_loop_invariant_is_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        // An acquisition the iteration does not release: every earlier
        // generation leaks (rule 11a).
        "void subject(int n) { void *p = 0; int i; for (i = 0; i < n; i++) p = q_alloc(); q_free(p); }",
        "void subject(int n) { while (n > 0) { void *p = q_alloc(); n--; } }",
        // A release of a resource the invariant does not hold on the second
        // iteration.
        "void subject(int n) { void *p = q_alloc(); int i; for (i = 0; i < n; i++) q_free(p); }",
    ] {
        let run = g(source);
        assert!(!run.safe(), "{source}: {run:?}");
    }
}

// ---------------------------------------------------------------------------
// Loop invariants in the counted-user pass
// ---------------------------------------------------------------------------

const COUNTED_CONTRACTS: &str = r#"module contract;

extern fn u_ref(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_added(t, refs);
    ensures normal resource_return_none();
};

extern fn u_unref(t: ptr<mut, u8>, check: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(check, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(check);
    ensures normal resource_user_dropped(t, refs);
    ensures normal resource_return_none();
};

extern fn u_release(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_release_check(t, refs);
    ensures normal resource_return_none();
};

extern fn u_enqueue(q: ptr<mut, u8>, t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_user_neutral();
    ensures normal resource_return_none();
};

extern fn u_log(n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn l_retry(q: ptr<mut, u8>, t: ptr<mut, u8>, n: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(n);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn l_forget(q: ptr<mut, u8>, t: ptr<mut, u8>, n: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(n);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};
"#;

const COUNTED_PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    r#"
struct u_task { int refs; };
struct u_queue { int size; };
void u_ref(struct u_task *t);
void u_unref(struct u_task *t, int check);
void u_release(struct u_task *t);
int u_enqueue(struct u_queue *q, struct u_task *t);
void u_log(int n);
int u_unknown(void);
"#
);

fn counted(body: &str, entry: &str) -> LegacyProgramSummary {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "loops.qcore".to_owned(),
        COUNTED_CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load");
    analyze_legacy_c_translation_unit(&format!("{COUNTED_PRELUDE}\n{body}"), entry, &contracts)
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn verdicts(summary: &LegacyFunctionSummary) -> Vec<(String, ClauseVerdict)> {
    summary
        .counted_users
        .iter()
        .map(
            |ClauseRecord {
                 clause, verdict, ..
             }| (clause.clone(), *verdict),
        )
        .collect()
}

fn diagnostics(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{:?}", diagnostic.kind))
        .collect()
}

const NOT_MODELLED: &str = "the body contains a loop, which this pass does not model";

#[test]
fn a_neutral_loop_keeps_every_count_and_the_clauses_hold() {
    let summary = counted(
        r#"
int l_retry(struct u_queue *q, struct u_task *t, int n)
{
    int ret;
    int i;

    u_ref(t);
    for (i = 0; i < n; i++) {
        u_log(i);
        if (i > 3)
            break;
    }
    ret = u_enqueue(q, t);
    while (n > 0) {
        n--;
        ret = u_enqueue(q, t);
    }
    if (ret == -1) {
        u_unref(t, 0);
        u_release(t);
        return -1;
    }
    return 0;
}

void l_balanced(struct u_task *t, int n)
{
    u_ref(t);
    do {
        u_log(n);
        n--;
    } while (n > 0);
    u_unref(t, 0);
    u_release(t);
}
"#,
        "l_retry",
    );
    let retry = function(&summary, "l_retry");
    assert_eq!(
        retry.status,
        AnalysisStatus::Safe,
        "{:?}",
        diagnostics(retry)
    );
    assert_eq!(verdicts(retry).len(), 3, "{:?}", verdicts(retry));
    assert!(
        verdicts(retry)
            .iter()
            .all(|(_, verdict)| *verdict == ClauseVerdict::Held),
        "{:?}",
        verdicts(retry)
    );
    let balanced = function(&summary, "l_balanced");
    assert_eq!(
        balanced.status,
        AnalysisStatus::Safe,
        "{:?}",
        diagnostics(balanced)
    );
}

#[test]
fn a_loop_that_breaks_the_count_invariant_is_refused_never_assumed() {
    for (case, body) in [
        ("an addition", "while (n > 0) { u_ref(t); n--; }"),
        ("a drop", "while (n > 0) { u_unref(t, 0); n--; }"),
        ("a release check", "while (n > 0) { u_release(t); n--; }"),
        ("a counter write", "while (n > 0) { t->refs++; n--; }"),
        ("an opaque call", "while (n > 0) { u_unknown(); n--; }"),
        (
            "a return",
            "while (n > 0) { if (u_enqueue(q, t)) return -1; n--; }",
        ),
        ("a jump out", "while (n > 0) { goto out; } out:"),
        // An open callee expression (a cast): rule 11p reads no candidate
        // set, so the older refusal stands.
        (
            "a call through an expression",
            "while (n > 0) { ((void (*)(int)) u_log)(n); n--; }",
        ),
    ] {
        let source = format!(
            "int l_forget(struct u_queue *q, struct u_task *t, int n)\n{{\n    u_ref(t);\n    {body}\n    u_unref(t, 0);\n    u_release(t);\n    return -1;\n}}\n"
        );
        let summary = counted(&source, "l_forget");
        let forget = function(&summary, "l_forget");
        assert_ne!(forget.status, AnalysisStatus::Safe, "{case}");
        assert!(
            verdicts(forget)
                .iter()
                .any(|(_, verdict)| *verdict != ClauseVerdict::Held),
            "{case}: {:?}",
            verdicts(forget)
        );
        if case != "a jump out" {
            assert!(
                diagnostics(forget)
                    .iter()
                    .any(|text| text.contains(NOT_MODELLED)),
                "{case}: {:?}",
                diagnostics(forget)
            );
        }
    }
}

/// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p): a conditional of one
/// designator with an effect-free condition is a closed candidate set, so
/// where Clang reads the unit the call is exactly its direct spelling and the
/// loop is summarized as that loop is.
#[test]
fn a_closed_callee_expression_in_a_loop_is_its_direct_call() {
    let run = |body: &str| {
        let source = format!(
            "int l_forget(struct u_queue *q, struct u_task *t, int n)\n{{\n    u_ref(t);\n    {body}\n    u_unref(t, 0);\n    u_release(t);\n    return -1;\n}}\n"
        );
        let summary = counted(&source, "l_forget");
        let forget = function(&summary, "l_forget");
        (forget.status, verdicts(forget), diagnostics(forget))
    };
    let closed = run("while (n > 0) { (n ? u_log : u_log)(n); n--; }");
    if !clang_available() {
        assert_ne!(closed.0, AnalysisStatus::Safe);
        return;
    }
    let direct = run("while (n > 0) { u_log(n); n--; }");
    assert_eq!(closed, direct);
}

#[test]
fn a_neutral_loop_does_not_hide_a_refuted_clause() {
    // Non-vacuity: the C2 shape (an addition, then an early return without
    // the drop) behind a summarized loop is still refuted.
    let summary = counted(
        r#"
int l_retry(struct u_queue *q, struct u_task *t, int n)
{
    int ret;

    u_ref(t);
    while (n > 0) {
        u_log(n);
        n--;
    }
    ret = u_enqueue(q, t);
    if (ret == -1) {
        return -1;
    }
    return 0;
}
"#,
        "l_retry",
    );
    let retry = function(&summary, "l_retry");
    assert_eq!(
        retry.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        diagnostics(retry)
    );
    assert!(verdicts(retry).contains(&(
        "resource_user_exit(t, -1, restored)".to_owned(),
        ClauseVerdict::Refuted
    )));
}

#[test]
fn a_loop_forgets_the_values_it_may_assign() {
    // `ret` is the literal -1 before the loop, and the loop may set it to 0.
    // Read on the stale literal, the fallthrough arm (count restored, value 0)
    // would be pruned and `added` would hold; the summary forgets `ret`, so
    // the arm is checked and the clause is refuted.
    let summary = counted(
        r#"
int l_retry(struct u_queue *q, struct u_task *t, int n)
{
    int ret;

    ret = -1;
    u_ref(t);
    while (n > 0) {
        ret = 0;
        n--;
    }
    if (ret == -1) {
        u_unref(t, 0);
        u_release(t);
        return -1;
    }
    u_unref(t, 0);
    u_release(t);
    return 0;
}
"#,
        "l_retry",
    );
    let retry = function(&summary, "l_retry");
    assert_eq!(
        retry.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        diagnostics(retry)
    );
    assert!(verdicts(retry).contains(&(
        "resource_user_exit(t, 0, added)".to_owned(),
        ClauseVerdict::Refuted
    )));
}

#[test]
fn the_counted_user_path_bound_is_reported_never_safe() {
    let ifs = |count: usize| -> String {
        (0..count)
            .map(|index| format!("    if (n > {index}) u_log({index});\n"))
            .collect()
    };
    let source = |count: usize| {
        format!(
            "void l_paths(struct u_task *t, int n)\n{{\n    u_ref(t);\n{}    u_unref(t, 0);\n    u_release(t);\n}}\n",
            ifs(count)
        )
    };
    // 2^8 = 256 paths: within the bound of 512, decided.
    let summary = counted(&source(8), "l_paths");
    let within = function(&summary, "l_paths");
    assert_eq!(
        within.status,
        AnalysisStatus::Safe,
        "{:?}",
        diagnostics(within)
    );
    // 2^10 = 1024 paths: the bound is exhausted, so the obligation is
    // undecided and the function is Unknown, never Safe.
    let summary = counted(&source(10), "l_paths");
    let exhausted = function(&summary, "l_paths");
    assert_eq!(
        exhausted.status,
        AnalysisStatus::Unknown,
        "{:?}",
        diagnostics(exhausted)
    );
    assert!(
        diagnostics(exhausted)
            .iter()
            .any(|text| text.contains("more than 512 paths")),
        "{:?}",
        diagnostics(exhausted)
    );
}

// ---------------------------------------------------------------------------
// Recursion: the signed threshold rank
// ---------------------------------------------------------------------------

fn unrestricted(body: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{body}\n");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"))
}

fn certified_summary(body: &str, function: &str) -> ResourceFunctionContract {
    let result = unrestricted(body);
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected a validated ranked summary: {result:?}\n{body}");
    };
    check_legacy_safety_certificate(&certificate).expect("the certificate rechecks");
    let summary = certificate
        .summaries
        .iter()
        .find(|summary| summary.function == function)
        .unwrap_or_else(|| panic!("missing summary for {function}"));
    assert_eq!(summary.trust, ContractTrust::InferredValidated);
    summary.contract.clone()
}

fn released_then_borrowed() -> ResourceFunctionContract {
    ResourceFunctionContract::new(
        [
            ResourceParameterMode::Released { nullable: false },
            ResourceParameterMode::Borrowed,
        ],
        ResourceReturnMode::None,
    )
}

const CALLER: &str = "void subject(void) { void *p = q_alloc(); release(p, 7); }";

#[test]
fn a_signed_threshold_recursion_has_a_ranked_summary() {
    if !clang_available() {
        return;
    }
    for control in ["int", "signed int", "long", "long long"] {
        for body in [
            "if (n <= 0) { q_free(p); return; } release(p, n - 1);",
            "if (n < 1) { q_free(p); return; } release(p, n - 1);",
            "if (n > 0) { release(p, n - 1); return; } q_free(p);",
            "if (0 >= n) { q_free(p); return; } release(p, n - 1);",
            "if (n >= 1) { release(p, n - 1); return; } q_free(p);",
            "if (n <= 2) { q_free(p); return; } release(p, n - 3);",
        ] {
            let source = format!("void release(void *p, {control} n) {{ {body} }} {CALLER}");
            assert_eq!(
                certified_summary(&source, "release"),
                released_then_borrowed(),
                "{source}"
            );
        }
    }
}

#[test]
fn multi_rank_recursion_has_a_ranked_summary() {
    if !clang_available() {
        return;
    }
    // Two recursive calls at two ranks, each consuming its own resource.
    let source = "void release(void *p, int n) { if (n <= 1) { q_free(p); return; } \
                  void *q = q_alloc(); release(q, n - 2); release(p, n - 1); } "
        .to_owned()
        + CALLER;
    assert_eq!(
        certified_summary(&source, "release"),
        released_then_borrowed()
    );
    // An owned result at several ranks.
    let make = "void *make(int n) { if (n <= 1) { void *p = q_alloc(); return p; } \
                void *a = make(n - 1); void *b = make(n - 2); q_free(b); return a; } \
                void subject(void) { void *p = make(9); q_free(p); }";
    assert_eq!(
        certified_summary(make, "make"),
        ResourceFunctionContract::new(
            [ResourceParameterMode::Borrowed],
            ResourceReturnMode::Owned { nullable: false },
        )
    );
}

#[test]
fn mutual_recursion_with_a_valid_summary_is_safe() {
    if !clang_available() {
        return;
    }
    let body = "void odd(void *p, int n); \
        void even(void *p, int n) { if (n <= 0) { q_free(p); return; } odd(p, n - 1); } \
        void odd(void *p, int n) { if (n <= 0) { q_free(p); return; } even(p, n - 1); } \
        void subject(void) { void *p = q_alloc(); even(p, 10); void *q = q_alloc(); odd(q, -4); }";
    for function in ["even", "odd"] {
        assert_eq!(
            certified_summary(body, function),
            released_then_borrowed(),
            "{function}"
        );
    }
    // Mixed rank steps across the pair.
    let mixed = body.replace("even(p, n - 1)", "even(p, n - 2)");
    certified_summary(&mixed, "odd");
}

#[test]
fn a_caller_leak_through_a_ranked_summary_is_kept() {
    if !clang_available() {
        return;
    }
    let make = "void *make(int n) { if (n <= 0) { void *p = q_alloc(); return p; } \
                void *r = make(n - 1); return r; }";
    for caller in [
        "void subject(void) { void *p = make(3); }",
        "void subject(void) { make(3); }",
    ] {
        assert!(
            !matches!(
                unrestricted(&format!("{make} {caller}")),
                UnrestrictedLegacyResult::Safe(_)
            ),
            "{caller}"
        );
    }
}

#[test]
fn non_ranked_signed_recursion_never_publishes() {
    if !clang_available() {
        return;
    }
    for (case, signature, body) in [
        // C07's BEFORE shape: a signed zero/nonzero partition is not a rank.
        (
            "zero partition",
            "int n",
            "if (n == 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "negated zero partition",
            "int n",
            "if (!n) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "not decreasing",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, n);",
        ),
        (
            "increasing",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, n + 1);",
        ),
        (
            "reversed",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, 1 - n);",
        ),
        (
            "nonzero literal",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, 5);",
        ),
        (
            "negative threshold",
            "int n",
            "if (n < 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "two thresholds",
            "int n",
            "if (n <= 0) { q_free(p); return; } if (n > 3) { release(p, n - 1); return; } release(p, n - 1);",
        ),
        (
            "recursion in the base",
            "int n",
            "if (n <= 0) { release(p, n - 1); return; } q_free(p);",
        ),
        (
            "control assigned",
            "int n",
            "n = n - 1; if (n <= 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "control addressed",
            "int n",
            "int *a = &n; if (n <= 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "control shadowed",
            "int n",
            "if (n <= 0) { q_free(p); return; } { int n = 7; release(p, n - 1); }",
        ),
        (
            "compound guard",
            "int n",
            "if (n <= 0 || !p) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "char control",
            "signed char n",
            "if (n <= 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "short control",
            "short n",
            "if (n <= 0) { q_free(p); return; } release(p, n - 1);",
        ),
        (
            "step too large",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, n - 4096);",
        ),
        (
            "double release",
            "int n",
            "if (n <= 0) { q_free(p); return; } release(p, n - 1); q_free(p);",
        ),
        (
            "inconsistent contexts",
            "int n",
            "if (n <= 0) { return; } release(p, n - 1);",
        ),
        (
            "released twice by two ranks",
            "int n",
            "if (n <= 1) { q_free(p); return; } release(p, n - 1); release(p, n - 2);",
        ),
        (
            "loop in the body",
            "int n",
            "while (n > 9) { q_free(p); return; } if (n <= 0) { q_free(p); return; } release(p, n - 1);",
        ),
    ] {
        let source = format!("void release(void *p, {signature}) {{ {body} }} {CALLER}");
        let result = unrestricted(&source);
        assert!(
            !matches!(result, UnrestrictedLegacyResult::Safe(_)),
            "{case}: {result:?}"
        );
    }
    let mixed_widths = "void odd(void *p, long n); \
        void even(void *p, int n) { if (n <= 0) { q_free(p); return; } odd(p, n - 1); } \
        void odd(void *p, long n) { if (n <= 0) { q_free(p); return; } even(p, n - 1); } \
        void subject(void) { void *p = q_alloc(); even(p, 10); }";
    assert!(!matches!(
        unrestricted(mixed_widths),
        UnrestrictedLegacyResult::Safe(_)
    ));
    let mixed_thresholds = "void odd(void *p, int n); \
        void even(void *p, int n) { if (n <= 0) { q_free(p); return; } odd(p, n - 1); } \
        void odd(void *p, int n) { if (n <= 1) { q_free(p); return; } even(p, n - 1); } \
        void subject(void) { void *p = q_alloc(); even(p, 10); }";
    assert!(!matches!(
        unrestricted(mixed_thresholds),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

#[test]
fn the_certificate_rechecks_the_rank_guard_and_the_step() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void release(void *p, int n) {{ if (n <= 0) {{ q_free(p); return; }} release(p, n - 1); }} {CALLER}"
    );
    let UnrestrictedLegacyResult::Safe(certificate) = unrestricted(&body) else {
        panic!("expected Safe");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    for (old, new) in [
        ("n - 1", "n + 1"),
        ("n - 1", "n"),
        ("n <= 0", "n == 0"),
        ("n <= 0", "n < 0"),
        ("int n", "short n"),
    ] {
        let mut stale = certificate.clone();
        stale.preprocessed_source = stale.preprocessed_source.replace(old, new);
        assert_ne!(stale.preprocessed_source, certificate.preprocessed_source);
        assert!(
            check_legacy_safety_certificate(&stale).is_err(),
            "{old} -> {new}"
        );
    }
    let mut forged = certificate.clone();
    forged
        .summaries
        .iter_mut()
        .find(|summary| summary.function == "release")
        .unwrap()
        .contract
        .parameters[0] = ResourceParameterMode::Borrowed;
    assert!(check_legacy_safety_certificate(&forged).is_err());
}

#[test]
fn the_zero_nonzero_route_is_unchanged_for_its_own_shapes() {
    if !clang_available() {
        return;
    }
    // The signed route is consulted only where the zero/nonzero route
    // refuses; the unsigned countdown and the literal-zero context keep their
    // own summaries.
    let unsigned = format!(
        "void release(void *p, unsigned n) {{ if (n) {{ release(p, n - 1); return; }} q_free(p); }} {CALLER}"
    );
    assert_eq!(
        certified_summary(&unsigned, "release"),
        released_then_borrowed()
    );
    let literal = format!(
        "void release(void *p, int flag) {{ if (flag) {{ release(p, 0); return; }} q_free(p); }} {CALLER}"
    );
    assert_eq!(
        certified_summary(&literal, "release"),
        released_then_borrowed()
    );
    // Unsigned multi-rank stays refused: `n - 2` from 1 wraps.
    let wrap = format!(
        "void release(void *p, unsigned n) {{ if (n) {{ release(p, n - 2); return; }} q_free(p); }} {CALLER}"
    );
    assert!(!matches!(
        unrestricted(&wrap),
        UnrestrictedLegacyResult::Safe(_)
    ));
}

// ---------------------------------------------------------------------------
// Budgets: exhaustion is coverage, never Safe
// ---------------------------------------------------------------------------

#[test]
fn the_custody_alias_widening_is_reported_never_safe() {
    if !clang_available() {
        return;
    }
    let assignments = |count: usize| -> (String, String, String, String) {
        let formals = (0..count)
            .map(|i| format!("int c{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let allocations = (0..count)
            .map(|i| format!("void *p{i} = q_alloc();"))
            .collect::<String>();
        let choices = (0..count)
            .map(|i| format!("if (c{i}) x = p{i};"))
            .collect::<String>();
        let releases = (0..count)
            .map(|i| format!("q_free(p{i});"))
            .collect::<String>();
        (formals, allocations, choices, releases)
    };
    let source = |count: usize| {
        let (formals, allocations, choices, releases) = assignments(count);
        format!(
            "void subject({formals}) {{ void *x = 0; {allocations} {choices} q_observe(x); {releases} }}"
        )
    };
    // Eight targets: within the bound, not widened.
    let within = g(&source(8));
    assert!(
        !within.refused("alias set exceeded 8 targets and was widened"),
        "{within:?}"
    );
    // Nine: widened, reported, Unknown.
    let widened = g(&source(9));
    assert!(
        widened.refused("alias set exceeded 8 targets and was widened"),
        "{widened:?}"
    );
    assert!(widened.unknown(), "{widened:?}");
}

#[test]
fn route_t_budgets_are_reported_never_safe() {
    if !clang_available() {
        return;
    }
    let branches = |count: usize| -> String {
        (1..=count)
            .map(|i| format!("if (a) f({i}); "))
            .collect::<String>()
    };
    let within = format!(
        "int f(int); int subject(int a) {{ {}return 0; }}",
        branches(6)
    );
    assert!(t(&within).is_ok(), "{within}");
    let exhausted = format!(
        "int f(int); int subject(int a) {{ {}return 0; }}",
        branches(7)
    );
    assert_eq!(t_refusal(&exhausted), "callback path limit exceeded");
}

// ---------------------------------------------------------------------------
// The re-pinned neutral-route shapes, under the signed threshold rank
// ---------------------------------------------------------------------------

/// The effectful `if (n > 0)` bodies C03-NFR, C03-REACH and the neutral probe
/// pinned as refused are what this card's rank admits. The neutral route still
/// refuses them first; the rank, consulted after it, publishes the effect, and
/// the census names that route. The effect is enforced at the caller: a
/// release after the recursive release is a double release, never `Safe`.
#[test]
fn the_effectful_neutral_pins_now_carry_a_rank() {
    if !clang_available() {
        return;
    }
    let walk =
        "static void walk(void *p, int n) { if (n > 0) { walk(p, n - 1); return; } q_free(p); }";
    let census = qcore_c_gateway::legacy_census::neutral_reach_census(
        &C11TranslationUnit {
            source: &format!(
                "{DECLARATIONS}\n{walk}\nvoid subject(void) {{ void *r = q_alloc(); walk(r, 3); }}\n"
            ),
            ..Default::default()
        },
        &contracts(),
        None,
    )
    .expect("the census runs");
    match census.components.as_slice() {
        [component] => assert!(
            matches!(
                &component.outcome,
                qcore_c_gateway::legacy_census::NeutralComponentOutcome::PublishedByRankedRecursion { members }
                    if members == &vec!["walk".to_owned()]
            ),
            "{component:?}"
        ),
        other => panic!("expected one component: {other:?}"),
    }
    assert_eq!(
        certified_summary(
            &format!("{walk} void subject(void) {{ void *r = q_alloc(); walk(r, 3); }}"),
            "walk"
        ),
        released_then_borrowed()
    );
    for (effect, released) in [("q_free(p)", true), ("q_transfer(p)", false)] {
        let depth = format!(
            "void depth(void *p, int n) {{ if (n > 0) {{ depth(p, n - 1); return; }} {effect}; }}"
        );
        let after = format!(
            "{depth} void subject(void) {{ void *p = q_alloc(); depth(p, 3); q_free(p); }}"
        );
        assert!(
            !matches!(unrestricted(&after), UnrestrictedLegacyResult::Safe(_)),
            "{effect}"
        );
        let summary = certified_summary(
            &format!("{depth} void subject(void) {{ void *p = q_alloc(); depth(p, 3); }}"),
            "depth",
        );
        assert_eq!(
            summary.parameters[0],
            if released {
                ResourceParameterMode::Released { nullable: false }
            } else {
                ResourceParameterMode::Consumed { nullable: false }
            },
            "{effect}"
        );
    }
}
