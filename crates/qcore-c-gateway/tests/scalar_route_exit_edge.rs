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

//! Card `SCALAR-ROUTE-EXIT-EDGE`: a loop header's exit edge is never removed
//! by the value route, end to end through the real gateway.
//!
//! `refine_pointer_null` answers `PointerRefinement::Infeasible` for a branch
//! edge whose null assumption contradicts the abstract state, and the custody
//! work list then does not propagate that successor — the only place in
//! `legacy_c.rs` where a successor is dropped. For an `if`, both arms rejoin at
//! the same continuation, so pruning one orphans nothing. For a **loop
//! header** the two successors do not rejoin: everything after the loop hangs
//! off the exit edge alone, so deleting it leaves the function's exit node with
//! no incoming state, `report_leaks` never runs there, and
//! `classify_diagnostics` returns `Safe`.
//!
//! Card `C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) pinned this route
//! "conservative on all four probed shapes" — but all four spell `while (flag)`
//! positively, so only the **back** edge was ever measured. Inverting the
//! polarity (`while (!flag)`, `while (flag == 0)`) makes the same arm delete
//! the exit edge; the second arm (a single known-non-null target) does it for
//! `while (p)`.
//!
//! Every test below places a live acquisition, a double release or a release
//! after escape **after** the loop and asserts on that diagnostic — not on the
//! `double_releases()` proxy the wave-35 row used, which a deleted exit edge
//! does not move.
//!
//! The rule is a refusal, so the tests are written in both directions: the
//! withdrawing half measures that the false `Safe` is gone, the accusing half
//! that the back-edge arm, the `if` arms and every unrelated shape answer
//! exactly as before. The theorems are `QCore.ScalarRouteExitEdge`.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal a retained exit edge reports. Pinned by whole-string equality so
/// a reworded message is a deliberate edit rather than a silent one.
const RETAINED: &str = "loop exit edge retained: a modeled value cannot remove it";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); \
int z_unknown(int n); void z_void(void);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[(
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        )]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
}

impl Run {
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

    fn retained_exit_edges(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| {
                matches!(kind, LegacyDiagnosticKind::Unsupported { description }
                    if description == RETAINED)
            })
            .count()
    }

    fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = self
            .diagnostics
            .iter()
            .map(|kind| format!("{kind:?}"))
            .collect();
        kinds.sort();
        kinds
    }
}

fn run(body: &str) -> Run {
    let source = format!("{DECLARATIONS}\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary.clone(),
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.expect("these fixtures always project partially")
        }
    };
    let function = summary
        .functions
        .get("subject")
        .expect("the entry is projected");
    Run {
        status: function.status,
        diagnostics: function
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    }
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

// ---------------------------------------------------------------------------
// The withdrawing half: the false `Safe` is gone.
// ---------------------------------------------------------------------------

/// The finding itself, in both inverted spellings and in all three loop kinds.
/// `int flag = 0;` makes `flag` a `Clear`-derived known null; an inverted
/// header then calls the **exit** edge infeasible. The acquisition after the
/// loop is never released, and before this card every row answered `Safe` with
/// no diagnostic at all.
#[test]
fn an_inverted_loop_header_does_not_delete_its_exit_edge() {
    if !clang_available() {
        return;
    }
    for header in [
        "while (!flag) { }",
        "while (flag == 0) { }",
        "while (0 == flag) { }",
        "while (!(flag)) { }",
        "do { } while (!flag);",
        "for (; !flag; ) { }",
    ] {
        let run = run(&format!(
            "void subject(void) {{ int flag = 0; (void)flag; void *p = q_alloc(); \
             {header} (void)p; }}"
        ));
        assert_eq!(
            run.retained_exit_edges(),
            1,
            "`{header}` must retain its exit edge and say so: {:?}",
            run.diagnostics
        );
        assert_ne!(
            run.status,
            AnalysisStatus::Safe,
            "`{header}` must not be Safe with a live acquisition after it: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.leaks(),
            1,
            "the acquisition after `{header}` is never released: {:?}",
            run.diagnostics
        );
    }
}

