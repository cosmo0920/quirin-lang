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

import QCore.DeclaredAbiAgreement

/-!
# Declared-contract / derived-summary custody **mode** agreement

Card **LOADER-ABI-MODES**, the named continuation of card LOADER-ABI, which
compared the *types* a declaration writes against the C declaration in the
analysed unit and recorded its own limit:

> this card compares **types**. It does not compare a declared *mode* against
> anything the source says.

A type-faithful declaration may still say `resource_borrowed(p)` for a body that
frees `p`, `resource_released(p)` for a body that never releases it, or
`resource_return_none()` for a body that hands back a fresh acquisition. Each is
admitted by the type gate and each buys a **caller** a `Safe` verdict the body
contradicts.

This is the model of the comparison
(`crates/qcore-c-gateway/src/legacy_declared_modes.rs`) and of what a
contradiction does.

## What is proved

* **Only contradictions refuse.** A name this unit does not define, and a body
  that yields no validated summary, are compared against nothing and their
  declarations apply exactly as before. Absence of evidence is not evidence —
  requiring validation before applying a contract would withdraw the whole
  declared-trust architecture, and this decision does not.
* **A faithful declaration is never withdrawn**, for *any* contract: the
  relation is reflexive-free, so the gate cannot refuse what it agrees with.
* **Every contradiction class refuses, and each is reachable**, so no branch is
  vacuous, and the parameter and result classes are not one another in disguise.
* **A refinement inside one duty is not a contradiction**, in either direction:
  nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
  parameter change what the caller must additionally prove, never which side
  holds the resource after the call. This is where the decision deliberately
  stops.
* **Withdrawal is monotone in the refusing direction**, and *growing* the
  withdrawn set only ever removes applied contracts — which is what makes the
  implementation's fixed point sound in the direction that matters.
* **The relation is symmetric on duties**: a contradiction is a statement about
  a pair, not about which side is called "declared", so it cannot be
  accidentally one-sided.
* **The whole finite duty domain is decided**, and reduced to one checksum the
  Rust test computes independently.

## What this is not

It is a model of a **decision**. That the derived summary really describes the
body, and that a validated W4 summary is sound under the environment's other
declared premises, are premises supplied from outside — the same trust boundary
every W4 summary already sits behind (`LOADER-ABI-MODES-RELATIVE`). A withdrawn
declaration says nothing about the C body either: it says this analysis will not
apply that declaration here.
-/

namespace QCore.DeclaredModeAgreement

/-- The resource-contract v1 parameter vocabulary. `nullable` and the owner
index are carried, because the point of the refinement theorems is that they are
*present* and still do not contradict. -/
inductive ParameterMode where
  | borrowed
  | owned (nullable : Bool)
  | consumed (nullable : Bool)
  | released (nullable : Bool)
  | out (nullable : Bool)
  | stored (owner : Nat) (nullable : Bool)
  | removed (owner : Nat) (nullable : Bool)
  deriving DecidableEq, BEq, Repr

/-- The result vocabulary. -/
inductive ResultMode where
  | absent
  | owned (nullable : Bool)
  | reallocated (parameter : Nat) (nullable : Bool)
  deriving DecidableEq, BEq, Repr

/-- A resource function contract: the modes, and nothing else. -/
structure Contract where
  parameters : List ParameterMode
  result : ResultMode
  deriving DecidableEq, Repr

/-- What a parameter mode promises the **caller** about custody.

This is the abstraction the comparison is taken in, chosen so that a difference
is a difference the caller can be wrong about. -/
inductive ParameterDuty where
  /-- The caller still holds the resource after the call. -/
  | retained
  /-- The caller's direct duty ends at the call. -/
  | transferred
  /-- The callee writes fresh custody into the pointed-to slot. -/
  | delivered
  /-- The callee detaches the resource and hands direct custody back. -/
  | restored
  deriving DecidableEq, BEq, Repr

/-- What a result mode promises the caller. -/
inductive ResultDuty where
  | absent
  | produced
  deriving DecidableEq, BEq, Repr

def parameterDuty : ParameterMode → ParameterDuty
  | .borrowed | .owned _ => .retained
  | .released _ | .consumed _ | .stored _ _ => .transferred
  | .out _ => .delivered
  | .removed _ _ => .restored

def resultDuty : ResultMode → ResultDuty
  | .absent => .absent
  | .owned _ | .reallocated _ _ => .produced

