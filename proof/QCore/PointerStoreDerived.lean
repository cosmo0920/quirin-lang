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

import QCore.PointerStore

/-!
# Pointer arithmetic over the destination's own object (card K3-POINTER-VALUED-STORES, rule 10v amended)

The abstract rule implemented by `crates/qcore-c-gateway/src/pointer_store_derived.rs`
(`read`, `same_spelling`) and its hook in `pointer_store.rs` (`classify_store`).

A pointer store `P = Q ± E` (`E` an integer, casts pointer-to-pointer only) is
read as `PointerStore.Value.sameObjectDerived` when `Q` is a pointer read of
the same place as `P` (the same side-effect-free spelling) or of a pointer
member of `P`'s own object (both places root-only over one root). The class is
neutral: the owner then decides exactly as for every other neutral value
(`decideStore`), and rule 10v's old-value check covers `Q`, which is rooted at
the destination's root.

Why the store moves no custody, in this model:

* **provenance** (C11 6.5.6p8): `Q ± E` points into the object `Q` points into;
  `offset_keeps_provenance`;
* **no new reference**: after the store the owner references only objects it
  already referenced (`a_derived_store_adds_no_reference`), which is exactly
  what Q36's example C lacks: a formal's pointer may add one
  (`a_foreign_value_may_add_a_reference`), the missing owner premise.

## What this is not

A model of Clang. Faithful extraction of types, cast kinds, declaration ids and
spellings is trust boundary 28; `readingVectors` binds the Rust `read` table
mechanically (`tests/unit/pointer_store_derived.rs`).
-/

namespace QCore.PointerStoreDerived

open QCore.PointerStore (Value OwnerClass Decision decideStore)

/-! ## The reading -/

inductive Reading where
  | samePlace
  | sameObject
  | refused
  deriving DecidableEq, Repr

/-- The facts `read` decides on: the offset is an integer and every cast is
pointer-to-pointer; the read place has the destination's spelling; the roots
agree; each place is root-only. -/
structure Facts where
  integerOffset : Bool
  pointerCasts : Bool
  sameSpelling : Bool
  sameRoot : Bool
  destRootOnly : Bool
  valueRootOnly : Bool
  deriving DecidableEq, Repr

def read (f : Facts) : Reading :=
  if !(f.integerOffset && f.pointerCasts) then .refused
  else if f.sameSpelling then .samePlace
  else if f.sameRoot && f.destRootOnly && f.valueRootOnly then .sameObject
  else .refused

/-- The class a reading gives the store's value. -/
def valueOf : Reading → Value
  | .samePlace | .sameObject => .sameObjectDerived
  | .refused => .arithmetic

theorem a_non_integer_offset_refuses (f : Facts) (h : f.integerOffset = false) :
    read f = .refused := by
  simp [read, h]

theorem an_integer_cast_refuses (f : Facts) (h : f.pointerCasts = false) :
    read f = .refused := by
  simp [read, h]

theorem another_root_refuses (f : Facts) (hs : f.sameSpelling = false) (hr : f.sameRoot = false) :
    read f = .refused := by
  simp [read, hs, hr]

theorem a_place_past_the_root_refuses (f : Facts) (hs : f.sameSpelling = false)
    (h : f.destRootOnly = false ∨ f.valueRootOnly = false) : read f = .refused := by
  rcases h with h | h <;> simp [read, hs, h]

/-- An admitted reading is the same place or the same object, never anything
else. -/
theorem admitted_only_from_the_own_object (f : Facts) (h : read f ≠ .refused) :
    f.integerOffset = true ∧ f.pointerCasts = true ∧
      (f.sameSpelling = true ∨ (f.sameRoot = true ∧ f.destRootOnly = true ∧ f.valueRootOnly = true)) := by
  unfold read at h
  cases hi : f.integerOffset <;> cases hp : f.pointerCasts <;> cases hs : f.sameSpelling <;>
    cases hr : f.sameRoot <;> cases hd : f.destRootOnly <;> cases hv : f.valueRootOnly <;>
    simp_all

/-- A refused reading is the gate's arithmetic refusal (`valueShape`), whatever
the owner. -/
theorem a_refused_reading_keeps_the_refusal (o : OwnerClass) :
    decideStore (valueOf .refused) false o = .valueShape := by
  rfl

/-- An admitted reading decides exactly as rule 10v decides a neutral value: an
untracked owner admits, a tracked or unresolved owner refuses. -/
theorem an_admitted_reading_is_decided_by_the_owner (r : Reading) (h : r ≠ .refused)
    (o : OwnerClass) :
    decideStore (valueOf r) false o = .admit ↔ o = .untracked := by
  cases r <;> cases o <;> simp_all [valueOf, decideStore, Value.neutral]

