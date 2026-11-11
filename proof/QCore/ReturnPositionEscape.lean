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

import QCore.ChainedAccess

/-!
# The pointer-valued chain leaf in a `return` (C-GATEWAY-RETURN-POSITION)

Card REACH-GATE admitted a pointer-valued bounded typed access chain
(`QCore.ChainedAccess.admitsChain`, unchanged here) only where the position
discards the value into a control-flow decision, refusing it in a `return`. The
recorded ground was that `legacy_c`'s `EscapeIfOwned` models only a *canonical
place*, so "a compound return expression that produced a pointer would escape it
unmodeled".

Card C-GATEWAY-RETURN-POSITION **supplies that argument's missing answer rather
than loosening it**, and this module is the argument. The chain rule's base
premise is `¬ baseBound` — `a_bound_root_never_admits_a_chain` — while every
place the projection tracks is rooted at a name the body binds (card O04-B08's
aggregate store; `TrackedBasesAreBound` below states it as the premise it is).
The two together say that an admitted chain names **no tracked place**, so there
is nothing at that exit for an escape to carry.

## What is proved, and what is a premise

Proved, about the rule:

* the strict rule's answer is **unchanged** wherever it already answered, so a
  `return p;` still escapes `p` — the discharging direction is untouched;
* the widening replaces **refusals only**, and never manufactures an escape;
* an expression neither rule admits keeps its refusal — the recognition is
  positive and the fall-through is refusal, never admission;
* the widened rule is **accusing**: every obligation the narrow rule reported at
  an exit is still reported, and the exits that change report *more*, because a
  refusal suppressed reporting rather than resolving anything.

A premise, stated and not proved here: `TrackedBasesAreBound`, that the
projection tracks no place whose base the body does not bind. That is the
aggregate store's own construction fact, and `QCore.AggregateStore` is where it
lives; faithful extraction of the declaration facts a `Chain` records is the
same obligation `QCore.TypedFieldAccess` already carries.

## The measured boundary this module exists to carry

`the_two_answers_agree_on_every_admitted_chain` is the reason the Rust arm that
forces `returned_place := none` is **unobservable today**, which was measured by
mutation and not assumed: replacing it with the canonical place leaves the test
suite green. The arm is therefore a *backstop*, and
`without_the_base_premise_the_answers_part` is why it is kept rather than
simplified away — drop the base premise and the two answers separate at once, in
the discharging direction invariant 1 forbids. Residual
`RETURN-POSITION-ESCAPE-UNREACHABLE` records this: the arm's justification is
this theorem, never the green suite.
-/

namespace QCore.ReturnPositionEscape

open QCore.ChainedAccess

/-! ## The projection's side -/

/-- What a body still owes at one exit. One bit is enough: every theorem here is
    about whether the obligation survives the exit, never about which resource
    it is. -/
structure ExitState where
  /-- An acquisition of this body is live at this exit. -/
  liveObligation : Bool
  deriving DecidableEq

/-- Card O04-B08's aggregate store invariant, stated as the premise it is: every
    place the projection tracks is rooted at a name the body binds. A
    body-acquired name is bound by construction, which is what gives this its
    content. -/
def TrackedBasesAreBound (tracked : Chain → Bool) : Prop :=
  ∀ c, tracked c = true → c.baseBound = true

/-- `EscapeIfOwned`: the obligation leaves with the returned value exactly when
    the projection tracks the place named. A place it does not track is a
    complete no-op — that is the `IfOwned` in the name. -/
def escapeIfOwned (tracked : Chain → Bool) (c : Chain) (s : ExitState) : ExitState :=
  if tracked c then { liveObligation := false } else s

/-! ## An admitted chain names no tracked place -/

/-- The card's soundness argument, in one step: the chain rule refuses a bound
    base, and every tracked place has one. -/
theorem an_admitted_chain_names_no_tracked_place
    (tracked : Chain → Bool) (inv : TrackedBasesAreBound tracked)
    (c : Chain) (adm : admitsChain c = true) : tracked c = false := by
  cases h : tracked c with
  | false => rfl
  | true =>
    have bound : c.baseBound = true := inv c h
    have : admitsChain c = false := a_bound_root_never_admits_a_chain c bound
    rw [this] at adm
    exact absurd adm (by simp)

