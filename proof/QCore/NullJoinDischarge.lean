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
# A null constant on some reaching path is never rule 14n's premised alias (card UNTRACKED-BASE-NULL-JOIN-DISCHARGE, rule 83)

The base obligation of a typed scalar access (`indirect_scalar_base_resolves`)
discharges an unbound base, or a target-free binding that is `unknown` and
not possibly null. Rule 14n adds one reading: a target-free, `unknown`,
possibly-null identity binding is read non-null under a named premise. A
*known* binding is never premised, so the null constant (`p = 0`, a `Clear`:
known, possibly null) refuses.

The pointer join forgot that knowledge. Joining the null constant with an
unbound path (or with a pointer read) gives an `unknown`, possibly-null
binding, which is the premised shape, so the access discharged under a
premise the function's own null constant refutes on one path
(`old_premises_a_joined_null`).

Rule 83 keeps a flag beside the binding, `nullOnPath`: some reaching path
held the name definitely null since its last rebinding. The join is the
disjunction of both sides' flags and of "this side is definitely null"; a
rebinding clears it; the premise refuses a flagged name.

* **Soundness.** Whenever some joined path holds the null constant, the base
  obligation does not discharge (`a_null_on_some_path_is_never_discharged`),
  for every number and order of joined paths.
* **Only refuses more** (`the_fix_only_refuses_more`), and **nothing else
  moves**: with no null constant on any path the flag is clear and the
  verdict is the base's (`no_null_keeps_the_verdict`), so an unbound formal,
  a pointer read and their joins keep it.
* **A guard-proved non-null discharges** whatever the flag says
  (`a_guard_proved_non_null_is_discharged`), and a rebinding forgets the
  null (`an_overwritten_null_keeps_the_premise`).

## What this is not

This models one name's binding over target-free identity bindings, the only
ones rule 14n premises (`QCore.NullDeref.a_tracked_binding_is_never_premised`
covers the rest). It does not model which C statements produce which path
binding (the lowering, trust boundary 28) or whether a null test of an
`unknown` binding is itself admitted (it is refused: rule 10b); the Rust join
is bound to `joinVectors` by
`crates/qcore-c-gateway/tests/unit/legacy_null_join.rs`.
-/

namespace QCore.NullJoinDischarge

/-- One reaching path's binding of a name, as the flow check holds it. -/
inductive PathBinding where
  /-- No binding: an unbound formal, or rule 62's custody-free copy of one. -/
  | unbound
  /-- The null constant (`Clear`), a copy of it, or a null test's null arm. -/
  | nullConst
  /-- A pointer-valued path read: target-free, `unknown`, possibly null. -/
  | pathRead
  /-- A null test's non-null arm of an untracked binding. -/
  | nonNull
  deriving DecidableEq, Repr

/-- A target-free identity binding's two bits. -/
structure Shape where
  unknown : Bool
  mayBeNull : Bool
  deriving DecidableEq, Repr

def shape : PathBinding → Option Shape
  | .unbound => none
  | .nullConst => some ⟨false, true⟩
  | .pathRead => some ⟨true, true⟩
  | .nonNull => some ⟨true, false⟩

/-- `AbstractState::join` on one name: a one-sided binding becomes `unknown`
and possibly null; two bindings join their bits by disjunction. -/
def joinShape : Option Shape → Option Shape → Option Shape
  | none, none => none
  | some _, none => some ⟨true, true⟩
  | none, some _ => some ⟨true, true⟩
  | some a, some b => some ⟨a.unknown || b.unknown, a.mayBeNull || b.mayBeNull⟩

/-- `null_join::definitely_null`: no target, known, possibly null. -/
def definitelyNull : Option Shape → Bool
  | some s => !s.unknown && s.mayBeNull
  | none => false

/-- One name's abstract fact: its binding and rule 83's flag. -/
structure Abs where
  binding : Option Shape
  nullOnPath : Bool
  deriving DecidableEq, Repr

/-- A path's binding right after its statement: a rebinding clears the flag. -/
def single (b : PathBinding) : Abs := ⟨shape b, false⟩

