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

import QCore.ScalarPayloads
import QCore.FloatTypes

/-! Float payloads and float values on the byte-buffer route (card
`W48-QR-FLOAT-BUFFER-PAYLOADS`).

The float card (`QCore.FloatTypes`) admitted `f32`/`f64` everywhere but the
byte-buffer route, the composed string route and the two payload positions.
This card admits them there, and states in Lean the parts that are new:

* **The extent of a float payload.** A payload type is an integer, `bool` or
  float type (`PTy`); its size is `QCore.ScalarPayloads.size` on the integer
  table — unchanged — and 4 and 8 bytes for `f32` and `f64`, with no recorded
  target. `Buffer<f32>`/`Buffer<f64>` lease extents and `protected f32`/`f64`
  regions are those sizes.
* **The byte view of a float payload is total.** A float value is its bit
  pattern (`scalar_floats`: every pattern is a value), so a payload of `n`
  bytes holds exactly the patterns below `256^n`. The little-endian byte view
  and its inverse round-trip (`fromBytes_toBytes`), every byte sequence of the
  payload's size decodes to a pattern of it (`fromBytes_lt`), and so a byte
  write into a float payload always leaves a value of the payload's type
  (`a_byte_write_keeps_a_float_value`) — unlike `bool`, whose byte write the
  checker refuses (`QCore.ScalarPayloads.a_bool_byte_is_mostly_not_a_bool`).
  Which float a pattern denotes is IEEE's; NaN payload bits are not specified
  and nothing here depends on them.
* **`WRITE-LEND-TASKS` at float extents, re-proved rather than weakened.** The
  typed ledger (`QCore.ScalarPayloads.TypedReachable`) over a float payload's
  extent keeps the exclusivity theorem — no byte has two write authorities —
  and every byte a lease holds is a byte of the payload; a lease sized for the
  other float type is refused (`an_f64_sized_lease_on_an_f32_is_refused`).
* **Comparisons on the byte-buffer route.** The route's entry is one function
  with no calls, whose loop bodies run as repeated sites; it is admitted
  exactly when every float comparison it (or any unrolled iteration) makes is
  declared by its row — the float card's `Admitted` specialised to a single
  call-free entry (`buffer_entry_admitted_iff`), and unrolling adds no
  comparison kind (`unrolled_comparisons_are_the_body's`).

The Rust transcription — `ScalarType::size_bytes` and
`task_execution::payload_extent` at `F32`/`F64`, `scalar_types::object_bytes`
and `from_object_bytes`, the buffer route's `Decider` with the row's
`partial_eq`/`partial_ord` — is bound to these definitions by the vector table
`floatSizeAgreement` (replayed by the Rust test
`lean_float_payload_sizes_match_the_rust_extent`) and by native execution;
neither is an extraction theorem. No project axiom is used. -/
namespace QCore.FloatPayloads

open QCore.ScalarTypes (Ty Width)
open QCore.ScalarPayloads
open QCore.SlicePermissions QCore.WriteLendTasks

/-! ## 1. The extent of a payload, floats included -/

/-- A payload type: the integer table and `bool`, or a float type. -/
inductive PTy where
  | scalar (ty : Ty)
  | f32
  | f64
  deriving DecidableEq, Repr

/-- The byte size of one value of a payload type. -/
def psize (t : Option Width) : PTy → Option Nat
  | .scalar ty => size t ty
  | .f32 => some 4
  | .f64 => some 8

/-- **A float payload is its IEEE format's size**: binary32 four bytes and
binary64 eight, with or without a recorded target. -/
theorem a_float_payload_is_its_format_size (t : Option Width) :
    psize t .f32 = some 4 ∧ psize t .f64 = some 8 := ⟨rfl, rfl⟩

/-- **The integer table's sizes are unchanged** — the floats extend the
payload table without moving a row of it. -/
theorem integer_payload_sizes_are_unchanged (t : Option Width) (ty : Ty) :
    psize t (.scalar ty) = size t ty := rfl

/-- Every known payload size is between one and eight bytes. -/
theorem payload_sizes_are_one_to_eight {t : Option Width} {p : PTy} {n : Nat}
    (h : psize t p = some n) : 1 ≤ n ∧ n ≤ 8 := by
  cases p with
  | scalar ty => exact sizes_are_one_to_eight h
  | f32 => simp [psize] at h; omega
  | f64 => simp [psize] at h; omega

/-! ## 2. The byte view of a float payload -/

/-- Whether `bits` is a value of a float payload of `n` bytes: a bit pattern
of that width. Every pattern is one (NaNs, infinities, signed zeros and
subnormals included) — `scalar_floats::is_pattern`. -/
def isPattern (n bits : Nat) : Bool := decide (bits < 256 ^ n)

/-- The little-endian object representation of `bits` in `n` bytes —
`scalar_types::object_bytes` on a little-endian host. -/
def toBytes : Nat → Nat → List Nat
  | 0, _ => []
  | n + 1, bits => bits % 256 :: toBytes n (bits / 256)