/// The second arm: a single **known non-null** target. `while (p)` asks the
/// same question of the other side of the lattice, and `p` is a real pointer
/// rather than a scalar, so this arm is not reached by any reading of the
/// finding that treats it as only about `int` bindings.
#[test]
fn the_known_non_null_arm_does_not_delete_its_exit_edge() {
    if !clang_available() {
        return;
    }
    for header in ["while (p) { }", "while (p != 0) { }", "while (0 != p) { }"] {
        let run = run(&format!(
            "void subject(void) {{ void *p = q_alloc(); if (!p) return; \
             void *r = q_alloc(); {header} (void)r; }}"
        ));
        assert_eq!(
            run.retained_exit_edges(),
            1,
            "`{header}` must retain its exit edge: {:?}",
            run.diagnostics
        );
        assert_ne!(
            run.status,
            AnalysisStatus::Safe,
            "`{header}` must not be Safe: {:?}",
            run.diagnostics
        );
    }
}

/// A deleted exit edge suppresses **every** downstream diagnostic, not only a
/// leak: the exit node is never visited, so nothing after the loop is judged at
/// all. This is the accusing-direction consequence and the reason the rule is a
/// soundness repair rather than a precision preference.
#[test]
fn a_retained_exit_edge_still_reports_a_double_release_after_the_loop() {
    if !clang_available() {
        return;
    }
    let run = run(
        "void subject(void) { int flag = 0; (void)flag; void *p = q_alloc(); \
         while (!flag) { } q_free(p); q_free(p); }",
    );
    assert!(
        run.double_releases() >= 1,
        "the two releases after the loop are still a double release: {:?}",
        run.diagnostics
    );
    assert_ne!(run.status, AnalysisStatus::Safe);
}

/// The soundness case, and the one that makes this a repair rather than a
/// tightening. A `volatile` object may be modified by means outside the
/// program (C11 §5.1.2.3), so `while (!flag) { }` on a volatile `flag`
/// **terminates** in perfectly well-defined C — and the code after it runs. The
/// abstract state models no such modification, so the value route's must-fact
/// is simply false here, and before this card the function was `Safe`.
///
/// The gateway does not read the `volatile` qualifier; the point is that it
/// does not have to, because the rule no longer rests on the absence of one.
#[test]
fn a_volatile_flag_loop_is_not_safe() {
    if !clang_available() {
        return;
    }
    for declaration in ["volatile int flag = 0;", "static volatile int flag = 0;"] {
        let run = run(&format!(
            "void subject(void) {{ {declaration} (void)flag; void *p = q_alloc(); \
             while (!flag) {{ }} (void)p; }}"
        ));
        assert_ne!(
            run.status,
            AnalysisStatus::Safe,
            "`{declaration}` with `while (!flag)` must not be Safe: {:?}",
            run.diagnostics
        );
    }
}

// ---------------------------------------------------------------------------
// The accusing half: nothing else moved.
// ---------------------------------------------------------------------------

