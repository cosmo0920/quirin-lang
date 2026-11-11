-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-!
# A refused whole-call `return` is lowered to its own refusal (W43-RETURN-EXPRESSION)

The W43 shape census measured the refused return expressions of the pinned
corpus in the analyzer's own token view. Its dominant family is one call
`NAME ( … )` spanning the whole expression whose synthetic assignment
`PLACE = NAME ( … );` the ordinary statement classifier **refuses** — mostly an
uncontracted callee. Before this card the `return` node was the position's own
refusal, which names none of those causes.

`return f(a);` and `{ T = f(a); return T; }` are the same C. Card
C-GATEWAY-RETURN-CALL-W4 performs that desugaring for an *admitted* call
(`QCore.ReturnCallCustody.the_lowering_is_the_desugaring`). This card performs
it for a *refused* one and keeps the classifier's answer verbatim: the
evaluation node is the classifier's `Unknown`, and the exit is a `NoOp`.

## What is proved, about the rule

* the lowering replaces the return-position refusal and **nothing else**: an
  admitted classification, any other shape, a type operator and a spelled
  reserved place all keep exactly the answer they had;
* a lowered `return` is **still a refusal**: its path is poisoned, so the
  function can never be `Safe` through it — a function that loses the return
  refusal moves on to its remaining obligations and is not thereby `Safe`;
* the exit reports **exactly** the leaks the old refusal reported: a refusal
  touches no resource state, and `report_leaks` runs at every exit, so neither
  form hides or manufactures an accusation;
* the lowered exit **escapes nothing**, so no obligation is discharged on this
  route (invariant 1);
* the probe **restores the site counter**, so site identities are unchanged;
* the rule is **not vacuous**: a witness moves, and the published reason is the
  classifier's own.

## Premises, stated and not proved here

Faithful C token extraction, the classifier's own refusal decisions, and the
correspondence between this model and `legacy_c.rs` /
`legacy_return_call::lowers_refused_whole_call` are review premises under
standing constraint 28. Nothing here claims anything about the callee, its
contract, C aliasing, or whether an exit is concretely reachable.
-/

namespace QCore.ReturnRefusedCallLowering

/-- What the ordinary statement classifier answers for `PLACE = f(args);`. -/
inductive Classified where
  /-- An instruction some admitting route (W4, B01) may accept. -/
  | admitted
  /-- `LegacyInstruction::Unknown` with the classifier's own description. -/
  | refused (description : String)
  deriving DecidableEq, Repr

/-- One `return` position after the strict, pointer-leaf, W4 and B01 routes. -/
structure ReturnSite where
  /-- Some earlier route already admitted this expression. -/
  earlierRouteAdmitted : Bool
  /-- `return_call::whole_expression_call`: one call spans the expression. -/
  wholeCall : Bool
  /-- The callee spelling is a type operator or constant (`sizeof`, …). -/
  typeOperator : Bool
  /-- The body spells the reserved synthetic place. -/
  placeSpelled : Bool
  /-- The classifier's answer for the synthetic assignment. -/
  classified : Classified
  deriving DecidableEq, Repr

/-- The return-position refusal's wording. -/
def returnRefusal : String :=
  "unmodeled call, aggregate or indirect access in return expression"

/-- What the parser emits for the `return`. -/
inductive Lowering where
  /-- Whatever an earlier admitting route produced; unchanged by this card. -/
  | earlier
  /-- One `Unknown` exit node with the given description. -/
  | refuse (description : String)
  /-- The desugared pair: an `Unknown` evaluation node, then a `NoOp` exit. -/
  | lowered (description : String)
  deriving DecidableEq, Repr

/-- Before the card. -/
def before (r : ReturnSite) : Lowering :=
  if r.earlierRouteAdmitted then .earlier else .refuse returnRefusal

/-- `lowers_refused_whole_call`, as the parser asks it: only after every
    earlier route declined, only for a whole call to a real callee, only when
    the reserved place is unspelled, and only when the classifier refused. -/
def lowersRefusedWholeCall (r : ReturnSite) : Option String :=
  if r.wholeCall && !r.typeOperator && !r.placeSpelled then
    match r.classified with
    | .refused d => some d
    | .admitted => none
  else none

/-- After the card. -/
def after (r : ReturnSite) : Lowering :=
  if r.earlierRouteAdmitted then .earlier
  else match lowersRefusedWholeCall r with
    | some d => .lowered d
    | none => .refuse returnRefusal

/-! ## Replacement, and only replacement -/

theorem an_earlier_admission_is_unchanged (r : ReturnSite)
    (h : r.earlierRouteAdmitted = true) : after r = before r := by
  simp [after, before, h]

theorem the_lowering_only_replaces_the_return_refusal (r : ReturnSite)
    (moved : after r ≠ before r) :
    before r = .refuse returnRefusal ∧ ∃ d, after r = .lowered d ∧
      r.classified = .refused d := by
  cases hE : r.earlierRouteAdmitted with
  | true => exact absurd (an_earlier_admission_is_unchanged r hE) moved
  | false =>
    refine ⟨by simp [before, hE], ?_⟩
    cases hL : lowersRefusedWholeCall r with
    | none => exact absurd (by simp [after, before, hE, hL]) moved
    | some d =>
      refine ⟨d, by simp [after, hE, hL], ?_⟩
      unfold lowersRefusedWholeCall at hL
      split at hL
      · cases hC : r.classified with
        | refused d' => rw [hC] at hL; cases hL; rfl
        | admitted => rw [hC] at hL; cases hL
      · cases hL

