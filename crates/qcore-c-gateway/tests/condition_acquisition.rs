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

//! Card CG04-CONDITION-ACQUISITION (gateway rule 10x): ownership-producing
//! calls inside controlling expressions (gap entry E09, example A) and complex
//! condition leaves (E13), on route G.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`,
//! `r_*`, `k_*` and `cg4_*` surfaces. `Safe` here is `Safe` over synthetic
//! trusted test assumptions and promotes nothing. The statement pair each
//! condition is compared with (`p = f(); if (p …)`) is the flow's existing
//! reading, which the lowering reuses.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCfg, LegacyDiagnosticKind, LegacyInstruction,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t;
void *q_alloc(void); void q_free(void *p); void q_observe(void *p);
void *q_realloc(void *p, size_t n);
void *q_maybe_alloc(void); void q_nullable_free(void *p);
int r_borrow_code(void *p); int r_release_code(void *p);
int k_open(const char *path); int k_close(int fd);
int cg4_make(void **d); int opaque(void);
typedef void *handle_t;
";

const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";
const LOOP: &str = "unmodeled call, aggregate or indirect access in loop condition";
const NULLABLE: &str = "nullable resource `p` used by a non-null contract";

fn contracts() -> ResourceContracts {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    load_resource_contract_sources(&BTreeMap::from([
        file("q_base.qv", include_str!("fixtures/contracts/q_base.qv")),
        file(
            "q_rich_modes.qv",
            include_str!("fixtures/contracts/q_rich_modes.qv"),
        ),
        file(
            "q_realloc.qv",
            include_str!("fixtures/contracts/q_realloc.qv"),
        ),
        file(
            "q_nullable.qv",
            include_str!("fixtures/contracts/q_nullable.qv"),
        ),
        file(
            "r_return_call.qv",
            include_str!("fixtures/contracts/r_return_call.qv"),
        ),
        file(
            "descriptor_generic.qv",
            include_str!("fixtures/contracts/descriptor_generic.qv"),
        ),
        file(
            "cg04_condition.qv",
            include_str!("fixtures/contracts/cg04_condition.qv"),
        ),
    ]))
    .expect("the CG04 fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG04 test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    cfg: LegacyCfg,
}

impl Run {
    fn refused(&self, description: &str) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description: d } if d == description)
        })
    }

    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }

    /// The call nodes of `callee`, with their line.
    fn calls(&self, callee: &str) -> Vec<usize> {
        self.cfg
            .nodes
            .iter()
            .filter(|node| {
                matches!(&node.instruction, LegacyInstruction::ContractCall { function, .. } if function == callee)
            })
            .map(|node| node.line)
            .collect()
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("these fixtures project partially")
        }
    }
}

/// `body` is the function `subject(int flag, int n)`'s body, one statement per
/// line after the declarations; lines are numbered from the body's first.
fn analyze(body: &str) -> Run {
    analyze_returning("void", body)
}

