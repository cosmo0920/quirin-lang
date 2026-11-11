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
# Enumerations and pointers to pointers to const in the declared-ABI reader

Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rules 12u and 12v). This module
models the two readings the card added to
`crates/qcore-c-gateway/src/legacy_declared_abi.rs` and
`legacy_enum_projection.rs`.

USER DECISION 2026-10-05: an enumeration parameter or result is spelled as its
underlying integer type, taken from Clang for the target, never assumed to be
`int`, with an exact width and signedness match; when Clang does not state
the type, the reader refuses. The reader asks Clang, for each enumeration
type `T`, three things about the analysed unit: `sizeof(T) * CHAR_BIT`, the
sign of `(T)-1`, and which of the ten standard integer types `T` is compatible
with (`ClangFacts`). `projection` is the Rust `projection_of`.

The pointer reading (rule 12v) adds `ptr<mut, ptr<const, u8>>` for a pointer
to a pointer whose **innermost** pointee is `const`, and nothing for a pointer
whose slot (the pointer in between) is qualified.

## What is proved

* **Enumeration, exactness**: a projection exists only when exactly one
  standard type is compatible and its signedness is the probed one; the
  projection's width and signedness are the probed ones; `isize`/`usize`
  are never produced; an enumeration Clang says is unsigned never reads as
  `i32` (`int` is never assumed); without Clang's facts the reader refuses.
* **Enumeration, controls**: the four measured shapes (no negative value,
  a negative value, a value beyond `int`, a fixed `unsigned char` type).
* **Pointer to pointer to const**: produced exactly for two levels with a
  `const` innermost pointee and an unqualified slot; a qualified slot refuses;
  the widening only adds (every earlier reading is unchanged); `char **` and
  `const char **` are distinct readings; `resource_out` still admits only
  `ptr<mut, ptr<mut, u8>>`.

## What this is not

A model of the token reader or of Clang. That the facts are Clang's answers
for the analysed unit is the probe's transport premise (trust boundary 28);
that `__builtin_types_compatible_p` is true exactly for the integer type
Clang gives an enumeration is a Clang fact supplied from outside.
-/

namespace QCore.AbiEnumConstPointer

/-! ## The scalar spellings -/

inductive Scalar where
  | i8 | u8 | i16 | u16 | i32 | u32 | i64 | u64 | isize | usize
  deriving DecidableEq, Repr

/-- The ten standard integer types the probe compares with, each with its
signedness: `signed char`, `unsigned char`, `short`, `unsigned short`,
`int`, `unsigned int`, `long`, `unsigned long`, `long long`,
`unsigned long long`. -/
def candidateSigned : List Bool :=
  [true, false, true, false, true, false, true, false, true, false]

/-- What Clang states about one enumeration type. -/
structure ClangFacts where
  width : Nat
  signed : Bool
  compatible : List Bool
  deriving Repr

/-- The signedness of the one compatible candidate, when exactly one is. -/
def uniqueCompatible : List Bool → List Bool → Option Bool
  | flags, signs =>
    match (flags.zip signs).filter (fun pair => pair.1) with
    | [(_, sign)] => some sign
    | _ => none

/-- The spelling of a width and signedness. -/
def ofWidth : Nat → Bool → Option Scalar
  | 8, true => some .i8
  | 8, false => some .u8
  | 16, true => some .i16
  | 16, false => some .u16
  | 32, true => some .i32
  | 32, false => some .u32
  | 64, true => some .i64
  | 64, false => some .u64
  | _, _ => none

/-- The Rust `projection_of`. -/
def projection (facts : ClangFacts) : Option Scalar :=
  match uniqueCompatible facts.compatible candidateSigned with
  | some sign => if sign = facts.signed then ofWidth facts.width facts.signed else none
  | none => none

/-- The reader's reading: no facts (no analysed source, a failed launch, a
forged probe name) refuses. -/
def read : Option ClangFacts → Option Scalar
  | some facts => projection facts
  | none => none

def Scalar.width : Scalar → Nat
  | .i8 | .u8 => 8
  | .i16 | .u16 => 16
  | .i32 | .u32 => 32
  | .i64 | .u64 | .isize | .usize => 64

def Scalar.signed : Scalar → Bool
  | .i8 | .i16 | .i32 | .i64 | .isize => true
  | _ => false

theorem ofWidth_exact (w : Nat) (s : Bool) (spelling : Scalar)
    (h : ofWidth w s = some spelling) : spelling.width = w ∧ spelling.signed = s := by
  unfold ofWidth at h
  split at h <;> simp_all <;> subst h <;> decide

theorem ofWidth_never_pointer_width (w : Nat) (s : Bool) :
    ofWidth w s ≠ some .isize ∧ ofWidth w s ≠ some .usize := by
  unfold ofWidth
  split <;> simp

/-- A projection's width and signedness are exactly Clang's. -/
theorem a_projection_is_exactly_the_probed_width_and_signedness (facts : ClangFacts)
    (spelling : Scalar) (h : projection facts = some spelling) :
    spelling.width = facts.width ∧ spelling.signed = facts.signed := by
  unfold projection at h
  split at h
  · split at h
    · exact ofWidth_exact _ _ _ h
    · cases h
  · cases h