/// The body-write fixed point, which is the arm the rule's old correctness
/// argument rested on and which no test drove. A body that writes the deciding
/// scalar reaches the header a second time with the name **absent** from the
/// binding environment (`ForgetScalar` removes it), and `AbstractState::join`'s
/// `(Some, None)` arm turns that into `unknown`, so the header takes
/// `PointerRefinement::Unknown` and propagates both edges. That arm is
/// unchanged by this card and is pinned here so a future edit to `join` cannot
/// quietly make it `Infeasible` again.
#[test]
fn a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm() {
    if !clang_available() {
        return;
    }
    // Deliberate pin update, card W43-INDEX-TRAVERSAL. `flag` is a declared
    // scalar, and a declared scalar's fact joined with no fact is now no fact
    // (the index-fact join), rather than the unknown, possibly-null pointer
    // the generic arm made of it. The header then sees no binding at all,
    // which refines nothing and propagates both edges exactly as the Unknown
    // arm did, without an unresolved alias. A body whose write the projection
    // refuses (`z_unknown`) still poisons the binding, so it still reaches
    // the Unknown arm. Either way nothing is Infeasible, nothing is retained,
    // and the exit is reached.
    for (body, unknown_arm) in [
        ("flag = 1;", false),
        ("flag++;", false),
        ("flag = z_unknown(1);", true),
        ("flag = -1;", false),
    ] {
        let run = run(&format!(
            "void subject(void) {{ int flag = 0; (void)flag; void *p = q_alloc(); \
             while (!flag) {{ {body} }} (void)p; }}"
        ));
        assert_eq!(
            run.diagnostics.iter().any(|kind| matches!(
                kind,
                LegacyDiagnosticKind::UnknownAlias { variable } if variable == "flag"
            )),
            unknown_arm,
            "`{body}`: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.retained_exit_edges(),
            0,
            "`{body}` never reaches the Infeasible arm, so nothing is retained: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.leaks(),
            1,
            "the acquisition after the loop is still reported: {:?}",
            run.diagnostics
        );
    }
}

/// The sibling **back** edge is deliberately untouched
/// (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`). These are card
/// `C-GATEWAY-CONSTANT-DO-WHILE`'s own four probed rows, which must answer
/// exactly as they did before this card.
#[test]
fn the_back_edge_arm_is_unchanged() {
    if !clang_available() {
        return;
    }
    for (prelude, keeps_back_edge) in [
        ("int flag = 0;", false),
        ("int flag = 1;", true),
        ("int flag = 0; flag = 1;", true),
        ("int flag = z_unknown(1);", true),
    ] {
        let run = run(&format!(
            "void subject(void) {{ {prelude} (void)flag; void *p = q_alloc(); \
             do {{ q_free(p); }} while (flag); }}"
        ));
        assert_eq!(
            run.double_releases() >= 1,
            keeps_back_edge,
            "`{prelude}` then `while (flag)`: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.retained_exit_edges(),
            0,
            "a positively spelled header prunes its BODY edge, not its exit: {:?}",
            run.diagnostics
        );
    }
}

/// An `if` is not a loop header. Its two arms rejoin at the same continuation,
/// so pruning one orphans nothing and the rule must not fire — otherwise every
/// `if (p)` after an acquisition would lose its path sensitivity, which is the
/// precision the corpus rests on.
#[test]
fn a_branch_keeps_its_ordinary_path_sensitivity() {
    if !clang_available() {
        return;
    }
    for body in [
        "int flag = 0; (void)flag; void *p = q_alloc(); \
         if (flag) { q_free(p); q_free(p); } q_free(p);",
        "int flag = 0; (void)flag; void *p = q_alloc(); \
         if (!flag) { q_free(p); } else { q_free(p); q_free(p); }",
        "void *p = q_alloc(); if (!p) return; if (p) { q_free(p); } \
         else { q_free(p); q_free(p); }",
    ] {
        let run = run(&format!("void subject(void) {{ {body} }}"));
        assert_eq!(
            run.retained_exit_edges(),
            0,
            "a branch has no loop exit edge to retain: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.status,
            AnalysisStatus::Safe,
            "the pruned arm's diagnostics are still pruned: {:?}",
            run.diagnostics
        );
    }
}

/// `for (;;)` carries **no** exit successor at all — it is removed at CFG
/// construction from the absent condition, not by the value route — so this
/// card does not reach it. It is the separate unowned neighbour
/// `FOREVER-FOR-EXIT-EDGE-UNOWNED`, recorded by `STABILITY-REEVAL-2` and
/// deliberately left as it is; this row pins that the card changed nothing
/// about it.
#[test]
fn a_forever_for_is_untouched_by_this_card() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { void *p = q_alloc(); for (;;) { } (void)p; }");
    assert_eq!(
        run.retained_exit_edges(),
        0,
        "`for (;;)` has no exit successor for this rule to retain: {:?}",
        run.diagnostics
    );
    assert_eq!(run.status, AnalysisStatus::Safe);
}

