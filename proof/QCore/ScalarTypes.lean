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

/-! Fixed-width scalar types of `.qr` sources (card `W48-QR-SCALAR-TYPES`).

The Lean statement of `crates/qcore-core/src/scalar_types.rs`: the §G05
`scalar_abi` table (`bool`, `i8` … `i64`, `isize`, `u8` … `u64`, `usize`), each
type's value range, the five checked arithmetic operations, the one explicit
conversion, and the comparisons producing `bool`. `isize`/`usize` have a range
only under a recorded target width; without one every operation on them fails,
so no width is ever guessed.

What is proved here is the meaning of the operations, not the frontend or the
emitted C. The Rust transcription is bound to these definitions by the
boundary-vector table `agreement` below, which the Rust test
`lean_scalar_type_vectors_match_the_rust_kernel` parses and replays against
`qcore_core::scalar_types`; the emitted C is compared with the Rust reference by
native execution. Neither is an extraction or refinement theorem. -/
namespace QCore.ScalarTypes

inductive Ty where
  | bool | i8 | i16 | i32 | i64 | isize | u8 | u16 | u32 | u64 | usize
  deriving DecidableEq, Repr

/-- The pointer width a builder's recorded target states. -/
inductive Width where
  | w32 | w64
  deriving DecidableEq, Repr

def Ty.signed : Ty → Bool
  | .i8 | .i16 | .i32 | .i64 | .isize => true
  | _ => false

def signedRange : Width → Int × Int
  | .w32 => (-2147483648, 2147483647)
  | .w64 => (-9223372036854775808, 9223372036854775807)

def unsignedRange : Width → Int × Int
  | .w32 => (0, 4294967295)
  | .w64 => (0, 18446744073709551615)

/-- The inclusive range of each type; `none` for `isize`/`usize` without a
recorded target. `bool` is `0`/`1`. -/
def Ty.range (t : Option Width) : Ty → Option (Int × Int)
  | .bool => some (0, 1)
  | .i8 => some (-128, 127)
  | .i16 => some (-32768, 32767)
  | .i32 => some (-2147483648, 2147483647)
  | .i64 => some (-9223372036854775808, 9223372036854775807)
  | .isize => t.map signedRange
  | .u8 => some (0, 255)
  | .u16 => some (0, 65535)
  | .u32 => some (0, 4294967295)
  | .u64 => some (0, 18446744073709551615)
  | .usize => t.map unsignedRange

/-- Whether `n` is a value of `ty` under the target `t`. -/
def fits (t : Option Width) (ty : Ty) (n : Int) : Bool :=
  match ty.range t with
  | some (lo, hi) => decide (lo ≤ n) && decide (n ≤ hi)
  | none => false

theorem fits_bounds {t : Option Width} {ty : Ty} {n : Int} (h : fits t ty n = true) :
    ∃ lo hi, ty.range t = some (lo, hi) ∧ lo ≤ n ∧ n ≤ hi := by
  unfold fits at h
  split at h
  · rename_i lo hi hr
    simp only [Bool.and_eq_true, decide_eq_true_eq] at h
    exact ⟨lo, hi, hr, h.1, h.2⟩
  · contradiction

theorem fits_of_bounds {t : Option Width} {ty : Ty} {n lo hi : Int}
    (hr : ty.range t = some (lo, hi)) (hl : lo ≤ n) (hh : n ≤ hi) : fits t ty n = true := by
  unfold fits
  rw [hr]
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  exact ⟨hl, hh⟩

/-- Without a recorded target, `isize` and `usize` hold no value at all. -/
theorem no_target_no_value (n : Int) : fits none .isize n = false ∧ fits none .usize n = false := by
  simp [fits, Ty.range]

inductive Op where
  | add | sub | mul | div | rem
  deriving DecidableEq, Repr

/-- The mathematical result each operation must reproduce when it succeeds.
Division and remainder truncate toward zero, as C11 and Rust's `checked_div`. -/
def exact : Op → Int → Int → Int
  | .add, a, b => a + b
  | .sub, a, b => a - b
  | .mul, a, b => a * b
  | .div, a, b => a.tdiv b
  | .rem, a, b => a.tmod b