/// As [`analyze`], for a `subject` returning `result`.
fn analyze_returning(result: &str, body: &str) -> Run {
    let source = format!("{DECLARATIONS}{result} subject(int flag, int n) {{\n{body}\n}}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome);
    let result = &summary.functions["subject"];
    Run {
        status: result.status,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
        cfg: result.cfg.clone(),
    }
}

/// The line of `subject`'s body line `offset` (0-based) in the analysed text.
fn body_line(offset: usize) -> usize {
    DECLARATIONS.lines().count() + 2 + offset
}

/// Example A and its variants: the acquisition, its binding, the split and
/// the merge lowered together decide exactly as the statement pair does.
#[test]
fn example_a_decides_as_the_statement_pair() {
    if !clang_available() {
        return;
    }
    for (condition_form, statement_form) in [
        (
            "void *p;\nif ((p = q_alloc()) != 0) { q_free(p); }",
            "void *p = q_alloc();\nif (p != 0) { q_free(p); }",
        ),
        (
            "void *p;\nif ((p = q_maybe_alloc()) != 0) { q_free(p); }",
            "void *p = q_maybe_alloc();\nif (p != 0) { q_free(p); }",
        ),
        (
            "void *p;\nif ((p = q_maybe_alloc()) == 0) { return; }\nq_free(p);",
            "void *p = q_maybe_alloc();\nif (p == 0) { return; }\nq_free(p);",
        ),
        (
            "void *p;\nif (!(p = q_maybe_alloc())) { return; }\nq_free(p);",
            "void *p = q_maybe_alloc();\nif (!p) { return; }\nq_free(p);",
        ),
        (
            "void *p;\nif ((p = q_maybe_alloc())) { q_free(p); }",
            "void *p = q_maybe_alloc();\nif (p) { q_free(p); }",
        ),
        (
            "handle_t h;\nif ((h = q_maybe_alloc()) != 0) { q_free(h); }",
            "handle_t h = q_maybe_alloc();\nif (h != 0) { q_free(h); }",
        ),
        // The descriptor sentinel split.
        (
            "int fd;\nif ((fd = k_open(\"x\")) >= 0) { k_close(fd); }",
            "int fd = k_open(\"x\");\nif (fd >= 0) { k_close(fd); }",
        ),
        (
            "int fd;\nif ((fd = k_open(\"x\")) < 0) { return; }\nk_close(fd);",
            "int fd = k_open(\"x\");\nif (fd < 0) { return; }\nk_close(fd);",
        ),
        // Reallocation: the failure arm keeps the old identity live.
        (
            "void *p = q_alloc(); void *q;\nif ((q = q_realloc(p, 4)) == 0) { q_free(p); return; }\nq_free(q);",
            "void *p = q_alloc(); void *q = q_realloc(p, 4);\nif (q == 0) { q_free(p); return; }\nq_free(q);",
        ),
    ] {
        let condition = analyze(condition_form);
        let statement = analyze(statement_form);
        assert!(condition.safe(), "{condition_form}: {condition:?}");
        assert_eq!(condition.status, statement.status, "{statement_form}");
        assert_eq!(condition.diagnostics, statement.diagnostics);
    }
}

/// No hoisting: the acquisition is a node of the condition's own plan, at the
/// condition's line; a forward `goto` creates no cycle.
#[test]
fn the_acquisition_is_never_hoisted() {
    if !clang_available() {
        return;
    }
    let run = analyze("void *p;\nif ((p = q_maybe_alloc()) != 0) { q_free(p); }");
    assert_eq!(run.calls("q_maybe_alloc"), vec![body_line(1)], "{run:?}");
    let run = analyze(
        "void *p;\nif ((p = q_maybe_alloc()) == 0) { goto out; }\nq_free(p);\nout: return;",
    );
    assert!(run.safe(), "{run:?}");
    assert_eq!(run.calls("q_maybe_alloc"), vec![body_line(1)], "{run:?}");
}

/// Flipped by `CG04-LOOP-SITE-GENERATIONS` (its second, separately measured
/// step): a split acquisition on a node a CFG cycle may pass through was
/// refused (`acquisition-on-cycle`) while the flow kept one abstract site per
/// acquisition node. The flow now keeps the earlier generation apart and the
/// refuted failure edge removes only the fresh one, so the guard is lifted:
/// each form is admitted and decides as its statement pair (the call applied
/// once, where the condition stands). A form that leaks every earlier
/// generation reports it.
#[test]
fn an_acquisition_on_a_cycle_is_split_with_generations() {
    if !clang_available() {
        return;
    }
    for (body, safe, callee) in [
        (
            "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_free(p); }",
            true,
            "q_maybe_alloc",
        ),
        (
            "void *p;\nwhile ((p = q_maybe_alloc()) != 0) { q_observe(p); }",
            false,
            "q_maybe_alloc",
        ),
        (
            "for (int i = 0; i < n; i++) {\nvoid *p;\nif ((p = q_maybe_alloc()) == 0) { break; }\nq_observe(p);\n}",
            false,
            "q_maybe_alloc",
        ),
        (
            "void *p;\nretry:\nif ((p = q_maybe_alloc()) == 0) { goto retry; }\nq_free(p);",
            true,
            "q_maybe_alloc",
        ),
        (
            "int fd;\nwhile ((fd = k_open(\"x\")) >= 0) { k_close(fd); }",
            true,
            "k_open",
        ),
    ] {
        let run = analyze(body);
        assert!(
            !run.refused(LOOP) && !run.refused(BRANCH),
            "{body}: {run:?}"
        );
        assert_eq!(run.safe(), safe, "{body}: {run:?}");
        if !safe {
            assert_eq!(run.status, AnalysisStatus::Unsafe, "{body}: {run:?}");
            assert_eq!(run.leaks(), 1, "{body}: {run:?}");
        }
        assert_eq!(run.calls(callee).len(), 1, "{body}: applied once: {run:?}");
    }
    // A resource-free result binds no site: its loop split is admitted as
    // before.
    let run = analyze(
        "void *p = q_alloc(); int c;\nwhile ((c = r_borrow_code(p)) != -1) { }\nq_free(p);",
    );
    assert!(run.safe(), "{run:?}");
}

/// The defect the placement guard kept this card clear of, in the statement
/// form an older route already admits: the second generation's failure edge
/// erased the live first one, and the leak was not reported (`Safe`, pinned
/// as observed at `the_statement_form_loop_site_defect_is_pinned`). Flipped
/// by `CG04-LOOP-SITE-GENERATIONS` into this regression: the earlier
/// generation is the previous one, the refuted failure edge removes only the
/// new one, and both forms report the leak every earlier generation is.
#[test]
fn the_statement_form_loop_site_defect_stays_fixed() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *p;\nfor (;;) { p = q_maybe_alloc(); if (p == 0) break; q_observe(p); }",
        "void *p;\ndo { p = q_maybe_alloc(); if (p) q_observe(p); } while (p != 0);",
        // With another exit the live generation was already reported.
        "void *p;\nwhile (flag) { p = q_maybe_alloc(); if (p == 0) return; q_observe(p); }",
    ] {
        let run = analyze(body);
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{body}: {run:?}");
        assert!(run.leaks() >= 1, "{body}: {run:?}");
    }
}