/-- The Rust `READING_VECTORS`: a statement spelling and its reading. -/
def readingVectors : List (String × Reading) := [
  ("s->name = s->name + n", .samePlace),
  ("s->state = (char *) s->state + n", .samePlace),
  ("s->name = n + s->name", .samePlace),
  ("s->name = s->name - 1", .samePlace),
  ("s->end = s->name + n", .sameObject),
  ("s->slots[i] = s->slots[i] + 1", .samePlace),
  ("v->io[i].base = (char *) v->io[i].base + n", .samePlace),
  ("s->self->name = s->self->name + 1", .samePlace),
  ("s->name = b + n", .refused),
  ("s->name = t->name + n", .refused),
  ("s->self->name = s->name + 1", .refused),
  ("s->end = s->self->name + 1", .refused),
  ("s->slots[i] = s->slots[j] + 1", .sameObject),
  ("s->slots[f()] = s->slots[f()] + 1", .sameObject),
  ("s->name = (char *) (s->end - s->name)", .refused),
  ("s->name = (char *) ((long) s->name + n)", .refused)
]

/-- The facts each vector stands for (what Clang's AST gives `read`). -/
def vectorFacts : List Facts := [
  ⟨true, true, true, true, true, true⟩,
  ⟨true, true, true, true, true, true⟩,
  ⟨true, true, true, true, true, true⟩,
  ⟨true, true, true, true, true, true⟩,
  ⟨true, true, false, true, true, true⟩,
  ⟨true, true, true, true, true, true⟩,
  ⟨true, true, true, true, false, false⟩,
  ⟨true, true, true, true, false, false⟩,
  ⟨true, true, false, false, true, true⟩,
  ⟨true, true, false, false, true, true⟩,
  ⟨true, true, false, true, false, true⟩,
  ⟨true, true, false, true, true, false⟩,
  ⟨true, true, false, true, true, true⟩,
  ⟨true, true, false, true, true, true⟩,
  ⟨true, false, false, true, true, true⟩,
  ⟨true, false, false, true, true, true⟩
]

theorem reading_vectors_hold :
    (readingVectors.zip vectorFacts).all (fun ((_, r), f) => read f == r) = true ∧
      readingVectors.length = vectorFacts.length := by
  decide

/-! ## Provenance and references -/

/-- A pointer value: the object it points into and an offset. -/
structure Ptr where
  prov : Nat
  off : Int
  deriving DecidableEq, Repr

/-- `Q ± E`: C11 6.5.6p8 keeps the result in `Q`'s array object. -/
def offset (p : Ptr) (e : Int) : Ptr := { p with off := p.off + e }

theorem offset_keeps_provenance (p : Ptr) (e : Int) : (offset p e).prov = p.prov := rfl

/-- An object's pointer members: member id and value. -/
structure Obj where
  members : List (Nat × Ptr)

/-- The objects an object references through its pointer members. -/
def refs (o : Obj) : List Nat := o.members.map (fun m => m.2.prov)

def readMember (q : Nat) : List (Nat × Ptr) → Option Ptr
  | [] => none
  | (k, v) :: rest => if k = q then some v else readMember q rest

def setMember (o : Obj) (m : Nat) (p : Ptr) : Obj :=
  ⟨o.members.map (fun kv => if kv.1 = m then (kv.1, p) else kv)⟩

theorem readMember_mem (q : Nat) (p : Ptr) :
    ∀ l : List (Nat × Ptr), readMember q l = some p → p.prov ∈ l.map (fun m => m.2.prov)
  | [], h => by simp [readMember] at h
  | (k, v) :: rest, h => by
    unfold readMember at h
    by_cases hk : k = q
    · simp [hk] at h; subst h; simp
    · simp [hk] at h
      exact List.mem_cons_of_mem _ (readMember_mem q p rest h)

/-- The store of `Q ± E`, `Q` read from member `q` of the owner (the same place
when `q = m`, the same object otherwise), adds no reference the owner did not
already hold. -/
theorem a_derived_store_adds_no_reference (o : Obj) (m q : Nat) (p : Ptr) (e : Int)
    (h : readMember q o.members = some p) :
    ∀ x, x ∈ refs (setMember o m (offset p e)) → x ∈ refs o := by
  intro x hx
  simp only [refs, setMember, List.map_map, List.mem_map, Function.comp] at hx
  obtain ⟨⟨k, v⟩, hmem, hval⟩ := hx
  by_cases hk : k = m
  · simp [hk, offset] at hval
    subst hval
    exact readMember_mem q p o.members h
  · simp [hk] at hval
    subst hval
    simp only [refs, List.mem_map]
    exact ⟨(k, v), hmem, rfl⟩

/-- Example C's value is foreign: a store of it can add a reference the owner
did not hold, which is the owner premise Q36 names as missing. -/
theorem a_foreign_value_may_add_a_reference :
    ∃ (o : Obj) (m : Nat) (p : Ptr),
      p.prov ∉ refs o ∧ p.prov ∈ refs (setMember o m p) := by
  refine ⟨⟨[(0, ⟨1, 0⟩)]⟩, 0, ⟨2, 0⟩, ?_, ?_⟩ <;> decide

end QCore.PointerStoreDerived
