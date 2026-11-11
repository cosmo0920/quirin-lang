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

//! Card W44-BRANCH-CONDITION-CALLS — calls in controlling expressions,
//! desugared into their own nodes and wired by the short-circuit edges.
//!
//! Every fixture runs the real Clang-backed gateway over the synthetic `q_*`
//! and `r_*` surfaces (`fixtures/contracts/q_base.qcore`,
//! `fixtures/contracts/r_return_call.qcore`) and, for descriptors, the generic
//! `k_*` surface. `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCfg, LegacyDiagnosticKind, LegacyInstruction,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p); \
void *r_wrap(void *p); int opaque(void);";

const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";
const LOOP: &str = "unmodeled call, aggregate or indirect access in loop condition";
const FOR: &str = "unmodeled call, aggregate or indirect access in for-loop condition";
const DO_WHILE: &str = "unmodeled call, aggregate or indirect access in do-while condition";
const SWITCH: &str = "unmodeled call, aggregate or indirect access in switch expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "r_return_call.qcore".to_owned(),
            include_str!("fixtures/contracts/r_return_call.qv").to_owned(),
        ),
    ]))
    .unwrap()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W44 condition test: {reason}");
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

    fn double_releases(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. }))
            .count()
    }

    fn calls(&self, callee: &str) -> usize {
        self.cfg
            .nodes
            .iter()
            .filter(|node| {
                matches!(&node.instruction, LegacyInstruction::ContractCall { function, .. } if function == callee)
            })
            .count()
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

fn run_with(source: &str, function: &str, contracts: &ResourceContracts) -> Run {
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        function,
        contracts,
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome);
    let result = &summary.functions[function];
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

fn analyze(body: &str) -> Run {
    run_with(
        &format!("{DECLARATIONS}\n{body}\n"),
        "subject",
        &contracts(),
    )
}

#[test]
fn both_operands_of_a_conjunction_are_modeled_and_certify() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            if (r_borrow_code(p) == 0 && r_borrow_code(p) == 1) { }
            q_free(p);
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(!run.refused(BRANCH));
    assert_eq!(run.calls("r_borrow_code"), 2);
}

#[test]
fn a_short_circuited_release_is_applied_only_where_c_evaluates_it() {
    if !clang_available() {
        return;
    }
    // `flag` false skips the release, so the resource is still live at exit
    // on that path: the leak is reported, never hidden.
    let run =
        analyze("void subject(int flag) { void *p = q_alloc(); if (flag && (q_free(p), 1)) { } }");
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Unsafe);
    assert_eq!(run.leaks(), 1);
    // And the evaluated path releases it: a second release in the arm is a
    // double release, not a missed one.
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag && (q_free(p), 1)) { q_free(p); } else { q_free(p); } }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(run.double_releases() > 0);
}

#[test]
fn a_disjunction_evaluates_its_right_operand_on_the_false_edge_only() {
    if !clang_available() {
        return;
    }
    // Left true: the release is skipped and the arm releases once. Left
    // false: the release runs, and if the condition is then true the arm
    // releases again. Had the release been applied on the left's true edge
    // too, that arm would double-release as well; had it been dropped, the
    // left-false/right-false path would leak.
    let leak_free = analyze(
        "void subject(void) {
            void *p = q_alloc();
            if (r_borrow_code(p) || r_release_code(p)) { } else { }
            q_free(p);
        }",
    );
    assert!(!leak_free.refused(BRANCH));
    assert!(leak_free.double_releases() > 0, "{leak_free:?}");
    assert_eq!(leak_free.leaks(), 0, "{leak_free:?}");
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            if (r_borrow_code(p) || r_release_code(p)) { q_free(p); }
        }",
    );
    assert!(!run.refused(BRANCH));
    assert_ne!(run.status, AnalysisStatus::Safe);
    assert!(run.double_releases() > 0, "{run:?}");
}

#[test]
fn a_conditional_evaluates_exactly_one_arm() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag ? r_release_code(p) : r_consume_code(p)) { } }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag ? r_release_code(p) : r_borrow_code(p)) { } }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
}

