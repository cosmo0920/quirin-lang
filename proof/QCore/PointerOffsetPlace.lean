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

import QCore.TypedPlace

/-!
# A pointer offset read as its subscript (card K7-RESIDUAL-CAUSES, rule 78)

The abstract rule implemented by `crates/qcore-c-gateway/src/pointer_offset_place.rs`
(`offset_operands`) and its two hooks in `typed_access_path.rs` (`Reader::path`).

## The C fact

C11 6.5.2.1p2 defines `E1[E2]` as `(*((E1)+(E2)))`, and 6.5.6p8 makes `P + I`
(and `I + P`) the pointer `I` elements past `P`, `P - I` the pointer `I`
elements before it. A pointer value is modelled as an object and an element
offset; the object an indirection designates is its operand's value. So
`*(P + I)` designates what `P[I]` designates, `*(P - I)` what `P[-I]`
designates, and `(P + I)[K]` what `P[I + K]` designates
(`deref_offset_is_subscript`, `deref_difference_is_subscript`,
`subscript_of_offset`).

## The reader

`stepsOf` is the reader's walk over a spelled place (with this rule) and
`stepsOld` the walk before it. An admitted offset under an indirection or a
subscript makes exactly the step the subscript spelling makes, decided on the
pointer operand as for a subscript's base, so the place, every decision taken
on it (`TypedPlace.decidePlace`) and its lowering are the subscript
spelling's (`a_deref_offset_reads_as_the_subscript`,
`an_offset_subscript_reads_as_the_subscript`, `the_decision_is_the_subscripts`).
A form `offset_operands` refuses keeps the reader's older refusal
(`a_refused_offset_keeps_the_refusal`), and every older reading stands
(`every_older_reading_stands`).

## What this is not

