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

/-! The typed arithmetic error of `.qr` sources (card `W48-QR-SCALAR-TYPES-R2`).

The Lean statement of `qcore_core::scalar_types::ArithmeticError` and of the
`checked_error` / `negate_error` / `convert_error` classifications: a checked
operation at a position the checker could not decide does not fault, it
**raises** `ArithmeticError.overflow`, `.divisionByZero` or `.outOfRange`, and the
raise travels through `?` (propagation) or is handled by `match … Result::Err`.

What is proved:

* the typed path agrees with `QCore.ScalarTypes.checked`: it succeeds with
  exactly the value the checked operation returns, and raises exactly when the
  checked operation faults — so every theorem of that module about success
  (in range, exact) carries over;
* the classification: a zero divisor raises `divisionByZero`, `MIN / -1` and
  `MIN % -1` raise `overflow`, a failing conversion raises `outOfRange`;
* **the dynamic path agrees with the static refusal**: an expression the
  checker's constant fold refuses raises in every environment, a folded value
  is the dynamic value, and for one operation on constant operands the raised
  variant is the one the fault is classified as;
* `?` propagates a raised error unchanged and `match` selects the arm the
  outcome names.

The Rust transcription is bound to these definitions by the vector table
`typedAgreement`, which the Rust test
`lean_typed_error_vectors_match_the_rust_kernel` parses and replays against
`qcore_core::scalar_types::checked_error`; the emitted C is compared with the
Rust reference by native execution. Neither is an extraction or refinement
theorem. -/
namespace QCore.ScalarTypedErrors

open QCore.ScalarTypes

inductive ArithError where
  | overflow
  | divisionByZero
  | outOfRange
  deriving DecidableEq, Repr

/-- Which variant a fault of `a OP b` raises: a zero divisor of `/` or `%` is
`divisionByZero`; every other arithmetic fault is `overflow`. -/
def classify (op : Op) (b : Int) : ArithError :=
  match op with
  | .div | .rem => if b = 0 then .divisionByZero else .overflow
  | _ => .overflow

/-- Checked arithmetic on the typed path. -/
def checkedE (t : Option Width) (op : Op) (ty : Ty) (a b : Int) : Except ArithError Int :=
  match checked t op ty a b with
  | some v => .ok v
  | none => .error (classify op b)

/-- The one conversion on the typed path. -/
def convertE (t : Option Width) (src dst : Ty) (v : Int) : Except ArithError Int :=
  match convert t src dst v with
  | some w => .ok w
  | none => .error .outOfRange

theorem checkedE_ok_iff {t : Option Width} {op : Op} {ty : Ty} {a b v : Int} :
    checkedE t op ty a b = .ok v ↔ checked t op ty a b = some v := by
  unfold checkedE
  cases h : checked t op ty a b <;> simp

theorem checkedE_raises_iff {t : Option Width} {op : Op} {ty : Ty} {a b : Int} :
    (∃ e, checkedE t op ty a b = .error e) ↔ checked t op ty a b = none := by
  unfold checkedE
  cases h : checked t op ty a b <;> simp

/-- A raise always names the classified variant. -/
theorem checkedE_raises_classified {t : Option Width} {op : Op} {ty : Ty} {a b : Int}
    {e : ArithError} (h : checkedE t op ty a b = .error e) : e = classify op b := by
  unfold checkedE at h
  cases hc : checked t op ty a b <;> simp_all

/-- **The typed path never returns a value outside the type.** -/
theorem checkedE_in_range {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checkedE t op ty a b = .ok v) : fits t ty v = true :=
  checked_in_range (checkedE_ok_iff.mp h)

/-- **A typed operation that succeeds returns the mathematically exact result.** -/
theorem checkedE_exact {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checkedE t op ty a b = .ok v) : v = exact op a b :=
  checked_exact (checkedE_ok_iff.mp h)

/-- A zero divisor raises `divisionByZero`. -/
theorem division_by_zero_raises (t : Option Width) (op : Op) (ty : Ty) (a : Int)
    (hop : op = .div ∨ op = .rem) : checkedE t op ty a 0 = .error .divisionByZero := by
  unfold checkedE
  rw [division_by_zero_faults t op ty a hop]
  rcases hop with rfl | rfl <;> simp [classify]

/-- `MIN / -1` and `MIN % -1` of a signed type raise `overflow`. -/
theorem min_div_minus_one_raises_overflow (t : Option Width) (op : Op) (ty : Ty) (lo hi : Int)
    (hop : op = .div ∨ op = .rem) (hs : ty.signed = true) (hr : ty.range t = some (lo, hi)) :
    checkedE t op ty lo (-1) = .error .overflow := by
  unfold checkedE
  rw [min_div_minus_one_faults t op ty lo hi hop hs hr]
  rcases hop with rfl | rfl <;> simp [classify]

