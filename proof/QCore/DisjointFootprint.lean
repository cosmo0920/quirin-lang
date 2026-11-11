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

/-! Derived access footprints and alias identity (card T09).

`QCore.Concurrency` and `QCore.Concurrency.MutexInvariant` take an access
footprint as a *declared* interval. This namespace is the model of the profile
that **derives** those intervals instead: a slice names an allocation, the
generation it was taken under and an element range, and the byte interval is
computed.

Three questions are separated on purpose, because they are three different
questions:

* **memory-slice disjointness** is decided by interval arithmetic, and only
  against **one** allocation generation — two distinct allocation identities are
  a refusal, not a free "distinct objects do not alias";
* **record fields** are decided only under an explicit layout declaration, and
  two fields of one bit-field storage unit are one memory location, so they are
  never disjoint however their bit ranges lie;
* **descriptor aliases** are decided by protocol-cell identity and by nothing
  else — a memory footprint and a descriptor footprint are *incomparable*, and
  the model refuses the question rather than answering it.

These theorems concern the footprint algebra. They are not about source import,
the real C layout of a record (that is the declared premise the profile names),
allocator behaviour, threads or the C memory model. -/
namespace QCore.DisjointFootprint

/-- The declared extent of one allocation generation. -/
structure Extent where
  elemSize : Nat
  count : Nat
  deriving DecidableEq, Repr

/-- A half-open element range inside an allocation. -/
structure Range where
  offset : Nat
  length : Nat
  deriving DecidableEq, Repr

/-- A half-open byte interval. -/
structure Interval where
  start : Nat
  stop : Nat
  deriving DecidableEq, Repr

def Extent.bytes (e : Extent) : Nat := e.count * e.elemSize

def Extent.valid (e : Extent) : Bool := 0 < e.elemSize && 0 < e.count

def Range.valid (r : Range) : Bool := 0 < r.length

/-- The bounds obligation: an element range must stay inside the declared
extent. Checked, never assumed. -/
def inBounds (e : Extent) (r : Range) : Bool := decide (r.offset + r.length ≤ e.count)

/-- The derivation. The byte interval is computed from the element range and the
element size; a caller-supplied interval is compared against this, never used in
its place. -/
def sliceBytes (e : Extent) (r : Range) : Interval :=
  ⟨r.offset * e.elemSize, (r.offset + r.length) * e.elemSize⟩

/-- Byte membership, so that "disjoint" can be stated about bytes rather than
about the decision procedure that computed it. -/
def Interval.mem (i : Interval) (byte : Nat) : Prop := i.start ≤ byte ∧ byte < i.stop

def Interval.disjoint (a b : Interval) : Bool := decide (a.stop ≤ b.start ∨ b.stop ≤ a.start)

def Range.disjoint (a b : Range) : Bool :=
  decide (a.offset + a.length ≤ b.offset ∨ b.offset + b.length ≤ a.offset)

/-- How a record field occupies storage. `bitfieldUnit` fields sharing a unit
are one C11 memory location. -/
inductive Storage where
  | addressable
  | bitfieldUnit (unit bitOffset bitWidth : Nat)
  deriving DecidableEq, Repr

structure FieldLayout where
  field : Nat
  byteOffset : Nat
  byteSize : Nat
  storage : Storage
  deriving DecidableEq, Repr

def FieldLayout.unit (f : FieldLayout) : Option Nat :=
  match f.storage with
  | .addressable => none
  | .bitfieldUnit unit _ _ => some unit

/-- Two footprints share a bit-field storage unit. Note that `none` never shares
with anything: an ordinary field's bytes are separate memory locations. -/
def sharedUnit : Option Nat → Option Nat → Bool
  | some u, some v => decide (u = v)
  | _, _ => false

/-- A footprint the checker computed. `unit` is `some` exactly when the bytes
are a bit-field storage unit rather than a range of separate locations. -/
inductive Footprint where
  | memory (alloc gen : Nat) (bytes : Interval) (unit : Option Nat)
  | descriptor (descriptor cell : Nat)
  deriving DecidableEq, Repr

/-- Why the model refused to answer. Each is a named boundary of the profile,
not an omission. -/
inductive Refusal where
  | crossAllocation
  | staleGeneration
  | sharedBitfieldUnit
  | sharedProtocolCell
  | incomparableKinds
  deriving DecidableEq, Repr

inductive Decision where
  | disjoint
  | overlapping
  | refused (reason : Refusal)
  deriving DecidableEq, Repr

/-- The memory rule, in the order the Rust checker applies it: allocation
identity, then generation, then the bit-field storage rule, then interval
arithmetic. -/
def decideMemory (allocL genL : Nat) (bytesL : Interval) (unitL : Option Nat)
    (allocR genR : Nat) (bytesR : Interval) (unitR : Option Nat) : Decision :=
  if allocL ≠ allocR then .refused .crossAllocation
  else if genL ≠ genR then .refused .staleGeneration
  else if sharedUnit unitL unitR then .refused .sharedBitfieldUnit
  else if bytesL.disjoint bytesR then .disjoint else .overlapping

/-- The whole algebra. A memory footprint and a descriptor footprint fall into
the incomparable case: the question is refused, not answered. -/
def decideDisjoint : Footprint → Footprint → Decision
  | .memory allocL genL bytesL unitL, .memory allocR genR bytesR unitR =>
      decideMemory allocL genL bytesL unitL allocR genR bytesR unitR
  | .descriptor _ cellL, .descriptor _ cellR =>
      if cellL = cellR then .refused .sharedProtocolCell else .disjoint
  | _, _ => .refused .incomparableKinds

-- ---------------------------------------------------------------------------
-- Acceptance 1: bounds and disjointness against one generation.
-- ---------------------------------------------------------------------------

