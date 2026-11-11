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
import QCore.ScalarTypedErrors

/-! The fixed-width scalar table at the positions card `W48-QR-SCALAR-TYPES`
left at `i32` (card `W48-QR-SCALAR-TYPES-R3`).

Item 1 is the record field of `source.buffer.v1` and that route's value
environment, which the card makes **typed**: every name denotes a value of one
§G02t type (`TVal`), a construction takes at each field a value of exactly the
field's type, a field read binds a value of that type, every position that
takes an `i32` refuses a value of any other type, and the one explicit
conversion is `QCore.ScalarTypes.convert`. Every value on that route is a
check-time constant, so each refusal below is decided before emission; there is
no dynamic path on it to agree with.

What is proved:

* a field literal is admitted exactly when it is a value of the field's type
  (`fieldValue_lit_iff`), `true`/`false` exactly at a `bool` field, and a name
  exactly when its type **is** the field's — there is no implicit conversion
  (`fieldValue_name_same_type`);
* a construction of well-typed operands is a list of values each in its field's
  range (`construct_well_typed`), so every field read binds a well-typed value
  (`project_well_typed`);
* an `i32` position takes a value only of type `i32` (`i32Operand_type`,
  `i32Operand_refuses_other_types`);
* the conversion statement preserves the number and yields a well-typed value
  (`convertStmt_well_typed`), and succeeds exactly when the number fits the
  target (`convertStmt_succeeds_iff`); without a recorded target no field or
  conversion of `isize`/`usize` has a value (`no_target_no_field`).

The Rust checker is bound to these definitions by the vector table
`fieldAgreement`, which the Rust test `lean_field_vectors_match_the_checker`
parses and replays through `quirin`'s checker; the emitted C is compared with
the reference by native execution. Neither is an extraction or refinement
theorem. -/
namespace QCore.ScalarPositions

open QCore.ScalarTypes
open QCore.ScalarTypedErrors

/-- A value of the typed environment: its type and its number (`bool` is
`0`/`1`). -/
structure TVal where
  ty : Ty
  n : Int
  deriving DecidableEq, Repr

/-- A well-typed value: its number is a value of its type. -/
def TVal.ok (t : Option Width) (v : TVal) : Bool := fits t v.ty v.n

/-- What a construction writes at one field. -/
inductive FieldOp where
  | lit (n : Int)
  | bool (b : Bool)
  | name (v : TVal)

/-- The number a construction supplies at a field declared `ty`, or `none` —
the refusal. -/
def fieldValue (t : Option Width) (ty : Ty) : FieldOp → Option Int
  | .lit n => if ty ≠ .bool ∧ fits t ty n = true then some n else none
  | .bool b => if ty = .bool then some (if b then 1 else 0) else none
  | .name v => if v.ty = ty then some v.n else none

/-- **A field literal is admitted exactly when it is a value of the field's
type** (never `bool`, whose literals are `true`/`false`). -/
theorem fieldValue_lit_iff {t : Option Width} {ty : Ty} {n : Int} :
    fieldValue t ty (.lit n) = some n ↔ ty ≠ .bool ∧ fits t ty n = true := by
  simp only [fieldValue]
  constructor
  · intro h
    split at h
    · assumption
    · contradiction
  · intro h
    simp [h]

theorem fieldValue_lit_value {t : Option Width} {ty : Ty} {n m : Int}
    (h : fieldValue t ty (.lit n) = some m) : m = n := by
  simp only [fieldValue] at h
  split at h
  · cases h; rfl
  · contradiction

/-- **There is no implicit conversion at a field**: a name is admitted only
when its type is the field's, and then its number is the field's. -/
theorem fieldValue_name_same_type {t : Option Width} {ty : Ty} {v : TVal} {m : Int}
    (h : fieldValue t ty (.name v) = some m) : v.ty = ty ∧ m = v.n := by
  simp only [fieldValue] at h
  split at h
  · rename_i hty
    cases h
    exact ⟨hty, rfl⟩
  · contradiction

theorem fieldValue_bool_iff {t : Option Width} {ty : Ty} {b : Bool} :
    (∃ m, fieldValue t ty (.bool b) = some m) ↔ ty = .bool := by
  simp only [fieldValue]
  constructor
  · intro ⟨m, h⟩
    split at h
    · assumption
    · contradiction
  · intro h
    simp [h]

theorem bool_values_fit (t : Option Width) (b : Bool) :
    fits t .bool (if b then 1 else 0) = true := by
  cases b <;> simp [fits, Ty.range]

/-- An operand is well-typed when a name it carries is. -/
def FieldOp.ok (t : Option Width) : FieldOp → Bool
  | .name v => v.ok t
  | _ => true

