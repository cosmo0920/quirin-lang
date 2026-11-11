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

//! Card C-GATEWAY-RETURN-POSITION: the pointer-valued F4 chain leaf in a
//! `return` expression, end to end through the real gateway.
//!
//! Card REACH-GATE admitted a pointer-valued access-chain leaf only where the
//! position discards the value into a control-flow decision
//! (`ExpressionRole::Decides`), refusing it in a `return` on the ground that
//! `legacy_c`'s `EscapeIfOwned` models only a *canonical place*. This card
//! supplies that argument's missing answer rather than loosening it: the F4
//! base premise is `!FunctionTypes::may_bind(base)`, and every tracked `->`
//! place the projection holds is rooted at a body-acquired name, which
//! `may_bind` reports bound by construction. So no place along an admitted
//! chain is a place the projection binds, the pointer it produces is not a
//! tracked identity of this body, and the exit has nothing to escape.
//!
//! # What this file is evidence for, and what it is not
//!
//! The admission is answered with `returned_place: None`, so the node is a
//! `NoOp`. Two directions therefore need held-out evidence and both are driven
//! here:
//!
//! * the **admitting** direction — the refusal is withdrawn for exactly the
//!   shapes the premises cover, and for no other;
//! * the **discharging** direction — an admitted return escapes *nothing*, so a
//!   live obligation at that exit is reported rather than laundered through the
//!   returned chain. That is the arm invariant 1 constrains, and it is pinned
//!   on bodies that carry an obligation the widening could have discharged.
//!
//! Every refusal below is additionally shown **non-vacuous**: the same
//! expression is measured in a condition position, where it is admitted. A
//! refusal that held for an unrelated reason would refuse there too, so the
//! pairing is what makes each row evidence about *this* rule rather than about
//! the fixture.
//!
//! Nothing here promotes a corpus verdict. `Safe` in this family is `Safe` for
//! a synthetic declared custody projection under the premises this file names.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal an unmodeled return expression keeps where this card's premises
/// are not met.
const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";

/// The same record family card REACH-GATE uses, so the two files' admissions
/// and refusals are comparable shape for shape.
const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
struct z_leaf { int size; void *opaque; }; \
struct z_node { int flag; struct z_leaf *leaf; struct z_node *next; }; \
struct z_view { int kind; struct z_node *head; struct z_node *tail; \
                struct z_node *ring[2]; int *count; }; \
typedef struct z_opaque { int id; } z_opaque_t; \
typedef struct z_getter z_getter_t; \
struct z_getter { z_opaque_t *value; }; \
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
    certified: bool,
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

    fn refuses_return(&self) -> bool {
        self.unsupported().contains(&RETURN.to_owned())
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
    let (summary, certified) = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate)
                .expect("a Safe certificate must recheck independently");
            (certificate.summary.clone(), true)
        }
        UnrestrictedLegacyResult::Unsafe(summary) => (summary, false),
        UnrestrictedLegacyResult::Unknown { partial, .. } => (
            partial.expect("these fixtures always project partially"),
            false,
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
        certified,
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
            eprintln!("skipping Clang-dependent return-position test: {reason}");
            false
        }
    }
}

/// A refused `return` shape, paired with the same expression in a condition,
/// where card REACH-GATE already admits it.
///
/// This is the non-vacuity harness. `refused` must keep [`RETURN`]; `decided`
/// must be `Safe` with no refusal at all. If a fixture refused for some reason
/// other than the return position — an unresolved member, an unmodeled call, a
/// base the reader would not type — the condition arm would refuse too, and the
/// row would be silently measuring nothing.
fn refused_in_return_but_admitted_in_a_condition(refused: &str, decided: &str) {
    let returned = run(refused);
    assert!(
        returned.refuses_return(),
        "the return refusal must be retained for `{refused}`, got {:?}",
        returned.unsupported()
    );
    assert_ne!(returned.status, AnalysisStatus::Safe, "{refused}");

    let condition = run(decided);
    assert_eq!(
        condition.status,
        AnalysisStatus::Safe,
        "the paired condition must be admitted, or the refusal above is \
         evidence about the fixture rather than about the return position: \
         {decided}\ndiagnostics {:?}",
        condition.diagnostics
    );
    assert!(
        condition.unsupported().is_empty(),
        "{decided}: {:?}",
        condition.unsupported()
    );
}