#[test]
fn a_negation_swaps_the_exits_only() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) {
            void *p = q_alloc();
            if (!(r_borrow_code(p) && flag)) { q_free(p); } else { q_free(p); }
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn builtin_expect_is_read_through() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) {
            void *p = q_alloc();
            if (__builtin_expect(!!(flag && r_borrow_code(p) == 0), 0)) { }
            q_free(p);
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn an_assigned_scalar_result_is_the_c_programs_own_temporary() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            int c;
            while ((c = r_borrow_code(p)) != -1) { }
            q_free(p);
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(run.cfg.nodes.iter().any(|node| matches!(
        &node.instruction,
        LegacyInstruction::ContractCall { destination: Some(d), .. } if d == "c"
    )));
    // A stale fact about `c` does not survive the call: after `c = 0` the
    // assigned call makes both arms reachable.
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            int c = 0;
            if ((c = r_borrow_code(p)) == 0) { q_free(p); }
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
}

/// Q47 of the Clang gap analysis. **Flipped by CG04-CONDITION-ACQUISITION
/// (rule 10x):** an owned result assigned in a condition and split on its
/// failure value is lowered where it stands (the acquisition, its binding to
/// `p`, the null split and the merge together), so the function is decided as
/// the statement pair `p = q_alloc(); if (p == 0) …` is. The regression half
/// stands: the call is never hoisted out of the condition (it is one node of
/// the condition's plan), and a discarded owned result keeps its refusal.
#[test]
fn an_owned_result_assigned_in_a_condition_keeps_todays_exact_lowering() {
    if !clang_available() {
        return;
    }
    let run =
        analyze("void subject(void) { void *p; if ((p = q_alloc()) == 0) { return; } q_free(p); }");
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // One node applies the acquisition, in the condition's plan: nothing
    // was hoisted or duplicated.
    assert_eq!(run.calls("q_alloc"), 1);
    assert!(run.cfg.nodes.iter().any(|node| matches!(
        &node.instruction,
        LegacyInstruction::BranchOnNull { source, true_is_null: true } if source == "p"
    )));
    // A discarded owned result stays refused too.
    let run = analyze(
        "void subject(void) { void *p = q_alloc(); if (r_wrap(p) != 0 && p) { } q_free(p); }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag && r_wrap(p)) { } q_free(p); }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
}

#[test]
fn an_uncontracted_call_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag && opaque()) { } q_free(p); }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Unknown);
}

#[test]
fn two_effectful_calls_of_one_operand_are_unsequenced_and_refused() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(void) { void *p = q_alloc(); if (r_release_code(p) == r_borrow_code(p)) { } }",
    );
    assert!(run.refused(BRANCH), "{run:?}");
    assert_eq!(run.calls("r_release_code"), 0);
}

#[test]
fn shapes_outside_the_desugaring_keep_the_refusal() {
    if !clang_available() {
        return;
    }
    for condition in [
        // A connective below a leaf.
        "(flag && r_borrow_code(p)) == 0",
        // A call inside a type operator's operand (C evaluates none of it).
        "flag && sizeof(r_borrow_code(p))",
        // A write beside a call in one leaf. (CG04-CONDITION-ACQUISITION
        // flipped the former fixture `flag++ && r_borrow_code(p)`: the write
        // in its own leaf is the elaborator's, sequenced by `&&`; see
        // `a_write_in_its_own_leaf_is_sequenced_by_the_connective`.)
        "r_borrow_code(p) == flag++",
        // GNU elvis.
        "r_borrow_code(p) ?: flag",
    ] {
        let run = analyze(&format!(
            "void subject(int flag) {{ void *p = q_alloc(); if ({condition}) {{ }} q_free(p); }}"
        ));
        assert!(run.refused(BRANCH), "{condition}: {run:?}");
        assert_ne!(run.status, AnalysisStatus::Safe, "{condition}");
    }
}

