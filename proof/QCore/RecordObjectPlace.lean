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
# Record-object places with no pointer read (card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS, gateway rule 12j)

The abstract rule implemented by `crates/qcore-c-gateway/src/record_object_place.rs`
(`decide`, `root_storage`) and its hook in `typed_access_path::Reader::path`.

The typed reader admits a place whose first step is `.` only on a root the
custody closure proves never bound, and the tracked-root route of rule 10t
refuses every record-object root. This rule is asked only where the never-bound
premise refused, for a place

* whose every step is a `.` member selection (no `->`, subscript or `*`: no
  pointer is read from memory and dereferenced);
* whose root is a record object of this function's automatic storage (a
  by-value parameter, or a block-scope variable with no storage-class
  specifier);
* whose access is a scalar read or a scalar store.

Where it holds the place lowers to an access with no base and no chain: no
dereference obligation exists, and the store writes only the scalar member's
storage, which no binding names (bindings name pointer places). A store through
a union that can hold a pointer keeps rule 10w's refused-write frame.

## What this is not

A model of the decision and of the lowered access's transfer. That a
block-scope object with no storage-class specifier has automatic storage
duration and is designated only inside its block (C11 6.2.4p5, 6.2.1p4) and the
faithful Clang extraction of declaration ids, storage classes and member steps
are premises (trust boundary 28), not proved.
-/

namespace QCore.RecordObjectPlace

/-! ## The decision -/

inductive RootStorage where
  | automatic
  | parameter
  | specified
  | outside
  deriving DecidableEq, Repr

inductive Access where
  | scalarRead
  | scalarStore
  | other
  deriving DecidableEq, Repr

inductive Decision where
  | admit
  | hop
  | storage
  | access
  deriving DecidableEq, Repr

/-- `record_object_place::decide`. -/
def decide (dotOnly : Bool) (s : RootStorage) (a : Access) : Decision :=
  if !dotOnly then .hop
  else if !(s == .automatic || s == .parameter) then .storage
  else if a == .other then .access
  else .admit

/-- The decision vectors, spelled as `record_object_place::DECISION_VECTORS`. -/
def decisionVectors : List (Bool × RootStorage × Access × Decision) :=
  [ (true, .automatic, .scalarStore, .admit)
  , (true, .automatic, .scalarRead, .admit)
  , (true, .parameter, .scalarRead, .admit)
  , (true, .parameter, .scalarStore, .admit)
  , (false, .automatic, .scalarStore, .hop)
  , (false, .outside, .other, .hop)
  , (true, .specified, .scalarStore, .storage)
  , (true, .outside, .scalarRead, .storage)
  , (true, .automatic, .other, .access)
  , (true, .parameter, .other, .access) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (d, s, a, r) => decide d s a == r) = true := by
  decide

theorem admitted_exactly_on_a_dot_only_scalar_place_of_automatic_storage
    (d : Bool) (s : RootStorage) (a : Access) :
    decide d s a = .admit ↔
      d = true ∧ (s = .automatic ∨ s = .parameter) ∧ a ≠ .other := by
  cases d <;> cases s <;> cases a <;> simp [decide]

/-- A pointer read anywhere on the path keeps the refusal. -/
theorem a_hop_is_never_admitted (s : RootStorage) (a : Access) :
    decide false s a = .hop := by
  simp [decide]

/-- A `static`, `extern`, `register` or thread-local root, or a file-scope
object, keeps the refusal. -/
theorem a_root_outside_automatic_storage_is_refused (a : Access)
    (s : RootStorage) (h : s = .specified ∨ s = .outside) : decide true s a = .storage := by
  rcases h with h | h <;> subst h <;> simp [decide]

/-- A pointer value, an address, a pointer-valued store or a whole aggregate
keeps the refusal. -/
theorem a_non_scalar_access_is_refused (s : RootStorage)
    (h : s = .automatic ∨ s = .parameter) : decide true s .other = .access := by
  rcases h with h | h <;> subst h <;> simp [decide]

/-! ## The interlock with the never-bound premise -/

/-- The reader's answer for a `.`-rooted place: the never-bound premise first
(`old`, unchanged), and this rule only where it refused. -/
def read (neverBound : Bool) (old : Bool) (d : Bool) (s : RootStorage) (a : Access) : Bool :=
  if neverBound then old else decide d s a == .admit

theorem the_never_bound_answer_is_unchanged (old d : Bool) (s : RootStorage) (a : Access) :
    read true old d s a = old := by
  simp [read]

theorem the_rule_only_adds (neverBound old d : Bool) (s : RootStorage) (a : Access)
    (h : neverBound = true) (hold : old = true) : read neverBound old d s a = true := by
  simp [read, h, hold]

/-! ## The lowered access -/

/-- A binding of a pointer place (the projection's bindings name pointer places
only). -/
structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  deriving DecidableEq, Repr

abbrev State := Nat → Option Binding

/-- The access an admitted place lowers to: no base, no chain; `frame` is rule
10w's refused-write frame of a store through a union that can hold a pointer. -/
structure Lowered where
  bases : List Nat
  chains : List Nat
  frame : Bool
  deriving DecidableEq, Repr

def lower (unionFrame : Bool) (a : Access) : Lowered :=
  { bases := [], chains := [], frame := a == .scalarStore && unionFrame }

/-- The refused-write frame poisons every binding but the spared ones. -/
def poison (spared : Nat → Bool) (st : State) : State := fun p =>
  match st p with
  | some b => if spared p then some b else some { b with unknown := true, mayBeNull := true }
  | none => none

/-- `IndirectScalarAccess`'s transfer of the lowered access with no destination:
each base and chain is an obligation that reports and changes nothing; the
frame poisons. -/
def transfer (spared : Nat → Bool) (l : Lowered) (st : State) : State :=
  if l.frame then poison spared st else st

/-- The lowered access carries no dereference obligation. -/
theorem an_admitted_place_has_no_obligation (u : Bool) (a : Access) :
    (lower u a).bases = [] ∧ (lower u a).chains = [] := by
  simp [lower]

/-- A scalar read changes no binding. -/
theorem a_scalar_read_changes_no_binding (u : Bool) (spared : Nat → Bool) (st : State) :
    transfer spared (lower u .scalarRead) st = st := by
  simp [transfer, lower]

/-- A scalar store into a member of a custody-free record changes no binding. -/
theorem a_scalar_store_changes_no_binding (spared : Nat → Bool) (st : State) :
    transfer spared (lower false .scalarStore) st = st := by
  simp [transfer, lower]

/-- A scalar store through a union that can hold a pointer keeps the frame:
no binding but a spared one keeps a known value. -/
theorem a_union_store_keeps_the_frame (spared : Nat → Bool) (st : State) (p : Nat)
    (b : Binding) (hb : st p = some b) (hs : spared p = false) :
    transfer spared (lower true .scalarStore) st p =
      some { b with unknown := true, mayBeNull := true } := by
  simp [transfer, lower, poison, hb, hs]

/-- The frame only weakens: it never adds a target. -/
theorem the_frame_adds_no_target (spared : Nat → Bool) (st : State) (p : Nat) (b : Binding)
    (h : poison spared st p = some b) : ∃ b0, st p = some b0 ∧ b.targets = b0.targets := by
  unfold poison at h
  cases hp : st p with
  | none => simp [hp] at h
  | some b0 =>
      refine ⟨b0, rfl, ?_⟩
      simp only [hp] at h
      by_cases hs : spared p = true
      · simp [hs] at h; rw [← h]
      · simp [hs] at h; rw [← h]

end QCore.RecordObjectPlace