/-- A slice that passed the bounds check never leaves the declared extent. -/
theorem sliceBytes_within_extent (e : Extent) (r : Range) (bounded : inBounds e r = true) :
    (sliceBytes e r).stop ≤ e.bytes := by
  simp only [inBounds, decide_eq_true_eq] at bounded
  simpa [sliceBytes, Extent.bytes] using Nat.mul_le_mul_right e.elemSize bounded

/-- A slice of a nonempty element range over a nonzero element size has bytes. -/
theorem sliceBytes_nonempty (e : Extent) (r : Range) (extent : e.valid = true)
    (range : r.valid = true) : (sliceBytes e r).start < (sliceBytes e r).stop := by
  simp only [Extent.valid, Range.valid, Bool.and_eq_true, decide_eq_true_eq] at extent range
  have grow : r.offset < r.offset + r.length := by omega
  exact Nat.mul_lt_mul_of_lt_of_le grow (Nat.le_refl _) extent.1

/-- The derivation is *faithful*, not merely sound: over a nonzero element size,
element-range disjointness and byte-interval disjointness are the same
statement. Soundness alone would leave room for an encoding that never
concludes anything. -/
theorem range_disjoint_iff_bytes_disjoint (e : Extent) (a b : Range)
    (positive : 0 < e.elemSize) :
    a.disjoint b = true ↔ (sliceBytes e a).disjoint (sliceBytes e b) = true := by
  simp only [Range.disjoint, Interval.disjoint, decide_eq_true_eq]
  simp only [sliceBytes]
  constructor
  · rintro (left | right)
    · exact Or.inl (Nat.mul_le_mul_right e.elemSize left)
    · exact Or.inr (Nat.mul_le_mul_right e.elemSize right)
  · rintro (left | right)
    · exact Or.inl (Nat.le_of_mul_le_mul_right left positive)
    · exact Or.inr (Nat.le_of_mul_le_mul_right right positive)

/-- The semantic statement: an admitted disjointness judgement means no byte is
in both footprints. -/
theorem disjoint_intervals_share_no_byte {a b : Interval} (decided : a.disjoint b = true)
    (byte : Nat) : ¬(a.mem byte ∧ b.mem byte) := by
  simp only [Interval.disjoint, decide_eq_true_eq] at decided
  rintro ⟨⟨_, _⟩, ⟨_, _⟩⟩
  omega

/-- The converse, so the refusal is not vacuous: two nonempty intervals the
model refused to call disjoint really do share a byte. -/
theorem overlapping_intervals_share_a_byte {a b : Interval} (refused : a.disjoint b = false)
    (leftNonempty : a.start < a.stop) (rightNonempty : b.start < b.stop) :
    ∃ byte, a.mem byte ∧ b.mem byte := by
  simp only [Interval.disjoint, decide_eq_false_iff_not, not_or, Nat.not_le] at refused
  exact ⟨max a.start b.start, by unfold Interval.mem; omega, by unfold Interval.mem; omega⟩

theorem interval_disjoint_symm (a b : Interval) : a.disjoint b = b.disjoint a := by
  simp only [Interval.disjoint]
  rw [decide_eq_decide]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- A nonempty footprint is never disjoint from itself, so a handle cannot be
partitioned against a copy of its own range. -/
theorem nonempty_interval_is_not_disjoint_from_itself {a : Interval} (nonempty : a.start < a.stop) :
    a.disjoint a = false := by
  simp only [Interval.disjoint, decide_eq_false_iff_not, not_or, Nat.not_le]
  exact ⟨nonempty, nonempty⟩

/-- Two distinct allocation identities are refused, whatever the numbers are.
The model has no allocator, so it must not conclude non-aliasing from ids. -/
theorem cross_allocation_disjointness_is_refused {allocL allocR genL genR : Nat}
    {bytesL bytesR : Interval} {unitL unitR : Option Nat} (distinct : allocL ≠ allocR) :
    decideDisjoint (.memory allocL genL bytesL unitL) (.memory allocR genR bytesR unitR)
      = .refused .crossAllocation := by
  simp [decideDisjoint, decideMemory, distinct]

/-- A footprint of a retired generation is refused rather than compared. -/
theorem stale_generation_is_refused {alloc genL genR : Nat} {bytesL bytesR : Interval}
    {unitL unitR : Option Nat} (stale : genL ≠ genR) :
    decideDisjoint (.memory alloc genL bytesL unitL) (.memory alloc genR bytesR unitR)
      = .refused .staleGeneration := by
  simp [decideDisjoint, decideMemory, stale]

/-- Everything the model calls disjoint is disjoint for a stated reason: one
allocation generation with byte-disjoint intervals that are not one bit-field
unit, or two descriptors over distinct protocol cells. -/
theorem admitted_disjointness_has_a_ground {left right : Footprint}
    (admitted : decideDisjoint left right = .disjoint) :
    (∃ alloc gen bytesL bytesR unitL unitR,
        left = .memory alloc gen bytesL unitL ∧ right = .memory alloc gen bytesR unitR ∧
          sharedUnit unitL unitR = false ∧ bytesL.disjoint bytesR = true) ∨
      (∃ dL dR cellL cellR,
        left = .descriptor dL cellL ∧ right = .descriptor dR cellR ∧ cellL ≠ cellR) := by
  cases left with
  | memory allocL genL bytesL unitL =>
      cases right with
      | memory allocR genR bytesR unitR =>
          refine Or.inl ?_
          simp only [decideDisjoint, decideMemory] at admitted
          split at admitted
          · exact absurd admitted (by simp)
          · split at admitted
            · exact absurd admitted (by simp)
            · split at admitted
              · exact absurd admitted (by simp)
              · split at admitted
                · rename_i sameAlloc sameGen noUnit apart
                  simp only [ne_eq, Decidable.not_not] at sameAlloc sameGen
                  subst sameAlloc; subst sameGen
                  exact ⟨allocL, genL, bytesL, bytesR, unitL, unitR, rfl, rfl,
                    by simpa using noUnit, by simpa using apart⟩
                · exact absurd admitted (by simp)
      | descriptor _ _ => exact absurd admitted (by simp [decideDisjoint])
  | descriptor dL cellL =>
      cases right with
      | memory _ _ _ _ => exact absurd admitted (by simp [decideDisjoint])
      | descriptor dR cellR =>
          refine Or.inr ⟨dL, dR, cellL, cellR, rfl, rfl, ?_⟩
          simp only [decideDisjoint] at admitted
          split at admitted
          · exact absurd admitted (by simp)
          · assumption

