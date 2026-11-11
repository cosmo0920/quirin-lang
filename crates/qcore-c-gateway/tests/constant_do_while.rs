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

//! Card `C-GATEWAY-CONSTANT-DO-WHILE`: the constant `do … while (0)` exit,
//! end to end through the real gateway.
//!
//! `do S while (0)` executes `S` exactly once (C11 §6.8.5.2). The CFG builder
//! gave every `do`-`while` header both `body_entry` and `continuation` as
//! successors whatever its condition, so the fragment carried a back edge the
//! C does not have. On the pinned Fluent Bit corpus that is the pervasive
//! `do { … } while (0)` macro idiom — the card's preprocessed census measures
//! **15,053** occurrences across the 185 units — and it is why
//! `rd_kafka_jwt_b64_decode_payload` carried a provisional `double_release`
//! through `RD_IF_FREE(converted_src, rd_free)`.
//!
//! Removing an edge is the **admitting** direction, so these tests are written
//! in both directions. The withdrawing half measures that the spurious
//! repetition is gone. The accusing half measures that nothing real was
//! swallowed with it: a genuine double release inside one body, a genuine
//! leak, and the unmodeled-condition refusal are each still reported.
//!
//! The boundary tests are the point of the rule's key. Standing constraint 5a
//! forbids deciding control flow from an *evaluated* expression, so the rule
//! fires on the single literal token `0` and on nothing else — not on
//! `while ((0))`, not on `while (1 - 1)`, not on a binding known to be zero.
//! Those all keep the CFG they had, and the family below pins that.
//!
//! Nothing here promotes a corpus verdict. The theorems are
//! `QCore.ConstantDoWhile`.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal an unmodeled do-while condition keeps. These are the 32 leaves
/// of a *different* question (an unmodeled condition), and this card leaves
/// every one of them refused.
const DO_WHILE_REFUSAL: &str = "unmodeled call, aggregate or indirect access in do-while condition";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); \
int z_unknown(int n);";

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
    fn double_releases(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. }))
            .count()
    }

    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
    }

    fn unsupported(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    /// The comparison `constant_do_while_is_its_body` rests on: the multiset of
    /// diagnostic kinds, order-insensitive, so a node-numbering difference
    /// between the wrapped and unwrapped forms cannot make the test pass or
    /// fail for the wrong reason.
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

/// The corpus shape itself. `RD_IF_FREE(PTR, FUNC)` expands to
/// `do { if ((PTR)) FUNC(PTR); } while (0)`; before this card the back edge
/// made the release look repeatable and the function went provisional
/// `Unsafe`.
#[test]
fn the_release_in_a_constant_do_while_is_not_repeatable() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { void *p = q_alloc(); \
         do { if (p) q_free(p); } while (0); }");
    assert_eq!(
        run.double_releases(),
        0,
        "a `do … while (0)` body runs once: {:?}",
        run.diagnostics
    );
}

/// The `≡ S` claim, measured rather than argued: wrapping a body in
/// `do { … } while (0)` changes no diagnostic at all.
#[test]
fn a_constant_do_while_is_its_body() {
    if !clang_available() {
        return;
    }
    for (bare, wrapped) in [
        (
            "void *p = q_alloc(); if (p) q_free(p);",
            "void *p = q_alloc(); do { if (p) q_free(p); } while (0);",
        ),
        (
            "void *p = q_alloc(); q_free(p);",
            "void *p = q_alloc(); do { q_free(p); } while (0);",
        ),
        (
            "void *p = q_alloc(); q_free(p); q_free(p);",
            "void *p = q_alloc(); do { q_free(p); q_free(p); } while (0);",
        ),
    ] {
        let bare_run = run(&format!("void subject(void) {{ {bare} }}"));
        let wrapped_run = run(&format!("void subject(void) {{ {wrapped} }}"));
        assert_eq!(
            bare_run.kinds(),
            wrapped_run.kinds(),
            "the wrapper must change nothing\n bare:    {bare}\n wrapped: {wrapped}"
        );
        assert_eq!(
            bare_run.status, wrapped_run.status,
            "status must agree\n bare:    {bare}\n wrapped: {wrapped}"
        );
    }
}

/// The accusing direction is not swallowed. A body that really does release
/// twice is still a double release — the rule removed a repetition of the
/// body, not the analysis inside it.
#[test]
fn a_real_double_release_inside_a_constant_do_while_is_still_reported() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { void *p = q_alloc(); \
         do { q_free(p); q_free(p); } while (0); }");
    assert!(
        run.double_releases() >= 1,
        "two releases in one body is still a double release: {:?}",
        run.diagnostics
    );
    assert_ne!(run.status, AnalysisStatus::Safe);
}