/-- Why one declared contract's application is withdrawn for one unit. The
vocabulary is closed. -/
inductive Refusal where
  | parameterCustody
  | resultCustody
  deriving DecidableEq, BEq, Repr

def allRefusals : List Refusal := [.parameterCustody, .resultCustody]

/-- Whether any position of two equal-length mode vectors carries differing
duties. -/
def parametersContradict : List ParameterMode → List ParameterMode → Bool
  | [], [] => false
  | a :: as, b :: bs =>
      if parameterDuty a = parameterDuty b then parametersContradict as bs else true
  | _, _ => false

/-- The parameter axis.

A length difference decides **nothing** here: that is a statement about the
signature, which is card LOADER-ABI's gate, and for a function this unit defines
it is already decided there — a definition is a declaration. -/
def parameterAxis (declared derived : Contract) : Bool :=
  decide (declared.parameters.length = derived.parameters.length)
    && parametersContradict declared.parameters derived.parameters

/-- The result axis. -/
def resultAxis (declared derived : Contract) : Bool :=
  !decide (resultDuty declared.result = resultDuty derived.result)

/-- The whole relation, in the implementation's order: the parameter refusal is
reported in preference to the result one, so the key is a function of the pair
rather than of iteration order. -/
def contradiction (declared derived : Contract) : Option Refusal :=
  if parameterAxis declared derived then some .parameterCustody
  else if resultAxis declared derived then some .resultCustody
  else none

/-- A function name. Names are opaque, so they are modelled as indices. -/
abbrev Name := Nat

/-- One translation unit's view: the declared contracts, and the validated W4
summary derived from each function's own body here.

`derived n = none` covers both structurally uncheckable cases at once — the
unit does not define `n`, or it does and the body yields no validated summary —
because the decision is the same for both, and it is "compare nothing". -/
structure Environment where
  contracts : Name → Option Contract
  derived : Name → Option Contract

/-- Why `n`'s declaration is withdrawn in this unit, if it is. -/
def withdrawal (e : Environment) (n : Name) : Option Refusal :=
  match e.contracts n, e.derived n with
  | some declared, some derived => contradiction declared derived
  | _, _ => none

/-- The contract actually applied at this unit's call sites.

`carried` is the accumulated withdrawn set the implementation's fixed point
grows: a name in it stays withdrawn even when a later round can no longer derive
the summary that convicted it. -/
def applied (e : Environment) (carried : Name → Bool) (n : Name) : Option Contract :=
  if carried n || (withdrawal e n).isSome then none else e.contracts n

/-- The empty carried set: one round's decision on its own. -/
def noCarry : Name → Bool := fun _ => false

/-! ### Only contradictions refuse -/

/-- A name with no derived summary — undefined here, or defined with a body the
projection cannot summarise — is compared against nothing, and its declaration
applies exactly as it did before this card.

This is the half that keeps the declared-trust architecture. -/
theorem absence_of_evidence_applies_the_declaration {e : Environment} {n : Name}
    (h : e.derived n = none) : applied e noCarry n = e.contracts n := by
  unfold applied withdrawal noCarry
  cases e.contracts n <;> simp [h]

/-- Likewise for a name this environment declares nothing for: there is nothing
to apply and nothing to withdraw. -/
theorem an_undeclared_callee_is_never_applied {e : Environment} {c : Name → Bool} {n : Name}
    (h : e.contracts n = none) : applied e c n = none := by
  unfold applied withdrawal
  simp [h]

/-- **A faithful declaration is never withdrawn.** The relation is irreflexive
in the sense that matters: a derived summary equal to the declaration
contradicts nothing, for every contract without exception. -/
theorem equality_never_contradicts (k : Contract) : contradiction k k = none := by
  have hp : parametersContradict k.parameters k.parameters = false := by
    induction k.parameters with
    | nil => rfl
    | cons a as ih => simp [parametersContradict, ih]
  unfold contradiction parameterAxis resultAxis
  simp [hp]

theorem an_agreeing_summary_applies_the_declaration {e : Environment} {n : Name} {k : Contract}
    (hc : e.contracts n = some k) (hd : e.derived n = some k) :
    applied e noCarry n = some k := by
  unfold applied withdrawal noCarry
  simp [hc, hd, equality_never_contradicts]

/-! ### A contradiction withdraws -/

theorem a_contradiction_withdraws {e : Environment} {c : Name → Bool} {n : Name} {r : Refusal}
    (h : withdrawal e n = some r) : applied e c n = none := by
  unfold applied
  simp [h]