/-- The soundness corollary in terms of bytes. -/
theorem admitted_memory_disjointness_shares_no_byte {alloc gen : Nat}
    {bytesL bytesR : Interval} {unitL unitR : Option Nat}
    (admitted : decideDisjoint (.memory alloc gen bytesL unitL)
        (.memory alloc gen bytesR unitR) = .disjoint) (byte : Nat) :
    ¬(bytesL.mem byte ∧ bytesR.mem byte) := by
  rcases admitted_disjointness_has_a_ground admitted with
    ⟨_, _, bL, bR, _, _, hl, hr, _, disjointBytes⟩ | ⟨_, _, _, _, hl, _⟩
  · cases hl; cases hr
    exact disjoint_intervals_share_no_byte disjointBytes byte
  · exact absurd hl (by simp)

/-- Non-vacuity: two halves of one allocation are admitted. -/
theorem two_halves_of_one_allocation_are_admitted :
    decideDisjoint
        (.memory 1 1 (sliceBytes ⟨4, 8⟩ ⟨0, 4⟩) none)
        (.memory 1 1 (sliceBytes ⟨4, 8⟩ ⟨4, 4⟩) none) = .disjoint := by
  decide

-- ---------------------------------------------------------------------------
-- Acceptance 2: record fields need a layout declaration, and one bit-field
-- storage unit is one memory location.
-- ---------------------------------------------------------------------------

/-- A field footprint, derived from the element's base byte and the declared
layout. There is no other way to obtain one in this model: without a layout
declaration there is no `FieldLayout`, hence no field footprint at all. -/
def fieldInterval (base : Nat) (f : FieldLayout) : Interval :=
  ⟨base + f.byteOffset, base + f.byteOffset + f.byteSize⟩

def fieldFootprint (alloc gen base : Nat) (f : FieldLayout) : Footprint :=
  .memory alloc gen (fieldInterval base f) f.unit

/-- A declared field stays inside the element it was derived from, so a layout
contract validated against the record size cannot produce a footprint that
escapes the object. -/
theorem field_stays_inside_its_element (base recordSize : Nat) (f : FieldLayout)
    (declared : f.byteOffset + f.byteSize ≤ recordSize) :
    base ≤ (fieldInterval base f).start ∧
      (fieldInterval base f).stop ≤ base + recordSize := by
  simp only [fieldInterval]
  omega

/-- Two ordinary fields whose declared byte ranges do not overlap are disjoint.
This is CT09's "accept only after a layout proof": the layout *is* the proof. -/
theorem addressable_fields_with_disjoint_ranges_are_disjoint {alloc gen base : Nat}
    {f g : FieldLayout} (leftPlain : f.storage = .addressable)
    (rightPlain : g.storage = .addressable)
    (apart : f.byteOffset + f.byteSize ≤ g.byteOffset ∨ g.byteOffset + g.byteSize ≤ f.byteOffset) :
    decideDisjoint (fieldFootprint alloc gen base f) (fieldFootprint alloc gen base g)
      = .disjoint := by
  have units : sharedUnit f.unit g.unit = false := by
    simp [FieldLayout.unit, leftPlain, rightPlain, sharedUnit]
  have bytes : (fieldInterval base f).disjoint (fieldInterval base g) = true := by
    simp only [Interval.disjoint, decide_eq_true_eq]
    simp only [fieldInterval]
    omega
  simp [decideDisjoint, decideMemory, fieldFootprint, units, bytes]

/-- CT10. Two fields of one bit-field storage unit are one C11 memory location,
so the model never calls them disjoint — and this holds for *every* pair of bit
ranges and every pair of byte ranges, so it is not an accident of overlap. -/
theorem shared_bitfield_unit_is_never_disjoint {alloc gen : Nat} {bytesL bytesR : Interval}
    (unit : Nat) :
    decideDisjoint (.memory alloc gen bytesL (some unit)) (.memory alloc gen bytesR (some unit))
      = .refused .sharedBitfieldUnit := by
  simp [decideDisjoint, decideMemory, sharedUnit]

/-- Its positive control: distinct storage units fall through to the ordinary
interval rule. -/
theorem distinct_bitfield_units_are_decided_by_bytes {alloc gen unitL unitR : Nat}
    {bytesL bytesR : Interval} (distinct : unitL ≠ unitR)
    (apart : bytesL.disjoint bytesR = true) :
    decideDisjoint (.memory alloc gen bytesL (some unitL))
        (.memory alloc gen bytesR (some unitR)) = .disjoint := by
  simp [decideDisjoint, decideMemory, sharedUnit, distinct, apart]

/-- The bit-field rule precedes the interval rule, so identical bit-field bytes
are refused with the storage-unit reason rather than reported as an ordinary
overlap. The distinction is what makes the refusal legible. -/
theorem the_bitfield_rule_precedes_the_interval_rule {alloc gen unit : Nat} {bytes : Interval} :
    decideDisjoint (.memory alloc gen bytes (some unit)) (.memory alloc gen bytes (some unit))
      ≠ .overlapping := by
  simp [decideDisjoint, decideMemory, sharedUnit]

/-- An ordinary field never shares a storage unit with anything, so the
bit-field rule cannot be triggered by an addressable member. -/
theorem an_addressable_field_shares_no_storage_unit {f : FieldLayout} (g : Option Nat)
    (plain : f.storage = .addressable) :
    sharedUnit f.unit g = false ∧ sharedUnit g f.unit = false := by
  cases g <;> simp [FieldLayout.unit, plain, sharedUnit]

