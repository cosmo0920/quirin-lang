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

import QCore.CustodySummary

/-!
Contextual custody inference with exhaustive integer contexts and strictly
lower-rank call dependencies. This extends the existing exit-validation
relation; it does not assume a recursive function's own summary. Typed scalar
recognition, context-specific CFG extraction and complete call inventories are
premises supplied by the C gateway, not proved C parser facts. Well-foundedness
below concerns admitted contextual calls, not termination of arbitrary C loops.
-/
namespace QCore.ContextualCustody
open QCore.CustodySummary

inductive ScalarContext where
  | zero
  | nonzero
  deriving DecidableEq, Repr

structure Key where
  function : Nat
  context : ScalarContext
  deriving DecidableEq, Repr

structure Evidence where
  rank : Nat
  summary : Summary
  analysisSafe : Bool
  exits : List ExitObservation
  dependencies : List Key

abbrev Table := Key → Option Evidence

/-- Every installed context is validated, and every admitted call uses a
previously established, strictly lower-ranked context. -/
def Checked (table : Table) : Prop :=
  ∀ key evidence, table key = some evidence →
    validate evidence.summary evidence.analysisSafe evidence.exits = true ∧
    ∀ dependency ∈ evidence.dependencies,
      ∃ earlier, table dependency = some earlier ∧ earlier.rank < evidence.rank

def Calls (table : Table) (caller callee : Key) : Prop :=
  ∃ evidence, table caller = some evidence ∧ callee ∈ evidence.dependencies

def rankOf (table : Table) (key : Key) : Nat :=
  match table key with
  | some evidence => evidence.rank
  | none => 0

theorem checked_call_uses_validated_lower_context
    {table : Table} (checked : Checked table) {caller callee : Key}
    (call : Calls table caller callee) :
    ∃ evidence, table callee = some evidence ∧
      validate evidence.summary evidence.analysisSafe evidence.exits = true ∧
      evidence.rank < rankOf table caller := by
  obtain ⟨current, lookup, member⟩ := call
  obtain ⟨earlier, found, lower⟩ := (checked caller current lookup).2 callee member
  refine ⟨earlier, found, (checked callee earlier found).1, ?_⟩
  simpa [rankOf, lookup] using lower

theorem checked_call_decreases_rank
    {table : Table} (checked : Checked table) {caller callee : Key}
    (call : Calls table caller callee) :
    rankOf table callee < rankOf table caller := by
  obtain ⟨evidence, found, _, lower⟩ := checked_call_uses_validated_lower_context checked call
  simpa [rankOf, found] using lower

theorem checked_calls_are_well_founded
    {table : Table} (checked : Checked table) :
    WellFounded (fun callee caller => Calls table caller callee) := by
  apply Subrelation.wf (r := (measure (rankOf table)).rel)
  · intro callee caller call
    exact checked_call_decreases_rank checked call
  · exact (measure (rankOf table)).wf

theorem rank_zero_context_has_no_recursive_dependencies
    {table : Table} (checked : Checked table) {key : Key} {evidence : Evidence}
    (found : table key = some evidence) (base : evidence.rank = 0) :
    evidence.dependencies = [] := by
  cases dependencies : evidence.dependencies with
  | nil => rfl
  | cons dependency rest =>
    have member : dependency ∈ evidence.dependencies := by simp [dependencies]
    obtain ⟨_, _, lower⟩ := (checked key evidence found).2 dependency member
    rw [base] at lower
    exact False.elim (Nat.not_lt_zero _ lower)

theorem checked_context_never_assumes_itself
    {table : Table} (checked : Checked table) (key : Key) :
    ¬ Calls table key key := by
  intro call
  exact Nat.lt_irrefl _ (checked_call_decreases_rank checked call)

theorem checked_contexts_cannot_assume_each_other
    {table : Table} (checked : Checked table) {first second : Key}
    (forward : Calls table first second) : ¬ Calls table second first := by
  intro backward
  exact Nat.lt_asymm (checked_call_decreases_rank checked forward)
    (checked_call_decreases_rank checked backward)

