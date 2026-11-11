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

import QCore.LexicalClosureNestedAssignment

/-!
A W4 summary is validated on the typed-root premises its function's reported
row reads (W47-SUMMARY-VALIDATION-PROTECTED-FORMALS, gateway rule 4b). The CFG
construction, the analysis and the Rust-to-Lean correspondence remain review
premises (standing constraint 28). The model: a run of a body reads the body's
own binding events (rule 10l's `BindingEvent`) plus one `protectedFormal`
event per formal its own contract in force does not declare `Borrowed`; that
contract is the declaration when there is one, otherwise whatever the run
reads as the function's own. This module proves:

* **the gap**: before this card the validation run read the environment of
  the moment, which does not hold the candidate, so a copy of a formal the
  candidate seeds is never bound there and bound in the reported run — the
  `rd_kafka_topic_partition_cmp` shape, witnessed;
* **the runs coincide**: the reported run reads the environment after W4
  installed the candidate, whose own contract is exactly what the validation
  run now reads, so the two runs read the same events and every never-bound,
  `may_bind` and declared-local premise agrees;
* **fail-closed**: the change only adds bindings, so everything the old
  validation refused it still refuses; publication now implies the gate passed
  on the reported premises, so a caller never rests on a summary whose own
  premises differ;
* **exactly what is added**: a name the new run binds is bound by the body
  alone or is reached by copies from a formal the candidate protects;
* **no change elsewhere**: a declared function, and a candidate whose every
  formal is `Borrowed`, read the premises they read before.
-/

namespace QCore.SummaryValidationPremises

open QCore.TypedRootDeclarations

/-- A contract as the CFG construction reads it: per formal, whether it is not
`Borrowed`. -/
structure Contract where
  nonBorrowed : List Bool
  deriving DecidableEq, Repr

/-- The formals a contract protects as the function's own contract in force. -/
def ownProtected (formals : List Nat) (c : Contract) : List Nat :=
  ((formals.zip c.nonBorrowed).filter (·.2)).map (·.1)

/-- The contract in force for each function. -/
abbrev Env := Nat → Option Contract

/-- W4 installs a validated candidate only where no declaration governs; a
declaration always wins. -/
def install (env : Env) (f : Nat) (c : Contract) : Env :=
  fun g => if g = f then (match env f with
    | some d => some d
    | none => some c) else env g

/-- The contract a CFG construction reads as the function's own: the
declaration, else what the run is given. -/
def ownInForce (env : Env) (f : Nat) (supplied : Option Contract) : Option Contract :=
  match env f with
  | some d => some d
  | none => supplied

/-- The protection events of an own contract. -/
def protectionEvents (formals : List Nat) : Option Contract → List BindingEvent
  | some c => (ownProtected formals c).map .protectedFormal
  | none => []

/-- The binding events one run of the body reads. -/
def runEvents (body : List BindingEvent) (formals : List Nat) (own : Option Contract) :
    List BindingEvent :=
  body ++ protectionEvents formals own

/-- Before this card: the validation run read the environment of the moment. -/
def oldValidation (env : Env) (f : Nat) (body : List BindingEvent) (formals : List Nat) :
    List BindingEvent :=
  runEvents body formals (env f)

/-- This card: the validation run reads the candidate as the function's own. -/
def validation (env : Env) (f : Nat) (c : Contract) (body : List BindingEvent)
    (formals : List Nat) : List BindingEvent :=
  runEvents body formals (ownInForce env f (some c))

/-- The reported run: the body in the environment after installation. -/
def reported (env : Env) (f : Nat) (c : Contract) (body : List BindingEvent)
    (formals : List Nat) : List BindingEvent :=
  runEvents body formals (install env f c f)

/-! ## The runs coincide -/

/-- After installation the function's contract in force is the declaration if
there is one, else the candidate. -/
theorem the_installed_contract_is_the_own_contract (env : Env) (f : Nat) (c : Contract) :
    install env f c f = ownInForce env f (some c) := by
  unfold install ownInForce
  cases env f <;> simp

/-- **The runs read the same premises.** -/
theorem the_validation_run_reads_the_reported_premises (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat) :
    validation env f c body formals = reported env f c body formals := by
  unfold validation reported
  rw [the_installed_contract_is_the_own_contract]

/-- So a root is never bound in the validation run exactly when it is never
bound in the reported run: rule 5b's premise, rule 10o's join and every
reader seeded by the protected set answer alike. -/
theorem a_root_is_never_bound_in_both_runs_or_in_neither (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat) (n : Nat) :
    ¬ Bound (validation env f c body formals) n ↔ ¬ Bound (reported env f c body formals) n := by
  rw [the_validation_run_reads_the_reported_premises]

/-- The same for `may_bind`: an event of one run is an event of the other. -/
theorem may_bind_agrees (env : Env) (f : Nat) (c : Contract) (body : List BindingEvent)
    (formals : List Nat) (e : BindingEvent) :
    e ∈ validation env f c body formals ↔ e ∈ reported env f c body formals := by
  rw [the_validation_run_reads_the_reported_premises]

/-! ## Fail-closed -/

/-- The old events are contained in the new ones. -/
theorem the_old_events_are_contained (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat) :
    ∀ e ∈ oldValidation env f body formals, e ∈ validation env f c body formals := by
  intro e he
  unfold oldValidation validation runEvents ownInForce at *
  cases h : env f with
  | some d => simpa [h] using he
  | none =>
    simp only [h, protectionEvents, List.append_nil] at he
    exact List.mem_append_left _ he

/-- **Only refuses more.** Every binding of the old validation run is a binding
of the new one, so every root it refused it still refuses. -/
theorem every_old_binding_is_kept (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat) {n : Nat}
    (h : Bound (oldValidation env f body formals) n) :
    Bound (validation env f c body formals) n :=
  QCore.LexicalClosureNestedAssignment.bound_mono (the_old_events_are_contained env f c body formals) h

/-- The W4 gate, abstracted as a decision over the events a run reads (the
analysis and `observations_match_contract`). -/
def publishedWith (gate : List BindingEvent → Contract → Bool) (env : Env) (f : Nat)
    (c : Contract) (body : List BindingEvent) (formals : List Nat) : Bool :=
  gate (oldValidation env f body formals) c && gate (validation env f c body formals) c

/-- **A caller rests only on premises the reported row reads.** A published
summary passed the gate on the reported run's events. -/
theorem a_published_summary_passed_on_the_reported_premises
    (gate : List BindingEvent → Contract → Bool) (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat)
    (h : publishedWith gate env f c body formals = true) :
    gate (reported env f c body formals) c = true := by
  unfold publishedWith at h
  rw [← the_validation_run_reads_the_reported_premises]
  exact (Bool.and_eq_true_iff.mp h).2

/-- …and it still passed the old gate: nothing is published that was not
published before. -/
theorem a_published_summary_passed_the_old_gate
    (gate : List BindingEvent → Contract → Bool) (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat)
    (h : publishedWith gate env f c body formals = true) :
    gate (oldValidation env f body formals) c = true :=
  (Bool.and_eq_true_iff.mp h).1

/-! ## Exactly what the protection adds -/

/-- Names reached by copies from a protected formal. -/
inductive CopyReach (body : List BindingEvent) (guarded : List Nat) : Nat → Prop where
  | base {n : Nat} : n ∈ guarded → CopyReach body guarded n
  | copied {d m : Nat} {o : Bool} {names : List Nat} :
      BindingEvent.assign d o names ∈ body → m ∈ names → CopyReach body guarded m →
      CopyReach body guarded d

theorem an_assignment_of_the_run_is_the_body's {body : List BindingEvent}
    {guarded : List Nat} {d : Nat} {o : Bool} {names : List Nat}
    (h : BindingEvent.assign d o names ∈ body ++ guarded.map .protectedFormal) :
    BindingEvent.assign d o names ∈ body := by
  rcases List.mem_append.mp h with h | h
  · exact h
  · obtain ⟨_, _, he⟩ := List.mem_map.mp h
    cases he

/-- **Characterisation.** A name the protected run binds is bound by the body
alone, or is copy-reachable from a protected formal. -/
theorem what_protection_adds {body : List BindingEvent} {guarded : List Nat} {n : Nat}
    (h : Bound (body ++ guarded.map .protectedFormal) n) :
    Bound body n ∨ CopyReach body guarded n := by
  induction h with
  | seed he hn =>
    rcases List.mem_append.mp he with he | he
    · exact Or.inl (.seed he hn)
    · obtain ⟨p, hp, rfl⟩ := List.mem_map.mp he
      simp only [seeds, List.mem_singleton] at hn
      subst hn
      exact Or.inr (.base hp)
  | opaqueAssign he => exact Or.inl (.opaqueAssign (an_assignment_of_the_run_is_the_body's he))
  | copied he hm _ ih =>
    have hb := an_assignment_of_the_run_is_the_body's he
    rcases ih with ih | ih
    · exact Or.inl (.copied hb hm ih)
    · exact Or.inr (.copied hb hm ih)

/-- Conversely every copy-reachable name is bound in the protected run. -/
theorem a_copy_of_a_protected_formal_is_bound {body : List BindingEvent}
    {guarded : List Nat} {n : Nat} (h : CopyReach body guarded n) :
    Bound (body ++ guarded.map .protectedFormal) n := by
  induction h with
  | base hp =>
    exact .seed (e := .protectedFormal _)
      (List.mem_append_right _ (List.mem_map.mpr ⟨_, hp, rfl⟩)) (by simp [seeds])
  | copied he hm _ ih => exact .copied (List.mem_append_left _ he) hm ih

/-! ## No change elsewhere -/

/-- A declared function reads its declaration's premises in all three runs. -/
theorem a_declaration_keeps_its_premises (env : Env) (f : Nat) (c d : Contract)
    (body : List BindingEvent) (formals : List Nat) (h : env f = some d) :
    validation env f c body formals = oldValidation env f body formals ∧
      reported env f c body formals = oldValidation env f body formals := by
  have hv : validation env f c body formals = oldValidation env f body formals := by
    unfold validation oldValidation ownInForce
    rw [h]
  exact ⟨hv, (the_validation_run_reads_the_reported_premises env f c body formals).symm.trans hv⟩

/-- A candidate that declares every formal `Borrowed` protects nothing. -/
theorem an_all_borrowed_candidate_protects_nothing (formals : List Nat) (c : Contract)
    (h : ∀ b ∈ c.nonBorrowed, b = false) : ownProtected formals c = [] := by
  unfold ownProtected
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro p hp
  have := h p.2 (List.of_mem_zip hp).2
  simp [this]

/-- …so the validation run reads what it read before this card. -/
theorem an_all_borrowed_candidate_changes_no_premise (env : Env) (f : Nat) (c : Contract)
    (body : List BindingEvent) (formals : List Nat) (hnone : env f = none)
    (h : ∀ b ∈ c.nonBorrowed, b = false) :
    validation env f c body formals = oldValidation env f body formals := by
  unfold validation oldValidation ownInForce runEvents
  rw [hnone]
  simp [protectionEvents, an_all_borrowed_candidate_protects_nothing formals c h]

/-! ## The gap, witnessed: `rd_kafka_topic_partition_cmp` -/

/-- `_a` = 0, `_b` = 1 are the formals; `a` = 2, `b` = 3 their copies
`const T *a = _a; const T *b = _b;`. No declaration governs the function (7),
and the candidate seeds both formals (`Owned`, `Owned`). -/
def cmpBody : List BindingEvent := [.assign 2 false [0], .assign 3 false [1]]
def cmpFormals : List Nat := [0, 1]
def cmpCandidate : Contract := ⟨[true, true]⟩
def noDeclaration : Env := fun _ => none

/-- The old validation run's closure: the empty set is seeded and settled… -/
theorem the_old_validation_set_is_empty_and_settled :
    seededB (oldValidation noDeclaration 7 cmpBody cmpFormals) [] = true ∧
      settledB (oldValidation noDeclaration 7 cmpBody cmpFormals) [] = true := by
  decide

/-- …so the copy `a` is a never-bound root there, and its typed reads were
admitted. -/
theorem the_copy_was_never_bound_in_the_old_validation :
    ¬ Bound (oldValidation noDeclaration 7 cmpBody cmpFormals) 2 :=
  an_unlisted_name_is_never_bound the_old_validation_set_is_empty_and_settled.1
    the_old_validation_set_is_empty_and_settled.2 (by simp)

/-- In the reported run `_a` is protected and the copy is bound. -/
theorem the_copy_is_bound_in_the_reported_run :
    Bound (reported noDeclaration 7 cmpCandidate cmpBody cmpFormals) 2 :=
  .copied (m := 0) (o := false) (names := [0]) (by decide) (by decide)
    (.seed (e := .protectedFormal 0) (by decide) (by decide))

/-- **The gap.** The old validation admitted a root the reported run binds. -/
theorem the_old_validation_disagreed_with_the_reported_run :
    ¬ Bound (oldValidation noDeclaration 7 cmpBody cmpFormals) 2 ∧
      Bound (reported noDeclaration 7 cmpCandidate cmpBody cmpFormals) 2 :=
  ⟨the_copy_was_never_bound_in_the_old_validation, the_copy_is_bound_in_the_reported_run⟩

/-- **Closed.** The validation run now binds it too. -/
theorem the_copy_is_bound_in_the_new_validation :
    Bound (validation noDeclaration 7 cmpCandidate cmpBody cmpFormals) 2 := by
  rw [the_validation_run_reads_the_reported_premises]
  exact the_copy_is_bound_in_the_reported_run

/-- A genuinely read-only body — the formal read directly, never copied —
gains no bound name besides the formal itself. -/
theorem a_read_only_body_binds_only_the_formal {n : Nat}
    (h : Bound (validation noDeclaration 7 ⟨[true]⟩ [] [0]) n) : n = 0 := by
  have hv : validation noDeclaration 7 ⟨[true]⟩ [] [0] = [] ++ [0].map .protectedFormal := by
    decide
  rw [hv] at h
  rcases what_protection_adds h with h | h
  · cases h with
    | seed he _ => simp at he
    | opaqueAssign he => simp at he
    | copied he _ _ => simp at he
  · cases h with
    | base hp => simpa using hp
    | copied he _ _ => simp at he

end QCore.SummaryValidationPremises