-- ---------------------------------------------------------------------------
-- Acceptance 3: descriptor aliases over a protocol cell are a different
-- question, and the two never mix.
-- ---------------------------------------------------------------------------

/-- CT42. Two descriptors that advance one logical state are never disjoint,
whatever bytes either of them happens to touch. -/
theorem descriptors_sharing_a_cell_are_never_disjoint (dL dR cell : Nat) :
    decideDisjoint (.descriptor dL cell) (.descriptor dR cell)
      = .refused .sharedProtocolCell := by
  simp [decideDisjoint]

theorem descriptors_over_distinct_cells_are_disjoint {dL dR cellL cellR : Nat}
    (distinct : cellL ≠ cellR) :
    decideDisjoint (.descriptor dL cellL) (.descriptor dR cellR) = .disjoint := by
  simp [decideDisjoint, distinct]

/-- Memory disjointness does not transfer to a protocol cell, and cannot be
made to: the model refuses the mixed question in both argument orders instead of
answering it. -/
theorem memory_and_descriptor_are_incomparable (alloc gen : Nat) (bytes : Interval)
    (unit : Option Nat) (descriptor cell : Nat) :
    decideDisjoint (.memory alloc gen bytes unit) (.descriptor descriptor cell)
        = .refused .incomparableKinds ∧
      decideDisjoint (.descriptor descriptor cell) (.memory alloc gen bytes unit)
        = .refused .incomparableKinds := by
  constructor <;> simp [decideDisjoint]

/-- The decision is symmetric wherever it concludes, so neither operand order
can buy an answer the other order refuses. -/
theorem admitted_disjointness_is_symmetric {left right : Footprint}
    (admitted : decideDisjoint left right = .disjoint) :
    decideDisjoint right left = .disjoint := by
  rcases admitted_disjointness_has_a_ground admitted with
    ⟨alloc, gen, bytesL, bytesR, unitL, unitR, hl, hr, noUnit, apart⟩ |
    ⟨dL, dR, cellL, cellR, hl, hr, distinct⟩
  · subst hl; subst hr
    have swapped : bytesR.disjoint bytesL = true := by
      rw [interval_disjoint_symm]; exact apart
    have units : sharedUnit unitR unitL = false := by
      cases unitL <;> cases unitR <;>
        simp_all [sharedUnit] <;> omega
    simp [decideDisjoint, decideMemory, units, swapped]
  · subst hl; subst hr
    have flipped : cellR ≠ cellL := fun h => distinct h.symm
    simp [decideDisjoint, flipped]

-- ---------------------------------------------------------------------------
-- Generations: a handle of a retired generation decides nothing.
-- ---------------------------------------------------------------------------

/-- The live generation of each allocation. `none` means destroyed: dead
forever, never revived. -/
abbrev Store := Nat → Option Nat

/-- A handle, as recorded when it was derived. -/
structure Handle where
  alloc : Nat
  gen : Nat
  bytes : Interval
  unit : Option Nat
  deriving DecidableEq, Repr

/-- Resolving a handle checks its generation against the live one. -/
def resolve (store : Store) (h : Handle) : Option Footprint :=
  if store h.alloc = some h.gen then some (.memory h.alloc h.gen h.bytes h.unit) else none

/-- The checked obligation over a store. A stale handle is refused before any
interval is compared. -/
def assertDisjoint (store : Store) (left right : Handle) : Decision :=
  match resolve store left with
  | none => .refused .staleGeneration
  | some l =>
      match resolve store right with
      | none => .refused .staleGeneration
      | some r => decideDisjoint l r

/-- Advance (or end, with `none`) the live generation of one allocation. -/
def setGeneration (store : Store) (alloc : Nat) (gen : Option Nat) : Store :=
  fun a => if a = alloc then gen else store a

/-- Reallocation retires every handle of the previous generation, and there is
no route back: `resolve` yields nothing for such a handle. -/
theorem reallocation_retires_every_handle_of_the_old_generation (store : Store) (h : Handle)
    {fresh : Nat} (advanced : fresh ≠ h.gen) :
    resolve (setGeneration store h.alloc (some fresh)) h = none := by
  simp [resolve, setGeneration, advanced]

/-- Destruction retires them too, and a destroyed allocation resolves nothing at
any generation. -/
theorem destruction_retires_every_handle (store : Store) (h : Handle) :
    resolve (setGeneration store h.alloc none) h = none := by
  simp [resolve, setGeneration]

/-- Retiring one allocation leaves every handle of every other allocation
untouched. -/
theorem retirement_preserves_other_allocations (store : Store) (h : Handle)
    {alloc : Nat} (gen : Option Nat) (other : h.alloc ≠ alloc) :
    resolve (setGeneration store alloc gen) h = resolve store h := by
  simp [resolve, setGeneration, other]

/-- A stale handle yields no judgement at all: the obligation is refused, never
answered "disjoint" and never answered "overlapping". -/
theorem a_stale_handle_decides_nothing (store : Store) (left right : Handle)
    (stale : resolve store left = none ∨ resolve store right = none) :
    assertDisjoint store left right = .refused .staleGeneration := by
  unfold assertDisjoint
  rcases stale with h | h
  · rw [h]
  · cases hl : resolve store left
    · rfl
    · rw [h]

/-- Non-vacuity for the store level: live handles over one generation are
decided. -/
theorem live_handles_of_one_generation_are_decided :
    assertDisjoint (fun a => if a = 1 then some 1 else none)
        ⟨1, 1, ⟨0, 16⟩, none⟩ ⟨1, 1, ⟨16, 32⟩, none⟩ = .disjoint := by
  decide

/-- And the same pair after a reallocation is refused. -/
theorem the_same_pair_after_a_reallocation_is_refused :
    assertDisjoint (fun a => if a = 1 then some 2 else none)
        ⟨1, 1, ⟨0, 16⟩, none⟩ ⟨1, 1, ⟨16, 32⟩, none⟩ = .refused .staleGeneration := by
  decide