/// The admission this card exists for, at each chain depth the premises cover.
#[test]
fn a_pointer_leaf_chain_is_admitted_in_a_return_at_every_depth() {
    if !clang_available() {
        return;
    }
    for body in [
        "struct z_node *subject(struct z_view *view) { return view->head; }",
        "struct z_node *subject(struct z_view *view) { return view->head->next; }",
        "struct z_node *subject(struct z_view *view) { \
         return view->head->next->next; }",
    ] {
        let measured = run(body);
        assert_eq!(
            measured.status,
            AnalysisStatus::Safe,
            "{body}\ndiagnostics {:?}",
            measured.diagnostics
        );
        assert!(
            measured.unsupported().is_empty(),
            "{body}: no refusal must remain, got {:?}",
            measured.unsupported()
        );
        assert!(
            measured.certified,
            "{body}: the admission must carry an independently rechecked \
             certificate"
        );
    }
}

/// The discharging direction, which invariant 1 constrains: an admitted return
/// escapes nothing, so an obligation live at that exit is **reported**.
///
/// `legacy_c::canonical_place` can spell `view->head->next`. Had the parser
/// answered with it instead of `None`, the exit would consult the projection
/// for a place this admission never established. The leak is what proves it did
/// not — and it is a leak the old refusal *suppressed*, because an `Unknown`
/// return marks the path unmodeled and `report_leaks` skips it. This row is
/// therefore both halves of the card at once: the refusal withdrawn, the
/// violation exposed.
#[test]
fn an_admitted_return_reports_the_obligation_it_could_have_laundered() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "struct z_node *subject(struct z_view *view) { void *p; p = q_alloc(); \
         return view->head->next; }",
    );
    assert!(
        !measured.refuses_return(),
        "the return must be admitted: {:?}",
        measured.unsupported()
    );
    assert_eq!(
        measured.leaks(),
        1,
        "the live acquisition must be reported at the now-modeled exit: {:?}",
        measured.diagnostics
    );
    assert!(
        !measured.certified,
        "a leaking body must not certify: {:?}",
        measured.diagnostics
    );
}

/// A body that releases what it acquired and then returns an untracked chain is
/// admitted with **no** leak — the companion to the row above, so that the leak
/// there is attributable to the live obligation rather than to the widening.
#[test]
fn an_admitted_return_beside_a_discharged_obligation_reports_nothing() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "struct z_node *subject(struct z_view *view) { void *p; p = q_alloc(); \
         q_free(p); return view->head->next; }",
    );
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert_eq!(measured.leaks(), 0, "{:?}", measured.diagnostics);
}

/// The base premise, which is the whole soundness argument: a base this body
/// **binds** keeps its refusal, at every one of `may_bind`'s three grounds.
///
/// If this arm were lost, the chain could name a place the projection tracks
/// and the `NoOp` would be suppressing an escape rather than modelling one.
#[test]
fn a_bound_base_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    // written
    refused_in_return_but_admitted_in_a_condition(
        "struct z_node *subject(struct z_view *view, struct z_view *other) { \
         view = other; return view->head->next; }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->next) { q_free(p); return; } q_free(p); }",
    );
    // address taken
    refused_in_return_but_admitted_in_a_condition(
        "struct z_node *subject(struct z_view *view) { \
         struct z_view **alias; alias = &view; return view->head->next; }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->next) { q_free(p); return; } q_free(p); }",
    );
    // named inside a call's argument list
    refused_in_return_but_admitted_in_a_condition(
        "struct z_node *subject(struct z_view *view) { \
         z_hold(view->head); return view->head->next; }",
        "void subject(struct z_view *view) { void *p; p = q_alloc(); \
         if (view->head->next) { q_free(p); return; } q_free(p); }",
    );
}