/// Flipped by CG04-CONDITION-ACQUISITION (rule 10x): a write in a call-free
/// leaf of its own is decided by the elaborator (rule 10s), sequenced before
/// the right operand by `&&`.
#[test]
fn a_write_in_its_own_leaf_is_sequenced_by_the_connective() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); if (flag++ && r_borrow_code(p)) { } q_free(p); }",
    );
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn a_validated_local_summary_is_consumed_and_a_recursive_callee_is_not() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "static int helper(void *q) { return 0; }
        void subject(int flag) {
            void *p = q_alloc();
            if (flag && helper(p) == 0) { }
            q_free(p);
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert_eq!(run.calls("helper"), 1);
    // `walk` calls itself in its own condition: its contract while its
    // component is validated may be a provisional assumption.
    let source = format!(
        "{DECLARATIONS}\nint walk(int n) {{ if (n > 0 && walk(n - 1) == 0) {{ return 1; }} return 0; }}\n"
    );
    let run = run_with(&source, "walk", &contracts());
    assert!(run.refused(BRANCH), "{run:?}");
}

#[test]
fn loop_conditions_are_desugared_and_re_evaluated_on_every_visit() {
    if !clang_available() {
        return;
    }
    for (header, refusal) in [
        ("while (flag && r_release_code(p)) { }", LOOP),
        ("for (; flag && r_release_code(p); ) { }", FOR),
        ("do { } while (flag && r_release_code(p));", DO_WHILE),
    ] {
        let run = analyze(&format!(
            "void subject(int flag) {{ void *p = q_alloc(); {header} }}"
        ));
        assert!(!run.refused(refusal), "{header}: {run:?}");
        assert_ne!(run.status, AnalysisStatus::Safe, "{header}");
        // The back edge re-enters the plan: a second visit releases again.
        assert!(run.double_releases() > 0, "{header}: {run:?}");
    }
    for header in [
        "while (flag && r_borrow_code(p)) { }",
        "for (; r_borrow_code(p) || flag; ) { }",
        "do { } while (r_borrow_code(p) == 0 && flag);",
    ] {
        let run = analyze(&format!(
            "void subject(int flag) {{ void *p = q_alloc(); {header} q_free(p); }}"
        ));
        assert_eq!(run.status, AnalysisStatus::Safe, "{header}: {run:?}");
    }
}

#[test]
fn every_loop_exit_edge_is_recorded() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(int flag) { void *p = q_alloc(); while (r_borrow_code(p) && flag) { } q_free(p); }",
    );
    // Both decision nodes of `A && B` leave the loop on their false edge.
    let exits = run
        .cfg
        .nodes
        .iter()
        .filter(|node| node.loop_exit_edge.is_some())
        .count();
    assert_eq!(exits, 2, "{run:?}");
}

#[test]
fn a_switch_runs_its_identity_calls_once_and_refuses_unsequenced_effects() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            switch (r_borrow_code(p) + r_borrow_code(p)) { case 0: break; default: break; }
            q_free(p);
        }",
    );
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(!run.refused(SWITCH));
    let run = analyze(
        "void subject(void) {
            void *p = q_alloc();
            switch (r_release_code(p) + r_borrow_code(p)) { case 0: break; default: break; }
        }",
    );
    assert!(run.refused(SWITCH), "{run:?}");
}

const DESCRIPTOR_DECLARATIONS: &str = "\
int k_open(const char *path); \
int k_close(int fd); \
long k_read(int fd, void *buffer, unsigned long size);";

fn descriptor_contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "descriptor_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
    )]))
    .unwrap()
}

