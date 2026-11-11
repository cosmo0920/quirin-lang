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

import QCore.DisjointFootprint

/-!
# Union-member and bit-field place steps (card CG13-UNION-AND-BITFIELD-PLACES, rule 10w)

The abstract rule implemented by `crates/qcore-c-gateway/src/union_bitfield.rs`
(`decide`, `decide_path`, `punned`, the storage-unit reading and
`shared_unit_refusal`) and its hooks in `typed_access_path.rs` (`Reader::path`),
`typed_place.rs` (`MemberStorage` on each step, the union-hop refusal) and the
elaborator (`Refusal::SharedBitfieldUnit`).

## The decision

A member step's storage is `ordinary` (not this rule's), `union` (storage
shared with every sibling), `bitfield` (a bit range of a known storage unit),
or a form this rule refuses (`unionBitfield`, `unread`). `decideStep` decides
one step from the access through it, whether the function puns the union
(stores through another member of it) and whether the union is custody-free
(no member can hold a pointer).

## The storage model

* A bit-field store is a read-modify-write of its whole storage unit: it emits
  a read and a write of the unit, keeps every bit outside its range, and its
  footprint is the unit's, whatever its bit range (`bitfield_store_is_unit_rmw`,
  `a_bitfield_footprint_is_its_unit`).
* Two bit-fields of one unit with disjoint bit ranges are still one memory
  location: the core footprint model (`QCore.DisjointFootprint`) refuses to call
  them disjoint (`disjoint_bits_do_not_make_disjoint_footprints`).
* A store through a member of a union that can hold a pointer carries the
  refused-write frame, which spares only plain locals: no member place of any
  object, and so no sibling of the stored member, keeps a tracked identity
  (`a_union_store_invalidates_sibling_identities`). A store through a
  custody-free union changes no binding (`a_custody_free_union_store_is_frame_neutral`).

## What this is not

A model of the decision and of the frame, not of Clang extraction: the
field ids, the union owner, the bit-field widths and Clang's record layout are
review premises (trust boundary 28), and `decisionVectors` binds the Rust table
mechanically (`tests/unit/union_bitfield.rs`). A callee's store through another
member is the stated premise `union-live-member-view`.
-/

namespace QCore.UnionBitfieldPlace

open QCore.DisjointFootprint (Interval Footprint Decision Refusal decideDisjoint)

/-! ## The decision table -/

inductive Storage where
  | ordinary
  | union
  | bitfield
  | unionBitfield
  | unread
  deriving DecidableEq, Repr

inductive Access where
  | scalarRead
  | pointerRead
  | scalarStore
  | pointerStore
  deriving DecidableEq, Repr

inductive Outcome where
  | ordinary
  | neutral
  | framed
  | borrow
  | unitRead
  | unitWrite
  | refusedPunned
  | refusedPointerStore
  | refusedUnlisted
  deriving DecidableEq, Repr

def Outcome.admits : Outcome → Bool
  | .refusedPunned => false
  | .refusedPointerStore => false
  | .refusedUnlisted => false
  | _ => true

/-- `decide` in `union_bitfield.rs`. -/
def decideStep : Storage → Access → Bool → Bool → Outcome
  | .ordinary, _, _, _ => .ordinary
  | .union, .scalarRead, _, _ => .neutral
  | .union, .pointerRead, punned, _ => if punned then .refusedPunned else .borrow
  | .union, .scalarStore, _, custodyFree => if custodyFree then .neutral else .framed
  | .union, .pointerStore, _, _ => .refusedPointerStore
  | .bitfield, .scalarRead, _, _ => .unitRead
  | .bitfield, .scalarStore, _, _ => .unitWrite
  | _, _, _, _ => .refusedUnlisted