/// Refusal control: an acquisition in the right operand of `&&` happens only
/// on the paths that evaluate it.
#[test]
fn an_acquisition_in_the_right_operand_of_and_happens_only_where_evaluated() {
    if !clang_available() {
        return;
    }
    // Skipped when `flag` is false: nothing is acquired there, nothing leaks.
    let run = analyze("void *p;\nif (flag && (p = q_maybe_alloc()) != 0) { q_free(p); }");
    assert!(run.safe(), "{run:?}");
    // The release after the condition meets a `p` the skipped path never
    // bound: not Safe.
    let run = analyze("void *p;\nif (flag && (p = q_alloc()) != 0) { }\nq_free(p);");
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(NULLABLE), "{run:?}");
    // Acquired on the left and released only when the right holds: the
    // source leaks when `flag` is false, and the leak is reported.
    let run = analyze("void *p;\nif ((p = q_maybe_alloc()) != 0 && flag) { q_free(p); }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
    let run = analyze(
        "void *p;\nif ((p = q_maybe_alloc()) != 0 && flag) { q_free(p); } else if (p) { q_free(p); }",
    );
    assert!(run.safe(), "{run:?}");
}

/// Refusal control: a failed reallocation in a condition keeps the old
/// identity live, so a failure branch that drops it leaks (the source does).
#[test]
fn a_failed_realloc_in_a_condition_keeps_the_old_identity() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void *p = q_alloc(); void *q;\nif ((q = q_realloc(p, 4)) == 0) { return; }\nq_free(q);",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
    // `p = realloc(p, n)` loses the old identity on failure: a real leak.
    let run =
        analyze("void *p = q_alloc();\nif ((p = q_realloc(p, 4)) == 0) { return; }\nq_free(p);");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
}