/-- Publication requires both exhaustive contexts, with the same custody
contract. A successful base context alone is insufficient. -/
def Published (table : Table) (function : Nat) (summary : Summary) : Prop :=
  Checked table ∧
  ∃ zero nonzero,
    table ⟨function, .zero⟩ = some zero ∧
    table ⟨function, .nonzero⟩ = some nonzero ∧
    zero.summary = summary ∧ nonzero.summary = summary

def classify (value : Int) : ScalarContext := if value = 0 then .zero else .nonzero

theorem integer_contexts_are_exhaustive (value : Int) :
    (value = 0 ∧ classify value = .zero) ∨
    (value ≠ 0 ∧ classify value = .nonzero) := by
  by_cases zero : value = 0
  · exact Or.inl ⟨zero, by simp [classify, zero]⟩
  · exact Or.inr ⟨zero, by simp [classify, zero]⟩

theorem publication_covers_every_integer_context
    {table : Table} {function : Nat} {summary : Summary}
    (published : Published table function summary) (value : Int) :
    ∃ evidence, table ⟨function, classify value⟩ = some evidence ∧
      evidence.summary = summary ∧
      validate summary evidence.analysisSafe evidence.exits = true := by
  obtain ⟨checked, zero, nonzero, foundZero, foundNonzero, sameZero, sameNonzero⟩ := published
  cases context : classify value with
  | zero =>
    refine ⟨zero, foundZero, sameZero, ?_⟩
    rw [← sameZero]
    exact (checked _ _ foundZero).1
  | nonzero =>
    refine ⟨nonzero, foundNonzero, sameNonzero, ?_⟩
    rw [← sameNonzero]
    exact (checked _ _ foundNonzero).1

theorem publication_agrees_with_every_context_exit
    {table : Table} {function : Nat} {summary : Summary}
    (published : Published table function summary) (value : Int) :
    ∃ evidence, table ⟨function, classify value⟩ = some evidence ∧
      evidence.exits.isEmpty = false ∧
      ∀ observation ∈ evidence.exits, exitMatches summary observation = true := by
  obtain ⟨evidence, found, _, valid⟩ := publication_covers_every_integer_context published value
  exact ⟨evidence, found,
    validated_summary_has_a_reachable_exit _ _ _ valid,
    validated_summary_matches_every_exit _ _ _ valid⟩

theorem missing_nonzero_context_prevents_publication
    {table : Table} {function : Nat} (summary : Summary)
    (missing : table ⟨function, .nonzero⟩ = none) :
    ¬ Published table function summary := by
  rintro ⟨_, _, _, _, found, _, _⟩
  rw [missing] at found
  contradiction

end QCore.ContextualCustody

/-!
Unsigned countdown extension. The finite zero/nonzero table above is NOT a
proof of countdown calls: two nonzero calls can depend on one another at
*different concrete ranks*. Here the natural counter is the rank, and all
functions in a component are proved together by strong induction.

The C importer supplies explicit unsigned types, immutable controls, complete
call inventories, zero/nonzero guard extraction and a resource-scalar erasure
lemma. `step` below is the body-checking premise under lower-rank contracts;
this model does not prove the C parser, machine stack capacity or all C behavior.
-/
namespace QCore.UnsignedCountdown
open QCore.CustodySummary

inductive Argument where
  | zero
  | predecessor
  deriving DecidableEq, Repr

/-- Conversion to an unsigned callee uses its positive modulus. On nonzero
input the subtraction itself is representable after integer promotion. -/
def target (argument : Argument) (counter modulus : Nat) : Nat :=
  match argument with
  | .zero => 0
  | .predecessor => (counter - 1) % modulus

theorem predecessor_before_conversion_decreases {counter : Nat} (positive : 0 < counter) :
    counter - 1 < counter := by omega

theorem unsigned_conversion_cannot_increase (value modulus : Nat) :
    value % modulus ≤ value := Nat.mod_le _ _

