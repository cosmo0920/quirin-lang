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

import QCore.ScalarTypes
import QCore.WriteLendTasks

/-! Typed Buffer and protected payloads (card `W48-QR-SCALAR-TYPES-R6`,
`W48-QR-TYPES-PAYLOADS`).

Before this card the owned Buffer's payload was one `int32_t`
(`task_execution::BUFFER_PAYLOAD_BYTES = 4`), and a range write lease's bounds
were decided against those four bytes. Now the payload type is part of the
Buffer's type — `own<Buffer<T>>` for every §G02t type `T` — and a lease's byte
extent is the payload type's **size**, never an assumed four. The same size is
the extent of a `protected T` payload on the checked-mutex route.

What is proved here:

* **The extent.** `size` is the byte size of one value of each type: a
  quarter of the `hex` digit count's width in bytes, `bool` one byte, and
  `isize`/`usize` only under a recorded width. An `i32` payload's extent is
  four, so every Buffer before the card has exactly its old extent.
* **`WRITE-LEND-TASKS` over typed extents.** The typed lease admission is the
  untyped one (`QCore.WriteLendTasks.admitsLease`, imported, not restated)
  plus "the range ends inside the payload". Every ledger state a spawn/join
  interleaving reaches under the typed admission is a state the untyped ledger
  reaches, so **no byte has two write authorities** at any point — the
  exclusivity theorem re-proved over typed extents, not weakened — and in
  addition every byte any lease holds is a byte of the payload. The worker's
  rebased footprint, which fits its lease, therefore names only payload bytes.
* **Why a `bool` payload has no byte write.** Every byte pattern of an integer
  payload's size is a value of its type (the range has exactly `2^(8·size)`
  values); a `bool`'s one byte has 256 patterns and two values. So the byte
  view of an integer payload is total and a `bool`'s is not — which is why the
  checker refuses a byte write into a `bool` payload, and why a `protected
  bool` write stores the fill's low bit.

The Rust transcription (`scalar_types::ScalarType::size_bytes`,
`task_execution::payload_extent`, the lease check in `State::value`) is bound to
these definitions by the table `sizeAgreement`, replayed by the Rust test
`lean_payload_sizes_match_the_rust_extent`, and by native execution; neither is
an extraction theorem. -/
namespace QCore.ScalarPayloads

open QCore.ScalarTypes QCore.SlicePermissions QCore.WriteLendTasks

/-! ## 1. The byte extent of a payload -/

/-- The byte size of one value of each type; `none` for `isize`/`usize`
without a recorded target. -/
def size (t : Option Width) : Ty → Option Nat
  | .bool => some 1
  | .i8 | .u8 => some 1
  | .i16 | .u16 => some 2
  | .i32 | .u32 => some 4
  | .i64 | .u64 => some 8
  | .isize | .usize => t.map fun | .w32 => 4 | .w64 => 8

/-- An integer type's size is its width in bytes. -/
theorem size_is_bits_over_eight (t : Option Width) (ty : Ty) (h : ty ≠ .bool) :
    size t ty = (ty.bits t).map (· / 8) := by
  cases ty <;> first
    | exact absurd rfl h
    | rfl
    | (cases t with
        | none => rfl
        | some w => cases w <;> rfl)

/-- **An `i32` payload's extent is four** — `BUFFER_PAYLOAD_BYTES`, so every
Buffer and every lease written before the card keeps exactly its extent. -/
theorem an_i32_payload_is_four_bytes (t : Option Width) : size t .i32 = some 4 := rfl

/-- A target width has no size without the recorded target: never guessed. -/
theorem a_target_width_payload_needs_the_target :
    size none .isize = none ∧ size none .usize = none := ⟨rfl, rfl⟩

