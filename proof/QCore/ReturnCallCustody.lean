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
Card **C-GATEWAY-RETURN-CALL-W4**: the custody content of a modeled call in
`return` position.

Card `C-GATEWAY-RETURN-POSITION` (wave 33) withdrew `return CALL(args)` partly
on a **soundness** ground: applying a callee's parameter custody at a return
position would let a `Consumed`/`Released` mode *discharge* a live obligation of
the caller at that exit, which is invariant 1's forbidden direction. This module
is that ground's answer, stated about the one thing the return position itself
decides — what the exit does with the call's **result**.

The parameter half is deliberately **not** modelled here, and that is the point:
`apply_contract_call` applies parameter modes identically at every statement
position and has since Cycle 9, so the return position adds no trust to them.
`the_lowering_is_the_desugaring` is where that is stated formally — the one
admitted lowering is *definitionally* the two-statement form the analyzer already
decides, so whatever the call does to its arguments, it does it in the position
it always did.

What remains is the result, and `ResourceReturnMode` has exactly three
constructors, so the question is finite:

* `none` — the exit escapes nothing (`a_none_result_escapes_nothing`), so every
  obligation the body still holds survives to `report_leaks`. This is the
  *accusing* direction: the blanket `Unknown` this card withdraws marked the exit
  unmodeled, and `report_leaks` skips an unmodeled exit.
* `owned` — the result is a site drawn from the parser's monotonically
  increasing counter. `Wf` is that counter's invariant, and under it the produced
  site is **not a site of the pre-state**, so escaping it moves exactly that one
  site and nothing else
  (`an_owned_result_escapes_only_the_site_it_produced`).
* `reallocated p` — the result carries the replaced parameter's obligation, so
  escaping it would discharge `p`. **Refused.**

`admitted_modes_discharge_nothing_prior` is the theorem the card rests on, and
`a_reallocated_result_would_have_discharged_its_parameter` is its non-vacuity
partner in the style constraint 15n established for
`the_dropped_encoding_would_have_been_admitted`: the refused arm is refused
because admitting it *would* have moved a prior obligation, measured here rather
than asserted. Together with `only_reallocation_is_refused` — which says the
implemented gate refuses exactly that arm and no other — the three give
"admitted ⟺ discharges nothing prior" over the whole finite mode vocabulary.

The model is the custody map the gateway's `AbstractState` carries, reduced to
what this rule reads: a partial map from site to disposition, plus the site
counter. Nothing here claims anything about C aliasing, about the faithfulness
of any declared contract, or about whether an exit is concretely reachable;
those are the gateway's standing premises (invariant 2, constraint 4). The
implementation this models is `crates/qcore-c-gateway/src/legacy_return_call.rs`
and the `Statement::Return` lowering in `legacy_c.rs`, pinned behaviourally by
`crates/qcore-c-gateway/tests/return_position_modeled_call.rs`.
Record: `docs/reports/c-gateway-return-call-w4/README.md`.
-/

namespace QCore.ReturnCallCustody

/-- A resource identity the projection tracks. -/
abbrev Site := Nat

/-- What a site's custody looks like at one program point. -/
inductive Disposition where
  | live
  | released
  | escaped
  deriving DecidableEq, Repr

/-- The custody half of `AbstractState`, plus the parser's site counter. -/
structure State where
  custody : Site → Option Disposition
  nextSite : Site

/-- The counter's invariant: no site at or above it has been handed out.

    In the implementation this holds by construction — `produced_sites` are
    drawn from `Parser::next_site`, which only ever increases — and it is what
    makes a produced site fresh without any analysis. -/
def Wf (st : State) : Prop := ∀ s, st.nextSite ≤ s → st.custody s = none

/-- `ResourceReturnMode`, in full. The vocabulary is closed, which is why the
    case analysis below is a decision rather than a heuristic. -/
inductive ResultMode where
  | none
  | owned
  /-- On success the result replaces the resource passed at this site. -/
  | reallocated (param : Site)
  deriving DecidableEq, Repr

/-- `legacy_return_call::escapes_only_fresh_custody`. -/
def escapesOnlyFreshCustody : ResultMode → Bool
  | .none => true
  | .owned => true
  | .reallocated _ => false

/-- `apply_contract_call`'s `create_resource` on a site drawn from the counter. -/
def produce (st : State) : Site × State :=
  (st.nextSite,
    { custody := fun s => if s = st.nextSite then some .live else st.custody s,
      nextSite := st.nextSite + 1 })

/-- One site moves to `escaped`. -/
def markEscaped (st : State) (t : Site) : State :=
  { st with custody := fun s => if s = t then some .escaped else st.custody s }

/-- The result a call binds to its destination, and the state after the bind.
    A `none` result binds no tracked identity; the other two allocate. -/
def bindResult : ResultMode → State → Option Site × State
  | .none, st => (Option.none, st)
  | .owned, st => ((produce st).1, (produce st).2)
  | .reallocated _, st => ((produce st).1, (produce st).2)

