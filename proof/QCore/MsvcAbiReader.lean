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

import QCore.TypedefProjection

/-!
# MSVC default-C declarations in the opaque ABI reader

Two additions to `legacy_declared_abi.rs`: exactly one function-prefix
`__cdecl` preserves the already-read parameter/result types, and the closed
`__int64` / `signed __int64` / `unsigned __int64` family has the existing
64-bit scalar projection. `isize` / `usize` assume 64-bit pointers. Ordinary
`long` retains its LP64 premise; this is not a general LLP64 target model.

These are abstract word counts and convention categories. Compiler language
facts, faithful extraction and Rust correspondence are boundary-28 premises.
No theorem verifies the CRT implementation.
-/
namespace QCore.MsvcAbiReader
open QCore.DeclaredAbiAgreement
open QCore.TypedefProjection

structure IntegerWords where
  sized64 : Nat
  signedWords : Nat
  unsignedWords : Nat
  otherWords : Nat
  deriving DecidableEq, Repr

/-- No mixed arithmetic keywords or repeated/conflicting signs. -/
def read64 (w : IntegerWords) : Projection :=
  if w.sized64 = 1 ∧ w.otherWords = 0 ∧ w.signedWords + w.unsignedWords ≤ 1 then
    if w.unsignedWords = 1 then some [.u64, .usize] else some [.i64, .isize]
  else none

def plain64 : IntegerWords := ⟨1, 0, 0, 0⟩
def signed64 : IntegerWords := ⟨1, 1, 0, 0⟩
def unsigned64 : IntegerWords := ⟨1, 0, 1, 0⟩

theorem the_native_unsigned_size_reading :
    read64 unsigned64 = some [.u64, .usize] := by decide

theorem explicit_and_default_signed_readings :
    read64 plain64 = some [.i64, .isize] ∧
    read64 signed64 = some [.i64, .isize] := by decide

theorem accepted_words_are_exact (w : IntegerWords) (ts : List AbiType)
    (h : read64 w = some ts) :
    w.sized64 = 1 ∧ w.otherWords = 0 ∧ w.signedWords + w.unsignedWords ≤ 1 := by
  by_cases hv : w.sized64 = 1 ∧ w.otherWords = 0 ∧ w.signedWords + w.unsignedWords ≤ 1
  · exact hv
  · simp [read64, hv] at h

theorem a_missing_or_repeated_sized_keyword_refuses (w : IntegerWords)
    (h : w.sized64 ≠ 1) : read64 w = none := by simp [read64, h]

theorem mixed_arithmetic_words_refuse (w : IntegerWords)
    (h : w.otherWords ≠ 0) : read64 w = none := by simp [read64, h]

theorem repeated_or_conflicting_signs_refuse (w : IntegerWords)
    (h : ¬ w.signedWords + w.unsignedWords ≤ 1) : read64 w = none := by
  simp [read64, h]

theorem signedness_and_width_cannot_be_bought :
    admits (read64 signed64) .u64 = false ∧
    admits (read64 unsigned64) .i64 = false ∧
    admits (read64 unsigned64) .u32 = false := by decide

theorem no_sized_integer_reading_is_a_pointer (w : IntegerWords) :
    admits (read64 w) .ptrMut = false ∧
    admits (read64 w) .ptrConst = false ∧
    admits (read64 w) .ptrMutMut = false := by
  unfold read64
  split
  · split <;> decide
  · decide

theorem a_scalar_size_carries_no_owned_result :
    ownedResultAdmissible .u64 = false ∧
    ownedResultAdmissible .usize = false := by decide

theorem borrowed_is_the_only_mode_on_a_native_size (m : Mode) :
    modeAdmissible m .usize = true ↔ m = .borrowed := by
  cases m <;> decide

inductive Convention where
  | defaultC | cdecl | repeatedCdecl | unsupported
  deriving DecidableEq, Repr

/-- Result pointer levels, qualifiers and parameter readings are unchanged
for the default C convention. Unreadable prefixes cannot admit a contract. -/
def readConvention (c : Convention) (d : CDeclaration) : CDeclaration :=
  match c with
  | .defaultC | .cdecl => d
  | .repeatedCdecl | .unsupported => { d with result := none }

theorem cdecl_preserves_the_entire_declaration (d : CDeclaration) :
    readConvention .cdecl d = d := by rfl

theorem default_declarations_keep_their_comparison (s : Signature) (d : CDeclaration) :
    DeclaredAbiAgreement.compare s (readConvention .defaultC d) = DeclaredAbiAgreement.compare s d := by rfl

theorem cdecl_keeps_all_contract_disagreements (s : Signature) (d : CDeclaration) :
    DeclaredAbiAgreement.compare s (readConvention .cdecl d) = DeclaredAbiAgreement.compare s d := by rfl

theorem another_convention_refuses (s : Signature) (d : CDeclaration) :
    DeclaredAbiAgreement.compare s (readConvention .unsupported d) = some .unreadableDeclaration := by
  rfl

theorem repeated_cdecl_refuses (s : Signature) (d : CDeclaration) :
    DeclaredAbiAgreement.compare s (readConvention .repeatedCdecl d) = some .unreadableDeclaration := by
  rfl

/-- Applied independently to each result/parameter position. Ordinary long
must not gain the LP64 reading through a newly readable cdecl declaration. -/
def readType (cdecl ordinaryLong : Bool) (earlier : Projection) : Projection :=
  if cdecl && ordinaryLong then none else earlier

theorem earlier_type_readings_are_preserved (ambiguous : Bool) (p : Projection) :
    readType false ambiguous p = p := by cases ambiguous <;> rfl

theorem an_unambiguous_cdecl_type_preserves_its_reading (p : Projection) :
    readType true false p = p := by rfl

theorem a_target_dependent_long_result_refuses (s : Signature) (d : CDeclaration) :
    DeclaredAbiAgreement.compare s { readConvention .cdecl d with result := readType true true d.result }
      = some .unreadableDeclaration := by rfl

theorem a_target_dependent_long_parameter_refuses :
    DeclaredAbiAgreement.compare ⟨[.usize], .unit, false⟩
      ⟨some [.unit], [readType true true (some [.u64, .usize])], false, false⟩
      = some .unreadableDeclaration := by decide

def memcmpContract : Signature := ⟨[.ptrConst, .ptrConst, .usize], .i32, false⟩
def memcmpDeclaration : CDeclaration :=
  ⟨some [.i32], [some [.ptrConst], some [.ptrConst], read64 unsigned64], false, false⟩

theorem the_native_memcmp_signature_agrees :
    DeclaredAbiAgreement.compare memcmpContract (readConvention .cdecl memcmpDeclaration) = none := by decide

theorem the_same_native_declaration_refuses_a_forged_width :
    DeclaredAbiAgreement.compare { memcmpContract with parameters := [.ptrConst, .ptrConst, .u32] }
      (readConvention .cdecl memcmpDeclaration) = some .parameterType := by decide

theorem the_same_native_declaration_refuses_a_forged_qualifier :
    DeclaredAbiAgreement.compare { memcmpContract with parameters := [.ptrMut, .ptrConst, .usize] }
      (readConvention .cdecl memcmpDeclaration) = some .parameterType := by decide

theorem a_cdecl_pointer_result_cannot_be_hidden_as_unit :
    DeclaredAbiAgreement.compare ⟨[.usize], .unit, false⟩
      (readConvention .cdecl ⟨some [.ptrMut], [read64 unsigned64], false, false⟩)
      = some .returnType := by decide

end QCore.MsvcAbiReader