/-- Every known size is between one and eight bytes. -/
theorem sizes_are_one_to_eight {t : Option Width} {ty : Ty} {n : Nat}
    (h : size t ty = some n) : 1 ≤ n ∧ n ≤ 8 := by
  cases ty <;> simp [size] at h <;> try omega
  all_goals
    cases t with
    | none => simp at h
    | some w => cases w <;> simp at h <;> omega

/-! ## 2. Every byte pattern of an integer payload is a value -/

/-- The number of values of a type. -/
def cardinality (t : Option Width) (ty : Ty) : Option Int :=
  (ty.range t).map fun (lo, hi) => hi - lo + 1

/-- **An integer payload's byte view is total**: its type has exactly
`2^(8·size)` values, one per byte pattern of its size. -/
theorem every_integer_pattern_is_a_value (w : Width) (ty : Ty) (h : ty ≠ .bool) :
    ∃ n, size (some w) ty = some n ∧ cardinality (some w) ty = some (2 ^ (8 * n)) := by
  cases ty <;> first
    | exact absurd rfl h
    | exact ⟨_, rfl, by decide⟩
    | (cases w <;> exact ⟨_, rfl, by decide⟩)

/-- **A `bool` payload's is not**: one byte, 256 patterns, two values. -/
theorem a_bool_byte_is_mostly_not_a_bool (t : Option Width) :
    size t .bool = some 1 ∧ cardinality t .bool = some 2 ∧ (2 : Int) < 2 ^ (8 * 1) :=
  ⟨rfl, rfl, by decide⟩

/-- What a `protected bool` write stores: the fill's low bit, a `bool` value. -/
def boolFill (fill : Nat) : Nat := fill % 2

theorem a_bool_fill_is_a_bool (fill : Nat) : fits none .bool (boolFill fill) = true := by
  unfold boolFill fits
  simp [Ty.range]
  omega

/-! ## 3. `WRITE-LEND-TASKS` over typed extents -/

/-- **The typed lease admission**: the untyped admission — nonempty, within the
holder bound, disjoint from every outstanding lease — plus the range ending
inside the payload's `extent` bytes. -/
def admitsTypedLease (extent : Nat) (leases : List Lease) (r : Range) : Bool :=
  decide (r.high ≤ extent) && admitsLease leases r

/-- Open one lease under the typed admission, total-or-nothing. -/
def openTypedLease (extent : Nat) (leases : List Lease) (holder : Nat) (r : Range) :
    Option (List Lease) :=
  if admitsTypedLease extent leases r then some ({ holder := holder, range := r } :: leases)
  else none

/-- Every ledger state a spawn/join interleaving reaches over a payload of
`extent` bytes. -/
inductive TypedReachable (extent : Nat) : List Lease → Prop
  | start : TypedReachable extent []
  | opened {s t : List Lease} {holder : Nat} {r : Range} :
      TypedReachable extent s → openTypedLease extent s holder r = some t →
      TypedReachable extent t
  | closed {s : List Lease} (holder : Nat) :
      TypedReachable extent s → TypedReachable extent (closeLease s holder)

/-- A typed open is an open of the untyped ledger. -/
theorem a_typed_open_is_an_open {extent : Nat} {s t : List Lease} {holder : Nat} {r : Range}
    (h : openTypedLease extent s holder r = some t) : openLease s holder r = some t := by
  unfold openTypedLease at h
  unfold openLease
  by_cases admitted : admitsTypedLease extent s r = true
  · rw [ite_eq_left admitted] at h
    have untyped : admitsLease s r = true := by
      simp only [admitsTypedLease, Bool.and_eq_true] at admitted
      exact admitted.2
    rw [ite_eq_left untyped]
    exact h
  · rw [ite_eq_right admitted] at h
    cases h

/-- Every typed-reachable state is reachable by the untyped ledger. -/
theorem typed_reachable_is_reachable {extent : Nat} {s : List Lease}
    (h : TypedReachable extent s) : Reachable s := by
  induction h with
  | start => exact Reachable.start
  | opened _ step ih => exact Reachable.opened ih (a_typed_open_is_an_open step)
  | closed holder _ ih => exact Reachable.closed holder ih