/-- `return PLACE;` — `EscapeIfOwned` on whatever the place holds.

    The `reallocated` line is what the refusal is about: the replaced
    parameter's obligation travels out with the result, so the exit discharges
    `p` as well as the fresh site. It is written out here precisely so that the
    refusal can be shown non-vacuous rather than merely stipulated. -/
def returnPlace : ResultMode → Option Site → State → State
  | _, Option.none, st => st
  | .none, some _, st => st
  | .owned, some s, st => markEscaped st s
  | .reallocated p, some s, st => markEscaped (markEscaped st s) p

/-- `T = f(args); return T;` — the two-statement form, written by hand. -/
def assignThenReturn (m : ResultMode) (st : State) : State :=
  returnPlace m (bindResult m st).1 (bindResult m st).2

/-- `return f(args);` — the one-statement form this card lowers. -/
def returnCall (m : ResultMode) (st : State) : State :=
  returnPlace m (bindResult m st).1 (bindResult m st).2

/-- **The rule is a desugaring, not a new semantics.**

    This is `rfl`, and that is the whole content: the lowering the card installs
    *is* the two-statement form, so `observe_exit` needs no arm for a call
    result and the parameter modes are applied by the position that always
    applied them. -/
theorem the_lowering_is_the_desugaring (m : ResultMode) (st : State) :
    returnCall m st = assignThenReturn m st := rfl

/-- A `none` result escapes nothing at all: the state is untouched.

    So a body that still owes a resource at such an exit still owes it after
    the exit, and `report_leaks` — which skips an *unmodeled* exit — now sees
    it. That is the accusing direction this widening moves in. -/
theorem a_none_result_escapes_nothing (st : State) :
    returnCall .none st = st := rfl

/-- The produced site is the counter's value, and under `Wf` the pre-state says
    nothing about it. Freshness is structural, not analysed. -/
theorem a_produced_site_is_absent_from_the_pre_state (st : State) (hw : Wf st) :
    st.custody (produce st).1 = Option.none :=
  hw st.nextSite (Nat.le_refl _)

/-- **The theorem the card rests on.** For every admitted result mode, the
    return position's own step leaves every site of the pre-state exactly as it
    found it — so it discharges no obligation this body already owed. -/
theorem admitted_modes_discharge_nothing_prior
    (m : ResultMode) (st : State) (s : Site)
    (hm : escapesOnlyFreshCustody m = true) (hs : s < st.nextSite) :
    (returnCall m st).custody s = st.custody s := by
  have hne : s ≠ st.nextSite := Nat.ne_of_lt hs
  cases m with
  | none => rfl
  | owned => simp [returnCall, bindResult, returnPlace, markEscaped, produce, hne]
  | reallocated p => simp [escapesOnlyFreshCustody] at hm

/-- Sharper, for the one mode that can produce custody: **exactly one** site
    moves, and it is the site the call itself produced. -/
theorem an_owned_result_escapes_only_the_site_it_produced (st : State) :
    (returnCall .owned st).custody st.nextSite = some .escaped ∧
      ∀ s, s ≠ st.nextSite → (returnCall .owned st).custody s = st.custody s := by
  refine ⟨by simp [returnCall, bindResult, returnPlace, markEscaped, produce], ?_⟩
  intro s hs
  simp [returnCall, bindResult, returnPlace, markEscaped, produce, hs]

/-- **Non-vacuity of the refusal**, in the style of constraint 15n's
    `the_dropped_encoding_would_have_been_admitted`: the refused arm is refused
    because admitting it really would have moved a prior obligation.

    A `reallocated p` result whose parameter was live before the call leaves `p`
    **escaped** after the exit — a discharge of custody the body already owed,
    which is exactly invariant 1's forbidden direction.

    All three conjuncts are stated together so the claim cannot be read as
    weaker than it is: `p` is a site of the pre-state, it was **live** there,
    and the exit would have left it **escaped**. The middle conjunct is what
    makes this a change of state rather than a relabelling of something already
    discharged. -/
theorem a_reallocated_result_would_have_discharged_its_parameter
    (st : State) (p : Site) (hp : p < st.nextSite)
    (hlive : st.custody p = some .live) :
    p < st.nextSite ∧ st.custody p = some .live ∧
      (returnCall (.reallocated p) st).custody p = some .escaped := by
  refine ⟨hp, hlive, ?_⟩
  simp [returnCall, bindResult, returnPlace, markEscaped, produce]

/-- The implemented gate refuses **exactly** the reallocating arm — no more, so
    the admission is not narrower than the soundness argument supports, and no
    less, so it is not wider. With the two theorems above this closes the
    vocabulary: admitted ⟺ discharges nothing prior. -/
theorem only_reallocation_is_refused (m : ResultMode) :
    escapesOnlyFreshCustody m = false ↔ ∃ p, m = .reallocated p := by
  cases m with
  | none => simp [escapesOnlyFreshCustody]
  | owned => simp [escapesOnlyFreshCustody]
  | reallocated p => simp [escapesOnlyFreshCustody]

end QCore.ReturnCallCustody
