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

import QCore.ScalarPositions
import QCore.ScalarLoopBounds

/-! Checked arithmetic on the byte-buffer route (card `W48-QR-SCALAR-TYPES-R5`,
`W48-QR-BUFFER-ROUTE-ARITHMETIC`).

`source.buffer.v1` gains the value statement the extern and task routes share —
one arithmetic operation, comparison or negation — decided by the same
decision (`QCore.ScalarPositions.decideBinary`, the Lean statement of
`scalar_values::Decider`). What is particular to this route is that **every
operand is known at check time**: a literal, a value the entry bound, a field
read, a conversion, and — inside a `for` body — the loop index, which the
checker binds to each iteration's number in turn. So the decision is always
taken on known operands, with no permission to raise, and:

* a statement is admitted exactly when the checked operation succeeds, with its
  exact value inside the type (`bufferBinary_iff_checked`,
  `bufferBinary_in_range`, `bufferBinary_exact`);
* no admitted statement can raise (`buffer_statement_never_raises`), so the
  route needs no error edge and no `throws<ArithmeticError>`;
* a refused statement is exactly one whose typed path would raise the
  classified variant (`buffer_refusal_is_the_raise`) — the refusal names it;
* inside a loop, the checker's unrolled decision admits a statement over the
  index exactly when it succeeds at **every** index value
  (`unrolled_admits_iff`), and then the emitted checked helper computes the
  exact result at every iteration (`admitted_body_statement_never_faults`), so
  its `exit(86)` witness is never taken;
* negation is `0 - a` (`negate_is_zero_minus`), on an unsigned type admitted
  only at `0`.

The binding to Rust is the vector table `bufferAgreement`, replayed through
whole source programs by `lean_buffer_value_vectors_match_the_checker`, and the
native boundary tables; neither is an extraction theorem. -/
namespace QCore.ScalarBufferValues

open QCore.ScalarTypes
open QCore.ScalarTypedErrors
open QCore.ScalarPositions

/-- The route's decision of `a OP b` at `ty`: operands known, no permission. -/
def bufferBinary (t : Option Width) (op : Op) (ty : Ty) (a b : Int) : Option Int :=
  match decideBinary t false op ty (.known a) (.known b) with
  | some (.known v, _) => some v
  | _ => none

theorem bufferBinary_iff_checked {t : Option Width} {op : Op} {ty : Ty} {a b v : Int} :
    bufferBinary t op ty a b = some v ↔ checked t op ty a b = some v := by
  unfold bufferBinary decideBinary
  cases h : checked t op ty a b <;> simp [h]

/-- **Nothing on this route raises.** Every statement's operands are known, so
the decision never returns the raising (dynamic) arm. -/
theorem buffer_statement_never_raises {t : Option Width} {op : Op} {ty : Ty} {a b : Int}
    {r : Known} {raises : Bool}
    (h : decideBinary t false op ty (.known a) (.known b) = some (r, raises)) :
    raises = false ∧ ∃ v, r = .known v := by
  simp only [decideBinary] at h
  cases hc : checked t op ty a b with
  | none => simp [hc] at h
  | some v =>
    simp [hc] at h
    exact ⟨h.2, v, h.1.symm⟩

theorem bufferBinary_in_range {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : bufferBinary t op ty a b = some v) : fits t ty v = true :=
  checked_in_range (bufferBinary_iff_checked.mp h)

theorem bufferBinary_exact {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : bufferBinary t op ty a b = some v) : v = exact op a b :=
  checked_exact (bufferBinary_iff_checked.mp h)

/-- **The refusal is the raise.** A statement the route refuses is one whose
typed path raises, with the classified variant the refusal names. -/
theorem buffer_refusal_is_the_raise {t : Option Width} {op : Op} {ty : Ty} {a b : Int}
    (h : bufferBinary t op ty a b = none) : checkedE t op ty a b = .error (classify op b) := by
  apply decideBinary_refusal_is_raise (perm := false)
  unfold bufferBinary at h
  cases hd : decideBinary t false op ty (.known a) (.known b) with
  | none => rfl
  | some p =>
    obtain ⟨r, raises⟩ := p
    obtain ⟨_, v, rfl⟩ := buffer_statement_never_raises hd
    simp [hd] at h

/-! ## Negation -/

/-- `-a` is `0 - a`, as §G02t defines it. -/
def bufferNegate (t : Option Width) (ty : Ty) (a : Int) : Option Int :=
  bufferBinary t .sub ty 0 a