/// The zero spellings, the exact sibling of `CONSTANT-DO-WHILE-ZERO-SPELLINGS`.
/// `is_null_constant` matches `NULL`, `nullptr`, `0` and `(void *)0` only, so
/// `int flag = 0x0;` binds no known null and the value route never reaches
/// either arm. The rows are pinned **in the direction of conservatism**: they
/// keep every edge, so widening `is_null_constant` would create new exit-edge
/// deletions on this route — which is now harmless, because this card refuses
/// them whatever the binding says.
#[test]
fn the_other_zero_spellings_reach_neither_arm() {
    if !clang_available() {
        return;
    }
    for spelling in ["0x0", "0u", "0L", "00", "'\\0'", "1 - 1"] {
        let run = run(&format!(
            "void subject(void) {{ int flag = {spelling}; (void)flag; void *p = q_alloc(); \
             while (!flag) {{ }} (void)p; }}"
        ));
        assert_eq!(
            run.retained_exit_edges(),
            0,
            "`int flag = {spelling};` binds no known null: {:?}",
            run.diagnostics
        );
        assert_eq!(
            run.leaks(),
            1,
            "`int flag = {spelling};` keeps both edges, so the exit reports: {:?}",
            run.diagnostics
        );
    }
}

/// A `break` already gave the continuation an incoming edge, so the exit was
/// reachable even with the header's edge deleted. Those programs must answer
/// exactly as they did: the rule adds an edge that was already there, and a
/// join with an identical state changes nothing.
#[test]
fn a_break_in_the_body_answers_as_before() {
    if !clang_available() {
        return;
    }
    for body in ["break;", "if (z_unknown(1)) break;", "goto out;"] {
        let tail = if body == "goto out;" { "out:" } else { "" };
        let run = run(&format!(
            "void subject(void) {{ int flag = 0; (void)flag; void *p = q_alloc(); \
             while (!flag) {{ {body} }} {tail} (void)p; }}"
        ));
        assert_eq!(
            run.leaks(),
            1,
            "`{body}` reaches the continuation either way: {:?}",
            run.diagnostics
        );
    }
}

/// The retained edge carries the **unrefined** state, so the exit sees exactly
/// what the header saw. Measured against the same program with the loop
/// deleted: the two must report the same custody diagnostics, because a loop
/// the model believes never runs contributes nothing.
#[test]
fn the_retained_edge_carries_the_state_the_header_had() {
    if !clang_available() {
        return;
    }
    for (looped, straight) in [
        (
            "int flag = 0; (void)flag; void *p = q_alloc(); while (!flag) { } (void)p;",
            "int flag = 0; (void)flag; void *p = q_alloc(); (void)p;",
        ),
        (
            "int flag = 0; (void)flag; void *p = q_alloc(); while (!flag) { } q_free(p);",
            "int flag = 0; (void)flag; void *p = q_alloc(); q_free(p);",
        ),
        (
            "int flag = 0; (void)flag; void *p = q_alloc(); while (!flag) { } \
             q_free(p); q_free(p);",
            "int flag = 0; (void)flag; void *p = q_alloc(); q_free(p); q_free(p);",
        ),
    ] {
        let looped_run = run(&format!("void subject(void) {{ {looped} }}"));
        let straight_run = run(&format!("void subject(void) {{ {straight} }}"));
        let looped_custody: Vec<String> = looped_run
            .kinds()
            .into_iter()
            .filter(|kind| !kind.contains(RETAINED))
            .collect();
        assert_eq!(
            looped_custody,
            straight_run.kinds(),
            "the retained edge must carry the header's own state\n looped:   {looped}\n straight: {straight}"
        );
    }
}