/-- The pattern whose little-endian bytes are `bs` —
`scalar_types::from_object_bytes`. -/
def fromBytes : List Nat → Nat
  | [] => 0
  | b :: bs => b % 256 + 256 * fromBytes bs

theorem toBytes_length (n bits : Nat) : (toBytes n bits).length = n := by
  induction n generalizing bits with
  | zero => rfl
  | succ n ih => simp [toBytes, ih]

/-- **Every byte sequence decodes to a pattern of its width.** -/
theorem fromBytes_lt (bs : List Nat) : fromBytes bs < 256 ^ bs.length := by
  induction bs with
  | nil => simp [fromBytes]
  | cons b bs ih =>
      simp only [fromBytes, List.length_cons, Nat.pow_succ]
      have hb : b % 256 < 256 := Nat.mod_lt _ (by decide)
      have hmul : 256 * fromBytes bs + 256 ≤ 256 ^ bs.length * 256 := by
        have := Nat.mul_le_mul_left 256 (Nat.succ_le_of_lt ih)
        rw [Nat.mul_succ] at this
        rw [Nat.mul_comm (256 ^ bs.length) 256]
        exact this
      omega

/-- **The byte view round-trips**: a pattern of the payload's width is the
pattern its bytes decode to. -/
theorem fromBytes_toBytes (n bits : Nat) (h : bits < 256 ^ n) :
    fromBytes (toBytes n bits) = bits := by
  induction n generalizing bits with
  | zero => simp [toBytes, fromBytes] at *; omega
  | succ n ih =>
      simp only [toBytes, fromBytes]
      have hq : bits / 256 < 256 ^ n := by
        rw [Nat.pow_succ] at h
        exact Nat.div_lt_of_lt_mul (by rw [Nat.mul_comm]; exact h)
      rw [ih _ hq, Nat.mod_mod]
      omega

/-- **A byte write into a float payload keeps a value of its type**: whichever
byte of an `f32` (or `f64`) payload is overwritten, with whatever byte, the
payload still decodes to a pattern of its width — a value. -/
theorem a_byte_write_keeps_a_float_value {p : PTy} {n bits : Nat}
    (hp : p = .f32 ∨ p = .f64) (hn : psize none p = some n) (_ : isPattern n bits = true)
    (i byte : Nat) : isPattern n (fromBytes ((toBytes n bits).set i byte)) = true := by
  have _ := hp
  have _ := hn
  unfold isPattern
  have hl : ((toBytes n bits).set i byte).length = n := by
    rw [List.length_set, toBytes_length]
  have := fromBytes_lt ((toBytes n bits).set i byte)
  rw [hl] at this
  exact decide_eq_true this

/-- Every pattern of a float payload's width is its own bytes' decoding, so the
reference's byte view and the whole value never disagree. -/
theorem every_float_pattern_is_its_bytes {p : PTy} {n bits : Nat}
    (hn : psize none p = some n) (h : isPattern n bits = true) :
    fromBytes (toBytes n bits) = bits := by
  have _ := hn
  exact fromBytes_toBytes n bits (of_decide_eq_true h)

/-- A float payload has exactly `256^size` values: `2^32` for `f32` and
`2^64` for `f64` — one per byte pattern, as for an integer payload. -/
theorem float_patterns_are_every_byte_pattern :
    (∀ bits, isPattern 4 bits = true ↔ bits < 2 ^ 32) ∧
      (∀ bits, isPattern 8 bits = true ↔ bits < 2 ^ 64) := by
  refine ⟨fun bits => ?_, fun bits => ?_⟩ <;> simp [isPattern]

/-! ## 3. `WRITE-LEND-TASKS` at float extents -/

/-- **No byte of a float payload has two write authorities**, at any point of
any spawn/join interleaving — the typed exclusivity theorem at the payload's
own extent, re-proved over every payload type with a size, floats included. -/
theorem no_byte_has_two_write_authorities_payload {t : Option Width} {p : PTy} {n : Nat}
    (_ : psize t p = some n) {s : List Lease} (h : TypedReachable n s) (b : Nat) :
    writeAuthorities s b ≤ 1 :=
  no_byte_has_two_write_authorities_typed h b

/-- Every byte a lease on a payload holds is a byte of that payload. -/
theorem every_held_byte_is_a_payload_byte_payload {t : Option Width} {p : PTy} {n : Nat}
    (_ : psize t p = some n) {s : List Lease} (h : TypedReachable n s) {lease : Lease}
    (member : lease ∈ s) {b : Nat} (holds : lease.range.contains b = true) : b < n :=
  every_held_byte_is_a_payload_byte h member holds