theorem negate_is_zero_minus {t : Option Width} {ty : Ty} {a v : Int} :
    bufferNegate t ty a = some v ↔ checked t .sub ty 0 a = some v :=
  bufferBinary_iff_checked

/-- On an unsigned type only `0` negates. -/
theorem unsigned_negation_only_at_zero (t : Option Width) :
    bufferNegate t .u8 0 = some 0 ∧ bufferNegate t .u8 1 = none := by
  constructor <;> simp [bufferNegate, bufferBinary, decideBinary, checked, Ty.range, fits, raw,
    exact, narrow]

/-! ## Inside a loop: decided at every index value -/

/-- The checker's unrolled decision of `i OP b` in a body counting `c`
iterations: the statement is decided at each index value in turn. -/
def unrolledAdmits (t : Option Width) (op : Op) (ty : Ty) (c : Nat) (b : Int) : Bool :=
  (List.range c).all fun i => (bufferBinary t op ty i b).isSome

/-- **The unrolling decides every iteration.** The statement is admitted
exactly when it succeeds at every index value the loop takes. -/
theorem unrolled_admits_iff {t : Option Width} {op : Op} {ty : Ty} {c : Nat} {b : Int} :
    unrolledAdmits t op ty c b = true ↔ ∀ i, i < c → ∃ v, checked t op ty i b = some v := by
  unfold unrolledAdmits
  simp only [List.all_eq_true, List.mem_range, Option.isSome_iff_exists]
  constructor
  · intro h i hi
    obtain ⟨v, hv⟩ := h i hi
    exact ⟨v, bufferBinary_iff_checked.mp hv⟩
  · intro h i hi
    obtain ⟨v, hv⟩ := h i hi
    exact ⟨v, bufferBinary_iff_checked.mpr hv⟩

/-- **An admitted body statement never faults at run time**: at every index
value the operation the emitted helper computes succeeds with the exact,
in-range result, so the helper's fault witness is never taken. -/
theorem admitted_body_statement_never_faults {t : Option Width} {op : Op} {ty : Ty} {c : Nat}
    {b : Int} (h : unrolledAdmits t op ty c b = true) (i : Nat) (hi : i < c) :
    ∃ v, checked t op ty i b = some v ∧ v = exact op i b ∧ fits t ty v = true := by
  obtain ⟨v, hv⟩ := unrolled_admits_iff.mp h i hi
  exact ⟨v, hv, checked_exact hv, checked_in_range hv⟩

/-- A statement that faults at one index value refuses the whole loop: the
non-vacuous converse, at `u8` over `0..200` with `i + 100`. -/
theorem one_faulting_iteration_refuses_the_loop :
    unrolledAdmits (some .w64) .add .u8 200 100 = false ∧
      unrolledAdmits (some .w64) .add .u8 156 100 = true := by
  constructor <;> decide

/-! ## Vectors -/

/-- `(operation, type, a, b, admitted value)` at the 64-bit target, replayed
through whole `source.buffer.v1` programs by the Rust test
`lean_buffer_value_vectors_match_the_checker`. -/
def bufferAgreement : List (Op × Ty × Int × Int × Option Int) := [
  (.add, .u8, 200, 55, some 255), (.add, .u8, 200, 56, none), (.sub, .u8, 0, 1, none),
  (.mul, .i16, 181, 181, some 32761), (.mul, .i16, 182, 182, none),
  (.div, .i8, -128, -1, none), (.rem, .i8, -128, -1, none), (.rem, .i8, -7, 2, some (-1)),
  (.div, .i32, -7, 2, some (-3)), (.rem, .u32, 7, 0, none), (.div, .u16, 65535, 256, some 255),
  (.sub, .i64, -9223372036854775807, 1, some (-9223372036854775808)),
  (.sub, .i64, -9223372036854775808, 1, none),
  (.add, .u64, 18446744073709551614, 1, some 18446744073709551615),
  (.add, .u64, 18446744073709551615, 1, none),
  (.mul, .isize, 3037000499, 3037000499, some 9223372030926249001),
  (.div, .usize, 10, 3, some 3)]

theorem bufferAgreement_holds :
    bufferAgreement.all (fun (op, ty, a, b, v) => bufferBinary (some .w64) op ty a b == v) = true := by
  decide

end QCore.ScalarBufferValues
