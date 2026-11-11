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

//! Card **C-GATEWAY-RETURN-CALL-W4**: the modeled call in `return` position,
//! end to end through the real gateway.
//!
//! Card `C-GATEWAY-RETURN-POSITION` withdrew this sub-shape on two grounds, one
//! of which was a **soundness** ground: applying a callee's parameter custody at
//! a return position would let a `Consumed`/`Released` mode discharge a live
//! obligation of the caller at that exit, on a route whose `observe_exit` has no
//! arm for a call result at all. That ground is answered by a rule, not set
//! aside, and this file is the rule's held-out evidence.
//!
//! # The rule, and therefore the shape of the evidence
//!
//! `return f(a);` and `{ T = f(a); return T; }` are the same C. The gateway has
//! modeled the second form since Cycle 9, so the card desugars the first into
//! it. The central property is therefore an **equivalence**, and that is what
//! most rows below assert: an admitted `return f(a);` produces exactly the
//! status and exactly the diagnostics that the hand-written two-statement form
//! produces, in a file where both are compiled by the real Clang and analysed by
//! the real analyzer. An equivalence is a much stronger pin than a status
//! assertion, because it cannot be satisfied by refusing.
//!
//! The refused arm is pinned by the *same* comparison run the other way: for a
//! `Reallocated` result the two forms deliberately **disagree** — the
//! two-statement form is admitted, the `return` form refuses — which is what
//! makes the refusal evidence about this rule rather than about the fixture.
//!
//! # The three directions held out
//!
//! * the **admitting** direction — the refusal is withdrawn for exactly the
//!   shapes the premises cover, and the result agrees with the desugaring;
//! * the **discharging** direction — invariant 1's constrained arm. A
//!   `Reallocated` result is refused because its result carries the *replaced
//!   parameter's* obligation out of the exit. Pinned positively (the refusal is
//!   kept) and by contrast (the same call in a statement position is admitted);
//! * the **accusing** direction — an admitted return *exposes* what the old
//!   blanket `Unknown` hid. `report_leaks` skips an unmodeled exit, so a body
//!   that leaks across an admitted `return f(a);` now reports that leak where it
//!   reported nothing before. This is the direction that makes the widening
//!   safe, and it is pinned rather than argued.
//!
//! Nothing here promotes a corpus verdict. `Safe` in this family is `Safe` for a
//! synthetic declared custody projection under the premises this file names.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal an unmodeled return expression keeps where this card's premises
/// are not met. Identical text to wave 33's, deliberately: this card withdraws
/// the refusal for more shapes and reworded none.
const RETURN: &str = "unmodeled call, aggregate or indirect access in return expression";

/// The reserved synthetic destination. Spelled here so the collision arm can
/// put a body that names it under test.
const RESERVED: &str = "__qcore_returned_value";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
void *q_realloc(void *p, __SIZE_TYPE__ size); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p); \
void *r_wrap(void *p); \
int z_unknown(int n);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "q_realloc.qcore".to_owned(),
                include_str!("fixtures/contracts/q_realloc.qv").to_owned(),
            ),
            (
                "r_return_call.qcore".to_owned(),
                include_str!("fixtures/contracts/r_return_call.qv").to_owned(),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

#[derive(Debug)]
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

    fn has_synthetic_unknown_alias(&self) -> bool {
        self.diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::UnknownAlias { variable } if variable == RESERVED)
        })
    }

    /// The comparable content of a run: everything a consumer reads, with the
    /// diagnostics sorted so the comparison is about content and not order.
    fn shape(&self) -> (AnalysisStatus, bool, Vec<String>) {
        let mut rendered = self
            .diagnostics
            .iter()
            .map(|kind| format!("{kind:?}"))
            .collect::<Vec<_>>();
        rendered.sort();
        (self.status, self.certified, rendered)
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
            eprintln!("skipping Clang-dependent return-call test: {reason}");
            false
        }
    }
}