/-- `null_join::join`: union of both flags and of each side's definite null,
kept only where the joined state still binds the name. -/
def join (x y : Abs) : Abs :=
  let b := joinShape x.binding y.binding
  ⟨b, (x.nullOnPath || y.nullOnPath || definitelyNull x.binding || definitelyNull y.binding)
      && b.isSome⟩

/-- Every reaching path joined, in order. -/
def joinAll (first : PathBinding) (rest : List PathBinding) : Abs :=
  rest.foldl (fun acc b => join acc (single b)) (single first)

/-- Rule 14n's premised shape (target-free identity, unwidened). -/
def premisedOld : Option Shape → Bool
  | some s => s.unknown && s.mayBeNull
  | none => false

/-- Rule 14n's premise under rule 83. -/
def premisedNew (a : Abs) : Bool := premisedOld a.binding && !a.nullOnPath

/-- `indirect_scalar_base_resolves` on a target-free identity binding. -/
def resolves : Option Shape → Bool
  | none => true
  | some s => !s.mayBeNull && s.unknown

def dischargedOld (a : Abs) : Bool := resolves a.binding || premisedOld a.binding
def dischargedNew (a : Abs) : Bool := resolves a.binding || premisedNew a

/-! ## Soundness -/

/-- The invariant of a fact some joined path made definitely null. -/
def Tainted (a : Abs) : Prop :=
  ∃ s, a.binding = some s ∧ s.mayBeNull = true ∧ (a.nullOnPath = true ∨ s.unknown = false)

theorem tainted_not_discharged (a : Abs) (h : Tainted a) : dischargedNew a = false := by
  obtain ⟨s, hb, hm, hf⟩ := h
  cases s with
  | mk u m =>
    simp at hm
    subst hm
    rcases hf with hf | hf
    · simp [dischargedNew, premisedNew, resolves, premisedOld, hb, hf]
    · simp at hf
      subst hf
      simp [dischargedNew, premisedNew, resolves, premisedOld, hb]

theorem tainted_join_left (x y : Abs) (h : Tainted x) : Tainted (join x y) := by
  obtain ⟨⟨u, m⟩, hb, hm, hf⟩ := h
  simp at hm
  subst hm
  cases hy : y.binding with
  | none =>
    refine ⟨⟨true, true⟩, ?_, rfl, Or.inl ?_⟩
    · simp [join, joinShape, hb, hy]
    · rcases hf with hf | hf
      · simp [join, joinShape, hb, hy, hf]
      · simp at hf
        subst hf
        simp [join, joinShape, definitelyNull, hb, hy]
  | some t =>
    refine ⟨⟨u || t.unknown, true⟩, ?_, rfl, Or.inl ?_⟩
    · simp [join, joinShape, hb, hy]
    · rcases hf with hf | hf
      · simp [join, joinShape, hb, hy, hf]
      · simp at hf
        subst hf
        simp [join, joinShape, definitelyNull, hb, hy]

theorem tainted_join_null (x : Abs) : Tainted (join x (single .nullConst)) := by
  cases hx : x.binding with
  | none =>
    refine ⟨⟨true, true⟩, ?_, rfl, Or.inl ?_⟩
    · simp [join, joinShape, single, shape, hx]
    · simp [join, joinShape, single, shape, definitelyNull, hx]
  | some t =>
    refine ⟨⟨t.unknown || false, t.mayBeNull || true⟩, ?_, ?_, Or.inl ?_⟩
    · simp [join, joinShape, single, shape, hx]
    · simp
    · simp [join, joinShape, single, shape, definitelyNull, hx]

theorem tainted_fold (rest : List PathBinding) (acc : Abs) (h : Tainted acc) :
    Tainted (rest.foldl (fun acc b => join acc (single b)) acc) := by
  induction rest generalizing acc with
  | nil => exact h
  | cons b bs ih => exact ih _ (tainted_join_left _ _ h)

