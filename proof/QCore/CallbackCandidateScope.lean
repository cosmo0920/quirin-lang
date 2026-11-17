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

import QCore.CalleeSummaryScope
import QCore.CallbackContracts

/-!
# Callback candidates in the W4 derivation scope

Card **CG22-CALLBACK-CANDIDATE-CONTRACTS** (gateway rule 4a amended,
rule 13u amended; `crates/qcore-c-gateway/src/legacy_callee_scope.rs`,
`invocation_designators` in `legacy_callback_route.rs`).

Rule 4a's derivation scope followed direct calls only. A `static` comparator
that a header defines and a body hands to `bsearch` or `qsort` (librdkafka's
`rd_kafka_ApiVersion_key_cmp`) is never called directly, so W4 never derived
its summary, and rule 13u's synchronous invocation refused with
`callback-uncontracted` although the unit carries the one body the call runs.
The card lets the reachability also follow the **designators a body names
beside a call to a callback-invoking declaration** (rule 13u's W4 order
edges), still only to **unselected internal-linkage definitions the unit
carries**. Nothing else changes: an admitted body publishes a summary only
when it validates, and the invocation still decides over the candidates'
contracts in force exactly as before.

## What is proved

* **The widened scope admits only what linkage pins**
  (`every_member_is_selected_or_internal`), so an unselected external
  definition (`an_unselected_external_definition_is_never_admitted`) and a
  name the unit does not define (`an_undefined_name_is_never_admitted`) are
  never in it, whatever the designator edges say.
* **It only widens**: rule 4a's direct-call scope is contained in it
  (`the_direct_call_scope_is_contained`), so no summary derived before is
  lost (with `CalleeSummaryScope.an_existing_summary_survives_the_widening`).
* **An admitted candidate is the body the invocation runs**
  (`an_admitted_candidate_runs_only_its_analysed_body`): the designator of an
  internal-linkage definition denotes that definition (C11 6.2.2p3), so a
  summary validated against it is a statement about every invocation of it.
* **No candidate is read benign**: a candidate whose body publishes no
  validated summary and that no declaration covers has no contract in force,
  so the invocation stays refused (`an_unvalidated_candidate_keeps_the_invocation_refused`);
  a table with such a member is refused whatever its other members are.
* Witness: a designated static comparator is admitted; a designated external
  comparator and an undesignated static are not (`the_witness_scope`).

## What is not proved

`invocation_designators` (the token reader) and `has_internal_linkage` are
Rust transcriptions of `designates` and `candidate`, maintained by review
(trust boundary 28). Validation is `QCore.CustodySummary`'s relation, taken
here as an abstract predicate, as in `QCore.CalleeSummaryScope`.
-/

namespace QCore.CallbackCandidateScope

open QCore.CalleeSummaryScope

/-- One round: add every candidate a scope member calls directly or names as
a designator beside a callback-invoking call (`designates`). -/
def stepWith (unit : List Definition) (designates : Name → List Name) (s : List Name) :
    List Name :=
  s ++ ((unit.filter fun d =>
      candidate d && s.any fun m =>
        (callsOf unit m).contains d.name || (designates m).contains d.name).map
    (fun d => d.name))

def iterateWith (unit : List Definition) (designates : Name → List Name) :
    Nat → List Name → List Name
  | 0, s => s
  | k + 1, s => iterateWith unit designates k (stepWith unit designates s)

/-- The widened derivation scope. -/
def scopeWith (unit : List Definition) (designates : Name → List Name) : List Name :=
  iterateWith unit designates unit.length (selectedNames unit)

theorem stepWith_preserves_admissible {unit : List Definition} {designates : Name → List Name}
    {s : List Name} (hs : ∀ m ∈ s, Admissible unit m) :
    ∀ m ∈ stepWith unit designates s, Admissible unit m := by
  intro m hm
  unfold stepWith at hm
  rcases List.mem_append.mp hm with hm | hm
  · exact hs m hm
  · rcases List.mem_map.mp hm with ⟨d, hd, rfl⟩
    have hd' := List.mem_filter.mp hd
    have hc : candidate d = true := by
      have := hd'.2
      simp only [Bool.and_eq_true] at this
      exact this.1
    exact ⟨d, hd'.1, rfl, Or.inr (candidate_is_internal hc)⟩