-- ---------------------------------------------------------------------------
-- Decidable scenarios mirroring the Rust unit tests.
-- ---------------------------------------------------------------------------

/-- `int32_t data[8]`: bounds hold and the halves are disjoint. -/
example : inBounds ⟨4, 8⟩ ⟨4, 4⟩ = true := by decide

/-- One element past the extent is refused. -/
example : inBounds ⟨4, 8⟩ ⟨4, 5⟩ = false := by decide

/-- The derived byte interval is computed, and it is what the Rust checker
compares a supplied interval against. -/
example : sliceBytes ⟨4, 8⟩ ⟨2, 3⟩ = ⟨8, 20⟩ := by decide

/-- Touching at one boundary is disjoint; one element of overlap is not. -/
example : decideDisjoint (.memory 1 1 ⟨0, 16⟩ none) (.memory 1 1 ⟨16, 32⟩ none) = .disjoint := by
  decide

example : decideDisjoint (.memory 1 1 ⟨0, 20⟩ none) (.memory 1 1 ⟨16, 32⟩ none) = .overlapping := by
  decide

/-- A nonempty range is not disjoint from itself. -/
example : decideDisjoint (.memory 1 1 ⟨0, 16⟩ none) (.memory 1 1 ⟨0, 16⟩ none) = .overlapping := by
  decide

/-- Two allocations are refused even when the numbers look disjoint. -/
example : decideDisjoint (.memory 1 1 ⟨0, 16⟩ none) (.memory 2 1 ⟨16, 32⟩ none)
    = .refused .crossAllocation := by decide

/-- `struct { int32_t a; int32_t b; uint32_t x:3, y:5; }`: the two scalars are
disjoint and the two bit-fields are one memory location. -/
example : decideDisjoint (fieldFootprint 1 1 0 ⟨0, 0, 4, .addressable⟩)
    (fieldFootprint 1 1 0 ⟨1, 4, 4, .addressable⟩) = .disjoint := by decide

example : decideDisjoint (fieldFootprint 1 1 0 ⟨2, 8, 4, .bitfieldUnit 0 0 3⟩)
    (fieldFootprint 1 1 0 ⟨3, 8, 4, .bitfieldUnit 0 3 5⟩)
      = .refused .sharedBitfieldUnit := by decide

/-- A bit-field and an ordinary field of the same record are decided by bytes. -/
example : decideDisjoint (fieldFootprint 1 1 0 ⟨0, 0, 4, .addressable⟩)
    (fieldFootprint 1 1 0 ⟨2, 8, 4, .bitfieldUnit 0 0 3⟩) = .disjoint := by decide

/-- Two descriptors over one open file: refused. Over two: decided. -/
example : decideDisjoint (.descriptor 3 1) (.descriptor 4 1) = .refused .sharedProtocolCell := by
  decide

example : decideDisjoint (.descriptor 3 1) (.descriptor 4 2) = .disjoint := by decide

/-- A byte range and a descriptor are incomparable in both orders. -/
example : decideDisjoint (.memory 1 1 ⟨0, 16⟩ none) (.descriptor 3 1)
    = .refused .incomparableKinds := by decide

example : decideDisjoint (.descriptor 3 1) (.memory 1 1 ⟨0, 16⟩ none)
    = .refused .incomparableKinds := by decide

-- ---------------------------------------------------------------------------
-- Card CG18-FOOTPRINT-CONNECTION: named premises, views, byte transfers, the
-- mutex-invariant composition and rule 11a's generations.
-- ---------------------------------------------------------------------------

/-- Card CG18 (M21). The memory rule with the named premise
`P-DISTINCT-ALLOCATIONS-DISJOINT`: `fresh` means both live allocation
generations were declared fresh by an allocator contract. Only a pair of
distinct allocation identities reads it; every other pair is `decideMemory`. -/
def decideMemoryWith (fresh : Bool) (allocL genL : Nat) (bytesL : Interval)
    (unitL : Option Nat) (allocR genR : Nat) (bytesR : Interval) (unitR : Option Nat) :
    Decision :=
  if allocL ≠ allocR then (if fresh then .disjoint else .refused .crossAllocation)
  else decideMemory allocL genL bytesL unitL allocR genR bytesR unitR

def decideDisjointWith (fresh : Bool) : Footprint → Footprint → Decision
  | .memory allocL genL bytesL unitL, .memory allocR genR bytesR unitR =>
      decideMemoryWith fresh allocL genL bytesL unitL allocR genR bytesR unitR
  | left, right => decideDisjoint left right

/-- Without the premise the checker is exactly the pre-card algebra. -/
theorem without_the_premise_nothing_changes (left right : Footprint) :
    decideDisjointWith false left right = decideDisjoint left right := by
  cases left with
  | memory allocL genL bytesL unitL =>
      cases right with
      | memory allocR genR bytesR unitR =>
          by_cases same : allocL = allocR
          · simp [decideDisjointWith, decideMemoryWith, decideDisjoint, same]
          · simp [decideDisjointWith, decideMemoryWith, decideDisjoint, decideMemory, same]
      | descriptor _ _ => rfl
  | descriptor _ _ => cases right <;> rfl

/-- The premise decides only a pair of distinct allocation identities: on one
allocation it changes nothing, whatever it says. -/
theorem the_premise_reads_only_distinct_allocations (fresh : Bool) {alloc genL genR : Nat}
    {bytesL bytesR : Interval} {unitL unitR : Option Nat} :
    decideDisjointWith fresh (.memory alloc genL bytesL unitL) (.memory alloc genR bytesR unitR)
      = decideDisjoint (.memory alloc genL bytesL unitL) (.memory alloc genR bytesR unitR) := by
  simp [decideDisjointWith, decideMemoryWith, decideDisjoint]