/// Refusal control: a use after the condition that no branch guards meets a
/// possibly-null acquisition and is not Safe.
#[test]
fn an_unguarded_use_after_the_condition_is_not_safe() {
    if !clang_available() {
        return;
    }
    let run = analyze("void *p;\nif ((p = q_maybe_alloc()) == 0) { }\nq_observe(p);\nq_free(p);");
    assert!(!run.safe(), "{run:?}");
    assert!(run.refused(NULLABLE), "{run:?}");
    // An acquisition no branch releases leaks on the success edge.
    let run = analyze("void *p;\nif ((p = q_alloc()) != 0) { }");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
}

/// Shapes this card keeps refused, each with the position's refusal.
#[test]
fn unlisted_acquisition_shapes_keep_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        // A discarded acquisition: nothing binds it.
        "if (q_alloc()) { }",
        "if (q_alloc() != 0) { }",
        // Bound but not split on its failure value.
        "void *p; void *q = 0;\nif ((p = q_alloc()) == q) { q_free(p); }",
        // A descriptor tested as a pointer is no validity split.
        "int fd;\nif ((fd = k_open(\"x\"))) { k_close(fd); }",
        // A formal destination.
        "if ((n = r_borrow_code(0)) != 0) { }",
    ] {
        let run = analyze(body);
        assert!(!run.safe(), "{body}: {run:?}");
    }
    // An out-parameter acquisition with a status result: no contract can
    // relate the status to the slot (CG23), and this card adds no relation.
    // Correction to the brief's premise: rule 5h already admits the call (its
    // result owns nothing), with the contract's unconditional `Out` on both
    // edges, so the failure edge reports the leak the contract implies.
    let run = analyze("void *p;\nif (cg4_make(&p) == 0) { q_free(p); }");
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
    // The scalar `return` operand reads the same leaf (rule 10u); its split is
    // no custody-neutral exit, so it keeps the return's refusal.
    let run = analyze_returning("int", "void *p;\nreturn (p = q_maybe_alloc()) != 0;");
    assert!(!run.safe(), "{run:?}");
    assert!(
        run.refused("unmodeled call, aggregate or indirect access in return expression"),
        "{run:?}"
    );
}

/// E13: a type operator and an increment are decided per leaf.
#[test]
fn complex_leaves_compose_per_leaf() {
    if !clang_available() {
        return;
    }
    for condition in [
        "r_borrow_code(p) == sizeof(int)",
        "n < sizeof(int) && r_borrow_code(p)",
        "i++ < 3 && r_borrow_code(p)",
        "r_borrow_code(p) && i-- > 0",
        "i-- > 0 && flag",
    ] {
        let run = analyze(&format!(
            "void *p = q_alloc(); int i = n;\nif ({condition}) {{ }}\nq_free(p);"
        ));
        assert!(run.safe(), "{condition}: {run:?}");
    }
    for condition in [
        // A call inside a type operand is not evaluated and never applied.
        "n < sizeof(r_borrow_code(p)) && flag",
        "r_borrow_code(p) == sizeof(r_borrow_code(p))",
        // An increment beside a call in one leaf: unsequenced (CG05).
        "r_borrow_code(p) == i++",
        // A connective below a leaf holding a call (CG06).
        "(flag && r_borrow_code(p)) == 0",
    ] {
        let run = analyze(&format!(
            "void *p = q_alloc(); int i = n;\nif ({condition}) {{ }}\nq_free(p);"
        ));
        assert!(run.refused(BRANCH), "{condition}: {run:?}");
        assert!(!run.safe(), "{condition}");
    }
    // The release in a skipped operand is not applied: `i-- > 0` false
    // leaves `p` live for the leak check.
    let run = analyze("void *p = q_alloc(); int i = n;\nif (i-- > 0 && r_release_code(p)) { }");
    assert!(!run.safe(), "{run:?}");
}
