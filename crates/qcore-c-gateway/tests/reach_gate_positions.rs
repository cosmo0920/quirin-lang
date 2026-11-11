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

//! Card REACH-GATE: the INDIRECT half of the joint expression-position gate,
//! end to end through the real gateway.
//!
//! Card C03-REACH measured that the neutral-frame recursive route's corpus
//! reach is gated on `POSITION + INDIRECT` **jointly**
//! (`docs/reports/remaining-c03-reach/README.md` §5). Card W4-EXPR-POS landed
//! the POSITION half with the proved sentence that an admitted expression names
//! no pointer-valued operand anywhere. This card lands the INDIRECT half in the
//! same six positions, which forces that sentence to be *refined*: a member
//! access on a record pointer is a pointer-valued base by construction.
//!
//! The refined sentence — an admitted expression binds, releases, escapes and
//! addresses nothing the projection tracks — is stated and argued in
//! `crate::neutral_position`'s header. These tests are its held-out evidence:
//! the admissions it claims, measured through `analyze_unrestricted_c11`, and
//! the adversarial family each of whose members must keep its existing
//! `Unknown` and its existing diagnostic.
//!
//! Nothing here promotes a verdict. `Safe` in this family is `Safe` for a
//! synthetic declared custody projection under premises this file names.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacySafetyCertificate,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal an unmodeled branch condition keeps.
const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";
/// The refusal an unmodeled return expression keeps.
const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";

/// A three-record chain with a scalar leaf at every level, one self-referential
/// link so a chain can be made arbitrarily long, and a `void *` member the
/// reader will not resolve.
const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
struct z_leaf { int size; void *opaque; }; \
struct z_node { int flag; struct z_leaf *leaf; struct z_node *next; }; \
struct z_view { int kind; struct z_node *head; struct z_node *tail; \
                struct z_node *ring[2]; int *count; }; \
void z_hold(struct z_node *n); \
int z_unknown(int n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "z_reach.qcore".to_owned(),
                include_str!("fixtures/contracts/z_reach.qv").to_owned(),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    certificate: Option<LegacySafetyCertificate>,
}

impl Run {
    fn unsupported(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    fn unknown_aliases(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::UnknownAlias { variable } => Some(variable.clone()),
                _ => None,
            })
            .collect()
    }

    fn leaks(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
            .count()
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
    let (summary, certificate) = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe certificate must recheck independently");
            (certificate.summary.clone(), Some(certificate))
        }
        UnrestrictedLegacyResult::Unsafe(summary) => (summary, None),
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            partial.expect("these fixtures always project partially"),
            None,
        ),
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
        certificate,
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent REACH-GATE test: {reason}");
            false
        }
    }
}

/// The admission this card exists for: a bounded typed access chain, on a base
/// the body never binds, in a branch condition — the shape card W4-EXPR-POS's
/// scalar-leaf family refused because it was not one selection.
#[test]
fn a_chain_to_a_scalar_leaf_discharges_a_branch_condition() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->leaf->size > 0) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert!(
        measured.unsupported().is_empty(),
        "no refusal must remain, got {:?}",
        measured.unsupported()
    );
    assert!(
        measured.certificate.is_some(),
        "the admission must carry an independently rechecked certificate"
    );
}

/// The same chain in every one of the five other positions.
#[test]
fn the_chain_admission_reaches_every_value_discarding_position() {
    if !clang_available() {
        return;
    }
    let bodies = [
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         while (view->head->leaf->size > 0) { break; } q_free(p); }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         for (; view->head->leaf->size > 0; ) { break; } q_free(p); }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         do { break; } while (view->head->leaf->size > 0); q_free(p); }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         switch (view->head->leaf->size) { default: break; } q_free(p); }",
        "int subject(struct z_view *view) { void *p; p = q_alloc(); q_free(p); \
         return view->head->leaf->size; }",
    ];
    for body in bodies {
        let measured = run(body);
        assert_eq!(
            measured.status,
            AnalysisStatus::Safe,
            "{body}\ndiagnostics {:?}",
            measured.diagnostics
        );
        assert!(
            measured.unsupported().is_empty(),
            "{body}: {:?}",
            measured.unsupported()
        );
    }
}