/// A base this body **acquired** is the sharpest case of the premise above: the
/// chain would then be rooted at a tracked identity, and admitting it would let
/// the return launder a live acquisition.
#[test]
fn a_chain_rooted_at_an_acquisition_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    let measured = run("struct z_node *subject(void) { struct z_view *view; \
         view = q_alloc(); return view->head->next; }");
    assert!(
        measured.refuses_return(),
        "a chain rooted at a body-acquired name must keep its refusal, got {:?}",
        measured.unsupported()
    );
    assert_ne!(measured.status, AnalysisStatus::Safe);
}

/// A released base, the C02 §2 hazard in return position: the read must never
/// be neutralized, and no arm may report a discharge the release contradicts.
#[test]
fn a_released_base_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    let measured = run(
        "struct z_node *subject(void) { struct z_node *n; n = q_alloc(); \
         if (!n) { return n; } q_free(n); return n->next; }",
    );
    assert!(
        measured.refuses_return(),
        "the released base's read must keep its refusal, got {:?}",
        measured.unsupported()
    );
}

/// B01's production-moving opaque getter family. The member's typedef does
/// not establish a tracked resource identity, so the ordinary assignment
/// classifier lowers the read to `OpaqueValue` and the return adds no escape.
#[test]
fn an_untracked_opaque_getter_is_evaluated_at_return() {
    if !clang_available() {
        return;
    }
    let measured = run("z_opaque_t *subject(const z_getter_t *getter) { return getter->value; }");
    assert!(!measured.refuses_return(), "{:?}", measured.unsupported());
    assert_eq!(measured.status, AnalysisStatus::Safe);
    assert!(measured.certified);
}

/// The opaque getter is admitted only for a base the risk closure leaves
/// untracked. A call that releases the formal makes the base protected, and an
/// acquired local makes it bound; both keep the old refusal.
#[test]
fn protected_and_bound_opaque_getter_bases_stay_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "z_opaque_t *subject(z_getter_t *getter) { q_free(getter); return getter->value; }",
        "z_opaque_t *subject(void) { z_getter_t *getter; getter = q_alloc(); return getter->value; }",
    ] {
        let measured = run(body);
        assert!(
            measured.refuses_return(),
            "{body}: {:?}",
            measured.unsupported()
        );
        assert_ne!(measured.status, AnalysisStatus::Safe, "{body}");
    }
}

/// `OpaqueValue` covers one complete direct member read, not a conditional
/// choice of pointer results; that choice needs branch-sensitive value
/// identity, which rule 10z's branching gives it.
#[test]
fn a_conditional_opaque_pointer_result_stays_refused() {
    if !clang_available() {
        return;
    }
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // branch-sensitive value identity is the branch itself. Each edge's
    // `return getter->value;` / `return (z_opaque_t *)0;` is decided alone by
    // the routes above (BEFORE: the return refusal).
    let measured = run("z_opaque_t *subject(const z_getter_t *getter, int flag) { \
         return flag ? getter->value : (z_opaque_t *)0; }");
    assert!(!measured.refuses_return(), "{:?}", measured.unsupported());
    let direct = run("z_opaque_t *subject(const z_getter_t *getter) { return getter->value; }");
    assert_eq!(measured.status, direct.status);
}