theorem an_admitted_classification_is_never_lowered (r : ReturnSite)
    (h : r.classified = .admitted) : after r = before r := by
  cases hE : r.earlierRouteAdmitted <;>
    simp [after, before, lowersRefusedWholeCall, hE, h]

theorem another_shape_keeps_the_return_refusal (r : ReturnSite)
    (hE : r.earlierRouteAdmitted = false) (h : r.wholeCall = false) :
    after r = .refuse returnRefusal := by
  simp [after, lowersRefusedWholeCall, hE, h]

theorem a_type_operator_keeps_the_return_refusal (r : ReturnSite)
    (hE : r.earlierRouteAdmitted = false) (h : r.typeOperator = true) :
    after r = .refuse returnRefusal := by
  simp [after, lowersRefusedWholeCall, hE, h]

theorem a_spelled_reserved_place_keeps_the_return_refusal (r : ReturnSite)
    (hE : r.earlierRouteAdmitted = false) (h : r.placeSpelled = true) :
    after r = .refuse returnRefusal := by
  simp [after, lowersRefusedWholeCall, hE, h]

/-! ## A lowered return is still a refusal -/

/-- One exit's reachable outcome: whether its path carries an `Unsupported`
    refusal, and whether the exit escapes a tracked identity. -/
structure ExitOutcome where
  poisoned : Bool
  escapes : Bool
  deriving DecidableEq, Repr

/-- The custody content of the two emitted forms. A refusal node poisons its
    path and never touches a resource state; the lowered pair's `Unknown`
    evaluation node does the same before the `NoOp` exit, which escapes
    nothing. `earlier` is outside this card. -/
def outcome : Lowering → Option ExitOutcome
  | .earlier => none
  | .refuse _ => some { poisoned := true, escapes := false }
  | .lowered _ => some { poisoned := true, escapes := false }

/-- `report_leaks` at an exit, as `legacy_c.rs` runs it: at **every** exit,
    whatever the path carries, accusing each live site the exit does not carry
    away. A refusal does not suppress it; only an escape does. -/
def reportsLeak (o : ExitOutcome) (liveObligation : Bool) : Bool :=
  !o.escapes && liveObligation

theorem a_lowered_return_is_still_poisoned (d : String) :
    (outcome (.lowered d)).map (·.poisoned) = some true := rfl

theorem the_lowering_escapes_nothing (d : String) :
    (outcome (.lowered d)).map (·.escapes) = some false := rfl

/-- The lowered exit reports exactly what the return refusal reported: no leak
    is hidden that was reported, and none is manufactured. -/
theorem the_lowering_reports_exactly_what_the_refusal_reported
    (d e : String) (live : Bool) :
    ((outcome (.lowered d)).map fun o => reportsLeak o live) =
      ((outcome (.refuse e)).map fun o => reportsLeak o live) := rfl

/-- A function is `Safe` only when no exit's path is poisoned. -/
def functionSafe (exits : List ExitOutcome) : Bool :=
  exits.all fun o => !o.poisoned

/-- **Not thereby Safe.** Whatever the other exits are, a function carrying a
    lowered `return` is not `Safe`. -/
theorem a_function_with_a_lowered_return_is_never_safe
    (d : String) (others : List ExitOutcome) (o : ExitOutcome)
    (h : outcome (.lowered d) = some o) :
    functionSafe (o :: others) = false := by
  simp [outcome] at h
  subst h
  simp [functionSafe]

/-- And the lowering never promotes: wherever the card changes the answer, the
    function's `Safe` bit is the same before and after — false. -/
theorem the_lowering_never_promotes (r : ReturnSite) (others : List ExitOutcome)
    (ob oa : ExitOutcome)
    (hb : outcome (before r) = some ob) (ha : outcome (after r) = some oa) :
    functionSafe (oa :: others) = functionSafe (ob :: others) := by
  have pb : ob.poisoned = true := by
    cases h : before r <;> rw [h] at hb <;> simp [outcome] at hb <;> (subst hb; rfl)
  have pa : oa.poisoned = true := by
    cases h : after r <;> rw [h] at ha <;> simp [outcome] at ha <;> (subst ha; rfl)
  simp [functionSafe, pb, pa]

/-! ## Site identity -/

/-- The parser's site counter across one speculative classification: the probe
    may draw sites, and the rule restores the counter whatever it decides. -/
def probe (nextSite drawn : Nat) : Nat × Nat :=
  let during := nextSite + drawn
  (during, nextSite)

theorem the_site_counter_is_restored (nextSite drawn : Nat) :
    (probe nextSite drawn).2 = nextSite := rfl

/-! ## Non-vacuity -/

def uncontractedWitness : ReturnSite where
  earlierRouteAdmitted := false
  wholeCall := true
  typeOperator := false
  placeSpelled := false
  classified := .refused "call to uncontracted function `f`"

/-- The rule moves a real witness, and the published reason is the
    classifier's own rather than the position's. -/
theorem the_lowering_is_not_vacuous :
    after uncontractedWitness = .lowered "call to uncontracted function `f`" ∧
      before uncontractedWitness = .refuse returnRefusal := by
  decide

/-- The same witness under a cast (not a whole call) keeps the refusal. -/
theorem the_cast_witness_is_not_lowered :
    after { uncontractedWitness with wholeCall := false } = .refuse returnRefusal := by
  decide

end QCore.ReturnRefusedCallLowering