/-- Disjointness across two allocations is admitted only under the premise. -/
theorem cross_allocation_disjointness_needs_the_premise {fresh : Bool}
    {allocL allocR genL genR : Nat} {bytesL bytesR : Interval} {unitL unitR : Option Nat}
    (distinct : allocL ≠ allocR)
    (admitted : decideDisjointWith fresh (.memory allocL genL bytesL unitL)
        (.memory allocR genR bytesR unitR) = .disjoint) :
    fresh = true := by
  cases fresh <;> simp_all [decideDisjointWith, decideMemoryWith]

/-- Every admitted judgement is the pre-card algebra's, or a cross-allocation
pair decided under the premise, which the certificate then names. -/
theorem an_admitted_judgement_is_grounded_or_premised {fresh : Bool} {left right : Footprint}
    (admitted : decideDisjointWith fresh left right = .disjoint) :
    decideDisjoint left right = .disjoint ∨
      (fresh = true ∧ ∃ allocL genL bytesL unitL allocR genR bytesR unitR,
        left = .memory allocL genL bytesL unitL ∧ right = .memory allocR genR bytesR unitR ∧
          allocL ≠ allocR) := by
  cases left with
  | memory allocL genL bytesL unitL =>
      cases right with
      | memory allocR genR bytesR unitR =>
          by_cases same : allocL = allocR
          · subst same
            left
            simpa [decideDisjointWith, decideMemoryWith, decideDisjoint] using admitted
          · right
            refine ⟨cross_allocation_disjointness_needs_the_premise same admitted, ?_⟩
            exact ⟨allocL, genL, bytesL, unitL, allocR, genR, bytesR, unitR, rfl, rfl, same⟩
      | descriptor _ _ => left; simpa [decideDisjointWith] using admitted
  | descriptor _ _ => left; simpa [decideDisjointWith] using admitted

/-- The premise never reaches a stale, shared-unit, descriptor or mixed
question: two footprints of one allocation keep every refusal. -/
theorem the_premise_never_admits_a_shared_unit (fresh : Bool) {alloc gen unit : Nat}
    {bytesL bytesR : Interval} :
    decideDisjointWith fresh (.memory alloc gen bytesL (some unit))
        (.memory alloc gen bytesR (some unit)) = .refused .sharedBitfieldUnit := by
  simp [decideDisjointWith, decideMemoryWith, decideMemory, sharedUnit]

/-- The outcome of one pair, as the shared vectors name it. -/
inductive PairOutcome where
  | disjoint
  | disjointUnderPremise
  | overlapping
  | refusedCross
  | refusedOther
  deriving DecidableEq, Repr

def pairOutcome (fresh : Bool) (allocL startL stopL allocR startR stopR : Nat) : PairOutcome :=
  match decideDisjointWith fresh (.memory allocL 1 ⟨startL, stopL⟩ none)
      (.memory allocR 1 ⟨startR, stopR⟩ none) with
  | .disjoint => if allocL = allocR then .disjoint else .disjointUnderPremise
  | .overlapping => .overlapping
  | .refused .crossAllocation => .refusedCross
  | .refused _ => .refusedOther

/-- The shared vectors: `PAIR_VECTORS` in `tests/unit/disjoint_footprint.rs`,
row by row: fresh, left allocation and bytes, right allocation and bytes. -/
def pairVectors : List (Bool × Nat × Nat × Nat × Nat × Nat × Nat × PairOutcome) := [
  (false, 1, 0, 8, 1, 8, 16, .disjoint),
  (true, 1, 0, 8, 1, 8, 16, .disjoint),
  (false, 1, 0, 8, 1, 4, 12, .overlapping),
  (true, 1, 0, 8, 1, 4, 12, .overlapping),
  (false, 1, 0, 8, 2, 0, 8, .refusedCross),
  (true, 1, 0, 8, 2, 0, 8, .disjointUnderPremise),
  (true, 1, 0, 8, 2, 8, 16, .disjointUnderPremise)
]

theorem pair_vectors_hold :
    pairVectors.all (fun (f, aL, sL, eL, aR, sR, eR, o) => pairOutcome f aL sL eL aR sR eR == o)
      = true := by
  decide

/-- Card CG18 (M22): how a view reads its base's bytes. -/
inductive ViewKind where
  | character
  | effectiveType
  | punned
  deriving DecidableEq, Repr

inductive ViewOutcome where
  /-- Admitted with no premise (a character view). -/
  | admitted
  /-- Admitted, carrying `P-EFFECTIVE-TYPE`. -/
  | admittedUnderPremise
  | refusedPunned
  | refusedNotAddressable
  deriving DecidableEq, Repr

/-- The view rule, in the Rust checker's order: the base must be memory that is
not a bit-field unit, then the kind decides. -/
def decideView : Footprint → ViewKind → ViewOutcome
  | .memory _ _ _ none, .character => .admitted
  | .memory _ _ _ none, .effectiveType => .admittedUnderPremise
  | .memory _ _ _ none, .punned => .refusedPunned
  | _, _ => .refusedNotAddressable

/-- A type-punned view is never admitted, with or without a premise. -/
theorem a_punned_view_is_refused (base : Footprint) :
    decideView base .punned ≠ .admitted ∧ decideView base .punned ≠ .admittedUnderPremise := by
  cases base with
  | memory _ _ _ unit => cases unit <;> simp [decideView]
  | descriptor _ _ => simp [decideView]

/-- An admitted view carries a premise exactly when it is an effective-type
view: a character view needs none (C11 6.5p7), any other needs
`P-EFFECTIVE-TYPE`. -/
theorem an_admitted_view_names_its_premise {base : Footprint} {view : ViewKind}
    (admitted : decideView base view = .admittedUnderPremise) : view = .effectiveType := by
  cases base with
  | memory _ _ _ unit => cases unit <;> cases view <;> simp_all [decideView]
  | descriptor _ _ => simp [decideView] at admitted

