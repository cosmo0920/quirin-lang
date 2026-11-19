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

import QCore.CallbackLiteralPremise

/-!
# A literal-argument premise at a callback candidate

Card **CALLBACK-CANDIDATE-LITERAL-PREMISE** (gateway rule 57 amended in
place; contract specification §C04.5). Beside `QCore.CallbackLiteralPremise`,
which models route G's rewrite of a *call* of a premise-carrying declaration.

A callback candidate is not a call the rewrite sees: it is a function
designator handed to an invoker, and the library chooses the actuals. Before
this card the callback route classified a candidate by its contract alone, so
a premise-carrying declaration with the identity's shape was applied at an
invocation whatever its actuals. Modelled as the Rust code decides it
(`Parser::candidate_withdrawn`, read before every other reading in
`classified_in`, and `Parser::candidate_contract` at a registration and a
constructed member registration):

* `guarded e n k`: a candidate `n` whose declaration carries a premise, or
  any candidate under a poisoned premise set, is `missing` (no contract in
  force); every other candidate keeps the class `k` the route read before
  (its contract, its composition, its hand-off).
* `decide`: rule 13u's decision over the classes, extended by rule 13w's
  releasing registry: refused on any `missing`, `effectful` or `refused`
  class; applied when every class is the identity; releasing when every
  class releases its element.

Proved: a premise-carrying or poisoned candidate is `missing` and a
premise-free one is classified as before; the guard never makes a class
better; the guarded decision applies (or releases) only where the old one
did; an applied or releasing decision has no premise-carrying candidate; and
a premise-free set is decided as before. `cand_*` are the card's vectors:
`flb_hash_table_add`'s identity shape, `rd_kafka_topic_partition_destroy0`'s
releasing shape and `d2i_PKCS12_bio`'s composed shape.

## What this is not

Candidate resolution, the classification of a contract and the composition
of a candidate's own invocations are inputs (the class `k`), not modelled
here (trust boundary 28; `crates/qcore-c-gateway/tests/callback_literal_premise.rs`).
-/

namespace QCore.CallbackCandidatePremise

open QCore.CallbackLiteralPremise

/-- A candidate's class at one invocation, as the callback route reads it. -/
inductive Class where
  | missing
  | identity
  | releasesElement
  | effectful
  | refused
  deriving DecidableEq, Repr

/-- Whether a candidate carries a premise or the premise set is poisoned. -/
def withdrawn (e : Environment) (n : Name) : Bool :=
  e.poisoned || !(e.premises n).isEmpty

/-- The class of candidate `n` after the card, `k` the class before. -/
def guarded (e : Environment) (n : Name) (k : Class) : Class :=
  if withdrawn e n then .missing else k

/-- The outcome of an invocation (or a registry) over its candidates. -/
inductive Decision where
  | applied
  | releases
  | refused
  deriving DecidableEq, Repr

/-- Rule 13u's decision over the classes, with rule 13w's releasing
registry: an empty closed set is the caller's business (not modelled). -/
def decide (ks : List Class) : Decision :=
  if ks.all (fun k => k == .identity) then .applied
  else if ks.all (fun k => k == .releasesElement) then .releases
  else .refused

/-- The decision after the card over named candidates. -/
def decideGuarded (e : Environment) (cs : List (Name × Class)) : Decision :=
  decide (cs.map fun c => guarded e c.1 c.2)

/-- The decision before the card. -/
def decideOld (cs : List (Name × Class)) : Decision :=
  decide (cs.map fun c => c.2)

/-! ### The guard -/

theorem a_premise_carrying_candidate_is_missing {e : Environment} {n : Name} {k : Class}
    (h : e.premises n ≠ []) : guarded e n k = .missing := by
  have : (e.premises n).isEmpty = false := by
    cases hp : e.premises n with
    | nil => exact absurd hp h
    | cons _ _ => rfl
  simp [guarded, withdrawn, this]

theorem a_poisoned_set_leaves_no_candidate_contracted {e : Environment} {n : Name}
    {k : Class} (h : e.poisoned = true) : guarded e n k = .missing := by
  simp [guarded, withdrawn, h]

theorem a_premise_free_candidate_is_classified_as_before {e : Environment} {n : Name}
    {k : Class} (hp : e.premises n = []) (hq : e.poisoned = false) :
    guarded e n k = k := by
  simp [guarded, withdrawn, hp, hq]

/-- The guard never makes a class better: it is the old class or `missing`. -/
theorem the_guard_only_withdraws {e : Environment} {n : Name} {k : Class} :
    guarded e n k = k ∨ guarded e n k = .missing := by
  unfold guarded
  cases withdrawn e n <;> simp

theorem a_contracted_class_has_no_premise {e : Environment} {n : Name} {k : Class}
    (h : guarded e n k ≠ .missing) : e.premises n = [] ∧ e.poisoned = false := by
  unfold guarded withdrawn at h
  cases hq : e.poisoned with
  | true => simp [hq] at h
  | false =>
      refine ⟨?_, rfl⟩
      cases hp : e.premises n with
      | nil => rfl
      | cons _ _ => simp [hq, hp] at h

/-! ### The decision only refuses more -/