/-- Consequently an escape at such a place carries nothing away. -/
theorem escaping_an_admitted_chain_changes_nothing
    (tracked : Chain → Bool) (inv : TrackedBasesAreBound tracked)
    (c : Chain) (adm : admitsChain c = true) (s : ExitState) :
    escapeIfOwned tracked c s = s := by
  have : tracked c = false := an_admitted_chain_names_no_tracked_place tracked inv c adm
  simp [escapeIfOwned, this]

/-! ## The two answers the parser could give -/

/-- What the `return` node becomes. -/
inductive ReturnInstruction where
  /-- `Unknown`: the `unsupported` leaf, and the path marked unmodeled, so
      `report_leaks` skips this exit. -/
  | refuse
  /-- `EscapeIfOwned` at the place `canonical_place` spelled. -/
  | escape
  /-- A resource-neutral exit. -/
  | noop
  deriving DecidableEq

/-- One `return` position, as the parser sees it. -/
structure ReturnPosition where
  /-- `ExpressionRole::Returned` — the strict rule, unchanged by this card. -/
  strictAdmits : Bool
  /-- `ExpressionRole::Decides` — the same walk with the pointer leaf allowed. -/
  decidesAdmits : Bool
  /-- `canonical_place` spells a place for this expression. Note it can spell
      `view->head->next` perfectly well, which is exactly the hazard. -/
  canonicalPlace : Bool
  deriving DecidableEq

/-- Before the card. -/
def narrow (p : ReturnPosition) : ReturnInstruction :=
  if p.strictAdmits then (if p.canonicalPlace then .escape else .noop) else .refuse

/-- After it. The strict rule is asked **first** and its answer is never
    weakened; only when it refuses does the widening get a say, and what it
    admits is answered `noop` — never `escape`, whatever `canonical_place`
    would have spelled. -/
def widened (p : ReturnPosition) : ReturnInstruction :=
  if p.strictAdmits then (if p.canonicalPlace then .escape else .noop)
  else if p.decidesAdmits then .noop
  else .refuse

/-! ## What the widening does and does not do -/

/-- The ordering premise: wherever the strict rule answered, the answer stands.
    This is what keeps `return p;` escaping `p`. -/
theorem the_strict_answer_is_unchanged (p : ReturnPosition)
    (strict : p.strictAdmits = true) : widened p = narrow p := by
  simp [widened, narrow, strict]

/-- The widening replaces refusals and nothing else. -/
theorem the_widening_only_replaces_refusals (p : ReturnPosition)
    (moved : widened p ≠ narrow p) : narrow p = .refuse ∧ widened p = .noop := by
  cases strict : p.strictAdmits with
  | true => exact absurd (the_strict_answer_is_unchanged p strict) moved
  | false =>
    cases decides : p.decidesAdmits with
    | true => exact ⟨by simp [narrow, strict], by simp [widened, strict, decides]⟩
    | false =>
      exact absurd (by simp [widened, narrow, strict, decides]) moved

/-- **Invariant 1's direction.** The widening never manufactures an escape: a
    discharging exit under the widened rule was already one under the narrow
    rule. This is the theorem the Rust arm forcing `returned_place := none`
    implements. -/
theorem the_widening_never_creates_an_escape (p : ReturnPosition)
    (esc : widened p = .escape) : narrow p = .escape := by
  cases strict : p.strictAdmits with
  | true => rw [← the_strict_answer_is_unchanged p strict]; exact esc
  | false =>
    cases decides : p.decidesAdmits with
    | true => simp [widened, strict, decides] at esc
    | false => simp [widened, strict, decides] at esc

/-- Positive recognition: an expression neither rule admits keeps its refusal.
    There is no fall-through to an admission. -/
theorem an_unadmitted_expression_keeps_its_refusal (p : ReturnPosition)
    (strict : p.strictAdmits = false) (decides : p.decidesAdmits = false) :
    widened p = .refuse := by
  simp [widened, strict, decides]