/-- A bit-field unit and a descriptor have no bytes to view. -/
theorem a_unit_or_descriptor_has_no_view (view : ViewKind) (alloc gen unit : Nat)
    (bytes : Interval) (descriptor cell : Nat) :
    decideView (.memory alloc gen bytes (some unit)) view = .refusedNotAddressable ∧
      decideView (.descriptor descriptor cell) view = .refusedNotAddressable := by
  constructor <;> simp [decideView]

/-- Card CG18 (M26): the first `bytes` bytes of a handle's interval. -/
def prefixBytes (handle : Interval) (bytes : Nat) : Interval := ⟨handle.start, handle.start + bytes⟩

inductive CopyOutcome where
  | admitted
  /-- Admitted across two allocations under `P-DISTINCT-ALLOCATIONS-DISJOINT`. -/
  | admittedUnderPremise
  /-- `memcpy` over two ranges sharing a byte: the excluded `OverlappingCopy`. -/
  | refusedOverlap
  | refusedBounds
  | refusedCross
  | refusedExtent
  deriving DecidableEq, Repr

/-- The byte-copy rule, in the Rust checker's order: the byte count, then each
range inside its handle, then (for `memcpy` only) disjointness of the two
ranges by `decideDisjointWith`. -/
def decideCopy (mayOverlap fresh : Bool) (allocD : Nat) (dst : Interval) (allocS : Nat)
    (src : Interval) (bytes : Nat) : CopyOutcome :=
  if bytes = 0 then .refusedExtent
  else if dst.start + bytes > dst.stop ∨ src.start + bytes > src.stop then .refusedBounds
  else if mayOverlap then .admitted
  else
    match decideDisjointWith fresh (.memory allocD 1 (prefixBytes dst bytes) none)
        (.memory allocS 1 (prefixBytes src bytes) none) with
    | .disjoint => if allocD = allocS then .admitted else .admittedUnderPremise
    | .overlapping => .refusedOverlap
    | .refused _ => .refusedCross

/-- An admitted transfer never leaves either handle. -/
theorem an_admitted_transfer_stays_inside_its_handles {mayOverlap fresh : Bool}
    {allocD allocS bytes : Nat} {dst src : Interval}
    (admitted : decideCopy mayOverlap fresh allocD dst allocS src bytes = .admitted ∨
      decideCopy mayOverlap fresh allocD dst allocS src bytes = .admittedUnderPremise) :
    (prefixBytes dst bytes).stop ≤ dst.stop ∧ (prefixBytes src bytes).stop ≤ src.stop := by
  unfold decideCopy at admitted
  split at admitted
  · simp at admitted
  · split at admitted
    · simp at admitted
    · rename_i inside
      simp only [prefixBytes]
      omega

/-- An admitted `memcpy` within one allocation copies between two ranges that
share no byte: the overlapping copy stays excluded. -/
theorem an_admitted_memcpy_shares_no_byte {fresh : Bool} {alloc bytes : Nat}
    {dst src : Interval} (admitted : decideCopy false fresh alloc dst alloc src bytes = .admitted)
    (byte : Nat) :
    ¬((prefixBytes dst bytes).mem byte ∧ (prefixBytes src bytes).mem byte) := by
  unfold decideCopy at admitted
  split at admitted
  · simp at admitted
  · split at admitted
    · simp at admitted
    · simp only [Bool.false_eq_true, ↓reduceIte] at admitted
      split at admitted
      · rename_i decided
        rw [the_premise_reads_only_distinct_allocations] at decided
        exact admitted_memory_disjointness_shares_no_byte decided byte
      · simp at admitted
      · simp at admitted

/-- A `memcpy` across two allocations is admitted only under the premise. -/
theorem a_cross_allocation_memcpy_needs_the_premise {fresh : Bool} {allocD allocS bytes : Nat}
    {dst src : Interval} (distinct : allocD ≠ allocS)
    (admitted : decideCopy false fresh allocD dst allocS src bytes = .admittedUnderPremise) :
    fresh = true := by
  unfold decideCopy at admitted
  split at admitted
  · simp at admitted
  · split at admitted
    · simp at admitted
    · simp only [Bool.false_eq_true, ↓reduceIte] at admitted
      split at admitted
      · rename_i decided
        exact cross_allocation_disjointness_needs_the_premise distinct decided
      · simp at admitted
      · simp at admitted

/-- The shared vectors: `COPY_VECTORS` in `tests/unit/disjoint_footprint.rs`:
memmove?, fresh, destination allocation and bytes, source allocation and
bytes, the byte count. -/
def copyVectors :
    List (Bool × Bool × Nat × Nat × Nat × Nat × Nat × Nat × Nat × CopyOutcome) := [
  (false, false, 1, 0, 8, 1, 1, 9, 4, .refusedOverlap),
  (false, false, 1, 0, 8, 1, 1, 9, 1, .admitted),
  (false, false, 1, 0, 8, 1, 8, 16, 8, .admitted),
  (true, false, 1, 0, 8, 1, 1, 9, 4, .admitted),
  (false, false, 1, 0, 8, 1, 8, 16, 9, .refusedBounds),
  (true, false, 1, 0, 8, 1, 8, 16, 0, .refusedExtent),
  (false, false, 1, 0, 8, 2, 0, 8, 8, .refusedCross),
  (false, true, 1, 0, 8, 2, 0, 8, 8, .admittedUnderPremise),
  (true, false, 1, 0, 8, 2, 0, 8, 8, .admitted)
]

theorem copy_vectors_hold :
    copyVectors.all (fun (m, f, aD, sD, eD, aS, sS, eS, n, o) =>
      decideCopy m f aD ⟨sD, eD⟩ aS ⟨sS, eS⟩ n == o) = true := by
  decide

/-- Card CG18 (W08): one interval a mutex-invariant trace names. -/
structure MutexFootprint where
  resource : Nat
  bytes : Interval
  deriving DecidableEq, Repr