/-- The shared vectors: `DECISION_VECTORS` in `union_bitfield.rs`, row by row. -/
def decisionVectors : List (Storage × Access × Bool × Bool × Outcome) := [
  (.ordinary, .scalarStore, false, false, .ordinary),
  (.union, .scalarRead, false, false, .neutral),
  (.union, .scalarRead, true, false, .neutral),
  (.union, .pointerRead, false, false, .borrow),
  (.union, .pointerRead, true, false, .refusedPunned),
  (.union, .pointerRead, true, true, .refusedPunned),
  (.union, .scalarStore, false, false, .framed),
  (.union, .scalarStore, false, true, .neutral),
  (.union, .pointerStore, false, false, .refusedPointerStore),
  (.union, .pointerStore, false, true, .refusedPointerStore),
  (.bitfield, .scalarRead, false, false, .unitRead),
  (.bitfield, .scalarStore, false, false, .unitWrite),
  (.bitfield, .pointerRead, false, false, .refusedUnlisted),
  (.bitfield, .pointerStore, false, false, .refusedUnlisted),
  (.unionBitfield, .scalarRead, false, true, .refusedUnlisted),
  (.unionBitfield, .scalarStore, false, true, .refusedUnlisted),
  (.unread, .scalarRead, false, false, .refusedUnlisted),
  (.unread, .scalarStore, false, false, .refusedUnlisted)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (s, a, p, f, o) => decideStep s a p f == o) = true := by
  decide

/-- Every unlisted form is refused, whatever the access and flags. -/
theorem unlisted_forms_are_refused (a : Access) (punned custodyFree : Bool) :
    decideStep .unread a punned custodyFree = .refusedUnlisted ∧
      decideStep .unionBitfield a punned custodyFree = .refusedUnlisted := by
  cases a <;> simp [decideStep]

/-- A bit-field is never a pointer: a pointer access through one is refused. -/
theorem a_bitfield_pointer_access_is_refused (punned custodyFree : Bool) :
    decideStep .bitfield .pointerRead punned custodyFree = .refusedUnlisted ∧
      decideStep .bitfield .pointerStore punned custodyFree = .refusedUnlisted := by
  simp [decideStep]

/-- Only a listed storage is ever admitted. -/
theorem an_admitted_step_is_listed (s : Storage) (a : Access) (punned custodyFree : Bool)
    (admitted : (decideStep s a punned custodyFree).admits = true) :
    s = .ordinary ∨ s = .union ∨ s = .bitfield := by
  cases s <;> cases a <;> cases punned <;> cases custodyFree <;>
    simp_all [decideStep, Outcome.admits]

/-- A custody pointer stored through a union member is refused, custody-free
union or not. -/
theorem a_union_pointer_store_is_refused (punned custodyFree : Bool) :
    decideStep .union .pointerStore punned custodyFree = .refusedPointerStore := by
  simp [decideStep]

/-- A punned pointer read (the function stores through another member) is
refused; an unpunned one is a borrow with no custody. -/
theorem a_punned_pointer_read_is_refused (custodyFree : Bool) :
    decideStep .union .pointerRead true custodyFree = .refusedPunned ∧
      decideStep .union .pointerRead false custodyFree = .borrow := by
  simp [decideStep]

/-- A scalar read through a union member is neutral whatever member is live. -/
theorem a_union_scalar_read_is_neutral (punned custodyFree : Bool) :
    decideStep .union .scalarRead punned custodyFree = .neutral := by
  simp [decideStep]

/-! ## Bit-field storage units -/

/-- A bit range inside a storage unit. -/
structure BitRange where
  offset : Nat
  width : Nat
  deriving DecidableEq, Repr

def BitRange.holds (r : BitRange) (bit : Nat) : Bool :=
  decide (r.offset ≤ bit ∧ bit < r.offset + r.width)

/-- The unit-level events one access emits. -/
inductive UnitEvent where
  | readUnit (unit : Nat)
  | writeUnit (unit : Nat)
  deriving DecidableEq, Repr

/-- The events of a bit-field access decided `unitRead` or `unitWrite`. -/
def unitEvents : Outcome → Nat → List UnitEvent
  | .unitRead, unit => [.readUnit unit]
  | .unitWrite, unit => [.readUnit unit, .writeUnit unit]
  | _, _ => []

/-- The unit's contents after a store of `value` into range `r`. -/
def storeBits (bits : Nat → Bool) (r : BitRange) (value : Nat → Bool) : Nat → Bool :=
  fun bit => if r.holds bit then value (bit - r.offset) else bits bit

/-- A bit-field store is a read-modify-write of its whole unit: it reads and
then writes the unit, keeps every bit outside its range and sets every bit in
it. -/
theorem bitfield_store_is_unit_rmw (unit : Nat) (bits : Nat → Bool) (r : BitRange)
    (value : Nat → Bool) :
    unitEvents (decideStep .bitfield .scalarStore false false) unit
        = [.readUnit unit, .writeUnit unit] ∧
      (∀ bit, r.holds bit = false → storeBits bits r value bit = bits bit) ∧
      (∀ bit, r.holds bit = true → storeBits bits r value bit = value (bit - r.offset)) := by
  refine ⟨rfl, ?_, ?_⟩
  · intro bit outside
    simp [storeBits, outside]
  · intro bit inside
    simp [storeBits, inside]