/-! ## The accusing direction -/

/-- What a completed exit reports. A refusal reports nothing — it marks the path
    unmodeled and `report_leaks` skips it — and an escape reports nothing
    because the obligation left with the value. Only a resource-neutral exit
    reports what the body still owes. -/
def reported (i : ReturnInstruction) (s : ExitState) : Bool :=
  match i with
  | .refuse => false
  | .escape => false
  | .noop => s.liveObligation

/-- The widened rule reports at least what the narrow rule reported. -/
theorem the_widening_reports_at_least_as_much (p : ReturnPosition) (s : ExitState)
    (rep : reported (narrow p) s = true) : reported (widened p) s = true := by
  cases strict : p.strictAdmits with
  | true => rw [the_strict_answer_is_unchanged p strict]; exact rep
  | false => simp [narrow, strict, reported] at rep

/-- And where the two differ, the widened rule is the *accusing* one: it can
    only turn an unreported obligation into a reported one, never the reverse. -/
theorem the_widening_is_accusing (p : ReturnPosition) (s : ExitState)
    (was : reported (narrow p) s = false) (now : reported (widened p) s = true) :
    narrow p = .refuse ∧ widened p = .noop := by
  cases strict : p.strictAdmits with
  | true =>
    rw [the_strict_answer_is_unchanged p strict] at now
    exact absurd now (by rw [was]; simp)
  | false =>
    cases decides : p.decidesAdmits with
    | true => exact ⟨by simp [narrow, strict], by simp [widened, strict, decides]⟩
    | false => simp [widened, strict, decides, reported] at now

/-- Non-vacuity of the accusing direction: an exit the card really moves, whose
    live obligation the old refusal suppressed and the new `noop` reports. This
    is the shape of every leak the corpus re-pin adds. -/
theorem the_accusing_direction_is_not_vacuous :
    ∃ p s, reported (narrow p) s = false ∧ reported (widened p) s = true := by
  refine ⟨{ strictAdmits := false, decidesAdmits := true, canonicalPlace := true },
          { liveObligation := true }, ?_, ?_⟩ <;> simp [narrow, widened, reported]

/-! ## Why the escape arm is a backstop, and why it is kept -/

/-- **`RETURN-POSITION-ESCAPE-UNREACHABLE`.** On every chain the rule admits,
    answering with the canonical place and answering with `none` agree: the
    escape is a no-op there. This is why a mutation replacing the arm leaves the
    suite green — the arm is justified by this theorem, not by a test. -/
theorem the_two_answers_agree_on_every_admitted_chain
    (tracked : Chain → Bool) (inv : TrackedBasesAreBound tracked)
    (c : Chain) (adm : admitsChain c = true) (s : ExitState) :
    escapeIfOwned tracked c s = s :=
  escaping_an_admitted_chain_changes_nothing tracked inv c adm s

/-- And why the arm is kept rather than simplified away: the agreement above is
    a consequence of the base premise, not of the escape itself. Drop the
    premise — as a future card widening the chain rule's base would — and the
    two answers separate immediately, in the discharging direction. The witness
    is a tracked place with a bound base: exactly what the premise excludes. -/
theorem without_the_base_premise_the_answers_part :
    ∃ (tracked : Chain → Bool) (c : Chain) (s : ExitState),
      tracked c = true ∧ escapeIfOwned tracked c s ≠ s := by
  refine ⟨fun _ => true,
          { baseMatches := true, baseBound := true, declarationsRead := true,
            links := [], continuesAfterLast := false },
          { liveObligation := true }, rfl, ?_⟩
  simp [escapeIfOwned]

/-- The same fact from the rule's side, closing the loop: such a witness is
    never admitted, so the separation above is unreachable *while the premise
    holds*. -/
theorem the_parting_witness_is_never_admitted
    (c : Chain) (bound : c.baseBound = true) : admitsChain c = false :=
  a_bound_root_never_admits_a_chain c bound

end QCore.ReturnPositionEscape