theorem iterateWith_preserves_admissible (unit : List Definition)
    (designates : Name → List Name) :
    ∀ (k : Nat) (s : List Name), (∀ m ∈ s, Admissible unit m) →
      ∀ m ∈ iterateWith unit designates k s, Admissible unit m := by
  intro k
  induction k with
  | zero => intro s hs; exact hs
  | succ k ih =>
    intro s hs
    exact ih (stepWith unit designates s) (stepWith_preserves_admissible hs)

/-- Every member of the widened scope is selected or an internal-linkage
definition of the unit. -/
theorem every_member_is_selected_or_internal (unit : List Definition)
    (designates : Name → List Name) :
    ∀ m ∈ scopeWith unit designates, Admissible unit m :=
  iterateWith_preserves_admissible unit designates unit.length _ (selected_admissible unit)

/-- An unselected external definition is never admitted, whatever a body
designates. -/
theorem an_unselected_external_definition_is_never_admitted
    {unit : List Definition} (designates : Name → List Name) (unique : UniqueNames unit)
    {d : Definition} (hd : d ∈ unit) (unselected : d.selected = false)
    (external : d.linkage = .external) :
    d.name ∉ scopeWith unit designates := by
  intro hm
  rcases every_member_is_selected_or_internal unit designates d.name hm with
    ⟨d', hd', hname, hadm⟩
  have := unique d' hd' d hd hname
  subst this
  rcases hadm with h | h
  · rw [unselected] at h; exact Bool.false_ne_true h
  · rw [external] at h; exact Linkage.noConfusion h

/-- A name the unit does not define is never admitted. -/
theorem an_undefined_name_is_never_admitted {unit : List Definition}
    (designates : Name → List Name) {n : Name} (undefined : ∀ d ∈ unit, d.name ≠ n) :
    n ∉ scopeWith unit designates := by
  intro hm
  rcases every_member_is_selected_or_internal unit designates n hm with ⟨d, hd, hname, _⟩
  exact undefined d hd hname

theorem stepWith_superset (unit : List Definition) (designates : Name → List Name)
    (s : List Name) : ∀ m ∈ s, m ∈ stepWith unit designates s :=
  fun _ hm => List.mem_append_left _ hm

/-- One widened step contains one direct-call step of a smaller set. -/
theorem step_sub_stepWith (unit : List Definition) (designates : Name → List Name)
    {s t : List Name} (sub : ∀ m ∈ s, m ∈ t) :
    ∀ m ∈ step unit s, m ∈ stepWith unit designates t := by
  intro m hm
  unfold step at hm
  rcases List.mem_append.mp hm with hm | hm
  · exact List.mem_append_left _ (sub m hm)
  · rcases List.mem_map.mp hm with ⟨d, hd, rfl⟩
    have hd' := List.mem_filter.mp hd
    simp only [Bool.and_eq_true, List.any_eq_true] at hd'
    obtain ⟨hin, hc, ⟨w, hw, hcall⟩⟩ := hd'
    apply List.mem_append_right
    apply List.mem_map.mpr
    refine ⟨d, List.mem_filter.mpr ⟨hin, ?_⟩, rfl⟩
    simp only [Bool.and_eq_true, List.any_eq_true, Bool.or_eq_true]
    exact ⟨hc, ⟨w, sub w hw, Or.inl hcall⟩⟩

theorem iterate_sub_iterateWith (unit : List Definition) (designates : Name → List Name) :
    ∀ (k : Nat) (s t : List Name), (∀ m ∈ s, m ∈ t) →
      ∀ m ∈ iterate unit k s, m ∈ iterateWith unit designates k t := by
  intro k
  induction k with
  | zero => intro s t sub; exact sub
  | succ k ih =>
    intro s t sub
    exact ih (step unit s) (stepWith unit designates t)
      (step_sub_stepWith unit designates sub)

/-- Rule 4a's direct-call scope is contained in the widened one: the
widening only adds definitions, so it only adds summaries. -/
theorem the_direct_call_scope_is_contained (unit : List Definition)
    (designates : Name → List Name) :
    ∀ m ∈ scope unit, m ∈ scopeWith unit designates :=
  iterate_sub_iterateWith unit designates unit.length _ _ (fun _ h => h)