/-- The unchecked result, refusing only the operand pairs that have none:
a zero divisor, and `MIN / -1`, `MIN % -1` of a signed type. -/
def raw (signed : Bool) (lo : Int) (op : Op) (a b : Int) : Option Int :=
  match op with
  | .div | .rem =>
    if b = 0 ∨ (signed = true ∧ a = lo ∧ b = -1) then none else some (exact op a b)
  | op => some (exact op a b)

def narrow (t : Option Width) (ty : Ty) (n : Int) : Option Int :=
  if fits t ty n = true then some n else none

/-- Checked arithmetic at `ty`: `none` is the typed fault. -/
def checked (t : Option Width) (op : Op) (ty : Ty) (a b : Int) : Option Int :=
  if ty = .bool then none
  else
    match ty.range t with
    | none => none
    | some (lo, _) =>
      if fits t ty a = true ∧ fits t ty b = true then
        (raw ty.signed lo op a b).bind (narrow t ty)
      else none

theorem narrow_sound {t : Option Width} {ty : Ty} {n v : Int} (h : narrow t ty n = some v) :
    fits t ty v = true ∧ v = n := by
  unfold narrow at h
  split at h
  · rename_i hn
    cases h
    exact ⟨hn, rfl⟩
  · contradiction

theorem bind_narrow {t : Option Width} {ty : Ty} {o : Option Int} {v : Int}
    (h : o.bind (narrow t ty) = some v) : ∃ r, o = some r ∧ fits t ty v = true ∧ v = r := by
  cases o with
  | none => contradiction
  | some r => exact ⟨r, rfl, narrow_sound h⟩

theorem raw_exact {s : Bool} {lo : Int} {op : Op} {a b r : Int} (h : raw s lo op a b = some r) :
    r = exact op a b := by
  cases op with
  | div | rem =>
    simp only [raw] at h
    split at h
    · contradiction
    · cases h; rfl
  | add | sub | mul =>
    simp only [raw] at h
    cases h; rfl

/-- The unwrapped success of a checked operation: its operands were values
of the type, and the result is the raw one, narrowed. -/
theorem checked_success {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checked t op ty a b = some v) :
    ∃ lo hi r, ty ≠ .bool ∧ ty.range t = some (lo, hi) ∧ fits t ty a = true ∧
      fits t ty b = true ∧ raw ty.signed lo op a b = some r ∧ fits t ty v = true ∧ v = r := by
  unfold checked at h
  split at h
  · contradiction
  · rename_i hb
    split at h
    · contradiction
    · rename_i lo hi hr
      split at h
      · rename_i hab
        obtain ⟨r, hraw, hv, hvr⟩ := bind_narrow h
        exact ⟨lo, hi, r, hb, hr, hab.1, hab.2, hraw, hv, hvr⟩
      · contradiction

/-- **Checked arithmetic never returns a value outside its type's range.** -/
theorem checked_in_range {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checked t op ty a b = some v) : fits t ty v = true := by
  obtain ⟨_, _, _, _, _, _, _, _, hv, _⟩ := checked_success h
  exact hv

/-- **A checked operation that succeeds returns the mathematically exact
result.** -/
theorem checked_exact {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checked t op ty a b = some v) : v = exact op a b := by
  obtain ⟨_, _, r, _, _, _, _, hraw, _, hvr⟩ := checked_success h
  rw [hvr]
  exact raw_exact hraw

/-- `bool` is not an integer: it has no arithmetic. -/
theorem bool_has_no_arithmetic (t : Option Width) (op : Op) (a b : Int) :
    checked t op .bool a b = none := by
  simp [checked]

/-- Without a recorded target, arithmetic on `isize`/`usize` always faults:
the width is never guessed. -/
theorem no_target_no_arithmetic (op : Op) (a b : Int) :
    checked none op .isize a b = none ∧ checked none op .usize a b = none := by
  simp [checked, Ty.range]

