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

import QCore.VerdictSemigroup

/-!
# Imported C structs and the file-wide trust mark

Card **W48-QR-C-STRUCT-IMPORT** (card 1 of the `.qr` language track;
implemented grammars §G02u, language specification §L14.6, contract
specification §C04.9). The model of `crates/qcore-core/src/ffi.rs`
(`check_layout`, the field read of `check_ffi_program`) and of the published
verdict (`crates/qcore-schemas/src/verdict.rs`'s `AssumedVerdict`,
`crates/qcore-verification/src/driver.rs`'s `ffi_verdict`).

## What is proved

* **Layout agreement.** When a listing checks against the recorded compiler's
  members, every listed field is a member, at the member's recorded offset,
  of a type the member's canonical C type admits — so every admitted field
  read is at the recorded offset with the recorded type
  (`every_listed_read_is_at_the_recorded_offset`).
* **Opaque fields are unreachable.** A read names a listed field or reads
  nothing: a member the listing does not name — opaque bytes — is never read
  (`an_opaque_field_is_unreachable`).
* **Layout order.** The checked offsets are in listing order and ascending, so
  a listing out of layout order does not check (`checked_offsets_ascend`).
* **No plain `Safe` from an FFI file.** What an FFI file publishes always
  carries the unsafe-FFI assumption, so it is never `Safe` with no assumption
  (`an_ffi_file_never_publishes_plain_safe`), and the publication tripwire
  rules plain `Safe` out (`the_tripwire_rules_out_plain_safe`).
* **The join keeps it.** Composition joins the values (the §L21.2 semigroup)
  and unions the assumptions, so the assumption of any part is carried by every
  composed verdict (`every_composed_verdict_carries_the_assumption`) and no
  composition with an FFI part is plain `Safe`.

## What this is not

That the recorded compiler's layout is the one it uses to build the emitted C
is premise P-RECORDED-COMPILER, discharged at build time by the emitted
`_Static_assert`s rather than here; that the Rust reader extracts the layout
Clang reports is pinned by `compiler/tests/ffi_struct_import.rs`.
-/

namespace QCore.FfiStructImport

open QCore.VerdictSemigroup

/-- The §G02t table. -/
inductive Scalar where
  | bool | i8 | i16 | i32 | i64 | isize | u8 | u16 | u32 | u64 | usize
  deriving DecidableEq, Repr

/-- One direct member, as the recorded compiler reported it: its name, its
offset and the §G02t types whose C11 spelling has its canonical type. -/
structure Member where
  name : String
  offset : Nat
  scalars : List Scalar
  deriving DecidableEq, Repr

/-- A listed field: a name and a `.qr` type. No offset: the listing states
none. -/
structure Listed where
  name : String
  ty : Scalar
  deriving DecidableEq, Repr

/-- A listed field after the check: its type at the recorded offset. -/
structure Field where
  name : String
  ty : Scalar
  offset : Nat
  deriving DecidableEq, Repr

/-- The member a name denotes. -/
def findMember (members : List Member) (n : String) : Option Member :=
  members.find? (fun m => m.name == n)

/-- The previous listed offset admits `o`. -/
def after (prev : Option Nat) (o : Nat) : Bool :=
  match prev with
  | none => true
  | some p => decide (p ≤ o)

/-- `check_layout`: every listed field is a member, of a type the member
admits, in layout order. -/
def checkLayout (members : List Member) : List Listed → Option Nat → Option (List Field)
  | [], _ => some []
  | l :: rest, prev =>
    match findMember members l.name with
    | none => none
    | some m =>
      if m.scalars.contains l.ty && after prev m.offset then
        match checkLayout members rest (some m.offset) with
        | none => none
        | some tail => some (⟨l.name, l.ty, m.offset⟩ :: tail)
      else none

/-- A field read of an imported record: the type and offset of a listed
field, and nothing for any other name. -/
def read (fields : List Field) (n : String) : Option (Scalar × Nat) :=
  (fields.find? (fun f => f.name == n)).map (fun f => (f.ty, f.offset))

theorem checked_fields_agree (members : List Member) :
    ∀ (listing : List Listed) (prev : Option Nat) (fields : List Field),
      checkLayout members listing prev = some fields →
      ∀ f ∈ fields, ∃ m, findMember members f.name = some m ∧ m.offset = f.offset ∧
        f.ty ∈ m.scalars := by
  intro listing
  induction listing with
  | nil =>
    intro prev fields h f hf
    simp [checkLayout] at h
    subst h
    simp at hf
  | cons l rest ih =>
    intro prev fields h f hf
    simp only [checkLayout] at h
    cases hm : findMember members l.name with
    | none => simp [hm] at h
    | some m =>
      simp only [hm] at h
      split at h
      · rename_i hc
        cases ht : checkLayout members rest (some m.offset) with
        | none => simp [ht] at h
        | some tail =>
          simp only [ht, Option.some.injEq] at h
          subst h
          simp only [List.mem_cons] at hf
          rcases hf with hf | hf
          · subst hf
            refine ⟨m, hm, rfl, ?_⟩
            simp only [Bool.and_eq_true] at hc
            exact List.contains_iff_mem.mp hc.1
          · exact ih (some m.offset) tail ht f hf
      · simp at h

theorem checked_names (members : List Member) :
    ∀ (listing : List Listed) (prev : Option Nat) (fields : List Field),
      checkLayout members listing prev = some fields →
      fields.map (·.name) = listing.map (·.name) := by
  intro listing
  induction listing with
  | nil =>
    intro prev fields h
    simp [checkLayout] at h
    subst h
    rfl
  | cons l rest ih =>
    intro prev fields h
    simp only [checkLayout] at h
    cases hm : findMember members l.name with
    | none => simp [hm] at h
    | some m =>
      simp only [hm] at h
      split at h
      · rename_i hc
        cases ht : checkLayout members rest (some m.offset) with
        | none => simp [ht] at h
        | some tail =>
          simp only [ht, Option.some.injEq] at h
          subst h
          simp [ih (some m.offset) tail ht]
      · simp at h

/-- Layout agreement: an admitted read is at the recorded offset, with a type
the recorded member admits. -/
theorem every_listed_read_is_at_the_recorded_offset
    (members : List Member) (listing : List Listed) (fields : List Field)
    (h : checkLayout members listing none = some fields)
    (n : String) (ty : Scalar) (offset : Nat) (hr : read fields n = some (ty, offset)) :
    ∃ m, findMember members n = some m ∧ m.offset = offset ∧ ty ∈ m.scalars := by
  unfold read at hr
  cases hf : fields.find? (fun f => f.name == n) with
  | none => simp [hf] at hr
  | some f =>
    simp only [hf, Option.map_some, Option.some.injEq, Prod.mk.injEq] at hr
    obtain ⟨hty, hoff⟩ := hr
    have hmem : f ∈ fields := List.mem_of_find?_eq_some hf
    have hname : f.name = n := by
      have := List.find?_some hf
      simpa using this
    obtain ⟨m, hm, ho, hs⟩ := checked_fields_agree members listing none fields h f hmem
    exact ⟨m, hname ▸ hm, ho.trans hoff, hty ▸ hs⟩

/-- An opaque member — one the listing does not name — is never read. -/
theorem an_opaque_field_is_unreachable
    (members : List Member) (listing : List Listed) (fields : List Field)
    (h : checkLayout members listing none = some fields)
    (n : String) (hopaque : n ∉ listing.map (·.name)) :
    read fields n = none := by
  unfold read
  cases hf : fields.find? (fun f => f.name == n) with
  | none => rfl
  | some f =>
    exfalso
    have hmem : f ∈ fields := List.mem_of_find?_eq_some hf
    have hname : f.name = n := by
      have := List.find?_some hf
      simpa using this
    apply hopaque
    rw [← checked_names members listing none fields h]
    exact List.mem_map.mpr ⟨f, hmem, hname⟩

/-- Consecutive checked offsets ascend: the listing is in layout order. -/
theorem checked_offsets_ascend (members : List Member) :
    ∀ (listing : List Listed) (prev : Option Nat) (fields : List Field),
      checkLayout members listing prev = some fields →
      ∀ f ∈ fields, after prev f.offset = true := by
  intro listing
  induction listing with
  | nil =>
    intro prev fields h f hf
    simp [checkLayout] at h
    subst h
    simp at hf
  | cons l rest ih =>
    intro prev fields h f hf
    simp only [checkLayout] at h
    cases hm : findMember members l.name with
    | none => simp [hm] at h
    | some m =>
      simp only [hm] at h
      split at h
      · rename_i hc
        cases ht : checkLayout members rest (some m.offset) with
        | none => simp [ht] at h
        | some tail =>
          simp only [ht, Option.some.injEq] at h
          subst h
          simp only [Bool.and_eq_true] at hc
          simp only [List.mem_cons] at hf
          rcases hf with hf | hf
          · subst hf
            exact hc.2
          · have hnext := ih (some m.offset) tail ht f hf
            cases prev with
            | none => rfl
            | some p =>
              simp only [after, decide_eq_true_eq] at hc hnext ⊢
              exact Nat.le_trans hc.2 hnext
      · simp at h

/-! ## The published verdict -/

/-- A verdict with named assumptions (`AssumedVerdict`). -/
structure Assumed where
  verdict : V
  assumptions : List String

/-- Composition: the §L21.2 join of the values, the union of the
assumptions. -/
def Assumed.join (a b : Assumed) : Assumed :=
  ⟨V.join a.verdict b.verdict, a.assumptions ++ b.assumptions⟩

/-- `Safe` with no assumption at all. -/
def plainSafe (a : Assumed) : Prop :=
  a.verdict = V.safe ∧ a.assumptions = []

/-- The trust mark's name. -/
def unsafeFfi : String := "unsafe-ffi"

/-- What a route publishes: a result from an FFI file always names the trust
mark beside its other assumptions. -/
def publish (ffiFile : Bool) (v : V) (base : List String) : Assumed :=
  ⟨v, if ffiFile then unsafeFfi :: base else base⟩

/-- The publication tripwire (`ffi_verdict`): published only with the mark. -/
def tripwire (a : Assumed) : Bool :=
  a.assumptions.contains unsafeFfi

theorem an_ffi_file_never_publishes_plain_safe (v : V) (base : List String) :
    ¬ plainSafe (publish true v base) := by
  intro h
  simp [plainSafe, publish] at h

theorem the_tripwire_rules_out_plain_safe (a : Assumed) (h : tripwire a = true) :
    ¬ plainSafe a := by
  intro hp
  simp [tripwire, hp.2] at h

theorem an_ffi_publication_passes_the_tripwire (v : V) (base : List String) :
    tripwire (publish true v base) = true := by
  simp [tripwire, publish]

theorem join_keeps_the_assumption (a b : Assumed) (x : String)
    (h : x ∈ a.assumptions ∨ x ∈ b.assumptions) :
    x ∈ (a.join b).assumptions ∧ x ∈ (b.join a).assumptions := by
  simp only [Assumed.join, List.mem_append]
  rcases h with h | h
  · exact ⟨Or.inl h, Or.inr h⟩
  · exact ⟨Or.inr h, Or.inl h⟩

/-- The fold of a composition, from `Safe` with no assumption. -/
def fold (parts : List Assumed) : Assumed :=
  parts.foldl Assumed.join ⟨V.safe, []⟩

theorem foldl_keeps (x : String) :
    ∀ (parts : List Assumed) (acc : Assumed),
      (x ∈ acc.assumptions ∨ ∃ a ∈ parts, x ∈ a.assumptions) →
      x ∈ (parts.foldl Assumed.join acc).assumptions := by
  intro parts
  induction parts with
  | nil =>
    intro acc h
    rcases h with h | ⟨a, ha, _⟩
    · exact h
    · simp at ha
  | cons p rest ih =>
    intro acc h
    simp only [List.foldl_cons]
    apply ih
    rcases h with h | ⟨a, ha, hx⟩
    · exact Or.inl (join_keeps_the_assumption acc p x (Or.inl h)).1
    · simp only [List.mem_cons] at ha
      rcases ha with ha | ha
      · subst ha
        exact Or.inl (join_keeps_the_assumption acc a x (Or.inr hx)).1
      · exact Or.inr ⟨a, ha, hx⟩

/-- The assumption of any part is carried by the composed verdict. -/
theorem every_composed_verdict_carries_the_assumption (parts : List Assumed)
    (h : ∃ a ∈ parts, unsafeFfi ∈ a.assumptions) :
    unsafeFfi ∈ (fold parts).assumptions :=
  foldl_keeps unsafeFfi parts _ (Or.inr h)

/-- So no composition with an FFI part is plain `Safe`. -/
theorem a_composition_with_an_ffi_part_is_never_plain_safe (parts : List Assumed)
    (h : ∃ a ∈ parts, unsafeFfi ∈ a.assumptions) :
    ¬ plainSafe (fold parts) := by
  intro hp
  have hx := every_composed_verdict_carries_the_assumption parts h
  rw [hp.2] at hx
  simp at hx

/-! ## Non-vacuity: the example's layout -/

/-- `struct qr_header` as the recorded compiler reports it on x86-64 Linux. -/
def qrHeader : List Member :=
  [⟨"len", 0, [.u32]⟩, ⟨"flags", 4, [.u8]⟩, ⟨"reserved", 5, []⟩,
   ⟨"id", 8, [.i64, .isize]⟩, ⟨"valid", 16, [.bool]⟩, ⟨"label", 24, []⟩]

def exampleListing : List Listed :=
  [⟨"len", .u32⟩, ⟨"flags", .u8⟩, ⟨"id", .i64⟩, ⟨"valid", .bool⟩]

theorem the_example_listing_checks :
    checkLayout qrHeader exampleListing none =
      some [⟨"len", .u32, 0⟩, ⟨"flags", .u8, 4⟩, ⟨"id", .i64, 8⟩, ⟨"valid", .bool, 16⟩] := by
  decide

theorem a_mistyped_listing_does_not_check :
    checkLayout qrHeader [⟨"len", .u16⟩] none = none := by
  decide

theorem a_reordered_listing_does_not_check :
    checkLayout qrHeader [⟨"flags", .u8⟩, ⟨"len", .u32⟩] none = none := by
  decide

theorem an_unequivalent_member_cannot_be_listed :
    checkLayout qrHeader [⟨"reserved", .u8⟩] none = none := by
  decide

end QCore.FfiStructImport