/// The refined soundness sentence's second clause, measured: a chain whose leaf
/// is a **pointer** is admitted where the position discards the value, and —
/// since card C-GATEWAY-RETURN-POSITION — also in a `return`, where it escapes
/// nothing.
///
/// **This expectation was re-pinned deliberately.** Card REACH-GATE pinned the
/// two `return` bodies below as `Unknown` carrying [`RETURN`], on the stated
/// ground that `EscapeIfOwned` models only a canonical place. Card
/// C-GATEWAY-RETURN-POSITION supplies that reasoning's missing answer rather
/// than loosening it: by the F4 base premise the chain names no place the
/// projection binds, so the exit escapes nothing and the node is a `NoOp`. The
/// old assertion is replaced by the *stronger* pair — admitted **and**
/// escaping nothing — not deleted. The discharging direction is pinned
/// separately by `a_pointer_leaf_return_escapes_nothing_it_could_own`.
#[test]
fn a_pointer_leaf_is_admitted_in_a_condition_and_in_a_return() {
    if !clang_available() {
        return;
    }
    let condition = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->next) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(
        condition.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        condition.diagnostics
    );
    assert!(condition.unsupported().is_empty());

    for returned in [
        "struct z_node *subject(struct z_view *view) { return view->head->next; }",
        "struct z_node *subject(struct z_view *view) { \
         return view->kind ? view->head : view->tail; }",
    ] {
        let measured = run(returned);
        assert_eq!(
            measured.status,
            AnalysisStatus::Safe,
            "{returned}: diagnostics {:?}",
            measured.diagnostics
        );
        assert!(
            !measured.unsupported().contains(&RETURN.to_owned()),
            "{returned}: the return refusal must be withdrawn, got {:?}",
            measured.unsupported()
        );
    }
}

/// The obligation `admits_only_untracked_returned_pointer` places on its
/// caller, measured through the real gateway: an admitted pointer-leaf return
/// must escape **nothing**.
///
/// This is the arm that would be wrong in the *discharging* direction, so it is
/// pinned on a body that carries a live obligation the widened return could
/// have discharged. `canonical_place` can spell `view->head->next`; if the
/// parser answered with that instead of `None`, `EscapeIfOwned` would consult
/// the projection for a place the admission never established. The leak below
/// is what proves it did not: it is reported *because* the exit is now modeled,
/// which is this card's accusing direction working.
#[test]
fn a_pointer_leaf_return_escapes_nothing_it_could_own() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "struct z_node *subject(struct z_view *view) { void *p; p = q_alloc(); \
         return view->head->next; }",
    );
    assert!(
        !measured.unsupported().contains(&RETURN.to_owned()),
        "the return is admitted: {:?}",
        measured.unsupported()
    );
    assert_eq!(
        measured.leaks(),
        1,
        "the acquisition is live at the admitted exit and must be reported, \
         not escaped through the returned chain: {:?}",
        measured.diagnostics
    );
    assert_ne!(
        measured.status,
        AnalysisStatus::Safe,
        "a body that leaks must not be Safe: {:?}",
        measured.diagnostics
    );
}

/// The C02 §2 hazard, in an expression position: a member read through a base
/// this body released must never be neutralized.
#[test]
fn a_released_base_read_in_a_condition_keeps_its_refusal() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "void subject(void) { struct z_node *n; n = q_alloc(); if (!n) { return; } \
         q_free(n); if (n->flag) { return; } }",
    );
    assert_ne!(measured.status, AnalysisStatus::Safe);
    // Deliberate pin update, card W43-INDEX-TRAVERSAL: the condition is now
    // the typed read of `n->flag`, whose base was released on the reaching
    // path. The read is refused (`unknown_alias`) and accused: a
    // `use_after_release` candidate, provisional `Unsafe`, which is stronger
    // than the lexical branch refusal it replaces and still never neutralizes
    // the read.
    assert!(
        measured
            .diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::UseAfterRelease { .. }))
            && measured.diagnostics.iter().any(|kind| matches!(
                kind,
                LegacyDiagnosticKind::UnknownAlias { variable } if variable == "n"
            )),
        "the released read must be refused and accused, got {:?}",
        measured.diagnostics
    );
    // And the whole point: no arm of it reports a discharge the release would
    // have contradicted.
    assert_eq!(measured.leaks(), 0, "{:?}", measured.diagnostics);
}