theorem tainted_fold_of_mem (rest : List PathBinding) (acc : Abs)
    (h : PathBinding.nullConst ∈ rest) :
    Tainted (rest.foldl (fun acc b => join acc (single b)) acc) := by
  induction rest generalizing acc with
  | nil => cases h
  | cons b bs ih =>
    simp only [List.foldl]
    cases List.mem_cons.mp h with
    | inl hb =>
      subst hb
      exact tainted_fold bs _ (tainted_join_null acc)
    | inr hm => exact ih _ hm

/-- **Soundness**: if any joined path holds the null constant, the base
obligation does not discharge, premise or no premise. -/
theorem a_null_on_some_path_is_never_discharged (first : PathBinding)
    (rest : List PathBinding) (h : PathBinding.nullConst ∈ first :: rest) :
    dischargedNew (joinAll first rest) = false := by
  apply tainted_not_discharged
  unfold joinAll
  cases List.mem_cons.mp h with
  | inl hf =>
    subst hf
    exact tainted_fold rest _ ⟨⟨false, true⟩, rfl, rfl, Or.inr rfl⟩
  | inr hm => exact tainted_fold_of_mem rest _ hm

theorem a_null_on_some_path_is_never_premised (first : PathBinding)
    (rest : List PathBinding) (h : PathBinding.nullConst ∈ first :: rest) :
    premisedNew (joinAll first rest) = false := by
  have := a_null_on_some_path_is_never_discharged first rest h
  simp [dischargedNew] at this
  exact this.2

/-! ## Only refuses more, and nothing else moves -/

theorem the_fix_only_refuses_more (a : Abs) (h : dischargedNew a = true) :
    dischargedOld a = true := by
  cases hr : resolves a.binding <;> cases hp : premisedOld a.binding <;>
    simp_all [dischargedNew, dischargedOld, premisedNew]

/-- The invariant of a fact no joined path made definitely null. -/
def Clean (a : Abs) : Prop :=
  a.nullOnPath = false ∧ (a.binding = none ∨ ∃ s, a.binding = some s ∧ s.unknown = true)

theorem clean_single (b : PathBinding) (h : b ≠ .nullConst) : Clean (single b) := by
  cases b with
  | unbound => exact ⟨rfl, Or.inl rfl⟩
  | nullConst => exact absurd rfl h
  | pathRead => exact ⟨rfl, Or.inr ⟨⟨true, true⟩, rfl, rfl⟩⟩
  | nonNull => exact ⟨rfl, Or.inr ⟨⟨true, false⟩, rfl, rfl⟩⟩