/// A real leak is still a real leak: the exit edge was never the one removed.
#[test]
fn a_leak_through_a_constant_do_while_is_still_reported() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { do { void *p = q_alloc(); (void)p; } while (0); }");
    assert!(
        run.leaks() >= 1,
        "the acquisition is never released: {:?}",
        run.diagnostics
    );
}

/// The boundary, and the reason the key is a token. Every one of these
/// conditions is *not* the literal token `0`, so every one keeps the back edge
/// and keeps the repeated-release reading. Constraint 5a: an operand-class
/// marker is never a computed value, so the analyzer does not get to notice
/// that `1 - 1` is zero.
#[test]
fn only_the_literal_zero_token_loses_the_back_edge() {
    if !clang_available() {
        return;
    }
    for condition in [
        "(0)",          // parenthesized: three tokens, not one
        "1 - 1",        // a computed zero
        "0 == 0",       // a computed zero
        "0 && 0",       // a computed zero
        "z_unknown(0)", // a call, zero only by the callee's grace
    ] {
        let run = run(&format!(
            "void subject(void) {{ void *p = q_alloc(); \
             do {{ q_free(p); }} while ({condition}); }}"
        ));
        assert!(
            run.double_releases() >= 1,
            "`while ({condition})` must keep the back edge: {:?}",
            run.diagnostics
        );
        assert_ne!(
            run.status,
            AnalysisStatus::Safe,
            "`while ({condition})` must not become Safe"
        );
    }
}

/// A neighbouring route this card did **not** add and does not own: where the
/// analyzer already models a scalar binding's value precisely, a `do`-`while`
/// on that binding is already decided from the modeled value. Measured against
/// the unmodified base as well as against this card's tree — every row below
/// is identical on both — so it is recorded here as a pinned boundary rather
/// than claimed as this card's effect.
///
/// It answers conservatively on every shape probed: only a binding modeled as
/// zero *at the condition* loses the edge, a reassignment is tracked, and an
/// unmodeled initializer keeps the edge and its refusal. That is why this card
/// still had work to do — a macro's `while (0)` is a literal token and no
/// binding, so this route never reached it.
///
/// **Amended by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37).** Every row below
/// spells the header *positively*, so all four measure the **back** edge, and
/// the assertion was on `double_releases()` alone, which a deleted **exit**
/// edge does not move. Inverting the polarity (`while (!flag)`,
/// `while (flag == 0)`) makes the same arm delete the exit edge instead, which
/// stranded the function's exit node and returned `Safe` for a body that leaks
/// after the loop. The rows are kept exactly as wave 35 measured them — this is
/// still the back-edge boundary — and the missing half now has its own family
/// in `scalar_route_exit_edge.rs`. The added assertion here is the one that
/// says *which* edge these rows are about: the exit is still reached, so the
/// acquisition's disposition after the loop is still judged.
#[test]
fn a_modeled_scalar_condition_is_a_separate_pre_existing_route() {
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
        // SCALAR-ROUTE-EXIT-EDGE: the exit is reached on every row, so the
        // acquisition really is judged after the loop rather than left
        // unexamined. A row that deleted its exit edge would report no leak
        // *and* no release — indistinguishable from a clean function.
        assert_eq!(
            run.leaks(),
            0,
            "`{prelude}`: the exit is reached and the release discharges it: {:?}",
            run.diagnostics
        );
    }
}

/// The 32 leaves of the unmodeled-condition refusal are a different question
/// and stay refused. This is the arm the card deliberately does not touch.
#[test]
fn an_unmodeled_do_while_condition_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { do { } while (z_unknown(1)); }");
    assert!(
        run.unsupported()
            .iter()
            .any(|description| description == DO_WHILE_REFUSAL),
        "the unmodeled condition keeps its own refusal: {:?}",
        run.unsupported()
    );
}

/// A `continue` in a `do … while (0)` jumps to the controlling expression,
/// which is `0`, so it exits. The body's remaining statements are skipped —
/// the acquisition after the `continue` never happens, so there is no leak.
#[test]
fn a_continue_in_a_constant_do_while_exits() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) { void *p = q_alloc(); \
         do { q_free(p); continue; } while (0); }");
    assert_eq!(
        run.double_releases(),
        0,
        "`continue` reaches the zero condition and exits: {:?}",
        run.diagnostics
    );
    assert_eq!(
        run.leaks(),
        0,
        "the single acquisition is released before the continue: {:?}",
        run.diagnostics
    );
}

/// A `break` in a `do … while (0)` leaves by the same continuation the
/// condition would have taken.
#[test]
fn a_break_in_a_constant_do_while_reaches_the_same_continuation() {
    if !clang_available() {
        return;
    }
    let with_break = run("void subject(void) { void *p = q_alloc(); \
         do { q_free(p); break; } while (0); }");
    let without = run("void subject(void) { void *p = q_alloc(); \
         do { q_free(p); } while (0); }");
    assert_eq!(with_break.kinds(), without.kinds());
    assert_eq!(with_break.status, without.status);
}