Faithful Clang extraction (operand types, opcodes, the transparent casts the
reader strips) and the Rust-Lean correspondence are review premises (trust
boundary 28); `offsetVectors` binds the Rust table mechanically
(`pointer_offset_place.rs`'s unit test). Bounds are outside the property, as
for every subscript.
-/

namespace QCore.PointerOffsetPlace

open QCore.TypedPlace (Step RootKind decidePlace)

/-! ## The C fact -/

/-- A pointer value: an object and an element offset into it. -/
structure Ptr where
  object : Nat
  offset : Int
  deriving DecidableEq, Repr

/-- `P + I` (C11 6.5.6p8). -/
def add (p : Ptr) (i : Int) : Ptr := ⟨p.object, p.offset + i⟩

/-- The object `*P` designates. -/
def deref (p : Ptr) : Ptr := p

/-- `P[I]`, by its definition `*((P)+(I))` (C11 6.5.2.1p2). -/
def subscript (p : Ptr) (i : Int) : Ptr := deref (add p i)

theorem deref_offset_is_subscript (p : Ptr) (i : Int) : deref (add p i) = subscript p i := rfl

/-- `*(P - I)` is `P[-I]`. -/
theorem deref_difference_is_subscript (p : Ptr) (i : Int) :
    deref (add p (-i)) = subscript p (-i) := rfl

/-- `(P + I)[K]` is `P[I + K]`. -/
theorem subscript_of_offset (p : Ptr) (i k : Int) :
    subscript (add p i) k = subscript p (i + k) := by
  simp [subscript, deref, add, Int.add_assoc]

/-! ## The operand table (`offset_operands`) -/

inductive Operand where
  | pointer
  | integer
  | other
  deriving DecidableEq, Repr

inductive Op where
  | add
  | sub
  | other
  deriving DecidableEq, Repr

/-- A binary operator node: its opcode, its operands' classes and whether its
    own type is an object pointer. -/
structure Offset where
  op : Op
  left : Operand
  right : Operand
  pointerResult : Bool
  deriving DecidableEq, Repr

/-- `offset_operands`: `some true` when the pointer operand is on the left,
    `some false` when on the right, `none` when refused. -/
def side (o : Offset) : Option Bool :=
  if o.pointerResult then
    match o.op, o.left, o.right with
    | .add, .pointer, .integer => some true
    | .sub, .pointer, .integer => some true
    | .add, .integer, .pointer => some false
    | _, _, _ => none
  else none

def admitted (o : Offset) : Bool := (side o).isSome

/-- The Rust `OFFSET_VECTORS`. -/
def offsetVectors : List (Offset × Bool) := [
  (⟨.add, .pointer, .integer, true⟩, true),
  (⟨.add, .integer, .pointer, true⟩, true),
  (⟨.sub, .pointer, .integer, true⟩, true),
  (⟨.sub, .integer, .pointer, true⟩, false),
  (⟨.sub, .pointer, .pointer, false⟩, false),
  (⟨.add, .pointer, .other, true⟩, false),
  (⟨.add, .other, .pointer, true⟩, false),
  (⟨.add, .integer, .integer, false⟩, false),
  (⟨.add, .pointer, .integer, false⟩, false),
  (⟨.other, .pointer, .integer, true⟩, false)
]

theorem offset_vectors_hold :
    offsetVectors.all (fun (o, a) => admitted o == a) = true := by
  decide

/-- A difference of two pointers is an integer: never read as an offset. -/
theorem a_pointer_difference_is_refused (b : Bool) :
    admitted ⟨.sub, .pointer, .pointer, b⟩ = false := by
  cases b <;> rfl

/-- `I - P` is not a C pointer; never read. -/
theorem an_integer_minus_a_pointer_is_refused (b : Bool) :
    admitted ⟨.sub, .integer, .pointer, b⟩ = false := by
  cases b <;> rfl

/-- A sum whose type is not an object pointer is never read as an offset. -/
theorem a_non_pointer_result_is_refused (op : Op) (l r : Operand) :
    admitted ⟨op, l, r, false⟩ = false := rfl

/-- A floating or otherwise unlisted operand is refused. -/
theorem an_other_operand_is_refused (op : Op) (l : Operand) (b : Bool) :
    admitted ⟨op, l, .other, b⟩ = false ∧ admitted ⟨op, .other, l, b⟩ = false := by
  cases op <;> cases l <;> cases b <;> decide

/-! ## The reader -/

/-- A spelled place: a root, member, subscript and indirection steps, and an
    offset node (whose other operand is an index expression the reader reads
    as a scalar; `indexOk` says whether that read succeeded). -/
inductive Spelled where
  | root (kind : RootKind)
  | arrow (base : Spelled)
  | dot (base : Spelled)
  | index (base : Spelled)
  | deref (base : Spelled)
  | offset (o : Offset) (indexOk : Bool) (pointer : Spelled)
  | other
  deriving Repr

/-- Whether a subscript's base reads a pointer from memory
    (`union_bitfield::subscript_hop`): an array root decays, anything else
    is a pointer value. -/
def Spelled.hop : Spelled → Bool
  | .root .array => false
  | _ => true

def indexStep (b : Spelled) : Step := if b.hop then .indexPointer else .indexArray

def derefStep (b : Spelled) : Step := if b.hop then .derefPointer else .derefArray

/-- The walk before this rule: an offset node is an unlisted form. -/
def stepsOld : Spelled → Option (List Step)
  | .root _ => some []
  | .arrow b => (stepsOld b).map (· ++ [.arrow])
  | .dot b => (stepsOld b).map (· ++ [.dot])
  | .index b => (stepsOld b).map (· ++ [indexStep b])
  | .deref b => (stepsOld b).map (· ++ [derefStep b])
  | .offset _ _ _ => none
  | .other => none

/-- The walk with this rule: an admitted offset whose index expression reads
    is the subscript step on its pointer operand, under `*` and under `[]`. -/
def stepsOf : Spelled → Option (List Step)
  | .root _ => some []
  | .arrow b => (stepsOf b).map (· ++ [.arrow])
  | .dot b => (stepsOf b).map (· ++ [.dot])
  | .index (.offset o ok p) =>
      if admitted o && ok then (stepsOf p).map (· ++ [indexStep p]) else none
  | .index b => (stepsOf b).map (· ++ [indexStep b])
  | .deref (.offset o ok p) =>
      if admitted o && ok then (stepsOf p).map (· ++ [indexStep p]) else none
  | .deref b => (stepsOf b).map (· ++ [derefStep b])
  | .offset _ _ _ => none
  | .other => none

/-- Not an offset node. -/
def Spelled.plain : Spelled → Bool
  | .offset _ _ _ => false
  | _ => true

/-- `*(P + I)` reads as `P[I]` (the pointer operand not itself an offset: the
    reader reads one offset per step). -/
theorem a_deref_offset_reads_as_the_subscript (o : Offset) (p : Spelled)
    (h : admitted o = true) (hp : p.plain = true) :
    stepsOf (.deref (.offset o true p)) = stepsOf (.index p) := by
  cases p <;> simp_all [stepsOf, Spelled.plain]

/-- `(P + I)[K]` reads as `P[I + K]`: one subscript step on `P`. -/
theorem an_offset_subscript_reads_as_the_subscript (o : Offset) (p : Spelled)
    (h : admitted o = true) (hp : p.plain = true) :
    stepsOf (.index (.offset o true p)) = stepsOf (.index p) := by
  cases p <;> simp_all [stepsOf, Spelled.plain]

/-- A pointer operand that is itself an offset (`*((P + I) + J)`) is not a
    place: the reader keeps its refusal. -/
theorem a_nested_offset_is_refused (o o' : Offset) (ok ok' : Bool) (p : Spelled) :
    stepsOf (.deref (.offset o ok (.offset o' ok' p))) = none := by
  simp [stepsOf]

/-- The place's decision (rule 10t's table) is the subscript spelling's. -/
theorem the_decision_is_the_subscripts (o : Offset) (p : Spelled) (k : RootKind)
    (h : admitted o = true) (hp : p.plain = true) :
    (stepsOf (.deref (.offset o true p))).map (decidePlace k) =
      (stepsOf (.index p)).map (decidePlace k) := by
  rw [a_deref_offset_reads_as_the_subscript o p h hp]