/-- Every admitted field value is a value of the field's type. -/
theorem fieldValue_in_range {t : Option Width} {ty : Ty} {op : FieldOp} {m : Int}
    (hop : op.ok t = true) (h : fieldValue t ty op = some m) : fits t ty m = true := by
  cases op with
  | lit n =>
    have hv := fieldValue_lit_value h
    subst hv
    exact ((fieldValue_lit_iff).mp h).2
  | bool b =>
    simp only [fieldValue] at h
    split at h
    · rename_i hb
      cases h
      subst hb
      exact bool_values_fit t b
    · contradiction
  | name v =>
    obtain ⟨hty, hm⟩ := fieldValue_name_same_type h
    subst hty
    subst hm
    exact hop

/-- A construction: the declared field types in order, one operand each. -/
def construct (t : Option Width) : List Ty → List FieldOp → Option (List Int)
  | [], [] => some []
  | ty :: tys, op :: ops =>
    (fieldValue t ty op).bind fun v => (construct t tys ops).map fun vs => v :: vs
  | _, _ => none

/-- Each value is a value of its field's type. -/
def allFit (t : Option Width) : List Ty → List Int → Prop
  | [], [] => True
  | ty :: tys, v :: vs => fits t ty v = true ∧ allFit t tys vs
  | _, _ => False

def opsOk (t : Option Width) : List FieldOp → Prop
  | [] => True
  | op :: ops => op.ok t = true ∧ opsOk t ops

/-- **A construction of well-typed operands is well typed**, field by field. -/
theorem construct_well_typed {t : Option Width} :
    ∀ {tys : List Ty} {ops : List FieldOp} {vs : List Int},
      opsOk t ops → construct t tys ops = some vs → allFit t tys vs
  | [], [], vs, _, h => by
    simp [construct] at h
    subst h
    simp [allFit]
  | [], _ :: _, _, _, h => by simp [construct] at h
  | _ :: _, [], _, _, h => by simp [construct] at h
  | ty :: tys, op :: ops, vs, hops, h => by
    simp only [construct] at h
    cases hv : fieldValue t ty op with
    | none => simp [hv] at h
    | some v =>
      cases hr : construct t tys ops with
      | none => simp [hv, hr] at h
      | some rest =>
        simp [hv, hr] at h
        subst h
        exact ⟨fieldValue_in_range hops.1 hv, construct_well_typed hops.2 hr⟩

/-- A field read: the declared type and the constructed number at a position. -/
def project : List Ty → List Int → Nat → Option TVal
  | ty :: _, v :: _, 0 => some ⟨ty, v⟩
  | _ :: tys, _ :: vs, i + 1 => project tys vs i
  | _, _, _ => none

/-- **A field read binds a well-typed value** of exactly the declared type. -/
theorem project_well_typed {t : Option Width} :
    ∀ {tys : List Ty} {vs : List Int} {i : Nat} {r : TVal},
      allFit t tys vs → project tys vs i = some r → r.ok t = true
  | ty :: _, v :: _, 0, r, hfit, h => by
    simp [project] at h
    subst h
    exact hfit.1
  | _ :: tys, _ :: vs, i + 1, r, hfit, h => by
    simp only [project] at h
    exact project_well_typed hfit.2 h
  | [], _, _, _, _, h => by simp [project] at h
  | _ :: _, [], _, _, _, h => by simp [project] at h

/-- A position that takes an `i32`: an index, a stored byte, a lend bound. -/
def i32Operand (v : TVal) : Option Int := if v.ty = .i32 then some v.n else none

theorem i32Operand_type {v : TVal} {n : Int} (h : i32Operand v = some n) :
    v.ty = .i32 ∧ n = v.n := by
  unfold i32Operand at h
  split at h
  · rename_i hty
    cases h
    exact ⟨hty, rfl⟩
  · contradiction

/-- **An `i32` position refuses a value of any other type** — never widened,
never narrowed. -/
theorem i32Operand_refuses_other_types {v : TVal} (h : v.ty ≠ .i32) : i32Operand v = none := by
  simp [i32Operand, h]

/-- The conversion statement `let y = x as to;`. -/
def convertStmt (t : Option Width) (to : Ty) (v : TVal) : Option TVal :=
  (convert t v.ty to v.n).map fun n => ⟨to, n⟩

/-- **A conversion yields a well-typed value of the target type and the same
number.** -/
theorem convertStmt_well_typed {t : Option Width} {to : Ty} {v r : TVal}
    (h : convertStmt t to v = some r) : r.ty = to ∧ r.n = v.n ∧ r.ok t = true := by
  unfold convertStmt at h
  cases hc : convert t v.ty to v.n with
  | none => simp [hc] at h
  | some n =>
    simp [hc] at h
    subst h
    obtain ⟨hn, hfit⟩ := convert_value hc
    exact ⟨rfl, hn, hfit⟩