/-- **No byte has two write authorities over a typed payload**, at any point
of any spawn/join interleaving: `WRITE-LEND-TASKS`' exclusivity theorem,
re-proved over typed extents rather than weakened. -/
theorem no_byte_has_two_write_authorities_typed {extent : Nat} {s : List Lease}
    (h : TypedReachable extent s) (b : Nat) : writeAuthorities s b ≤ 1 :=
  no_byte_has_two_write_authorities (typed_reachable_is_reachable h) b

/-- Every outstanding lease ends inside the payload. -/
theorem every_lease_ends_inside_the_payload {extent : Nat} {s : List Lease}
    (h : TypedReachable extent s) : ∀ lease ∈ s, lease.range.high ≤ extent := by
  induction h with
  | start => intro lease member; cases member
  | opened reach step ih =>
      rename_i previous t holder r
      intro lease member
      unfold openTypedLease at step
      by_cases admitted : admitsTypedLease extent previous r = true
      · rw [ite_eq_left admitted] at step
        cases step
        rcases List.mem_cons.mp member with head | tail
        · subst head
          simp only [admitsTypedLease, Bool.and_eq_true, decide_eq_true_eq] at admitted
          exact admitted.1
        · exact ih lease tail
      · rw [ite_eq_right admitted] at step
        cases step
  | closed holder _ ih =>
      intro lease member
      exact ih lease (List.mem_filter.mp member).1

/-- **Every byte any lease holds is a byte of the payload.** -/
theorem every_held_byte_is_a_payload_byte {extent : Nat} {s : List Lease}
    (h : TypedReachable extent s) {lease : Lease} (member : lease ∈ s) {b : Nat}
    (holds : lease.range.contains b = true) : b < extent := by
  have ends := every_lease_ends_inside_the_payload h lease member
  simp only [Range.contains, Bool.and_eq_true, decide_eq_true_eq] at holds
  omega

/-- The worker's rebased footprint: a byte `i` it touches through its lease
satisfies `i < bound`, the spawn site admits the lease only when
`bound ≤ high - low`, and the lease ends inside the payload — so the owner-side
byte `low + i` is a byte of the payload. -/
theorem a_fitting_footprint_touches_only_payload_bytes {extent low high bound i : Nat}
    (inside : high ≤ extent) (fits : bound ≤ high - low) (touched : i < bound) :
    low + i < extent := by
  omega

/-- **Conservativity at `i32`**: over a four-byte payload the typed admission
is exactly `WRITE-LEND-TASKS`' rule — disjointness plus the range ending inside
`BUFFER_PAYLOAD_BYTES`. -/
theorem an_i32_lease_is_decided_as_before (leases : List Lease) (r : Range) :
    admitsTypedLease 4 leases r = (decide (r.high ≤ 4) && admitsLease leases r) := rfl

/-! ## 4. The vectors the Rust extent replays -/

/-- `(type, width, size)`: replayed against `scalar_types::ScalarType::size_bytes`
by `lean_payload_sizes_match_the_rust_extent`. -/
def sizeAgreement : List (Ty × Option Width × Option Nat) := [
  (.bool, none, some 1), (.i8, none, some 1), (.i16, none, some 2), (.i32, none, some 4),
  (.i64, none, some 8), (.isize, none, none), (.isize, some .w32, some 4),
  (.isize, some .w64, some 8), (.u8, none, some 1), (.u16, none, some 2),
  (.u32, none, some 4), (.u64, none, some 8), (.usize, none, none),
  (.usize, some .w32, some 4), (.usize, some .w64, some 8)
]

theorem sizeAgreement_holds :
    sizeAgreement.all (fun (ty, t, n) => size t ty == n) = true := by decide

end QCore.ScalarPayloads