/-- The pointer-width readings a plain `long` also admits are never an
enumeration's. -/
theorem an_enumeration_never_reads_as_a_pointer_width (facts : ClangFacts) :
    projection facts ≠ some .isize ∧ projection facts ≠ some .usize := by
  unfold projection
  split
  · split
    · exact ofWidth_never_pointer_width _ _
    · simp
  · simp

/-- `int` is never assumed: an enumeration Clang says is unsigned is never
`i32`. -/
theorem an_unsigned_enumeration_is_never_int (facts : ClangFacts)
    (unsigned : facts.signed = false) : projection facts ≠ some .i32 := by
  intro h
  have := (a_projection_is_exactly_the_probed_width_and_signedness facts .i32 h).2
  rw [unsigned] at this
  exact absurd this (by decide)

/-- No unique compatible type, no projection. -/
theorem no_unique_compatible_type_refuses (facts : ClangFacts)
    (h : uniqueCompatible facts.compatible candidateSigned = none) :
    projection facts = none := by
  unfold projection
  rw [h]

/-- A signedness disagreement refuses. -/
theorem a_signedness_disagreement_refuses (facts : ClangFacts) (sign : Bool)
    (h : uniqueCompatible facts.compatible candidateSigned = some sign)
    (disagree : sign ≠ facts.signed) : projection facts = none := by
  unfold projection
  rw [h]
  simp [disagree]

/-- Without Clang's facts the reader refuses. -/
theorem without_clang_the_reader_refuses : read none = none := rfl

/-! ### Controls (the four shapes the Rust tests and the probe measured) -/

/-- `typedef enum { A, B } e_t;`: no negative value, `unsigned int`. -/
def noNegative : ClangFacts :=
  ⟨32, false, [false, false, false, false, false, true, false, false, false, false]⟩
/-- `enum err { E_FAIL = -1, E_OK }`: `int`. -/
def negative : ClangFacts :=
  ⟨32, true, [false, false, false, false, true, false, false, false, false, false]⟩
/-- `enum big { BIG = 0x100000000 }`: `unsigned long` on LP64. -/
def wide : ClangFacts :=
  ⟨64, false, [false, false, false, false, false, false, false, true, false, false]⟩
/-- `enum : unsigned char { H }` (C23): `unsigned char`. -/
def fixedByte : ClangFacts :=
  ⟨8, false, [false, true, false, false, false, false, false, false, false, false]⟩

theorem enumeration_controls :
    projection noNegative = some .u32 ∧ projection negative = some .i32 ∧
    projection wide = some .u64 ∧ projection fixedByte = some .u8 := by
  decide

/-- Two compatible types (a malformed answer) refuse. -/
theorem two_compatible_types_refuse :
    projection ⟨32, true, [false, false, false, false, true, true, false, false, false, false]⟩
      = none := by
  decide

/-! ## Pointers to pointers to const (rule 12v) -/

inductive Pointer where
  | mutU8 | constU8 | mutPtrMutU8 | mutPtrConstU8
  deriving DecidableEq, Repr

/-- The reading of `stars` pointer levels over a pointee whose innermost
level is (`innerConst`) or is not `const`, with a qualifier on the slot
(`slotQualified`: written between two `*`, or on a typedef name that stands
for a pointer) or not. -/
def pointer : Nat → Bool → Bool → Option Pointer
  | 1, false, _ => some .mutU8
  | 1, true, _ => some .constU8
  | 2, false, false => some .mutPtrMutU8
  | 2, true, false => some .mutPtrConstU8
  | _, _, _ => none

/-- The reading before the card: the same, without the pointer to a pointer
to const. -/
def pointerBefore : Nat → Bool → Bool → Option Pointer
  | 1, false, _ => some .mutU8
  | 1, true, _ => some .constU8
  | 2, false, false => some .mutPtrMutU8
  | _, _, _ => none

theorem the_pointer_widening_only_adds (stars : Nat) (innerConst slot : Bool)
    (spelling : Pointer) (h : pointerBefore stars innerConst slot = some spelling) :
    pointer stars innerConst slot = some spelling := by
  unfold pointerBefore at h
  unfold pointer
  split at h <;> simp_all

theorem the_new_spelling_is_exactly_two_levels_with_a_const_innermost_pointee
    (stars : Nat) (innerConst slot : Bool) :
    pointer stars innerConst slot = some .mutPtrConstU8 ↔
      stars = 2 ∧ innerConst = true ∧ slot = false := by
  unfold pointer
  split <;> simp_all

theorem a_qualified_slot_refuses (innerConst : Bool) : pointer 2 innerConst true = none := by
  cases innerConst <;> rfl

theorem pointer_to_pointer_and_pointer_to_pointer_to_const_are_distinct :
    pointer 2 false false ≠ pointer 2 true false := by
  decide

/-- The loader's `resource_out` gate: exactly the pointer to a mutable pointer. -/
def outAdmitted (spelling : Pointer) : Bool := spelling == .mutPtrMutU8

theorem resource_out_never_admits_a_pointer_to_a_pointer_to_const :
    outAdmitted .mutPtrConstU8 = false := rfl

end QCore.AbiEnumConstPointer