#[test]
fn a_descriptor_keeps_the_sentinel_validity_rules() {
    if !clang_available() {
        return;
    }
    // The validity leaf refines the descriptor; the close runs only on its
    // valid edge.
    let source = format!(
        "{DESCRIPTOR_DECLARATIONS}\nvoid w(const char *path) {{ int fd = k_open(path); if (fd >= 0 && k_close(fd) == 0) {{ }} }}\n"
    );
    let run = run_with(&source, "w", &descriptor_contracts());
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // Without the validity leaf, a valid descriptor may reach the exit open.
    let source = format!(
        "{DESCRIPTOR_DECLARATIONS}\nvoid w(const char *path, int flag) {{ int fd = k_open(path); if (flag && k_close(fd) == 0) {{ }} }}\n"
    );
    let run = run_with(&source, "w", &descriptor_contracts());
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    // Flipped by CG04-CONDITION-ACQUISITION (rule 10x): an owned descriptor
    // result assigned in the condition and split on its sentinel is lowered
    // where it stands, as the statement pair `fd = k_open(path); if (fd < 0)`.
    let source = format!(
        "{DESCRIPTOR_DECLARATIONS}\nvoid w(const char *path) {{ int fd; if ((fd = k_open(path)) < 0) {{ return; }} k_close(fd); }}\n"
    );
    let run = run_with(&source, "w", &descriptor_contracts());
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    // A comparison that refines nothing (`== 3`) is the statement pair's
    // scalar decision: the early return leaks the descriptor, as the source
    // does, and the close meets a possibly invalid one.
    let source = format!(
        "{DESCRIPTOR_DECLARATIONS}\nvoid w(const char *path) {{ int fd; if ((fd = k_open(path)) == 3) {{ return; }} k_close(fd); }}\n"
    );
    let run = run_with(&source, "w", &descriptor_contracts());
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert_eq!(run.leaks(), 1);
    // A descriptor read as data beside a call stays refused.
    let source = format!(
        "{DESCRIPTOR_DECLARATIONS}\nvoid w(const char *path, char *b, int flag) {{ int fd = k_open(path); if (flag && k_read(fd, b, 1) > fd) {{ }} if (fd >= 0) {{ k_close(fd); }} }}\n"
    );
    let run = run_with(&source, "w", &descriptor_contracts());
    assert!(run.refused(BRANCH), "{run:?}");
}

const DERIVED_DECLARATIONS: &str = "\
char *d_alloc(__SIZE_TYPE__ n); void d_free(void *p); \
__SIZE_TYPE__ d_len(const char *s); char *d_chr(const char *s, int c);";

fn derived_contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "derived_result_generic.qcore".to_owned(),
        include_str!("fixtures/contracts/derived_result_generic.qv").to_owned(),
    )]))
    .unwrap()
}

#[test]
fn a_derived_result_assigned_in_a_condition_is_tracked_by_its_binding() {
    if !clang_available() {
        return;
    }
    // The program's own pointer carries the derived result; the second
    // operand reads it only on the edge where it is non-null.
    let source = format!(
        "{DERIVED_DECLARATIONS}\nvoid w(const char *s) {{ char *p; if ((p = d_chr(s, 58)) && d_len(p) > 1) {{ }} }}\n"
    );
    let run = run_with(&source, "w", &derived_contracts());
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(!run.refused(BRANCH));
    // Released base: the derived pointer read in the second operand is
    // reported, not hidden by the desugaring.
    let source = format!(
        "{DERIVED_DECLARATIONS}\nvoid w(void) {{ char *b = d_alloc(8); char *p; p = d_chr(b, 58); d_free(b); if (p && d_len(p) > 1) {{ }} }}\n"
    );
    let run = run_with(&source, "w", &derived_contracts());
    assert!(!run.refused(BRANCH), "{run:?}");
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
}

#[test]
fn batch4_call_free_member_predicate_composes_checked_reads() {
    if !clang_available() {
        return;
    }
    for condition in [
        "(cf->fd != -1) && (cf->map != (void *)0)",
        "(cf->fd != -1) || (cf->map != (void *)0)",
        "!(cf->fd != -1) || (cf->map != (void *)0)",
        "flag ? (cf->fd != -1) : (cf->map != (void *)0)",
    ] {
        let source = format!(
            "struct file {{ int fd; void *map; }}; int w(struct file *cf, int flag) {{ if ({condition}) return 1; return 0; }}"
        );
        let run = run_with(&source, "w", &ResourceContracts::default());
        assert_eq!(run.status, AnalysisStatus::Safe, "{condition}: {run:?}");
        assert!(run.diagnostics.is_empty(), "{run:?}");
        assert!(
            run.cfg.nodes.iter().any(|n| matches!(
                n.instruction,
                LegacyInstruction::IndirectScalarAccess { .. }
            )),
            "{run:?}"
        );
    }
}