theorem admitted_argument_decreases (argument : Argument) {counter : Nat}
    (positive : 0 < counter) (modulus : Nat) :
    target argument counter modulus < counter := by
  cases argument with
  | zero => exact positive
  | predecessor =>
    exact Nat.lt_of_le_of_lt (unsigned_conversion_cannot_increase _ _)
      (predecessor_before_conversion_decreases positive)

structure Invocation where
  function : Nat
  counter : Nat
  deriving DecidableEq, Repr

/-- Edges are admitted only from the nonzero branch. The zero body must check
with every recursive component summary absent. Function identities may differ. -/
def Calls (callee caller : Invocation) : Prop :=
  0 < caller.counter ∧ ∃ argument modulus, 0 < modulus ∧
    callee.counter = target argument caller.counter modulus

theorem countdown_call_decreases {callee caller : Invocation} (call : Calls callee caller) :
    callee.counter < caller.counter := by
  obtain ⟨positive, argument, modulus, _, same⟩ := call
  rw [same]
  exact admitted_argument_decreases argument positive modulus

theorem countdown_calls_are_well_founded : WellFounded Calls := by
  apply Subrelation.wf (r := (measure Invocation.counter).rel)
  · intro callee caller call
    exact countdown_call_decreases call
  · exact (measure Invocation.counter).wf

theorem zero_body_has_no_admitted_call (callee caller : Invocation)
    (zero : caller.counter = 0) : ¬ Calls callee caller := by
  intro call
  have lower := countdown_call_decreases call
  rw [zero] at lower
  exact Nat.not_lt_zero _ lower

theorem same_rank_cannot_supply_recursive_hypothesis (callee caller : Invocation)
    (same : callee.counter = caller.counter) : ¬ Calls callee caller := by
  intro call
  have lower := countdown_call_decreases call
  rw [same] at lower
  exact Nat.lt_irrefl _ lower

/-- Both body contexts must establish the SAME candidate contract. The step
premise may use the contract at lower counter values for any component member;
it may not assume validity at the current value, even for another function. -/
theorem publish_by_mutual_rank_induction
    (valid : Nat → Nat → Prop)
    (base : ∀ function, valid function 0)
    (step : ∀ counter, 0 < counter →
      (∀ smaller, smaller < counter → ∀ function, valid function smaller) →
      ∀ function, valid function counter) :
    ∀ counter function, valid function counter := by
  intro counter
  induction counter using Nat.strongRecOn with
  | ind counter lower =>
    cases counter with
    | zero => exact base
    | succ predecessor =>
      exact step (predecessor + 1) (Nat.zero_lt_succ _) lower

/-- Connect the induction rule to the existing nonempty exit/custody validator.
The supplied base and step are body-validation evidence, not assumed recursive
summary truth. Unsupported or missing base/step evidence cannot invoke this rule. -/
theorem published_countdown_agrees_with_every_exit
    (summary : Nat → Summary) (safe : Nat → Nat → Bool)
    (exits : Nat → Nat → List ExitObservation)
    (base : ∀ function, validate (summary function) (safe function 0) (exits function 0) = true)
    (step : ∀ counter, 0 < counter →
      (∀ smaller, smaller < counter → ∀ function,
        validate (summary function) (safe function smaller) (exits function smaller) = true) →
      ∀ function, validate (summary function) (safe function counter) (exits function counter) = true)
    (counter function : Nat) :
    safe function counter = true ∧ (exits function counter).isEmpty = false ∧
      ∀ observation ∈ exits function counter, exitMatches (summary function) observation = true := by
  have checked := publish_by_mutual_rank_induction
    (fun function counter => validate (summary function) (safe function counter) (exits function counter) = true)
    base step counter function
  exact ⟨validated_summary_is_analyzed_safely _ _ _ checked,
    validated_summary_has_a_reachable_exit _ _ _ checked,
    validated_summary_matches_every_exit _ _ _ checked⟩

end QCore.UnsignedCountdown