theorem division_by_zero_faults (t : Option Width) (op : Op) (ty : Ty) (a : Int)
    (hop : op = .div ∨ op = .rem) : checked t op ty a 0 = none := by
  unfold checked
  split
  · rfl
  · split
    · rfl
    · split
      · rcases hop with rfl | rfl <;> simp [raw]
      · rfl

theorem min_div_minus_one_faults (t : Option Width) (op : Op) (ty : Ty) (lo hi : Int)
    (hop : op = .div ∨ op = .rem) (hs : ty.signed = true) (hr : ty.range t = some (lo, hi)) :
    checked t op ty lo (-1) = none := by
  unfold checked
  split
  · rfl
  · rw [hr]
    simp only
    split
    · rcases hop with rfl | rfl <;> simp [raw, hs]
    · rfl

/-- The one conversion `x as dst`: the same number at the new type, or the
fault. `bool` converts to nothing and nothing converts to it. -/
def convert (t : Option Width) (src dst : Ty) (v : Int) : Option Int :=
  if src = .bool ∨ dst = .bool then none
  else if fits t src v = true ∧ fits t dst v = true then some v else none

/-- A conversion is total when every value of `src` is a value of `dst`. -/
def conversionTotal (t : Option Width) (src dst : Ty) : Bool :=
  match src.range t, dst.range t with
  | some (l1, h1), some (l2, h2) =>
    decide (src ≠ .bool) && decide (dst ≠ .bool) && decide (l2 ≤ l1) && decide (h1 ≤ h2)
  | _, _ => false

/-- **A total conversion is value-preserving**: it succeeds on every value of
its source type and returns that value. -/
theorem total_conversion_preserves {t : Option Width} {src dst : Ty} {v : Int}
    (ht : conversionTotal t src dst = true) (hv : fits t src v = true) :
    convert t src dst v = some v := by
  obtain ⟨lo, hi, hr, hl, hh⟩ := fits_bounds hv
  unfold conversionTotal at ht
  rw [hr] at ht
  split at ht
  · rename_i l1 h1 l2 h2 hs hd
    cases hs
    simp only [Bool.and_eq_true, decide_eq_true_eq] at ht
    obtain ⟨⟨⟨hsb, hdb⟩, hl2⟩, hh2⟩ := ht
    have hdst : fits t dst v = true := fits_of_bounds hd (by omega) (by omega)
    unfold convert
    simp [hsb, hdb, hv, hdst]
  · contradiction

/-- **A fallible conversion succeeds exactly when the value fits** its target,
and then it returns the value unchanged. -/
theorem convert_succeeds_iff {t : Option Width} {src dst : Ty} {v : Int}
    (hs : src ≠ .bool) (hd : dst ≠ .bool) (hv : fits t src v = true) :
    convert t src dst v = some v ↔ fits t dst v = true := by
  unfold convert
  constructor
  · intro h
    split at h
    · contradiction
    · split at h
      · rename_i hfit
        exact hfit.2
      · contradiction
  · intro hdst
    simp [hs, hd, hv, hdst]

theorem convert_value {t : Option Width} {src dst : Ty} {v w : Int}
    (h : convert t src dst v = some w) : w = v ∧ fits t dst w = true := by
  unfold convert at h
  split at h
  · contradiction
  · split at h
    · rename_i hfit
      cases h
      exact ⟨rfl, hfit.2⟩
    · contradiction

/-- There is no conversion to or from `bool`. -/
theorem no_bool_conversion (t : Option Width) (ty : Ty) (v : Int) :
    convert t .bool ty v = none ∧ convert t ty .bool v = none := by
  simp [convert]

inductive Cmp where
  | eq | ne | lt | le | gt | ge
  deriving DecidableEq, Repr