/-- The two instances the card's payload rows exercise: an `f32` payload's
leases hold bytes `0 … 3`, an `f64` payload's bytes `0 … 7`, and no byte of
either has two write authorities. -/
theorem float_lease_ledgers {s₄ s₈ : List Lease} (h₄ : TypedReachable 4 s₄)
    (h₈ : TypedReachable 8 s₈) (b : Nat) :
    writeAuthorities s₄ b ≤ 1 ∧ writeAuthorities s₈ b ≤ 1 ∧
      (∀ lease ∈ s₄, lease.range.high ≤ 4) ∧ (∀ lease ∈ s₈, lease.range.high ≤ 8) :=
  ⟨no_byte_has_two_write_authorities_typed h₄ b, no_byte_has_two_write_authorities_typed h₈ b,
    every_lease_ends_inside_the_payload h₄, every_lease_ends_inside_the_payload h₈⟩

/-- **A lease sized for the other float is refused** (the negative control): a
range reaching past an `f32` payload's four bytes — an `f64`'s eight, say — is
not admitted, whatever else is outstanding. -/
theorem an_f64_sized_lease_on_an_f32_is_refused (leases : List Lease) (r : Range)
    (h : 4 < r.high) : admitsTypedLease 4 leases r = false := by
  unfold admitsTypedLease
  have : decide (r.high ≤ 4) = false := decide_eq_false (by omega)
  rw [this, Bool.false_and]

/-- And the halves the native rows lease are admitted on an empty ledger. -/
theorem the_float_halves_are_admitted :
    admitsTypedLease 4 [] ⟨0, 2⟩ = true ∧ admitsTypedLease 8 [] ⟨4, 8⟩ = true ∧
      admitsTypedLease 8 [{ holder := 1, range := ⟨0, 4⟩ }] ⟨4, 8⟩ = true := by
  decide

/-! ## 4. Comparisons on the byte-buffer route -/

open QCore.FloatTypes (Effect Site Fn Admitted Reaches)

/-- A buffer-route entry: one function whose sites are its statements, with
no call (the route models its entry only). -/
def CallFree (f : Fn) : Prop := ∀ g, Site.call g ∉ f.body

/-- **The buffer route's decision is the float card's admission** for its one
call-free entry: admitted exactly when every float comparison of the entry is
declared by the entry's row. -/
theorem buffer_entry_admitted_iff {f : Fn} (hf : CallFree f) :
    Admitted [f] ↔ ∀ e, Site.floatCmp e ∈ f.body → e ∈ f.declared := by
  constructor
  · intro h e he
    exact (h 0 f rfl).1 e he
  · intro h i g hg
    cases i with
    | zero =>
        simp at hg
        subst hg
        exact ⟨h, fun g' _ hc _ => absurd hc (hf g')⟩
    | succ i => simp at hg

/-- An admitted buffer-route entry without `partial_eq` makes no float
`==`/`!=`, and without `partial_ord` no float ordering. -/
theorem buffer_entry_without_effect_compares_nothing {f : Fn} (hf : CallFree f)
    (h : Admitted [f]) {e : Effect} (hn : e ∉ f.declared) : Site.floatCmp e ∉ f.body :=
  fun hs => hn ((buffer_entry_admitted_iff hf).mp h e hs)

/-- A loop body run `k` times: its sites, repeated. -/
def unroll (k : Nat) (body : List Site) : List Site := (List.replicate k body).flatten

/-- **Unrolling adds no comparison kind**: every float comparison of an
unrolled loop is one of its body's, so deciding the body under the row decides
every iteration (the route checks each iteration's statements with the same
decision). -/
theorem unrolled_comparisons_are_the_body's {k : Nat} {body : List Site} {e : Effect}
    (h : Site.floatCmp e ∈ unroll k body) : Site.floatCmp e ∈ body := by
  unfold unroll at h
  rw [List.mem_flatten] at h
  obtain ⟨l, hl, hm⟩ := h
  rw [(List.eq_of_mem_replicate hl)] at hm
  exact hm

/-- An entry whose loop body compares floats is admitted only under the effect,
at any positive iteration count. -/
theorem a_loop_body_comparison_needs_its_effect {k : Nat} {body : List Site} {e : Effect}
    {declared : List Effect} (hk : 0 < k) (he : Site.floatCmp e ∈ body)
    (h : Admitted [{ declared := declared, body := unroll k body }]) : e ∈ declared := by
  have hmem : Site.floatCmp e ∈ unroll k body := by
    unfold unroll
    rw [List.mem_flatten]
    exact ⟨body, List.mem_replicate.mpr ⟨by omega, rfl⟩, he⟩
  exact (h 0 _ rfl).1 e hmem

/-! ## 5. The vectors the Rust extent replays -/

/-- `(type, size)`: replayed against `ScalarType::size_bytes` and
`task_execution::payload_extent` by `lean_float_payload_sizes_match_the_rust_extent`. -/
def floatSizeAgreement : List (PTy × Nat) := [(.f32, 4), (.f64, 8)]

theorem floatSizeAgreement_holds :
    floatSizeAgreement.all (fun (p, n) => psize none p == some n) = true := by decide

end QCore.FloatPayloads