theorem a_compared_contradiction_withdraws {e : Environment} {c : Name → Bool} {n : Name}
    {declared derived : Contract} {r : Refusal}
    (hc : e.contracts n = some declared) (hd : e.derived n = some derived)
    (hr : contradiction declared derived = some r) : applied e c n = none := by
  refine a_contradiction_withdraws (r := r) ?_
  unfold withdrawal
  simp [hc, hd, hr]

/-! ### The three reproduced classes, decided -/

/-- Class 1: a declared `Borrowed` the body releases. -/
theorem a_borrowed_parameter_the_body_releases_contradicts :
    contradiction ⟨[.borrowed], .absent⟩ ⟨[.released false], .absent⟩
      = some .parameterCustody := by decide

/-- Class 2: a declared `Released` the body never performs. -/
theorem a_released_parameter_the_body_retains_contradicts :
    contradiction ⟨[.released true], .absent⟩ ⟨[.borrowed], .absent⟩
      = some .parameterCustody := by decide

/-- Class 3: a declared absent result the body produces. -/
theorem an_absent_result_the_body_produces_contradicts :
    contradiction ⟨[], .absent⟩ ⟨[], .owned true⟩ = some .resultCustody := by decide

/-- The `Out` and `Removed` duties are decided too, and are not the other
classes in disguise: each contradicts `retained` in both directions. -/
theorem a_delivered_slot_the_body_never_writes_contradicts :
    contradiction ⟨[.out false], .absent⟩ ⟨[.borrowed], .absent⟩
      = some .parameterCustody := by decide

theorem a_restored_child_the_body_never_detaches_contradicts :
    contradiction ⟨[.removed 0 false], .absent⟩ ⟨[.owned false], .absent⟩
      = some .parameterCustody := by decide

/-- **The general statement behind the table.** For a one-parameter contract,
the parameter axis refuses *exactly* when the two duties differ — not merely on
the sampled representatives. The relation is therefore the duty inequality, and
nothing else. -/
theorem a_single_parameter_contradicts_exactly_when_its_duty_differs
    (a b : ParameterMode) (r : ResultMode) :
    contradiction ⟨[a], r⟩ ⟨[b], r⟩
      = (if parameterDuty a = parameterDuty b then none else some .parameterCustody) := by
  by_cases h : parameterDuty a = parameterDuty b
  · simp [contradiction, parameterAxis, resultAxis, parametersContradict, h]
  · simp [contradiction, parameterAxis, parametersContradict, h]

/-- Likewise on the result axis, with the parameters held equal. -/
theorem a_result_contradicts_exactly_when_its_duty_differs (a b : ResultMode) :
    contradiction ⟨[], a⟩ ⟨[], b⟩
      = (if resultDuty a = resultDuty b then none else some .resultCustody) := by
  by_cases h : resultDuty a = resultDuty b
  · simp [contradiction, parameterAxis, resultAxis, parametersContradict, h]
  · simp [contradiction, parameterAxis, resultAxis, parametersContradict, h]

/-- When both axes contradict, the key is the parameter one: the refusal is a
function of the pair, not of iteration order. -/
theorem the_parameter_refusal_takes_precedence :
    contradiction ⟨[.borrowed], .absent⟩ ⟨[.released false], .owned false⟩
      = some .parameterCustody := by decide

/-- The vocabulary is closed and duplicate-free. -/
theorem the_refusal_vocabulary_has_two_members : allRefusals.length = 2 := by decide

theorem the_refusal_vocabulary_has_no_duplicates : allRefusals.Nodup := by decide

theorem every_refusal_is_in_the_vocabulary (r : Refusal) : r ∈ allRefusals := by
  cases r <;> simp [allRefusals]

/-- Non-vacuity of the whole vocabulary: every refusal is produced by a decided
witness, so no class is unreachable. -/
theorem every_refusal_is_reachable :
    allRefusals.all (fun r =>
      (contradiction ⟨[.borrowed], .absent⟩ ⟨[.released false], .absent⟩ == some r)
      || (contradiction ⟨[], .absent⟩ ⟨[], .owned true⟩ == some r)) = true := by decide

/-! ### A refinement inside one duty is not a contradiction -/

/-- Nullability never contradicts, in either direction. -/
theorem nullability_is_not_a_contradiction (a b : Bool) :
    contradiction ⟨[.released a], .absent⟩ ⟨[.released b], .absent⟩ = none := by
  cases a <;> cases b <;> decide