/-- The footprint of a bit-field of unit `unit` at `bytes`: the unit's own,
whatever its bit range (the range is not an argument that can change it). -/
def bitfieldFootprint (alloc gen : Nat) (bytes : Interval) (unit : Nat) (_r : BitRange) :
    Footprint :=
  .memory alloc gen bytes (some unit)

theorem a_bitfield_footprint_is_its_unit (alloc gen unit : Nat) (bytes : Interval)
    (r s : BitRange) :
    bitfieldFootprint alloc gen bytes unit r = bitfieldFootprint alloc gen bytes unit s := rfl

/-- Example E: two bit-fields of one unit whose bit ranges are disjoint are
still refused as one memory location by the core footprint model. -/
theorem disjoint_bits_do_not_make_disjoint_footprints (alloc gen unit : Nat) (bytes : Interval)
    (r s : BitRange) (_apart : r.offset + r.width ≤ s.offset) :
    decideDisjoint (bitfieldFootprint alloc gen bytes unit r)
        (bitfieldFootprint alloc gen bytes unit s)
      = .refused .sharedBitfieldUnit :=
  QCore.DisjointFootprint.shared_bitfield_unit_is_never_disjoint unit

/-- Nor is any such pair ever decided disjoint. -/
theorem one_unit_is_never_disjoint (alloc gen unit : Nat) (bytes : Interval) (r s : BitRange) :
    decideDisjoint (bitfieldFootprint alloc gen bytes unit r)
        (bitfieldFootprint alloc gen bytes unit s) ≠ .disjoint := by
  simp [bitfieldFootprint, decideDisjoint, QCore.DisjointFootprint.decideMemory,
    QCore.DisjointFootprint.sharedUnit]

/-! ## Union stores and the refused-write frame -/

/-- A binding's place: a plain local, or a member place of an object. -/
inductive Place where
  | local (name : Nat)
  | member (object member : Nat)
  deriving DecidableEq, Repr

inductive Binding where
  | unbound
  | identity (site : Nat)
  | unknown
  deriving DecidableEq, Repr

/-- Poisoning keeps no identity. -/
def poison : Binding → Binding
  | .identity _ => .unknown
  | b => b

/-- W43's refused-write frame: a plain local the statement cannot write
(`spared`) keeps its binding; every other binding is poisoned. -/
def frame (spared : Nat → Bool) (bindings : Place → Binding) : Place → Binding
  | .local name => if spared name then bindings (.local name) else poison (bindings (.local name))
  | p => poison (bindings p)

/-- A scalar store through a union member, as the rule applies it. -/
def unionStore (custodyFree : Bool) (spared : Nat → Bool) (bindings : Place → Binding) :
    Place → Binding :=
  match decideStep .union .scalarStore false custodyFree with
  | .framed => frame spared bindings
  | _ => bindings

/-- A store through a member of a union that can hold a pointer leaves no
member place of any object — every sibling of the stored member included —
with a tracked identity. -/
theorem a_union_store_invalidates_sibling_identities (spared : Nat → Bool)
    (bindings : Place → Binding) (object sibling site : Nat) :
    unionStore false spared bindings (.member object sibling) ≠ .identity site := by
  have framed : unionStore false spared bindings (.member object sibling)
      = poison (bindings (.member object sibling)) := rfl
  rw [framed]
  cases bindings (.member object sibling) <;> simp [poison]

/-- A store through a custody-free union is frame-neutral. -/
theorem a_custody_free_union_store_is_frame_neutral (spared : Nat → Bool)
    (bindings : Place → Binding) :
    unionStore true spared bindings = bindings := by
  simp [unionStore, decideStep]

/-- The frame keeps a spared local's binding: the root the path is read
through survives a framed store. -/
theorem a_spared_local_keeps_its_binding (spared : Nat → Bool) (bindings : Place → Binding)
    (name : Nat) (kept : spared name = true) :
    unionStore false spared bindings (.local name) = bindings (.local name) := by
  simp [unionStore, decideStep, frame, kept]

end QCore.UnionBitfieldPlace