/// Every adversarial shape the card names, each keeping its existing refusal.
#[test]
fn the_adversarial_family_keeps_its_unknown_and_its_diagnostic() {
    if !clang_available() {
        return;
    }
    // Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS. The row
    // `("a base named in any call argument list is bound",
    //   "… z_hold(view->head); if (view->head->flag) …")` moved to
    // `a_borrowed_base_is_admitted_by_the_chain_route_with_its_obligations`.
    // REACH-GATE still refuses it as a NoOp. The chain route now admits the
    // condition as a modeled typed read that carries its root's dereference
    // and C02-P-ALIAS obligations.
    let cases: [(&str, &str); 7] = [
        (
            "the address of a member is not a read",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if (&view->kind) { q_free(p); return; } q_free(p); }",
        ),
        (
            "an explicit dereference has no model here",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if (*view->count) { q_free(p); return; } q_free(p); }",
        ),
        (
            "a subscript has no model here",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if (view->ring[0]) { q_free(p); return; } q_free(p); }",
        ),
        (
            "a chain one selection past the bound refuses",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if (view->head->next->next->next->flag) { q_free(p); return; } q_free(p); }",
        ),
        (
            "a link this reader will not type refuses",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if (view->head->leaf->opaque) { q_free(p); return; } q_free(p); }",
        ),
        (
            "an assignment could store the pointer this rule only discards",
            "void subject(struct z_view *view, struct z_node **out) { void *p; p = q_alloc(); \
             if ((*out = view->head)) { q_free(p); return; } q_free(p); }",
        ),
        (
            "a cast is not an admitted operand",
            "void subject(struct z_view *view) { void *p; p = q_alloc(); \
             if ((struct z_node *) view->head) { q_free(p); return; } q_free(p); }",
        ),
    ];
    // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins three rows
    // deliberately. This rule still refuses them; the Clang-typed condition
    // fallback asked after it reads each (a subscripted pointer element, a
    // chain past this rule's bound, a `void *` member) as the truthiness of a
    // pointer read, with a per-hop obligation over the untracked formal and
    // no refinement. The address, the dereference, the assignment and the cast
    // keep their refusal.
    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d) adds "an explicit
    // dereference has no model here" deliberately: rule 10t reads
    // `*view->count` as a tracked-root place (CG10-COMMON-TYPED-PLACE, where
    // the flow refused the root, `UnknownAlias` of `view`), and the flow now
    // reads `view`, which no path reaching the condition bound or handed to a
    // custody position, as an untracked borrowed object. BEFORE: Unknown.
    const TYPED: [&str; 4] = [
        "a subscript has no model here",
        "a chain one selection past the bound refuses",
        "a link this reader will not type refuses",
        "an explicit dereference has no model here",
    ];
    const TRACKED: [&str; 0] = [];
    for (reason, body) in cases {
        let measured = run(body);
        if TRACKED.contains(&reason) {
            assert_ne!(measured.status, AnalysisStatus::Safe, "{reason}");
            assert!(
                measured.diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic,
                    LegacyDiagnosticKind::UnknownAlias { variable } if variable == "view"
                )),
                "{reason}: {:?}",
                measured.diagnostics
            );
            continue;
        }
        if TYPED.contains(&reason) {
            assert_eq!(
                measured.status,
                AnalysisStatus::Safe,
                "{reason}: read by the typed fallback, got {:?}",
                measured.diagnostics
            );
            continue;
        }
        assert_ne!(measured.status, AnalysisStatus::Safe, "{reason}");
        assert!(
            !measured.unsupported().is_empty(),
            "{reason}: the existing refusal must be retained, got {:?}",
            measured.diagnostics
        );
    }
}

/// A chain exactly at the bound is admitted, so the bound is a refusal and not
/// a blanket rejection of chains.
#[test]
fn a_chain_exactly_at_the_bound_is_admitted() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->next->next->flag) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
}