/-- An owner index never contradicts. -/
theorem an_owner_index_is_not_a_contradiction (i j : Nat) (a b : Bool) :
    contradiction ⟨[.stored i a], .absent⟩ ⟨[.stored j b], .absent⟩ = none := by
  unfold contradiction parameterAxis resultAxis
  simp [parametersContradict, parameterDuty, resultDuty]

/-- A `Reallocated` result and an `Owned` one both produce custody, so they
never contradict each other — while either contradicts `absent`. -/
theorem reallocation_and_ownership_do_not_contradict (i : Nat) (a b : Bool) :
    contradiction ⟨[], .owned a⟩ ⟨[], .reallocated i b⟩ = none := by
  unfold contradiction parameterAxis resultAxis
  simp [parametersContradict, resultDuty]

/-! ### Monotonicity, in both senses the implementation needs -/

/-- Withdrawal only removes: an applied contract is the declared one, never a
derived one and never a widened one. -/
theorem an_applied_contract_is_the_declared_one {e : Environment} {c : Name → Bool} {n : Name}
    {k : Contract} (h : applied e c n = some k) : e.contracts n = some k := by
  unfold applied at h
  split at h
  · exact absurd h (by simp)
  · exact h

/-- **The derived summary is never installed.** Whatever is applied came from
the declaration, so a summary weaker than the declaration it convicts — one
that dropped a non-null precondition, say — can never replace it. This is the
sentence that makes "refusal only, never repair" a fact rather than an
intention. -/
theorem a_derived_summary_is_never_applied {e : Environment} {c : Name → Bool} {n : Name}
    {k : Contract} (hc : e.contracts n = none) : applied e c n ≠ some k := by
  simp [an_undeclared_callee_is_never_applied (c := c) hc]

/-- **Growing the carried set only removes.** This is what makes the fixed point
sound in the direction that matters: a later round can acquit a name only by
losing the evidence that convicted it, and the accumulated set keeps it
withdrawn. -/
theorem growing_the_carried_set_only_removes {e : Environment} {c d : Name → Bool} {n : Name}
    (h : ∀ m, c m = true → d m = true) (hd : applied e d n ≠ none) :
    applied e c n = applied e d n := by
  have hdn : d n = false := by
    cases hx : d n with
    | false => rfl
    | true => exact absurd (by unfold applied; simp [hx]) hd
  have hcn : c n = false := by
    cases hx : c n with
    | false => rfl
    | true => simp [h n hx] at hdn
  unfold applied
  simp [hcn, hdn]

/-- Withdrawal only removes: whatever this unit applies, the environment
declared. There is no name the gate can add. -/
theorem withdrawal_only_removes {e : Environment} {c : Name → Bool} {n : Name}
    (h : applied e c n ≠ none) : e.contracts n ≠ none := by
  intro hc
  exact h (an_undeclared_callee_is_never_applied (c := c) hc)

/-- A name in the carried set is withdrawn whatever a later round decides. -/
theorem a_carried_name_stays_withdrawn {e : Environment} {c : Name → Bool} {n : Name}
    (h : c n = true) : applied e c n = none := by
  unfold applied
  simp [h]

/-! ### The relation is about a pair, not about a side -/