theorem clean_join (x y : Abs) (hx : Clean x) (hy : Clean y) : Clean (join x y) := by
  obtain ⟨fx, bx⟩ := hx
  obtain ⟨fy, by'⟩ := hy
  rcases bx with bx | ⟨s, bx, us⟩ <;> rcases by' with by' | ⟨t, by', ut⟩
  · exact ⟨by simp [join, joinShape, bx, by'], Or.inl (by simp [join, joinShape, bx, by'])⟩
  · exact ⟨by simp [join, joinShape, definitelyNull, bx, by', fx, fy, ut],
      Or.inr ⟨⟨true, true⟩, by simp [join, joinShape, bx, by'], rfl⟩⟩
  · exact ⟨by simp [join, joinShape, definitelyNull, bx, by', fx, fy, us],
      Or.inr ⟨⟨true, true⟩, by simp [join, joinShape, bx, by'], rfl⟩⟩
  · exact ⟨by simp [join, joinShape, definitelyNull, bx, by', fx, fy, us, ut],
      Or.inr ⟨⟨s.unknown || t.unknown, s.mayBeNull || t.mayBeNull⟩,
        by simp [join, joinShape, bx, by'], by simp [us]⟩⟩

theorem clean_fold (rest : List PathBinding) (acc : Abs) (h : Clean acc)
    (hr : PathBinding.nullConst ∉ rest) :
    Clean (rest.foldl (fun acc b => join acc (single b)) acc) := by
  induction rest generalizing acc with
  | nil => exact h
  | cons b bs ih =>
    simp only [List.foldl]
    have hb : b ≠ .nullConst := fun e => hr (e ▸ List.mem_cons_self ..)
    have hbs : PathBinding.nullConst ∉ bs := fun m => hr (List.mem_cons_of_mem _ m)
    exact ih _ (clean_join _ _ h (clean_single b hb)) hbs

/-- **Precision**: with no null constant on any joined path, the flag is
clear and the verdict is exactly the base's. -/
theorem no_null_keeps_the_verdict (first : PathBinding) (rest : List PathBinding)
    (h : PathBinding.nullConst ∉ first :: rest) :
    dischargedNew (joinAll first rest) = dischargedOld (joinAll first rest) := by
  have hf : first ≠ .nullConst := fun e => h (e ▸ List.mem_cons_self ..)
  have hr : PathBinding.nullConst ∉ rest := fun m => h (List.mem_cons_of_mem _ m)
  obtain ⟨clear, _⟩ := clean_fold rest _ (clean_single first hf) hr
  unfold joinAll
  simp [dischargedNew, dischargedOld, premisedNew, clear]

/-! ## Guards and rebinding -/

/-- A null test's non-null arm discharges the obligation whatever the flag
says: rule 83 only withholds the *premise*, never a proved non-null. -/
theorem a_guard_proved_non_null_is_discharged (flag : Bool) :
    dischargedNew ⟨some ⟨true, false⟩, flag⟩ = true := by
  simp [dischargedNew, resolves]

/-- A rebinding replaces the whole fact (`single`), so a rebinding to a
pointer read forgets an earlier null and the premise is the base's again,
whatever the fact before it. -/
theorem an_overwritten_null_keeps_the_premise :
    premisedNew (single .pathRead) = premisedOld (single .pathRead).binding ∧
    premisedNew (single .pathRead) = true := by
  decide

/-- An unbound base (an unbound formal and its copies) discharges, as before. -/
theorem an_unbound_base_discharges : dischargedNew (joinAll .unbound [.unbound]) = true := by
  decide

/-! ## The gap -/

/-- At the base, the null constant alone refused, but its join with an
unbound copy discharged under the premise. -/
theorem old_premises_a_joined_null :
    dischargedOld (single .nullConst) = false ∧
    dischargedOld (joinAll .nullConst [.unbound]) = true ∧
    dischargedNew (joinAll .nullConst [.unbound]) = false := by
  decide

/-! ## Vectors bound to the Rust join -/

/-- (label, paths in join order, discharged at the base, discharged under
rule 83). `legacy_null_join.rs`'s unit test builds one state per path, joins
them with `AbstractState::join` and reads the base obligation with and
without rule 83's premise refusal. -/
def joinVectors : List (String × List PathBinding × Bool × Bool) := [
  ("null alone", [.nullConst], false, false),
  ("null then unbound", [.nullConst, .unbound], true, false),
  ("unbound then null", [.unbound, .nullConst], true, false),
  ("null then path read", [.nullConst, .pathRead], true, false),
  ("path read then null", [.pathRead, .nullConst], true, false),
  ("null then non-null", [.nullConst, .nonNull], true, false),
  ("null, unbound, path read", [.nullConst, .unbound, .pathRead], true, false),
  ("unbound alone", [.unbound], true, true),
  ("unbound twice", [.unbound, .unbound], true, true),
  ("path read alone", [.pathRead], true, true),
  ("unbound then path read", [.unbound, .pathRead], true, true),
  ("path read then non-null", [.pathRead, .nonNull], true, true),
  ("non-null alone", [.nonNull], true, true),
  ("non-null twice", [.nonNull, .nonNull], true, true)
]

def vectorVerdict : List PathBinding → Option (Bool × Bool)
  | [] => none
  | first :: rest =>
    let a := joinAll first rest
    some (dischargedOld a, dischargedNew a)

def joinVectorsAgree : Bool :=
  joinVectors.all (fun (_, paths, old, new) => vectorVerdict paths == some (old, new))

theorem join_vectors_hold : joinVectorsAgree = true := by decide

end QCore.NullJoinDischarge