/// Card C02-P-ALIAS's obligation is attached to *modeled reads*, which this
/// card never reclassifies. An admitted condition beside such a read neither
/// creates the obligation nor erases it: the diagnostic vector is identical
/// with and without the condition.
#[test]
fn the_may_alias_obligation_still_fires_beside_an_admitted_condition() {
    if !clang_available() {
        return;
    }
    let control = run(
        "void subject(struct z_node *a, struct z_node *b, struct z_view *view) { \
         q_free(a); z_hold(b); }",
    );
    let with_condition = run(
        "void subject(struct z_node *a, struct z_node *b, struct z_view *view) { \
         q_free(a); if (view->head->flag) { } z_hold(b); }",
    );
    assert!(
        control.unknown_aliases().contains(&"b/a".to_owned()),
        "the control must carry the may-alias obligation, got {:?}",
        control.diagnostics
    );
    assert_eq!(
        control.unknown_aliases(),
        with_condition.unknown_aliases(),
        "an admitted condition must neither add nor remove a may-alias obligation"
    );
    assert_ne!(with_condition.status, AnalysisStatus::Safe);
}

/// The structural reason the previous test can only ever read that way, made
/// explicit: a name that appears in *any* call argument list of the body is
/// bound, so no base this rule admits is ever a modeled call's actual.
#[test]
fn an_admitted_base_is_never_a_modeled_call_actual() {
    if !clang_available() {
        return;
    }
    let admitted = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->flag) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
    // Deliberate pin update, card W44-INDIRECT-CHAIN-ACCESS: the borrowed
    // half moved to
    // `a_borrowed_base_is_admitted_by_the_chain_route_with_its_obligations`.
    // REACH-GATE's NoOp still withdraws every admission through a base named
    // in a call argument list: `may_bind` is unchanged, and so is this rule's
    // premise. The branch is now admitted by a different route, which keeps
    // the obligations this test's comment says a NoOp would skip.
}

/// Card W44-INDIRECT-CHAIN-ACCESS. `z_hold` borrows `view->head`, so `view`
/// is a name the body *may* bind by REACH-GATE's blunt reading
/// (`FunctionTypes::may_bind`: named in a call argument list). It is still
/// never bound by C02-P premise 4 (`UntrackedBases`), because a `Borrowed`
/// position changes no custody. The chain route admits the condition as the
/// typed read. That read is a modeled instruction: `view` keeps its
/// dereference obligation, its C02-P-ALIAS may-alias obligation and its
/// `use_after_release` accusation, and the link `view->head` is untracked. It
/// is not REACH-GATE's NoOp, so the concern in
/// `an_admitted_base_is_never_a_modeled_call_actual` does not arise.
#[test]
fn a_borrowed_base_is_admitted_by_the_chain_route_with_its_obligations() {
    if !clang_available() {
        return;
    }
    let borrowed = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         z_hold(view->head); if (view->head->flag) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(
        borrowed.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        borrowed.diagnostics
    );
    // The same body with the root released first refuses at the condition,
    // so the admission is the obligation's, not a NoOp's.
    let released = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         q_free(view); if (view->head->flag) { q_free(p); return; } q_free(p); }",
    );
    assert_ne!(
        released.status,
        AnalysisStatus::Safe,
        "{:?}",
        released.diagnostics
    );
}

/// The admission must never manufacture a discharge: a body that acquires and
/// then leaks is still reported, with the leak intact, when its only newly
/// admitted construct is a condition.
#[test]
fn an_admitted_condition_never_hides_a_leak() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->leaf->size > 0) { return; } q_free(p); }",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Unsafe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert_eq!(measured.leaks(), 1, "{:?}", measured.diagnostics);
}

/// Additivity against card W4-EXPR-POS: every expression its families admitted
/// is still admitted, decided by the same code, and the `digits10` shape is
/// unaffected by the role split.
#[test]
fn the_earlier_families_are_unchanged() {
    if !clang_available() {
        return;
    }
    let scalar_only = run(
        "void subject(struct z_view *view, int n) { void *p; p = q_alloc(); \
         if (n + 1 > 0) { q_free(p); return; } q_free(p); }",
    );
    assert_eq!(scalar_only.status, AnalysisStatus::Safe);
    let single_selection = run("int subject(struct z_view *view) { return view->kind; }");
    assert_eq!(single_selection.status, AnalysisStatus::Safe);
    let uncontracted_call = run(
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (z_unknown(view->kind)) { q_free(p); return; } q_free(p); }",
    );
    assert_ne!(uncontracted_call.status, AnalysisStatus::Safe);
    assert!(uncontracted_call.unsupported().contains(&BRANCH.to_owned()));
}