/-- The parameter half is symmetric at equal lengths. -/
theorem parametersContradict_symm :
    ∀ (xs ys : List ParameterMode), xs.length = ys.length →
      parametersContradict xs ys = parametersContradict ys xs := by
  intro xs
  induction xs with
  | nil => intro ys _; cases ys <;> rfl
  | cons x xs ih =>
      intro ys hlen
      cases ys with
      | nil => simp at hlen
      | cons y ys =>
          simp only [List.length_cons, Nat.add_right_cancel_iff] at hlen
          by_cases h : parameterDuty x = parameterDuty y
          · -- Only `h` is handed to `simp`: giving it `h.symm` as well would let
            -- it rewrite the duties back and forth without terminating.
            simp [parametersContradict, h, ih ys hlen]
          · have h' : ¬ (parameterDuty y = parameterDuty x) := fun hs => h hs.symm
            simp [parametersContradict, h, h']

theorem parameterAxis_symm (a b : Contract) : parameterAxis a b = parameterAxis b a := by
  unfold parameterAxis
  by_cases h : a.parameters.length = b.parameters.length
  · rw [parametersContradict_symm a.parameters b.parameters h, decide_eq_true h,
      decide_eq_true h.symm]
  · rw [decide_eq_false h, decide_eq_false (fun hs => h (Eq.symm hs))]
    simp

theorem resultAxis_symm (a b : Contract) : resultAxis a b = resultAxis b a := by
  unfold resultAxis
  by_cases h : resultDuty a.result = resultDuty b.result
  · rw [decide_eq_true h, decide_eq_true h.symm]
  · rw [decide_eq_false h, decide_eq_false (fun hs => h (Eq.symm hs))]

/-- A contradiction is symmetric: swapping declared and derived changes nothing
about *whether* the pair contradicts. The relation therefore cannot be
accidentally one-sided, admitting an unfaithful declaration in one direction. -/
theorem the_relation_is_symmetric (a b : Contract) :
    (contradiction a b).isSome = (contradiction b a).isSome := by
  unfold contradiction
  rw [parameterAxis_symm a b, resultAxis_symm a b]

/-! ### An arity difference decides nothing on the parameter axis -/

theorem an_arity_difference_decides_nothing_on_the_parameter_axis :
    contradiction ⟨[.released false], .absent⟩ ⟨[.borrowed, .borrowed], .absent⟩ = none := by
  decide

/-- …but the result axis is still decided there, so the arity rule narrows one
axis rather than switching the gate off. -/
theorem an_arity_difference_still_decides_the_result_axis :
    contradiction ⟨[.released false], .absent⟩ ⟨[.borrowed, .borrowed], .owned false⟩
      = some .resultCustody := by decide

/-! ### The whole finite duty domain, decided and reduced to one checksum -/

/-- One representative of every parameter duty, plus a second `retained` and a
second `transferred` spelling, so the checksum would move if two modes were
regrouped. -/
def parameterDomain : List ParameterMode :=
  [.borrowed, .owned false, .released false, .consumed false, .stored 1 false,
   .out false, .removed 1 false, .owned true]

/-- One representative of every result duty, with both `produced` spellings. -/
def resultDomain : List ResultMode :=
  [.absent, .owned false, .owned true, .reallocated 0 false]

/-- The parameter half of the domain: every ordered pair, at a fixed second
position, encoded as 1 for a refusal and 0 for an admission. -/
def parameterVerdicts : List Nat :=
  parameterDomain.flatMap (fun a =>
    parameterDomain.map (fun b =>
      if (contradiction ⟨[a, .borrowed], .absent⟩ ⟨[b, .borrowed], .absent⟩).isSome then 1 else 0))

/-- The result half, likewise. -/
def resultVerdicts : List Nat :=
  resultDomain.flatMap (fun a =>
    resultDomain.map (fun b =>
      if (contradiction ⟨[], a⟩ ⟨[], b⟩).isSome then 1 else 0))

/-- A positional checksum over the whole domain: each verdict is weighted by its
index, so a permutation moves it and a single flipped row moves it. The Rust
test computes the same number from the production relation, independently. -/
def checksum : Nat :=
  let weigh := fun (vs : List Nat) =>
    (vs.zipIdx.map (fun (v, i) => v * (i + 1))).foldl (· + ·) 0
  weigh parameterVerdicts + 1000 * weigh resultVerdicts

/-- The domain sizes, pinned so a silently shrunk domain cannot keep the
checksum meaningful. -/
theorem the_domain_is_the_size_it_claims :
    parameterVerdicts.length = 64 ∧ resultVerdicts.length = 16 := by decide

/-- The whole parameter domain, decided row by row. -/
theorem the_parameter_domain_is_decided :
    parameterVerdicts =
      [0, 0, 1, 1, 1, 1, 1, 0,
       0, 0, 1, 1, 1, 1, 1, 0,
       1, 1, 0, 0, 0, 1, 1, 1,
       1, 1, 0, 0, 0, 1, 1, 1,
       1, 1, 0, 0, 0, 1, 1, 1,
       1, 1, 1, 1, 1, 0, 1, 1,
       1, 1, 1, 1, 1, 1, 0, 1,
       0, 0, 1, 1, 1, 1, 1, 0] := by decide

/-- The whole result domain, decided row by row. -/
theorem the_result_domain_is_decided :
    resultVerdicts =
      [0, 1, 1, 1,
       1, 0, 0, 0,
       1, 0, 0, 0,
       1, 0, 0, 0] := by decide

/-- The two-sided tie: this exact number is asserted in
`crates/qcore-c-gateway/tests/loader_abi_modes.rs`, computed from the production
relation over the same domain in the same order. -/
theorem the_checksum_is_pinned : checksum = 1502 + 1000 * 36 := by decide

end QCore.DeclaredModeAgreement
