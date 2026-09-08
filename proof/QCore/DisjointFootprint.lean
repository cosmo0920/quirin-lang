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

end QCore.DisjointFootprint