/-- A comparison of two values of one type produces a `bool`; `bool` has
equality but no ordering. -/
def compare (cmp : Cmp) (ty : Ty) (a b : Int) : Option Bool :=
  match cmp with
  | .eq => some (decide (a = b))
  | .ne => some (decide (a ≠ b))
  | .lt => if ty = .bool then none else some (decide (a < b))
  | .le => if ty = .bool then none else some (decide (a ≤ b))
  | .gt => if ty = .bool then none else some (decide (a > b))
  | .ge => if ty = .bool then none else some (decide (a ≥ b))

theorem bool_has_no_ordering (a b : Int) (cmp : Cmp) (h : cmp ≠ .eq ∧ cmp ≠ .ne) :
    compare cmp .bool a b = none := by
  cases cmp <;> simp_all [compare]

theorem integer_comparison_total (cmp : Cmp) (ty : Ty) (a b : Int) (h : ty ≠ .bool) :
    ∃ r, compare cmp ty a b = some r := by
  cases cmp
  all_goals first | exact ⟨_, rfl⟩ | simp [compare, h]

/-! The checker's static refusal. The route folds every subexpression whose
leaves are all literals with the same `checked` the evaluator uses, and refuses
the module when the fold faults. `fold_sound` states that a static fault is a
dynamic fault in every environment, and a static value is the dynamic value. -/

inductive CExpr where
  | lit (n : Int)
  | var (slot : Nat)
  | bin (op : Op) (a b : CExpr)

/-- Evaluation at one type: a literal outside the type faults, and every
operand is evaluated left to right before the checked operation. -/
def eval (t : Option Width) (ty : Ty) (env : List Int) : CExpr → Option Int
  | .lit n => narrow t ty n
  | .var i => env[i]?
  | .bin op a b =>
    (eval t ty env a).bind fun x => (eval t ty env b).bind fun y => checked t op ty x y

inductive Fold where
  | unknown
  | value (n : Int)
  | fault

def fold (t : Option Width) (ty : Ty) : CExpr → Fold
  | .lit n => if fits t ty n = true then .value n else .fault
  | .var _ => .unknown
  | .bin op a b =>
    match fold t ty a, fold t ty b with
    | .fault, _ => .fault
    | _, .fault => .fault
    | .value x, .value y =>
      match checked t op ty x y with
      | some v => .value v
      | none => .fault
    | _, _ => .unknown

/-- **The checker's static refusal implies the dynamic check would fail**, and
a statically known value is the value every evaluation computes. -/
theorem fold_sound (t : Option Width) (ty : Ty) :
    ∀ e : CExpr, (fold t ty e = .fault → ∀ env, eval t ty env e = none) ∧
      (∀ n, fold t ty e = .value n → ∀ env, eval t ty env e = some n)
  | .lit n => by
    constructor
    · intro h env
      simp only [fold] at h
      split at h
      · contradiction
      · rename_i hn
        simp [eval, narrow, hn]
    · intro m h env
      simp only [fold] at h
      split at h
      · rename_i hn
        cases h
        simp [eval, narrow, hn]
      · contradiction
  | .var i => by
    constructor
    · intro h; simp [fold] at h
    · intro m h; simp [fold] at h
  | .bin op a b => by
    have iha := fold_sound t ty a
    have ihb := fold_sound t ty b
    constructor
    · intro h env
      simp only [eval]
      cases ha : fold t ty a with
      | fault => simp [iha.1 ha env]
      | unknown =>
        cases hb : fold t ty b with
        | fault =>
          cases eval t ty env a <;> simp [ihb.1 hb env]
        | unknown => simp [fold, ha, hb] at h
        | value y => simp [fold, ha, hb] at h
      | value x =>
        cases hb : fold t ty b with
        | fault => simp [iha.2 x ha env, ihb.1 hb env]
        | unknown => simp [fold, ha, hb] at h
        | value y =>
          simp only [fold, ha, hb] at h
          split at h
          · contradiction
          · rename_i hc
            simp [iha.2 x ha env, ihb.2 y hb env, hc]
    · intro n h env
      simp only [eval]
      cases ha : fold t ty a with
      | fault => simp [fold, ha] at h
      | unknown =>
        cases hb : fold t ty b with
        | fault => simp [fold, ha, hb] at h
        | unknown => simp [fold, ha, hb] at h
        | value y => simp [fold, ha, hb] at h
      | value x =>
        cases hb : fold t ty b with
        | fault => simp [fold, ha, hb] at h
        | unknown => simp [fold, ha, hb] at h
        | value y =>
          simp only [fold, ha, hb] at h
          split at h
          · rename_i v hc
            cases h
            simp [iha.2 x ha env, ihb.2 y hb env, hc]
          · contradiction

