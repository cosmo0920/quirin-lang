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

//! W42-INDIRECT-ACCESS — bounded scalar stores through tracked record pointers.
//!
//! The destination syntax admitted here is exactly `BASE->FIELD = RHS`, with a
//! scalar field and a scalar-only RHS.  Classification alone is insufficient:
//! the transfer requires `BASE` to denote one live, settled, non-null identity
//! and retains the existing same-type may-alias obligation.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyInstruction, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn w_open() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nullable); };\n\
extern fn w_fresh() -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n\
extern fn w_release(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nonnull);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
struct record { int state; struct record *next; };\n\
struct record *w_open(void); struct record *w_fresh(void);\n\
void w_release(struct record *);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "scalar_member_store.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("skipping Clang-dependent scalar-member-store test: {reason}");
            false
        }
    }
}

fn analyze(program: &str, entry: &str) -> UnrestrictedLegacyResult {
    let source = format!("{DECLARATIONS}\n{program}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"))
}

fn diagnostics(result: &UnrestrictedLegacyResult, function: &str) -> Vec<LegacyDiagnosticKind> {
    let summary = match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("missing partial summary: {other:?}"),
    };
    summary.functions[function]
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind.clone())
        .collect()
}

fn status(result: &UnrestrictedLegacyResult, function: &str) -> AnalysisStatus {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            certificate.summary.functions[function].status
        }
        UnrestrictedLegacyResult::Unsafe(summary) => summary.functions[function].status,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary.functions[function].status,
        other => panic!("missing partial summary: {other:?}"),
    }
}

fn assert_safe(program: &str) {
    let result = analyze(program, "subject");
    let UnrestrictedLegacyResult::Safe(certificate) = result else {
        panic!("expected Safe, got {result:?}");
    };
    check_legacy_safety_certificate(&certificate).expect("certificate rechecks");
}

fn assert_unknown_alias(program: &str, expected: &str) {
    let result = analyze(program, "subject");
    assert!(
        !matches!(result, UnrestrictedLegacyResult::Safe(_)),
        "{result:?}"
    );
    let diagnostic = diagnostics(&result, "subject");
    assert!(
        diagnostic.iter().any(
            |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == expected)
        ),
        "{diagnostic:?}"
    );
}

#[test]
fn a_tracked_live_base_accepts_one_scalar_member_store() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "void subject(int n) { struct record *p = w_fresh(); p->state = n + 1; w_release(p); }",
    );
}

#[test]
fn a_released_base_is_refused_at_the_store() {
    if !clang_available() {
        return;
    }
    assert_unknown_alias(
        "void subject(void) { struct record *p = w_fresh(); w_release(p); p->state = 1; }",
        "p",
    );
}

#[test]
fn a_refused_store_preserves_unrelated_resource_evidence() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(void) { struct record *p = w_fresh(); struct record *q = w_fresh(); \
         w_release(p); p->state = 1; }",
        "subject",
    );
    let diagnostic = diagnostics(&result, "subject");
    assert_eq!(
        status(&result, "subject"),
        AnalysisStatus::Unsafe,
        "{diagnostic:?}"
    );
    assert!(
        diagnostic.iter().any(
            |kind| matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == "p")
        ),
        "{diagnostic:?}"
    );
    assert!(
        diagnostic
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "the refused scalar store must retain q's live obligation: {diagnostic:?}"
    );
}

#[test]
fn nullable_bases_need_their_own_nonnull_guard() {
    if !clang_available() {
        return;
    }
    assert_unknown_alias(
        "void subject(void) { struct record *p = w_open(); p->state = 1; if (p) w_release(p); }",
        "p",
    );
    assert_safe(
        "void subject(void) { struct record *p = w_open(); if (!p) return; p->state = 1; w_release(p); }",
    );
}

#[test]
fn a_released_same_typed_peer_keeps_the_alias_obligation() {
    if !clang_available() {
        return;
    }
    assert_unknown_alias(
        "void subject(struct record *a, struct record *b) { w_release(a); b->state = 1; w_release(b); }",
        "b/a",
    );
}

#[test]
fn a_distinct_body_acquisition_discharges_the_peer_obligation() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "void subject(struct record *a) { struct record *fresh = w_fresh(); w_release(a); \
         fresh->state = 1; w_release(fresh); }",
    );
}

/// Card W44-INDIRECT-CHAIN-ACCESS: `p->state = p->next->state` over a live
/// identity `p` is read by the chain route. Its root resolves to one live
/// identity, so the link `p->next` is a member this body could have bound. No
/// admitted store bound it, so there is no evidence for it, and it refuses with
/// its own reason. The function stays Unknown and never reaches `Safe`.
#[test]
fn a_chained_rhs_through_an_unstored_member_refuses_on_its_link() {
    if !clang_available() {
        return;
    }
    let program = "void subject(int n) { struct record *p = w_fresh(); \
                   p->state = p->next->state; w_release(p); }";
    let result = analyze(program, "subject");
    assert_eq!(status(&result, "subject"), AnalysisStatus::Unknown);
    assert_eq!(
        diagnostics(&result, "subject"),
        vec![LegacyDiagnosticKind::UnknownAlias {
            variable: "p->next".to_owned()
        }]
    );
}