/-- **A conversion of a well-typed integer succeeds exactly when the number is
a value of the target**; otherwise the route refuses it at check time. -/
theorem convertStmt_succeeds_iff {t : Option Width} {to : Ty} {v : TVal}
    (hs : v.ty ≠ .bool) (hd : to ≠ .bool) (hv : v.ok t = true) :
    (∃ r, convertStmt t to v = some r) ↔ fits t to v.n = true := by
  unfold convertStmt
  constructor
  · intro ⟨r, h⟩
    cases hc : convert t v.ty to v.n with
    | none => simp [hc] at h
    | some n =>
      obtain ⟨hn, hfit⟩ := convert_value hc
      rw [hn] at hfit
      exact hfit
  · intro hfit
    have := (convert_succeeds_iff hs hd hv).mpr hfit
    exact ⟨⟨to, v.n⟩, by simp [this]⟩

/-- Without a recorded target, a field of `isize`/`usize` has no value at all. -/
theorem no_target_no_field (op : FieldOp) (hop : ∀ v, op = .name v → v.ok none = true) :
    fieldValue none .usize op = none ∧ fieldValue none .isize op = none := by
  cases op with
  | lit n => simp [fieldValue, fits, Ty.range]
  | bool b => simp [fieldValue]
  | name v =>
    have hv := hop v rfl
    constructor
    · simp only [fieldValue]
      split
      · rename_i hty
        simp [TVal.ok, hty, fits, Ty.range] at hv
      · rfl
    · simp only [fieldValue]
      split
      · rename_i hty
        simp [TVal.ok, hty, fits, Ty.range] at hv
      · rfl

/-- The field-literal vectors: `(type, literal, admitted)` at the 64-bit target,
replayed through the Rust checker by `lean_field_vectors_match_the_checker`. -/
def fieldAgreement : List (Ty × Int × Bool) := [
  (.u8, 255, true), (.u8, 256, false), (.u8, 0, true), (.u8, (-1), false),
  (.i8, 127, true), (.i8, 128, false), (.i8, (-128), true), (.i8, (-129), false),
  (.i16, 32767, true), (.i16, 32768, false), (.u16, 65535, true), (.u16, 65536, false),
  (.i32, 2147483647, true), (.i32, 2147483648, false), (.i32, (-2147483648), true),
  (.u32, 4294967295, true), (.u32, 4294967296, false),
  (.i64, 9223372036854775807, true), (.i64, 9223372036854775808, false),
  (.i64, (-9223372036854775808), true), (.i64, (-9223372036854775809), false),
  (.u64, 18446744073709551615, true), (.u64, 18446744073709551616, false),
  (.usize, 18446744073709551615, true), (.usize, (-1), false),
  (.isize, (-9223372036854775808), true), (.isize, 9223372036854775808, false),
  (.bool, 1, false), (.bool, 0, false)]

theorem fieldAgreement_holds :
    fieldAgreement.all (fun (ty, n, b) => (fieldValue (some .w64) ty (.lit n)).isSome == b) = true := by
  decide

/-! Item 2: an extern result is a value (`W48-QR-TYPES-EXTERN-VALUES`). A value
of the extern route's namespace is **known** — a literal, a declared
`succeeds INTEGER` outcome, or a statement over known values — or known only at
run time. A statement over known operands is decided at check time exactly as
`QCore.ScalarTypes.checked` decides it, and refused when it faults; a statement
over a run-time operand is admitted only under the row's
`throws<ArithmeticError>`, and then runs the typed path of
`QCore.ScalarTypedErrors`. -/

/-- What the checker knows of a value's number. -/
inductive Known where
  | known (n : Int)
  | dynamic
  deriving DecidableEq, Repr

/-- The check-time decision for `a OP b` at `ty` under the row's permission
`perm`: the known result, the dynamic result (marked as able to raise), or
`none` — the refusal. -/
def decideBinary (t : Option Width) (perm : Bool) (op : Op) (ty : Ty) :
    Known → Known → Option (Known × Bool)
  | .known a, .known b => (checked t op ty a b).map fun v => (.known v, false)
  | _, _ => if perm = true ∧ ty ≠ .bool then some (.dynamic, true) else none

/-- **A statement over known operands is admitted exactly when the checked
operation succeeds, with its value**, and it never raises. -/
theorem decideBinary_known_iff {t : Option Width} {perm : Bool} {op : Op} {ty : Ty}
    {a b v : Int} :
    decideBinary t perm op ty (.known a) (.known b) = some (.known v, false) ↔
      checked t op ty a b = some v := by
  simp only [decideBinary]
  cases h : checked t op ty a b <;> simp