theorem static_fault_is_dynamic_fault {t : Option Width} {ty : Ty} {e : CExpr}
    (h : fold t ty e = .fault) (env : List Int) : eval t ty env e = none :=
  (fold_sound t ty e).1 h env

/-! `std.fmt`'s `hex` digit count: a quarter of the width, so `u8` renders two
digits, `i32` eight (the established `0x000000FF` style) and `u64` sixteen. -/

def Ty.bits (t : Option Width) : Ty → Option Nat
  | .bool => none
  | .i8 | .u8 => some 8
  | .i16 | .u16 => some 16
  | .i32 | .u32 => some 32
  | .i64 | .u64 => some 64
  | .isize | .usize => t.map fun | .w32 => 32 | .w64 => 64

/-- The bit pattern `hex` renders: the value itself when non-negative, and its
two's complement at the type's width when negative. -/
def pattern (bits : Nat) (v : Int) : Int := if v < 0 then v + 2 ^ bits else v

theorem pattern_in_width {bits : Nat} {v : Int} (hl : -(2 ^ bits : Int) ≤ v)
    (hh : v < 2 ^ bits) : 0 ≤ pattern bits v ∧ pattern bits v < 2 ^ bits := by
  unfold pattern
  split <;> omega

/-- The shared boundary vectors: `(op, type, a, b, result)`, replayed against
the Rust kernel by `lean_scalar_type_vectors_match_the_rust_kernel`. -/
def agreement : List (Op × Ty × Int × Int × Option Int) := [
  (.add, .i8, 127, 1, none), (.add, .i8, 126, 1, some 127), (.sub, .i8, -128, 1, none),
  (.mul, .i8, -128, -1, none), (.div, .i8, -128, -1, none), (.rem, .i8, -128, -1, none),
  (.div, .i8, -7, 2, some (-3)), (.rem, .i8, -7, 2, some (-1)),
  (.add, .u8, 255, 1, none), (.sub, .u8, 0, 1, none), (.mul, .u8, 16, 16, none),
  (.mul, .u8, 15, 17, some 255), (.div, .u8, 255, 0, none),
  (.add, .i16, 32767, 1, none), (.mul, .i16, 181, 181, some 32761), (.mul, .i16, 182, 181, none),
  (.sub, .u16, 0, 65535, none), (.add, .u16, 65534, 1, some 65535),
  (.add, .i32, 2147483647, 1, none), (.div, .i32, -2147483648, -1, none),
  (.mul, .u32, 65536, 65536, none), (.mul, .u32, 65535, 65537, some 4294967295),
  (.add, .i64, 9223372036854775807, 1, none), (.sub, .i64, -9223372036854775808, 1, none),
  (.mul, .i64, -9223372036854775808, -1, none), (.div, .i64, -9223372036854775808, -1, none),
  (.rem, .i64, -9223372036854775808, -1, none), (.div, .i64, -9223372036854775807, -1, some 9223372036854775807),
  (.add, .u64, 18446744073709551615, 1, none), (.mul, .u64, 4294967296, 4294967296, none),
  (.mul, .u64, 4294967295, 4294967297, some 18446744073709551615), (.sub, .u64, 1, 2, none),
  (.rem, .u64, 18446744073709551615, 10, some 5), (.add, .bool, 0, 1, none)]

theorem agreement_holds :
    agreement.all (fun (op, ty, a, b, v) => checked none op ty a b == v) = true := by decide

end QCore.ScalarTypes
