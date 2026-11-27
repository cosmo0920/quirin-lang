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
FB-CHUNK-LIFECYCLE (gateway rule 90). A `static` function's integer formal
that every call in the unit fixes to a constant is read at that constant
where the body tests it (`if (NAME)` / `if (!NAME)`). The census reads every
occurrence of the function's identifier: its definition, a file-scope
prototype, or a direct call whose actual at the formal is a small decimal
constant; anything else refuses. When the census fixes a truth, each branch
on the formal becomes a single edge, the one C takes under that truth.

Faithful tokenization of the whole preprocessed unit and the Rust/model
correspondence remain premises (trust boundary 28).
-/
namespace QCore.StaticConstantFormal

/-- One occurrence of the function's identifier in the unit. `actual` is the
    actual at the formal when it is a decimal constant the census reads,
    `none` otherwise; `arity` whether the call has the parameter count. -/
inductive Occurrence where
  | definition
  | prototype (inBody semicolon : Bool)
  | call (actual : Option Nat) (arity : Bool)
  | other
  deriving DecidableEq, Repr

/-- The largest constant read: representable in every admitted formal type. -/
def bound : Nat := 9999

def Occurrence.admissible : Occurrence → Bool
  | .definition => true
  | .prototype inBody semicolon => !inBody && semicolon
  | .call _ arity => arity
  | .other => false

/-- The truth of a constant actual, or `none` when it is not read. -/
def truthOf : Option Nat → Option Bool
  | some v => if v ≤ bound then some (v != 0) else none
  | none => none

def Occurrence.callTruth : Occurrence → Option (Option Bool)
  | .call a _ => some (truthOf a)
  | _ => none

/-- The truth every call agrees on, if any: no call, a call the census does
    not read, or calls of both truths all give `none`. -/
def agreed : List (Option Bool) → Option Bool
  | [] => none
  | none :: _ => none
  | some b :: rest => if rest.all (· == some b) then some b else none

/-- The census: the truth the formal is fixed to, or `none`. -/
def reading (internal formalOk testsOnly : Bool) (occ : List Occurrence) : Option Bool :=
  if internal && formalOk && testsOnly && occ.all Occurrence.admissible then
    agreed (occ.filterMap Occurrence.callTruth)
  else none

/-- C's choice at a branch on the formal holding `v`: edge 0 assumes the
    tested value zero exactly when `trueIsNull`. -/
def taken (v : Nat) (trueIsNull : Bool) : Nat :=
  if decide (v = 0) = trueIsNull then 0 else 1

/-- The rule's choice under the fixed truth `t` (as `contextual_cfg`). -/
def selectIndex (t trueIsNull : Bool) : Nat :=
  if trueIsNull != !t then 1 else 0

/-- One branch node's successors before and after the rule. -/
def selectSuccessors (fixed : Option Bool) (trueIsNull : Bool) (s : Nat × Nat) : List Nat :=
  match fixed with
  | none => [s.1, s.2]
  | some t => if selectIndex t trueIsNull = 0 then [s.1] else [s.2]

theorem agreed_mem {l : List (Option Bool)} {b : Bool} (h : agreed l = some b) :
    ∀ x ∈ l, x = some b := by
  match l, h with
  | [], h => simp [agreed] at h
  | none :: _, h => simp [agreed] at h
  | some c :: rest, h =>
    simp only [agreed] at h
    split at h
    · rename_i hall
      simp at h
      subst h
      intro x hx
      simp at hx
      rcases hx with hx | hx
      · exact hx
      · have := List.all_eq_true.mp hall x hx
        simpa using this
    · simp at h

/-- A fixed truth needs internal linkage. -/
theorem fixed_needs_internal_linkage (i f t : Bool) (occ : List Occurrence) (b : Bool)
    (h : reading i f t occ = some b) : i = true := by
  unfold reading at h
  cases i <;> simp_all

/-- An escaped use (address taken, designator, member or label of the same
    spelling) refuses. -/
theorem an_escape_refuses (i f t : Bool) (occ : List Occurrence) (h : Occurrence.other ∈ occ) :
    reading i f t occ = none := by
  unfold reading
  have : occ.all Occurrence.admissible = false := by
    rw [List.all_eq_false]
    exact ⟨_, h, by simp [Occurrence.admissible]⟩
  simp [this]

/-- A block-scope declaration refuses. -/
theorem a_block_scope_declaration_refuses (i f t s : Bool) (occ : List Occurrence)
    (h : Occurrence.prototype true s ∈ occ) : reading i f t occ = none := by
  unfold reading
  have : occ.all Occurrence.admissible = false := by
    rw [List.all_eq_false]
    exact ⟨_, h, by simp [Occurrence.admissible]⟩
  simp [this]