/-- A failing conversion raises `outOfRange`, and nothing else. -/
theorem convertE_raises_out_of_range {t : Option Width} {src dst : Ty} {v : Int}
    {e : ArithError} (h : convertE t src dst v = .error e) : e = .outOfRange := by
  unfold convertE at h
  cases hc : convert t src dst v <;> simp_all

/-- A conversion on the typed path succeeds exactly when the value fits its
target, and then returns it unchanged. -/
theorem convertE_ok_iff_fits {t : Option Width} {src dst : Ty} {v : Int}
    (hs : src ≠ .bool) (hd : dst ≠ .bool) (hv : fits t src v = true) :
    convertE t src dst v = .ok v ↔ fits t dst v = true := by
  rw [← convert_succeeds_iff hs hd hv]
  unfold convertE
  cases h : convert t src dst v <;> simp

/-- A total conversion never raises. -/
theorem total_conversion_never_raises {t : Option Width} {src dst : Ty} {v : Int}
    (ht : conversionTotal t src dst = true) (hv : fits t src v = true) :
    convertE t src dst v = .ok v := by
  unfold convertE
  rw [total_conversion_preserves ht hv]

/-! The dynamic path over the fold's expression language. Evaluation raises
where `QCore.ScalarTypes.eval` faults: a literal outside the type and an
unbound slot raise `outOfRange` (neither reaches a checked program — the first
is a static refusal, the second a checker invariant), and an operation raises
its classified variant. -/

def evalE (t : Option Width) (ty : Ty) (env : List Int) : CExpr → Except ArithError Int
  | .lit n => if fits t ty n = true then .ok n else .error .outOfRange
  | .var i =>
    match env[i]? with
    | some v => .ok v
    | none => .error .outOfRange
  | .bin op a b =>
    (evalE t ty env a).bind fun x => (evalE t ty env b).bind fun y => checkedE t op ty x y

/-- **The typed dynamic path agrees with the fault semantics**: it returns a
value exactly when `eval` does, the same value, and raises exactly when `eval`
faults. -/
theorem evalE_agrees (t : Option Width) (ty : Ty) (env : List Int) :
    ∀ e : CExpr, (∀ v, evalE t ty env e = .ok v ↔ eval t ty env e = some v) ∧
      ((∃ err, evalE t ty env e = .error err) ↔ eval t ty env e = none)
  | .lit n => by
    constructor
    · intro v
      simp only [evalE, eval, narrow]
      split <;> simp
    · simp only [evalE, eval, narrow]
      split <;> simp
  | .var i => by
    constructor
    · intro v
      simp only [evalE, eval]
      cases env[i]? <;> simp
    · simp only [evalE, eval]
      cases env[i]? <;> simp
  | .bin op a b => by
    have iha := evalE_agrees t ty env a
    have ihb := evalE_agrees t ty env b
    have ok_or_error : ∀ r : Except ArithError Int, (∃ v, r = .ok v) ∨ (∃ err, r = .error err) := by
      intro r
      cases r with
      | ok v => exact .inl ⟨v, rfl⟩
      | error err => exact .inr ⟨err, rfl⟩
    rcases ok_or_error (evalE t ty env a) with ⟨x, hx⟩ | ⟨ex, hx⟩
    · have hxe : eval t ty env a = some x := (iha.1 x).mp hx
      rcases ok_or_error (evalE t ty env b) with ⟨y, hy⟩ | ⟨ey, hy⟩
      · have hye : eval t ty env b = some y := (ihb.1 y).mp hy
        constructor
        · intro v
          simp only [evalE, eval, hx, hy, hxe, hye, Except.bind, Option.bind]
          exact checkedE_ok_iff
        · simp only [evalE, eval, hx, hy, hxe, hye, Except.bind, Option.bind]
          exact checkedE_raises_iff
      · have hye : eval t ty env b = none := ihb.2.mp ⟨ey, hy⟩
        constructor
        · intro v
          simp [evalE, eval, hx, hy, hxe, hye, Except.bind, Option.bind]
        · simp [evalE, eval, hx, hy, hxe, hye, Except.bind, Option.bind]
    · have hxe : eval t ty env a = none := iha.2.mp ⟨ex, hx⟩
      constructor
      · intro v
        simp [evalE, eval, hx, hxe, Except.bind, Option.bind]
      · simp [evalE, eval, hx, hxe, Except.bind, Option.bind]

/-- **The checker's static refusal is a dynamic raise**: an expression whose
constant fold faults raises in every environment. -/
theorem static_fault_is_dynamic_raise {t : Option Width} {ty : Ty} {e : CExpr}
    (h : fold t ty e = .fault) (env : List Int) : ∃ err, evalE t ty env e = .error err :=
  (evalE_agrees t ty env e).2.mpr (static_fault_is_dynamic_fault h env)