/-- An admitted candidate (an unselected member of the widened scope) is the
only body an invocation through its designator in this unit may run. -/
theorem an_admitted_candidate_runs_only_its_analysed_body
    {unit : List Definition} (designates : Name → List Name) (unique : UniqueNames unit)
    (elsewhere : Name → List Definition) {d : Definition} (hd : d ∈ unit)
    (unselected : d.selected = false) (inScope : d.name ∈ scopeWith unit designates)
    {b : Definition} (run : MayRun unit elsewhere d.name b) : b = d := by
  rcases every_member_is_selected_or_internal unit designates d.name inScope with
    ⟨d', hd', hname, hadm⟩
  have hsame := unique d' hd' d hd hname
  subst hsame
  have internal : d'.linkage = .internal := by
    rcases hadm with h | h
    · rw [unselected] at h; exact absurd h Bool.false_ne_true
    · exact h
  rcases run with ⟨hb, hbn⟩ | ⟨⟨e, he, hen, hext⟩, _⟩
  · exact unique b hb d' hd' hbn
  · have := unique e he d' hd' hen
    subst this
    rw [internal] at hext
    exact Linkage.noConfusion hext

/-! ## No candidate is read benign -/

open QCore.CallbackContracts in
/-- The class an invocation gives a candidate from its contract in force
(`classify` of `callback_contracts.rs`, its contract abstracted to whether it
is the identity on the caller's state). -/
def classOf {σ : Type} (inForce : Env σ) (identity : σ → Bool) (n : Name) :
    CandidateClass :=
  match inForce n with
  | none => .missing
  | some k => if identity k then .identity else .effectful

open QCore.CallbackContracts in
/-- A candidate with no declaration, whose body (in scope or not) publishes no
validated summary — an `Unknown` or unanalysed comparator — has no contract
in force, so the invocation that may call it is refused, whatever the other
candidates are. -/
theorem an_unvalidated_candidate_keeps_the_invocation_refused {σ : Type}
    (declared : Env σ) (inScope : Name → Bool) (derive : Name → Option σ)
    (identity : σ → Bool) (isOpen elements : Bool) (others : List Name) {n : Name}
    (hdecl : declared n = none) (hnone : derive n = none) :
    decide isOpen ((n :: others).map
        (classOf (install declared (inferredOver inScope derive)) identity)) elements
      = .refused := by
  apply decide_refuses_a_missing_contract
  apply List.mem_map.mpr
  refine ⟨n, List.mem_cons_self .., ?_⟩
  unfold classOf
  rw [an_installed_inference_is_exactly_the_derived_contract _ _ hdecl]
  unfold inferredOver
  cases inScope n <;> simp [hnone]

open QCore.CallbackContracts in
/-- A candidate in force whose validated summary is the identity is classified
the identity: the only way an admitted designator decides an invocation. -/
theorem a_validated_identity_candidate_is_in_force {σ : Type}
    (declared : Env σ) (inScope : Name → Bool) (derive : Name → Option σ)
    (identity : σ → Bool) {n : Name} {k : σ}
    (hdecl : declared n = none) (hin : inScope n = true) (hk : derive n = some k)
    (hid : identity k = true) :
    classOf (install declared (inferredOver inScope derive)) identity n = .identity := by
  unfold classOf
  rw [an_installed_inference_is_exactly_the_derived_contract _ _ hdecl]
  simp [inferredOver, hin, hk, hid]

/-! ## Witness -/

def witnessUnit : List Definition :=
  [ ⟨"check", .external, true, false, ["bsearch"]⟩
  , ⟨"key_cmp", .internal, false, false, []⟩
  , ⟨"broker_cmp", .external, false, false, []⟩
  , ⟨"unnamed_cmp", .internal, false, false, []⟩ ]

/-- `check` hands `key_cmp` and `broker_cmp` to `bsearch`. -/
def witnessDesignates : Name → List Name
  | "check" => ["key_cmp", "broker_cmp"]
  | _ => []

/-- The designated static comparator is admitted (rule 4a's direct-call scope
did not admit it); the designated external comparator and an undesignated
static are not. -/
theorem the_witness_scope :
    "key_cmp" ∈ scopeWith witnessUnit witnessDesignates ∧
      "key_cmp" ∉ scope witnessUnit ∧
      "broker_cmp" ∉ scopeWith witnessUnit witnessDesignates ∧
      "unnamed_cmp" ∉ scopeWith witnessUnit witnessDesignates := by
  decide

end QCore.CallbackCandidateScope