#[test]
fn batch4_composed_reads_preserve_leaks_and_double_release() {
    if !clang_available() {
        return;
    }
    for (before, after, double) in [
        ("void *p = q_alloc();", "", false),
        ("void *p = q_alloc();", "q_free(p); q_free(p);", true),
    ] {
        let run = analyze(&format!(
            "struct file {{ int fd; void *map; }}; void subject(struct file *cf) {{ {before} if ((cf->fd != -1) && (cf->map != (void *)0)) {{ }} {after} }}"
        ));
        assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
        assert!(!run.refused(BRANCH), "{run:?}");
        assert!(
            if double {
                run.double_releases() > 0
            } else {
                run.leaks() > 0
            },
            "{run:?}"
        );
    }
}

#[test]
fn batch4_call_free_predicates_keep_unknown_leaves_refused() {
    if !clang_available() {
        return;
    }
    for (record, condition) in [
        (
            "struct file { volatile int fd; void *map; };",
            "(cf->fd != -1) && (cf->map != (void *)0)",
        ),
        (
            "struct file { _Atomic int fd; void *map; };",
            "(cf->fd != -1) && (cf->map != (void *)0)",
        ),
        (
            "struct file { int fd; void *map; };",
            "(cf->fd != -1) && opaque()",
        ),
        (
            "struct file { int fd; void *map; };",
            "(cf->fd != -1) && (cf->map = (void *)0)",
        ),
    ] {
        let tag = if record.starts_with("union") {
            "union"
        } else {
            "struct"
        };
        let source = format!(
            "{record} int opaque(void); int w({tag} file *cf) {{ if ({condition}) return 1; return 0; }}"
        );
        let run = run_with(&source, "w", &ResourceContracts::default());
        assert_ne!(run.status, AnalysisStatus::Safe, "{condition}: {run:?}");
    }
}

#[test]
fn batch4_loop_predicates_recheck_reads_and_keep_exit_paths() {
    if !clang_available() {
        return;
    }
    for control in [
        "while ((cf->fd != -1) && (cf->map != (void *)0)) { break; }",
        "for (; (cf->fd != -1) && (cf->map != (void *)0); ) { break; }",
        "do { } while ((cf->fd != -1) && (cf->map != (void *)0));",
    ] {
        let source = format!(
            "struct file {{ int fd; void *map; }}; void w(struct file *cf) {{ {control} }}"
        );
        let run = run_with(&source, "w", &ResourceContracts::default());
        assert_eq!(run.status, AnalysisStatus::Safe, "{control}: {run:?}");
        assert!(run.diagnostics.is_empty(), "{run:?}");
    }
}

#[test]
fn batch4_a_released_predicate_base_stays_non_safe() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "struct file { int fd; void *map; }; void subject(void) { struct file *cf = q_alloc(); q_free(cf); if ((cf->fd != -1) && (cf->map != (void *)0)) { } }",
    );
    assert_ne!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(
        run.diagnostics
            .iter()
            .any(|d| matches!(d, LegacyDiagnosticKind::UseAfterRelease { .. })),
        "{run:?}"
    );
}

#[test]
fn batch4_literal_predicate_does_not_erase_read_obligations() {
    if !clang_available() {
        return;
    }
    let run = analyze(
        "struct file { int fd; void *map; }; void subject(void) { struct file *cf = q_alloc(); q_free(cf); if (0 && ((cf->fd != -1) && (cf->map != (void *)0))) { } }",
    );
    assert_eq!(run.status, AnalysisStatus::Unsafe, "{run:?}");
    assert!(
        run.diagnostics
            .iter()
            .any(|d| matches!(d, LegacyDiagnosticKind::UseAfterRelease { .. })),
        "{run:?}"
    );
}