/-- **The static refusal is the dynamic raise**: a statement over known
operands that the checker refuses is one whose typed path raises, and raises
the classified variant. -/
theorem decideBinary_refusal_is_raise {t : Option Width} {perm : Bool} {op : Op} {ty : Ty}
    {a b : Int} (h : decideBinary t perm op ty (.known a) (.known b) = none) :
    checkedE t op ty a b = .error (classify op b) := by
  simp only [decideBinary] at h
  cases hc : checked t op ty a b with
  | none => simp [checkedE, hc]
  | some v => simp [hc] at h

/-- **A statement that can raise is admitted only under the permission**, and
only over a run-time operand. -/
theorem decideBinary_raises_needs_permission {t : Option Width} {perm : Bool} {op : Op}
    {ty : Ty} {x y : Known} {r : Known} (h : decideBinary t perm op ty x y = some (r, true)) :
    perm = true ∧ (x = .dynamic ∨ y = .dynamic) := by
  cases x with
  | known a =>
    cases y with
    | known b =>
      simp only [decideBinary] at h
      cases hc : checked t op ty a b <;> simp [hc] at h
    | dynamic =>
      simp only [decideBinary] at h
      split at h
      · rename_i hp
        exact ⟨hp.1, .inr rfl⟩
      · contradiction
  | dynamic =>
    simp only [decideBinary] at h
    split at h
    · rename_i hp
      exact ⟨hp.1, .inl rfl⟩
    · contradiction

/-- Without the permission, nothing over a run-time operand is admitted. -/
theorem decideBinary_without_permission {t : Option Width} {op : Op} {ty : Ty} {y : Known} :
    decideBinary t false op ty .dynamic y = none := by
  cases y <;> simp [decideBinary]

/-- A known result is a value of its type. -/
theorem decideBinary_known_in_range {t : Option Width} {perm : Bool} {op : Op} {ty : Ty}
    {a b v : Int} (h : decideBinary t perm op ty (.known a) (.known b) = some (.known v, false)) :
    fits t ty v = true :=
  checked_in_range (decideBinary_known_iff.mp h)

/-- A declared outcome is admitted exactly when it is a value of the declared
result's type (with the recorded target's width for `isize`/`usize`). -/
def outcomeAdmitted (t : Option Width) (ty : Ty) (n : Int) : Bool := fits t ty n

/-- An argument is admitted at a declared parameter only at the parameter's own
type: there is no implicit conversion at the extern boundary either. -/
def argumentAdmitted (declared : Ty) (v : TVal) : Bool := decide (v.ty = declared)

theorem argumentAdmitted_type {declared : Ty} {v : TVal} (h : argumentAdmitted declared v = true) :
    v.ty = declared := by
  simpa [argumentAdmitted] using h

/-- A comparison produces a `bool` and never raises; `bool` has no ordering. -/
def decideCompare (cmp : Cmp) (ty : Ty) : Known → Known → Option Known
  | .known a, .known b => (compare cmp ty a b).map fun r => .known (if r then 1 else 0)
  | _, _ => if ty = .bool ∧ cmp ≠ .eq ∧ cmp ≠ .ne then none else some .dynamic

theorem decideCompare_bool_is_bool {cmp : Cmp} {ty : Ty} {a b n : Int}
    (h : decideCompare cmp ty (.known a) (.known b) = some (.known n)) :
    fits none .bool n = true := by
  simp only [decideCompare] at h
  cases hc : compare cmp ty a b with
  | none => simp [hc] at h
  | some r =>
    simp [hc] at h
    subst h
    cases r <;> simp [fits, Ty.range]

/-- The extern-value vectors: `(operation, type, a, b, admitted known value)`
at the 64-bit target, the known path; replayed through the Rust checker by
`lean_extern_value_vectors_match_the_checker`. -/
def externAgreement : List (Op × Ty × Int × Int × Option Int) := [
  (.add, .u8, 200, 55, some 255), (.add, .u8, 200, 56, none), (.sub, .u16, 0, 1, none),
  (.mul, .i64, 3037000499, 3037000499, some 9223372030926249001), (.mul, .i64, 3037000500, 3037000500, none),
  (.div, .i32, -2147483648, -1, none), (.rem, .u32, 7, 0, none), (.div, .usize, 10, 3, some 3),
  (.add, .u64, 18446744073709551614, 1, some 18446744073709551615), (.add, .isize, 9223372036854775807, 1, none)]

theorem externAgreement_holds :
    externAgreement.all (fun (op, ty, a, b, v) =>
      (decideBinary (some .w64) false op ty (.known a) (.known b)).map Prod.fst ==
        v.map Known.known) = true := by
  decide

end QCore.ScalarPositions