#[test]
fn rhs_mutation_and_indirect_rhs_shapes_keep_the_lexical_refusal() {
    if !clang_available() {
        return;
    }
    // Card W43-INDIRECT-ACCESS-GENERAL deliberately admits `p->state` — a
    // scalar member read of the same live identity — through its typed scalar
    // access route; `a_live_member_read_on_the_rhs_is_admitted_by_the_general_route`
    // pins that. The other two keep the lexical refusal.
    //
    // Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS: `p->next->state`
    // moved to `a_chained_rhs_through_an_unstored_member_refuses_on_its_link`
    // below. The chain route now reads it, and refuses it on its own link.
    //
    // Deliberate pin update, card CG02-ELABORATION-CORE (rule 10s): `n++` on
    // the scalar formal `n` is now the elaborator's (a typed write of
    // `p->state`, then `n` forgotten); `*(&n)` keeps the refusal.
    let run = analyze(
        "void subject(int n) { struct record *p = w_fresh(); p->state = n++; w_release(p); }",
        "subject",
    );
    assert_eq!(status(&run, "subject"), AnalysisStatus::Safe);
    let result = analyze(
        "void subject(int n) { struct record *p = w_fresh(); p->state = *(&n); w_release(p); }",
        "subject",
    );
    let diagnostic = diagnostics(&result, "subject");
    assert_eq!(
        status(&result, "subject"),
        AnalysisStatus::Unknown,
        "*(&n): {diagnostic:?}"
    );
    assert!(
        diagnostic
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Unsupported { .. })),
        "*(&n): {diagnostic:?}"
    );
}

#[test]
fn a_live_member_read_on_the_rhs_is_admitted_by_the_general_route() {
    if !clang_available() {
        return;
    }
    assert_safe(
        "void subject(int n) { struct record *p = w_fresh(); p->state = p->state; w_release(p); }",
    );
    assert_unknown_alias(
        "void subject(int n) { struct record *p = w_fresh(); w_release(p); p->state = p->state; }",
        "p",
    );
}

#[test]
fn a_mutating_rhs_cannot_leave_a_stale_scalar_branch_fact() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(void) { int flag = 0; struct record *p = w_fresh(); \
         p->state = flag++; if (flag) { w_release(p); w_release(p); } \
         else { w_release(p); } }",
        "subject",
    );
    let diagnostic = diagnostics(&result, "subject");
    assert_eq!(
        status(&result, "subject"),
        AnalysisStatus::Unsafe,
        "{diagnostic:?}"
    );
    // Deliberate pin update, card CG02-ELABORATION-CORE (rule 10s): the
    // statement is now the elaborator's, which forgets `flag` after its write
    // (no stale branch fact), so the double release on the reachable `flag`
    // edge is still found, and no refusal remains.
    assert!(
        !diagnostic
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Unsupported { .. })),
        "{diagnostic:?}"
    );
    assert!(
        diagnostic
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        "the increment's true branch must remain reachable rather than using the stale zero fact: \
         {diagnostic:?}"
    );
}

#[test]
fn a_pointer_field_store_is_not_this_rule() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(void) { struct record *p = w_fresh(); p->next = p; w_release(p); }",
        "subject",
    );
    assert_eq!(
        status(&result, "subject"),
        AnalysisStatus::Unknown,
        "{result:?}"
    );
}

#[test]
fn a_local_helper_cannot_launder_the_store_into_a_w4_summary() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "static void helper(struct record *p) { w_release(p); p->state = 1; }\n\
         void subject(void) { struct record *p = w_fresh(); helper(p); }",
        "subject",
    );
    // The caller was accused (`Unsafe`) until card W44-LEAK-PIN-AUDIT: the
    // refused call hands `p` to `helper`, so the refused hand-on rule
    // withdraws the accusation. The refused helper still hides nothing: the
    // caller never certifies.
    assert_eq!(
        status(&result, "subject"),
        AnalysisStatus::Unknown,
        "the refused helper must not hide the caller's residual: {result:?}"
    );
    assert!(
        diagnostics(&result, "subject").iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description }
            if description.contains("uncontracted function `helper`"))
        }),
        "a helper that releases then stores must publish no W4 summary: {result:?}"
    );
}

#[test]
fn lowering_uses_the_explicit_store_instruction() {
    if !clang_available() {
        return;
    }
    let result = analyze(
        "void subject(void) { struct record *p = w_fresh(); p->state = 1; w_release(p); }",
        "subject",
    );
    let summary = match result {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        other => panic!("expected Safe: {other:?}"),
    };
    assert!(summary.functions["subject"].cfg.nodes.iter().any(|node| {
        matches!(
            &node.instruction,
            LegacyInstruction::ScalarMemberStore { base, .. } if base == "p"
        )
    }));
}