/// Every shape outside the premises, each keeping the refusal it already had.
///
/// The widening is the `Decides` rule reached from a new position and nothing
/// more: it adds no vocabulary, so a dereference, a subscript, an address-of,
/// an uncontracted call, a member this reader will not type and a chain past
/// `ACCESS_CHAIN_LIMIT` all refuse exactly as before. (Since card
/// W43-RETURN-EXPRESSION the uncontracted whole call refuses with the statement
/// classifier's wording rather than the return position's; it still refuses.)
#[test]
fn the_adversarial_family_keeps_its_return_refusal() {
    if !clang_available() {
        return;
    }
    let decided = "void subject(struct z_view *view) { void *p; p = q_alloc(); \
                   if (view->head->next) { q_free(p); return; } q_free(p); }";
    // an address-of: an interior address is a derived result, which neither
    // this route nor the W47 typed return fallback answers.
    refused_in_return_but_admitted_in_a_condition(
        "struct z_node **subject(struct z_view *view) { return &view->head; }",
        decided,
    );
    // an explicit dereference: the protected-formal reading counts the
    // leading `*` as a use of the formal's slot, so `may_bind(view)` holds and
    // the W47 fallback refuses it too.
    refused_in_return_but_admitted_in_a_condition(
        "struct z_node *subject(struct z_view *view) { return *view->ring; }",
        decided,
    );
    // Card W47-ACCESS-PATH-POSITIONS (rule 10k) re-pins these deliberately.
    // They are outside this route's grammar, which is unchanged, but the
    // Clang-typed return fallback asked after it reads each as one pointer
    // read whose every root the custody closure proves never bound and
    // `may_bind` does not name (this route's own base premise), with a
    // `NoOp` exit: no place escapes, and a bound root still refuses
    // (`a_bound_base_keeps_the_return_refusal`,
    // `a_chain_rooted_at_an_acquisition_keeps_the_return_refusal`).
    for typed in [
        // a subscript
        "struct z_node *subject(struct z_view *view) { return view->ring[0]; }",
        // a `void *` member this route's reader will not resolve
        "void *subject(struct z_view *view) { return view->head->leaf->opaque; }",
        // a chain past this route's bound
        "struct z_node *subject(struct z_view *view) { \
         return view->head->next->next->next->next; }",
    ] {
        let returned = run(typed);
        assert!(!returned.refuses_return(), "{typed}");
        assert_eq!(returned.status, AnalysisStatus::Safe, "{typed}");
        assert!(returned.certified, "{typed}");
    }
    // An uncontracted call. Deliberately re-pinned by card
    // W43-RETURN-EXPRESSION: a whole-call return the statement classifier
    // refuses is lowered to that refusal, which here is the classifier's own
    // lexical indirect-access gate on the argument — exactly what
    // `x = z_unknown(view->kind);` reports in a statement. It is still refused,
    // not `Safe` and not certified; only the wording names the real blocker.
    // Deliberately re-pinned again by card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
    // (rule 12k): `view->kind` is an actual W44 reads at a `Borrowed`
    // position, so the uncontracted callee is what decides, and the refusal
    // names it, exactly as the statement form now does.
    let lowered = run("int subject(struct z_view *view) { return z_unknown(view->kind); }");
    assert!(!lowered.refuses_return(), "{:?}", lowered.unsupported());
    assert_eq!(
        lowered.unsupported(),
        vec!["call to uncontracted function `z_unknown`".to_owned()]
    );
    assert_ne!(lowered.status, AnalysisStatus::Safe);
    assert!(!lowered.certified);
}

/// The behaviour this card must **not** disturb: a `return` of a plain
/// canonical place still escapes it.
///
/// The widening is gated on the strict rule having refused first, so a plain
/// name — which the lexical predicate admits — never reaches it and keeps its
/// `EscapeIfOwned`. Measured in both directions: the transfer is modeled (no
/// leak) and the non-transfer is not.
#[test]
fn a_plain_canonical_place_still_escapes() {
    if !clang_available() {
        return;
    }
    let escaped = run("void *subject(void) { void *p; p = q_alloc(); return p; }");
    assert_eq!(
        escaped.leaks(),
        0,
        "a returned plain name must still escape: {:?}",
        escaped.diagnostics
    );
    assert_eq!(
        escaped.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        escaped.diagnostics
    );

    let dropped = run("int subject(void) { void *p; p = q_alloc(); return 0; }");
    assert_eq!(
        dropped.leaks(),
        1,
        "the same acquisition not returned must still leak: {:?}",
        dropped.diagnostics
    );
}

/// A scalar leaf in a return was admitted before this card and is unchanged by
/// it, so the widening is measured to be exactly the pointer-leaf arm.
#[test]
fn the_scalar_leaf_return_is_unchanged() {
    if !clang_available() {
        return;
    }
    let measured = run("int subject(struct z_view *view) { return view->head->leaf->size; }");
    assert_eq!(
        measured.status,
        AnalysisStatus::Safe,
        "diagnostics {:?}",
        measured.diagnostics
    );
    assert!(measured.unsupported().is_empty());
}