/-- A statically folded value is the value the typed dynamic path returns. -/
theorem static_value_is_dynamic_value {t : Option Width} {ty : Ty} {e : CExpr} {n : Int}
    (h : fold t ty e = .value n) (env : List Int) : evalE t ty env e = .ok n :=
  ((evalE_agrees t ty env e).1 n).mpr ((fold_sound t ty e).2 n h env)

/-- For one operation on two constants of the type, the static refusal and the
dynamic raise name **the same variant**: the one the fault is classified as. -/
theorem static_fault_variant {t : Option Width} {ty : Ty} {op : Op} {a b : Int}
    (ha : fits t ty a = true) (hb : fits t ty b = true)
    (h : fold t ty (.bin op (.lit a) (.lit b)) = .fault) (env : List Int) :
    evalE t ty env (.bin op (.lit a) (.lit b)) = .error (classify op b) := by
  have hc : checked t op ty a b = none := by
    simp only [fold, ha, hb, ↓reduceIte] at h
    split at h
    · contradiction
    · assumption
  simp [evalE, ha, hb, Except.bind, checkedE, hc]

/-! Propagation and handling. `CALL?` is `bind`: a raise leaves the caller at
once, unchanged. `match CALL { Result::Ok … Result::Err … }` runs exactly the
arm the outcome names. -/

/-- `CALL?` followed by the rest of the caller. -/
def propagate {α : Type} (r : Except ArithError Int) (rest : Int → Except ArithError α) :
    Except ArithError α :=
  r.bind rest

/-- `match CALL { Result::Ok { value: v } => ok v, Result::Err { error: e } => err e }`. -/
def handle {β : Type} (r : Except ArithError Int) (ok : Int → β) (err : ArithError → β) : β :=
  match r with
  | .ok v => ok v
  | .error e => err e

theorem propagate_raise {α : Type} (e : ArithError) (rest : Int → Except ArithError α) :
    propagate (.error e) rest = .error e := rfl

theorem propagate_value {α : Type} (v : Int) (rest : Int → Except ArithError α) :
    propagate (.ok v) rest = rest v := rfl

theorem handle_raise {β : Type} (e : ArithError) (ok : Int → β) (err : ArithError → β) :
    handle (.error e) ok err = err e := rfl

theorem handle_value {β : Type} (v : Int) (ok : Int → β) (err : ArithError → β) :
    handle (.ok v) ok err = ok v := rfl

/-- A handled operation never raises out of the `match`: whatever the operands,
the result is one of the two arms' own results. -/
theorem handled_never_raises {t : Option Width} {op : Op} {ty : Ty} {a b : Int}
    (ok : Int → Except ArithError Int) (fallback : Int)
    (hok : ∀ v, ∃ w, ok v = .ok w) :
    ∃ w, handle (checkedE t op ty a b) ok (fun _ => .ok fallback) = .ok w := by
  unfold handle
  split
  · rename_i v _
    exact hok v
  · exact ⟨fallback, rfl⟩

/-- The shared typed vectors: `(op, type, a, b, result)`, replayed against the
Rust kernel by `lean_typed_error_vectors_match_the_rust_kernel`. -/
def typedAgreement : List (Op × Ty × Int × Int × Except ArithError Int) := [
  (.add, .i8, 127, 1, .error .overflow), (.add, .i8, 126, 1, .ok 127),
  (.div, .i8, -128, -1, .error .overflow), (.rem, .i8, -128, -1, .error .overflow),
  (.div, .i8, 5, 0, .error .divisionByZero), (.rem, .i8, -128, 0, .error .divisionByZero),
  (.add, .u8, 255, 1, .error .overflow), (.sub, .u8, 0, 1, .error .overflow),
  (.div, .u8, 255, 0, .error .divisionByZero), (.mul, .u8, 15, 17, .ok 255),
  (.mul, .i16, 182, 181, .error .overflow), (.rem, .u16, 65535, 0, .error .divisionByZero),
  (.div, .i32, -2147483648, -1, .error .overflow), (.div, .i32, 7, 0, .error .divisionByZero),
  (.mul, .u32, 65536, 65536, .error .overflow), (.sub, .i64, -9223372036854775808, 1, .error .overflow),
  (.rem, .i64, -9223372036854775808, -1, .error .overflow),
  (.div, .u64, 18446744073709551615, 0, .error .divisionByZero),
  (.rem, .u64, 18446744073709551615, 10, .ok 5)]

/-- Equality of two outcomes, as a `Bool` the table can be decided with. -/
def sameOutcome : Except ArithError Int → Except ArithError Int → Bool
  | .ok a, .ok b => a == b
  | .error a, .error b => a == b
  | _, _ => false

theorem typed_agreement_holds :
    typedAgreement.all (fun (op, ty, a, b, r) => sameOutcome (checkedE none op ty a b) r) = true := by
  decide

end QCore.ScalarTypedErrors