/-- No call fixes nothing. -/
theorem no_call_refuses (i f t : Bool) (occ : List Occurrence)
    (h : occ.filterMap Occurrence.callTruth = []) : reading i f t occ = none := by
  unfold reading
  simp [h, agreed]

/-- Every call the unit contains passes a constant of the fixed truth. -/
theorem every_call_agrees (i f t : Bool) (occ : List Occurrence) (b : Bool)
    (h : reading i f t occ = some b) :
    ∀ a ar, Occurrence.call a ar ∈ occ → ∃ v, a = some v ∧ v ≤ bound ∧ (v != 0) = b := by
  intro a ar hmem
  unfold reading at h
  split at h
  · have hm : truthOf a ∈ occ.filterMap Occurrence.callTruth :=
      List.mem_filterMap.mpr ⟨_, hmem, rfl⟩
    have := agreed_mem h _ hm
    cases a with
    | none => simp [truthOf] at this
    | some v =>
      simp only [truthOf] at this
      split at this
      · rename_i hv
        exact ⟨v, rfl, hv, by simpa using this⟩
      · simp at this
  · simp at h

/-- A non-constant actual at any call refuses. -/
theorem a_variable_actual_refuses (i f t ar : Bool) (occ : List Occurrence)
    (h : Occurrence.call none ar ∈ occ) : reading i f t occ = none := by
  cases hr : reading i f t occ with
  | none => rfl
  | some b =>
    obtain ⟨v, hv, _⟩ := every_call_agrees i f t occ b hr none ar h
    simp at hv

/-- Calls of both truths refuse. -/
theorem mixed_truths_refuse (i f t a1 a2 : Bool) (occ : List Occurrence) (v w : Nat)
    (h0 : v = 0) (h1 : w ≠ 0)
    (m1 : Occurrence.call (some v) a1 ∈ occ) (m2 : Occurrence.call (some w) a2 ∈ occ) :
    reading i f t occ = none := by
  cases hr : reading i f t occ with
  | none => rfl
  | some b =>
    obtain ⟨_, e1, _, t1⟩ := every_call_agrees i f t occ b hr _ _ m1
    obtain ⟨_, e2, _, t2⟩ := every_call_agrees i f t occ b hr _ _ m2
    simp at e1 e2
    subst e1 e2
    simp_all

/-- The selected edge is the edge C takes for every value of the fixed truth. -/
theorem selection_is_cs_choice (v : Nat) (trueIsNull : Bool) :
    selectIndex (v != 0) trueIsNull = taken v trueIsNull := by
  unfold selectIndex taken
  by_cases h : v = 0 <;> cases trueIsNull <;> simp [h]

/-- So at every call the unit contains, the selected arm is C's arm. -/
theorem selected_arm_is_taken_at_every_call (i f t : Bool) (occ : List Occurrence) (b : Bool)
    (h : reading i f t occ = some b) (trueIsNull : Bool) :
    ∀ a ar, Occurrence.call a ar ∈ occ →
      ∃ v, a = some v ∧ selectIndex b trueIsNull = taken v trueIsNull := by
  intro a ar hmem
  obtain ⟨v, hv, _, ht⟩ := every_call_agrees i f t occ b h a ar hmem
  exact ⟨v, hv, by rw [← ht]; exact selection_is_cs_choice v trueIsNull⟩

/-- The rule only removes an edge: every successor after it was one before. -/
theorem selection_only_removes_edges (fixed : Option Bool) (tin : Bool) (s : Nat × Nat) :
    ∀ x ∈ selectSuccessors fixed tin s, x ∈ [s.1, s.2] := by
  intro x hx
  cases fixed with
  | none => simpa [selectSuccessors] using hx
  | some t =>
    simp only [selectSuccessors] at hx
    split at hx <;> simp_all

/-- No reading leaves the branch exactly as built. -/
theorem no_reading_is_the_identity (tin : Bool) (s : Nat × Nat) :
    selectSuccessors none tin s = [s.1, s.2] := rfl

/-- The false arm is not made safe: under a zero actual the zero arm is the
    one kept. -/
theorem a_zero_actual_keeps_the_zero_arm (trueIsNull : Bool) :
    selectIndex false trueIsNull = taken 0 trueIsNull := selection_is_cs_choice 0 trueIsNull

/-- A read constant converts to a value of the same truth in every admitted
    type: it is below 2^15, inside every `int`'s range, and `_Bool` keeps
    nonzero-ness by definition. -/
theorem a_read_constant_fits (v : Nat) (h : v ≤ bound) : v < 2 ^ 15 := by
  unfold bound at h
  omega

end QCore.StaticConstantFormal