/-- A refused offset, or one whose index expression the reader refuses,
    keeps the older refusal. -/
theorem a_refused_offset_keeps_the_refusal (o : Offset) (ok : Bool) (p : Spelled)
    (h : admitted o = false ∨ ok = false) :
    stepsOf (.deref (.offset o ok p)) = none ∧ stepsOf (.index (.offset o ok p)) = none := by
  rcases h with h | h <;> simp [stepsOf, h]

/-- A bare offset is never a place (the reader is asked for a place, not a
    pointer value). -/
theorem a_bare_offset_is_not_a_place (o : Offset) (ok : Bool) (p : Spelled) :
    stepsOf (.offset o ok p) = none := rfl

theorem stepsOf_index_plain (b : Spelled) (h : b.plain = true) :
    stepsOf (.index b) = (stepsOf b).map (· ++ [indexStep b]) := by
  cases b <;> simp_all [Spelled.plain, stepsOf]

theorem stepsOf_deref_plain (b : Spelled) (h : b.plain = true) :
    stepsOf (.deref b) = (stepsOf b).map (· ++ [derefStep b]) := by
  cases b <;> simp_all [Spelled.plain, stepsOf]

theorem stepsOld_offset (o : Offset) (ok : Bool) (p : Spelled) :
    stepsOld (.offset o ok p) = none := rfl

/-- Every reading the walk made before this rule stands, step for step. -/
theorem every_older_reading_stands (e : Spelled) (s : List Step)
    (h : stepsOld e = some s) : stepsOf e = some s := by
  induction e generalizing s with
  | root k => simpa [stepsOld, stepsOf] using h
  | arrow b ih =>
    simp only [stepsOld, Option.map_eq_some_iff] at h
    obtain ⟨t, ht, rfl⟩ := h
    simp [stepsOf, ih t ht]
  | dot b ih =>
    simp only [stepsOld, Option.map_eq_some_iff] at h
    obtain ⟨t, ht, rfl⟩ := h
    simp [stepsOf, ih t ht]
  | index b ih =>
    simp only [stepsOld, Option.map_eq_some_iff] at h
    obtain ⟨t, ht, rfl⟩ := h
    have hp : b.plain = true := by
      cases b <;> simp_all [Spelled.plain, stepsOld_offset]
    rw [stepsOf_index_plain b hp, ih t ht]
    rfl
  | deref b ih =>
    simp only [stepsOld, Option.map_eq_some_iff] at h
    obtain ⟨t, ht, rfl⟩ := h
    have hp : b.plain = true := by
      cases b <;> simp_all [Spelled.plain, stepsOld_offset]
    rw [stepsOf_deref_plain b hp, ih t ht]
    rfl
  | offset o ok p _ => simp [stepsOld] at h
  | other => simp [stepsOld] at h

/-- The census's two shapes: `*(str + 1)` on a pointer formal and
    `(b->content + b->offset)[0]` read as `str[1]` and `b->content[b->offset]`. -/
theorem the_census_shapes_read :
    stepsOf (.deref (.offset ⟨.add, .pointer, .integer, true⟩ true (.root .pointer))) =
        some [.indexPointer] ∧
      stepsOf (.index (.offset ⟨.add, .pointer, .integer, true⟩ true
        (.arrow (.root .pointer)))) = some [.arrow, .indexPointer] := by
  decide

end QCore.PointerOffsetPlace