theorem applied_only_over_premise_free_identities {e : Environment}
    {cs : List (Name × Class)} (h : decideGuarded e cs = .applied) :
    ∀ c ∈ cs, e.premises c.1 = [] ∧ e.poisoned = false ∧ c.2 = .identity := by
  intro c hc
  unfold decideGuarded decide at h
  cases hall : (cs.map fun c => guarded e c.1 c.2).all (fun k => k == .identity) with
  | false =>
      cases hrel : (cs.map fun c => guarded e c.1 c.2).all
          (fun k => k == .releasesElement) <;> simp [hall, hrel] at h
  | true =>
      have hk := List.all_eq_true.mp hall (guarded e c.1 c.2) (List.mem_map_of_mem hc)
      have hid : guarded e c.1 c.2 = .identity := by simpa using hk
      have hne : guarded e c.1 c.2 ≠ .missing := by rw [hid]; decide
      obtain ⟨hp, hq⟩ := a_contracted_class_has_no_premise hne
      refine ⟨hp, hq, ?_⟩
      rw [a_premise_free_candidate_is_classified_as_before hp hq] at hid
      exact hid

theorem releasing_only_over_premise_free_candidates {e : Environment}
    {cs : List (Name × Class)} (h : decideGuarded e cs = .releases) :
    ∀ c ∈ cs, e.premises c.1 = [] ∧ e.poisoned = false ∧ c.2 = .releasesElement := by
  intro c hc
  unfold decideGuarded decide at h
  cases hall : (cs.map fun c => guarded e c.1 c.2).all (fun k => k == .identity) with
  | true => simp [hall] at h
  | false =>
      cases hrel : (cs.map fun c => guarded e c.1 c.2).all
          (fun k => k == .releasesElement) with
      | false => simp [hall, hrel] at h
      | true =>
          have hk := List.all_eq_true.mp hrel (guarded e c.1 c.2) (List.mem_map_of_mem hc)
          have hr : guarded e c.1 c.2 = .releasesElement := by simpa using hk
          have hne : guarded e c.1 c.2 ≠ .missing := by rw [hr]; decide
          obtain ⟨hp, hq⟩ := a_contracted_class_has_no_premise hne
          refine ⟨hp, hq, ?_⟩
          rw [a_premise_free_candidate_is_classified_as_before hp hq] at hr
          exact hr

/-- The guarded map is the old map on a premise-free, unpoisoned set. -/
theorem guarded_map_is_old_when_premise_free {e : Environment} {cs : List (Name × Class)}
    (hp : ∀ c ∈ cs, e.premises c.1 = []) (hq : e.poisoned = false) :
    (cs.map fun c => guarded e c.1 c.2) = cs.map fun c => c.2 := by
  apply List.map_congr_left
  intro c hc
  exact a_premise_free_candidate_is_classified_as_before (hp c hc) hq

theorem a_premise_free_set_is_decided_as_before {e : Environment}
    {cs : List (Name × Class)} (hp : ∀ c ∈ cs, e.premises c.1 = [])
    (hq : e.poisoned = false) : decideGuarded e cs = decideOld cs := by
  unfold decideGuarded decideOld
  rw [guarded_map_is_old_when_premise_free hp hq]

/-- Nothing is applied (or released) that was not before. -/
theorem the_guarded_decision_applies_only_where_the_old_did {e : Environment}
    {cs : List (Name × Class)} (h : decideGuarded e cs = .applied) :
    decideOld cs = .applied := by
  cases cs with
  | nil => rfl
  | cons c rest =>
      have hall := applied_only_over_premise_free_identities h
      rw [← a_premise_free_set_is_decided_as_before (fun c hc => (hall c hc).1)
        (hall c (by simp)).2.1]
      exact h

theorem the_guarded_registry_releases_only_where_the_old_did {e : Environment}
    {cs : List (Name × Class)} (h : decideGuarded e cs = .releases) :
    decideOld cs = .releases := by
  have hall := releasing_only_over_premise_free_candidates h
  cases cs with
  | nil => simp [decideGuarded, decide] at h
  | cons c rest =>
      rw [← a_premise_free_set_is_decided_as_before (fun c hc => (hall c hc).1)
        (hall c (by simp)).2.1]
      exact h

/-! ### The card's vectors -/

/-- Name `1`: a premise-carrying declaration (`flb_hash_table_add`,
`rd_kafka_topic_partition_destroy0`, `d2i_PKCS12_bio`); name `2`: the same
declaration without a premise. -/
def candEnv : Environment where
  functions := fun n => n == 1 || n == 2
  retaining := fun _ => false
  premises := fun n => if n == 1 then [{ index := 2, literal := [0] }] else []
  poisoned := false

/-- `flb_hash_table_add`'s identity shape was applied; now it refuses. -/
theorem cand_identity_shape_was_applied : decideOld [(1, .identity)] = .applied := by decide

theorem cand_identity_shape_refuses :
    decideGuarded candEnv [(1, .identity)] = .refused := by decide

/-- `rd_kafka_topic_partition_destroy0`'s releasing shape made a releasing
registry; now the registry is not one. -/
theorem cand_releasing_shape_was_releasing :
    decideOld [(1, .releasesElement)] = .releases := by decide

theorem cand_releasing_shape_refuses :
    decideGuarded candEnv [(1, .releasesElement), (2, .releasesElement)] = .refused := by
  decide

/-- A composed candidate (`d2i_PKCS12_bio`'s shape, read `identity` by its
composition) refuses before its composition is read. -/
theorem cand_composed_shape_refuses :
    decideGuarded candEnv [(2, .identity), (1, .identity)] = .refused := by decide

/-- The same declarations without a premise decide as before. -/
theorem cand_premise_free_identity_applies :
    decideGuarded candEnv [(2, .identity)] = .applied := by decide

theorem cand_premise_free_releasing_releases :
    decideGuarded candEnv [(2, .releasesElement)] = .releases := by decide

end QCore.CallbackCandidatePremise