/-- `AccessFootprint.overlaps`: the mutex profile's own overlap test, which
calls two resources never overlapping. -/
def mutexOverlaps (f g : MutexFootprint) : Bool :=
  decide (f.resource = g.resource ∧ f.bytes.start < g.bytes.stop ∧ g.bytes.start < f.bytes.stop)

/-- The composition's admission: every named interval is a live derived export
(`exports`), and every pair on two resources has both declared fresh. -/
def composes (exports : List MutexFootprint) (fresh : Nat → Bool)
    (named : List MutexFootprint) : Bool :=
  named.all (fun f => exports.contains f) &&
    named.all (fun f => named.all (fun g =>
      decide (f.resource = g.resource) || (fresh f.resource && fresh g.resource)))

/-- Every interval an admitted composition names is derived. -/
theorem composition_binds_every_footprint {exports named : List MutexFootprint}
    {fresh : Nat → Bool} (admitted : composes exports fresh named = true)
    {f : MutexFootprint} (named_f : f ∈ named) : f ∈ exports := by
  simp only [composes, Bool.and_eq_true, List.all_eq_true] at admitted
  simpa using admitted.1 f named_f

/-- Soundness of the composition: wherever the mutex profile calls two named
footprints non-overlapping, the footprint model decides their derived
footprints disjoint — on one resource by interval arithmetic, across two only
under `P-DISTINCT-ALLOCATIONS-DISJOINT`, which the composition then carries. -/
theorem composition_backs_every_mutex_disjointness {exports named : List MutexFootprint}
    {fresh : Nat → Bool} (admitted : composes exports fresh named = true)
    {f g : MutexFootprint} (named_f : f ∈ named) (named_g : g ∈ named)
    (apart : mutexOverlaps f g = false) (gen : Nat → Nat) :
    decideDisjointWith (fresh f.resource && fresh g.resource)
        (.memory f.resource (gen f.resource) f.bytes none)
        (.memory g.resource (gen g.resource) g.bytes none) = .disjoint := by
  simp only [composes, Bool.and_eq_true, List.all_eq_true] at admitted
  have pair := admitted.2 f named_f g named_g
  by_cases same : f.resource = g.resource
  · have bytes : f.bytes.disjoint g.bytes = true := by
      simp only [mutexOverlaps, same, true_and, decide_eq_false_iff_not, not_and,
        Nat.not_lt] at apart
      simp only [Interval.disjoint, decide_eq_true_eq]
      by_cases h : f.bytes.start < g.bytes.stop
      · exact Or.inl (apart h)
      · exact Or.inr (Nat.le_of_not_lt h)
    simp [decideDisjointWith, decideMemoryWith, decideMemory, same, sharedUnit, bytes]
  · simp only [same, decide_false, Bool.false_or, Bool.and_eq_true] at pair
    simp [decideDisjointWith, decideMemoryWith, same, pair.1, pair.2]

/-- Its refusal control: two resources without the premise do not compose,
although the mutex profile alone calls them non-overlapping. -/
theorem two_resources_without_the_premise_do_not_compose :
    composes [⟨7, ⟨0, 8⟩⟩, ⟨8, ⟨0, 8⟩⟩] (fun _ => false) [⟨7, ⟨0, 8⟩⟩, ⟨8, ⟨0, 8⟩⟩] = false ∧
      mutexOverlaps ⟨7, ⟨0, 8⟩⟩ ⟨8, ⟨0, 8⟩⟩ = false := by
  decide

/-- And an interval no handle derived does not compose. -/
theorem an_underived_interval_does_not_compose :
    composes [⟨7, ⟨0, 8⟩⟩] (fun _ => false) [⟨7, ⟨0, 8⟩⟩, ⟨7, ⟨0, 4⟩⟩] = false := by
  decide

/-- Card CG18 (M20): rule 11a's three generations of an acquisition site. -/
inductive SiteGeneration where
  | current
  | previous
  | older
  deriving DecidableEq, Repr

/-- The core identity of a rule-11a generation, given the core allocations the
site's current and previous objects are: the older summary has none. -/
def coreIdentity (current previous : Option Nat) : SiteGeneration → Option Nat
  | .current => current
  | .previous => previous
  | .older => none

/-- Re-acquisition: a fresh allocation becomes current and the old current is
previous, the same object (rule 11a keeps it precise). -/
def reacquire (fresh : Nat) (current _previous : Option Nat) : Option Nat × Option Nat :=
  (some fresh, current)

/-- After a re-acquisition the previous generation names exactly the object the
current one named, so every handle derived of it still resolves. -/
theorem the_previous_generation_is_the_former_current (fresh : Nat) (current previous : Option Nat) :
    coreIdentity (reacquire fresh current previous).1 (reacquire fresh current previous).2
        .previous = current := by
  rfl

/-- The summary has no core identity, so no footprint is derived from it. -/
theorem the_summary_has_no_footprint (current previous : Option Nat) :
    coreIdentity current previous .older = none := by
  rfl

/-- Two generations of one site are two allocation identities, so they are
never assumed disjoint: without the premise the core refuses. -/
theorem two_generations_of_a_site_are_not_assumed_disjoint {fresh old : Nat}
    (distinct : fresh ≠ old) {genL genR : Nat} {bytesL bytesR : Interval}
    {unitL unitR : Option Nat} :
    decideDisjointWith false (.memory fresh genL bytesL unitL) (.memory old genR bytesR unitR)
      = .refused .crossAllocation := by
  simp [decideDisjointWith, decideMemoryWith, distinct]

/-- The shared vectors: `GENERATION_VECTORS` in the gateway's
`footprint_connection.rs`: after two acquisitions of one site, whether each
generation has a core identity. -/
def generationVectors : List (SiteGeneration × Bool) := [
  (.current, true),
  (.previous, true),
  (.older, false)
]

theorem generation_vectors_hold :
    generationVectors.all (fun (g, has) =>
      (coreIdentity (reacquire 2 (some 1) none).1 (reacquire 2 (some 1) none).2 g).isSome == has)
      = true := by
  decide

end QCore.DisjointFootprint