/// The card's central property, as a harness.
///
/// `returned` is a body ending `return f(args);`; `desugared` is the same body
/// with that statement written out as `T = f(args); return T;`. The two must
/// agree on status, on certification and on every diagnostic — that is what
/// "the rule is a desugaring" means, measured rather than asserted.
///
/// The harness additionally refuses to pass vacuously: a `returned` run that
/// kept the return refusal would not be exercising the admission at all, so
/// that is checked separately here.
fn desugaring_agrees(returned: &str, desugared: &str) -> Run {
    let admitted = run(returned);
    assert!(
        !admitted.refuses_return(),
        "the return-call admission must fire for `{returned}`, got {:?}",
        admitted.unsupported()
    );
    let hand_written = run(desugared);
    assert_eq!(
        admitted.shape(),
        hand_written.shape(),
        "`{returned}` must agree with its hand-written desugaring `{desugared}`"
    );
    admitted
}

// ---------------------------------------------------------------------------
// The admitting direction: the two admitted result modes
// ---------------------------------------------------------------------------

/// Result mode `Owned`. The call produces a fresh site and the exit escapes it,
/// so the body is `Safe` and leaks nothing — and it agrees with the desugaring.
///
/// This is the row wave 33's yield ground is about: before this card the whole
/// exit was one `Unknown`.
#[test]
fn an_owned_result_returned_directly_matches_its_desugaring() {
    if !clang_available() {
        return;
    }
    let admitted = desugaring_agrees(
        "void *subject(void) { return q_alloc(); }",
        "void *subject(void) { void *t = q_alloc(); return t; }",
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
    assert_eq!(admitted.leaks(), 0);
    assert!(admitted.certified);
}

/// Result mode `None`, over a parameter the callee **releases**. The release is
/// the call's own effect and applies exactly as it does in statement position;
/// the exit escapes nothing.
///
/// This is the shape wave 33's soundness ground named — a `Released` parameter
/// mode at a return position — and it is admitted here because the discharge is
/// the *call's*, not the return's. The desugaring comparison is what shows the
/// return position added nothing to it.
#[test]
fn a_released_parameter_at_a_return_matches_its_desugaring() {
    if !clang_available() {
        return;
    }
    let admitted = desugaring_agrees(
        "int subject(void) { void *p = q_alloc(); return r_release_code(p); }",
        "int subject(void) { void *p = q_alloc(); int t = r_release_code(p); return t; }",
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
    assert_eq!(admitted.leaks(), 0);
}

/// The same, for a `Consumed` parameter — custody moves to the callee without a
/// claim that it destroys the resource.
#[test]
fn a_consumed_parameter_at_a_return_matches_its_desugaring() {
    if !clang_available() {
        return;
    }
    let admitted = desugaring_agrees(
        "int subject(void) { void *p = q_alloc(); return r_consume_code(p); }",
        "int subject(void) { void *p = q_alloc(); int t = r_consume_code(p); return t; }",
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
    assert_eq!(admitted.leaks(), 0);
}

/// A borrowed parameter and an `Owned` result together: the argument keeps its
/// custody and the fresh result escapes.
#[test]
fn a_borrowed_argument_with_an_owned_result_matches_its_desugaring() {
    if !clang_available() {
        return;
    }
    let admitted = desugaring_agrees(
        "void *subject(void) { void *p = q_alloc(); void *w = r_wrap(p); q_free(p); return w; }",
        "void *subject(void) { void *p = q_alloc(); void *w = r_wrap(p); q_free(p); return w; }",
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
}

// ---------------------------------------------------------------------------
// The accusing direction: what the admission exposes
// ---------------------------------------------------------------------------

/// **The property that makes the widening safe.** `report_leaks` skips an
/// unmodeled exit, so the blanket `Unknown` this card withdraws was *hiding* a
/// leak at every one of these exits. Admitting the return exposes it.
///
/// The body acquires two resources, releases one through the returned call and
/// returns a fresh one — and still owes the third. Before this card the exit was
/// unmodeled and nothing was reported; the desugaring reports the leak, and so
/// must the `return` form.
#[test]
fn an_admitted_return_exposes_a_leak_the_blanket_refusal_hid() {
    if !clang_available() {
        return;
    }
    let admitted = desugaring_agrees(
        "void *subject(void) { void *p = q_alloc(); return q_alloc(); }",
        "void *subject(void) { void *p = q_alloc(); void *t = q_alloc(); return t; }",
    );
    assert_eq!(
        admitted.leaks(),
        1,
        "the retained acquisition must be reported at the now-modeled exit: {admitted:?}"
    );
    assert_ne!(admitted.status, AnalysisStatus::Safe);
    assert!(!admitted.certified);
}

/// Non-vacuity for the row above, in the direction that matters: the *same*
/// body with the leak removed is `Safe`. A leak count that fired for a reason
/// other than the retained acquisition would fire here too.
#[test]
fn the_exposed_leak_is_the_retained_acquisition_and_not_the_shape() {
    if !clang_available() {
        return;
    }
    let clean = run("void *subject(void) { void *p = q_alloc(); q_free(p); return q_alloc(); }");
    assert_eq!(clean.status, AnalysisStatus::Safe, "{clean:?}");
    assert_eq!(clean.leaks(), 0);
}

// ---------------------------------------------------------------------------
// The discharging direction: the `Reallocated` refusal
// ---------------------------------------------------------------------------

/// **The soundness arm wave 33 was right about.** A `Reallocated` result denotes
/// the identity the caller passed at the named parameter — `apply_contract_call`
/// records `conditional_replacements[fresh] = old` — so escaping it at the exit
/// would carry the *old* site's obligation out with it, discharging an
/// obligation this body already owed. The return position refuses it.
#[test]
fn a_reallocated_result_is_refused_at_a_return() {
    if !clang_available() {
        return;
    }
    let refused = run("void *subject(void *p) { return q_realloc(p, 8); }");
    assert!(
        refused.refuses_return(),
        "a reallocating callee must keep the return refusal: {:?}",
        refused.unsupported()
    );
    assert_ne!(refused.status, AnalysisStatus::Safe);
    assert!(!refused.certified);
}

/// Non-vacuity for the row above, and the sharpest form it can take: the *same
/// call* in a statement position is admitted. So the refusal is about the
/// return position's treatment of a `Reallocated` result — this card's rule —
/// and not about the callee, the contract, the fixture or Clang.
#[test]
fn the_reallocation_refusal_is_about_the_return_and_not_the_call() {
    if !clang_available() {
        return;
    }
    let statement = run("void *subject(void *p) { void *t = q_realloc(p, 8); return t; }");
    assert!(
        !statement.refuses_return(),
        "the same call in statement position must be admitted: {:?}",
        statement.unsupported()
    );
    assert!(
        !statement
            .unsupported()
            .iter()
            .any(|text| text.contains("reallocat")),
        "the statement form must carry no reallocation refusal: {:?}",
        statement.unsupported()
    );
}

// ---------------------------------------------------------------------------
// The shape premise: the call must be the whole return expression
// ---------------------------------------------------------------------------

/// A call whose result may carry custody, a nested call and the comma operator
/// keep the refusal. B01 admits only one resource-free call inside a scalar
/// expression; none of these shapes meets that rule.
#[test]
fn a_call_that_is_not_the_whole_return_expression_is_refused() {
    if !clang_available() {
        return;
    }
    // Deliberately re-pinned by card W45-RETURN-EXPRESSION-FAMILIES: a pointer
    // cast of an owned call whose declaration returns an object pointer keeps
    // the object (C11 6.3.2.3p7), so it is decided exactly as the uncast
    // `return q_alloc();` — the exit escapes the fresh site and nothing else.
    let cast = run("void *subject(void) { return (void *) q_alloc(); }");
    let uncast = run("void *subject(void) { return q_alloc(); }");
    assert!(!cast.refuses_return(), "{:?}", cast.unsupported());
    assert_eq!(
        (cast.status, cast.certified, cast.unsupported()),
        (uncast.status, uncast.certified, uncast.unsupported())
    );
    // Deliberate pin update, card CG02-ELABORATION-CORE (rule 10s): a comma
    // return is `r_release_code(p); return 0;` — the call is the classifier's
    // own statement, applied once, and the return escapes nothing.
    let comma = run("int subject(void) { void *p = q_alloc(); return r_release_code(p), 0; }");
    assert!(!comma.refuses_return(), "{:?}", comma.unsupported());
    assert!(comma.unsupported().is_empty(), "{:?}", comma.unsupported());
    // FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the
    // call is the `?:`'s deciding operand, applied once by the condition
    // route before either edge's `return 1;` / `return 0;`.
    let body = "int subject(void) { void *p = q_alloc(); return r_release_code(p) ? 1 : 0; }";
    let branched = run(body);
    assert!(!branched.refuses_return(), "{:?}", branched.unsupported());
    assert_eq!(branched.status, AnalysisStatus::Safe, "{body}");
    // Deliberate pin update, card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u):
    // `r_release_code(p) & 1` is one condition leaf, and the return operand
    // accepts exactly what the same leaf in an `if` accepts. B01 refused it
    // only lexically (its operand walker does not list `&`). The call is
    // applied once and the scalar exit escapes nothing.
    let leaf = run("int subject(void) { void *p = q_alloc(); return r_release_code(p) & 1; }");
    assert!(!leaf.refuses_return(), "{:?}", leaf.unsupported());
    assert!(leaf.unsupported().is_empty(), "{:?}", leaf.unsupported());
    let condition = run(
        "int subject(void) { void *p = q_alloc(); int r = 0; if (r_release_code(p) & 1) { r = 1; } return r; }",
    );
    assert_eq!(
        (leaf.status, leaf.unsupported()),
        (condition.status, condition.unsupported())
    );
    // Deliberately re-pinned by card W43-RETURN-EXPRESSION: an uncontracted
    // *whole* call whose argument is a contracted call is a whole-call return
    // the classifier refuses, so it is lowered to that refusal. The nested
    // release is not applied, nothing is admitted, and the function is not
    // `Safe`.
    let lowered =
        run("int subject(void) { void *p = q_alloc(); return z_unknown(r_release_code(p)); }");
    assert!(!lowered.refuses_return(), "{:?}", lowered.unsupported());
    assert!(
        lowered
            .unsupported()
            .contains(&"call to uncontracted function `z_unknown`".to_owned()),
        "{:?}",
        lowered.unsupported()
    );
    assert_ne!(lowered.status, AnalysisStatus::Safe);
    assert!(!lowered.certified);
}

/// B01's positive family: exactly one modeled resource-free call may sit
/// inside an otherwise custody-neutral scalar expression. Its parameter
/// effect is applied before the return, just as in the explicit two-statement
/// spelling.
#[test]
fn a_resource_free_call_is_evaluated_inside_a_scalar_return() {
    if !clang_available() {
        return;
    }
    let expression = "r_release_code(p) + 1";
    let lowered = run(&format!(
        "int subject(void) {{ void *p = q_alloc(); return {expression}; }}"
    ));
    let written = run("int subject(void) { void *p = q_alloc(); r_release_code(p); return 1; }");
    assert!(
        !lowered.refuses_return(),
        "{expression}: {:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.status, written.status, "{expression}");
    assert_eq!(lowered.unsupported(), written.unsupported(), "{expression}");
    assert_eq!(lowered.status, AnalysisStatus::Safe, "{expression}");
    assert!(!lowered.has_synthetic_unknown_alias(), "{expression}");
}

/// A fixed-point summary for a locally defined recursive function is not the
/// declared API-contract premise of B01. In particular, the older W4
/// pointer-actual refusal remains load-bearing for this shape.
#[test]
fn a_local_summary_does_not_admit_an_embedded_recursive_call() {
    if !clang_available() {
        return;
    }
    let refused = run("int subject(void *p) {
            if (p == 0) return 0;
            return 1 + subject(p);
        }");
    assert_eq!(refused.status, AnalysisStatus::Unknown);
    assert!(refused.refuses_return());
}

/// A conditionally evaluated call cannot be extracted and applied on every
/// path. Doing so would release `p` even when C skips the call, turning this
/// body falsely Safe. FLIPPED by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule
/// 10z): both are lowered by branching, the call applied only on the edge C
/// evaluates it, so the skipping edge's leak is reported (BEFORE: the blanket
/// return refusal). Neither is Safe.
#[test]
fn a_skippable_call_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "int subject(int flag) { void *p = q_alloc(); return flag ? r_release_code(p) : 0; }",
        "int subject(int flag) { void *p = q_alloc(); return flag && r_release_code(p); }",
    ] {
        let measured = run(body);
        assert!(!measured.refuses_return(), "{body}");
        assert_eq!(measured.status, AnalysisStatus::Unsafe, "{body}");
        assert_eq!(measured.leaks(), 1, "{body}");
    }
}

/// B01 also uses the reserved synthetic exit place. A source binding with that
/// spelling must refuse before the scalar-call path, just as it does for the
/// pre-existing whole-call lowering.
#[test]
fn the_scalar_call_path_keeps_the_synthetic_place_collision_guard() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "int subject(void) {{ void *{RESERVED} = q_alloc(); return r_release_code({RESERVED}) + 1; }}"
    );
    let measured = run(&body);
    assert!(measured.refuses_return(), "{:?}", measured.unsupported());
    assert_ne!(measured.status, AnalysisStatus::Safe);
}

/// The admission must not turn the old blanket refusal into a way to hide a
/// live obligation. A borrowed call changes no custody, so the allocation is
/// still reported at the now-modeled exit.
#[test]
fn the_scalar_call_admission_exposes_a_retained_obligation() {
    if !clang_available() {
        return;
    }
    let measured = run("int subject(void) { void *p = q_alloc(); return r_borrow_code(p) + 1; }");
    assert!(!measured.refuses_return(), "{:?}", measured.unsupported());
    assert_eq!(measured.status, AnalysisStatus::Unsafe);
    assert_eq!(measured.leaks(), 1);
    assert!(!measured.has_synthetic_unknown_alias());
    assert!(!measured.certified);
}

/// **The shape that makes the whole-expression premise load-bearing, measured
/// rather than assumed.**
///
/// `classify_simple`'s own extent guard —
/// `mentions_formal && (!direct_call || call_end != Some(len - 1))` — is
/// conditioned on the statement naming a **protected** name, so for a call whose
/// arguments name no tracked identity it does not fire. Without this card's
/// premise the tail after the call's closing parenthesis would then be silently
/// dropped and the call modeled as if it were the whole expression: the exit
/// would escape the site `q_alloc()` produced while the C returns `other`, and
/// the body below — which allocates, returns something else, and leaks — was
/// measured **`Safe`** with the premise mutated away.
///
/// That is a false `Safe`, invariant 1's forbidden direction, so this row is the
/// premise's soundness evidence and not a shape preference. Both spellings put
/// an untracked value after the call: the comma operator and a conditional.
#[test]
fn a_tail_after_the_call_cannot_launder_the_produced_site() {
    if !clang_available() {
        return;
    }
    for body in [
        "void *subject(void) { void *other = 0; return q_alloc(), other; }",
        "void *subject(void) { void *other = 0; return q_alloc() ? other : other; }",
    ] {
        let refused = run(body);
        assert!(
            refused.refuses_return(),
            "`{body}` must keep the return refusal, got {:?}",
            refused.unsupported()
        );
        assert_ne!(
            refused.status,
            AnalysisStatus::Safe,
            "`{body}` allocates, returns something else and leaks — it must never be Safe"
        );
        assert!(!refused.certified, "{body}");
    }
}

/// Non-vacuity for the row above: the innermost call of each refused shape is
/// admitted when it *is* the whole expression. A refusal that held for an
/// unrelated reason — an unresolved callee, a contract the loader dropped —
/// would refuse here too.
#[test]
fn the_whole_expression_premise_is_what_refuses_those_shapes() {
    if !clang_available() {
        return;
    }
    let admitted = run("int subject(void) { void *p = q_alloc(); return r_release_code(p); }");
    assert!(
        !admitted.refuses_return(),
        "the same call as the whole expression must be admitted: {:?}",
        admitted.unsupported()
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
}

// ---------------------------------------------------------------------------
// The collision premise
// ---------------------------------------------------------------------------

/// The synthetic destination shares a namespace with this body's C names, so a
/// body that spells it would have its own binding overwritten. The admission
/// requires the name to be absent and refuses otherwise — fail-closed, and
/// checked rather than argued from C11 §7.1.3's reservation alone.
#[test]
fn a_body_that_spells_the_reserved_place_is_refused() {
    if !clang_available() {
        return;
    }
    let body = format!(
        "void *subject(void) {{ void *{RESERVED} = q_alloc(); q_free({RESERVED}); return q_alloc(); }}"
    );
    let refused = run(&body);
    assert!(
        refused.refuses_return(),
        "a body spelling the reserved place must keep the return refusal: {:?}",
        refused.unsupported()
    );
}

/// Non-vacuity for the collision row: the identical body with the local renamed
/// is admitted. So the refusal is the collision guard and not the body's shape.
#[test]
fn the_collision_refusal_is_the_guard_and_not_the_body() {
    if !clang_available() {
        return;
    }
    let admitted =
        run("void *subject(void) { void *keep = q_alloc(); q_free(keep); return q_alloc(); }");
    assert!(
        !admitted.refuses_return(),
        "the renamed body must be admitted: {:?}",
        admitted.unsupported()
    );
    assert_eq!(admitted.status, AnalysisStatus::Safe);
}

// ---------------------------------------------------------------------------
// The uncontracted callee: bucket (2) is empty by construction
// ---------------------------------------------------------------------------

/// A callee with no contract in force is **not** admitted by this card's
/// route: nothing here models its call, so this card's bucket (2) stays empty.
///
/// **Deliberately re-pinned by card W43-RETURN-EXPRESSION.** This row used to
/// assert that the return-position refusal was kept verbatim, so that clearing
/// it could never be *published as progress* by exchanging it for a `call to
/// uncontracted function` leaf. W43's corpus census found that exchange to be
/// the dominant refused family (the call, not the return, is what blocks those
/// functions) and lowers it to the classifier's own refusal
/// (`return_call::lowers_refused_whole_call`), explicitly as a re-attribution
/// that decides nothing: the function stays non-`Safe`, uncertified, and no
/// obligation is discharged. What this row now pins is exactly that: the W4
/// route still admits nothing, and the published reason names the callee.
#[test]
fn an_uncontracted_callee_is_refused_by_the_call_classifier() {
    if !clang_available() {
        return;
    }
    let refused = run("int u_unknown(int n); int subject(void) { return u_unknown(1); }");
    assert!(
        !refused.refuses_return(),
        "W43 lowers a refused whole call to the classifier's refusal: {:?}",
        refused.unsupported()
    );
    assert_eq!(
        refused.unsupported(),
        vec!["call to uncontracted function `u_unknown`".to_owned()],
        "the published reason is the call classifier's"
    );
    assert_ne!(refused.status, AnalysisStatus::Safe);
    assert!(!refused.certified);
}
