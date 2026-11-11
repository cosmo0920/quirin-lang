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

import QCore.Ieee754Round
import QCore.FloatTypes

/-! IEEE 754 operations, comparisons and conversions (card
`W48-QR-FLOAT-IEEE-LEAN-MODEL`).

The operations `.qr` floats use (`crates/qcore-core/src/scalar_floats.rs`),
each defined as IEEE 754-2019 defines it: **the exact result, rounded once**
(`round`, `QCore.Ieee754Round`), with the special cases of §6 and §7:

* `add`, `sub`, `mul`, `div`, `neg`: NaN propagates; `inf - inf`, `0 · inf`,
  `0 / 0` and `inf / inf` are NaN; `x / 0` is a signed infinity for `x ≠ 0`;
  an exact zero sum is `+0` unless both addends are `-0` (§6.3); a product or
  quotient carries the exclusive or of the signs, zero and infinity included.
* The comparisons (§5.11): NaN is unordered, `-0 == +0`, the infinities are
  the extremes; `Cmp.eval` gives the six operators the float card gates, and
  `Cmp.effect` the effect (`partial_eq`, `partial_ord`) each needs.
* `ofInt` (integer → float) and `convert` (`f64 → f32`, `f32 → f64`) round
  the exact value with the same `round`; `toInt` (float → integer) is
  `QCore.FloatTypes.toInt` of the datum's exact dyadic value.

`sameValue` is `scalar_floats::same_value` (equal patterns, or both NaN: NaN
payload and sign are not specified). The bit-level wrappers (`arithBits`,
`negBits`, `cmpBits`, `ofIntBits`, `convertBits`, `toIntBits`) are what the
vector tables of `QCore.Ieee754Vectors32`/`Ieee754Vectors64` replay. No
project axiom is used. -/
namespace QCore.Ieee754

open QCore.ScalarTypes (Ty Width fits)

/-- Whether a datum is a NaN. -/
def Datum.isNaN : Datum → Bool
  | .nan => true
  | _ => false

/-- A signed magnitude as an integer (both zeros are `0`). -/
def signed (s : Bool) (N : Nat) : Int := if s then -(N : Int) else N

/-! ## Arithmetic -/

/-- Unary minus: the sign flipped, a NaN a NaN (§5.5.1). -/
def neg : Datum → Datum
  | .nan => .nan
  | .inf s => .inf (!s)
  | .fin s N => .fin (!s) N

/-- Addition (§6.1, §6.3): the exact sum rounded; an exact zero sum is `+0`
unless both addends are negative (then both are `-0`). -/
def add (f : Format) : Datum → Datum → Datum
  | .nan, _ => .nan
  | _, .nan => .nan
  | .inf s, .inf t => if s = t then .inf s else .nan
  | .inf s, .fin _ _ => .inf s
  | .fin _ _, .inf t => .inf t
  | .fin s N, .fin t M =>
    if signed s N + signed t M = 0 then .fin (s && t) 0
    else round f (decide (signed s N + signed t M < 0)) (signed s N + signed t M).natAbs 1

/-- Subtraction is addition of the negation (§5.4.1). -/
def sub (f : Format) (a b : Datum) : Datum := add f a (neg b)

/-- Multiplication (§6.1, §7.2): the exact product `N·M · 2^(-2q)`, i.e.
`N·M / 2^q` units, rounded; `0 · inf` is NaN. -/
def mul (f : Format) : Datum → Datum → Datum
  | .nan, _ => .nan
  | _, .nan => .nan
  | .inf s, .inf t => .inf (xor s t)
  | .inf s, .fin t M => if M = 0 then .nan else .inf (xor s t)
  | .fin s N, .inf t => if N = 0 then .nan else .inf (xor s t)
  | .fin s N, .fin t M => round f (xor s t) (N * M) (2 ^ f.qneg)

/-- Division (§6.1, §7.2, §7.3): the exact quotient `N / M`, i.e.
`N · 2^q / M` units, rounded; `0 / 0` and `inf / inf` are NaN; a nonzero
finite value divided by zero is a signed infinity. -/
def div (f : Format) : Datum → Datum → Datum
  | .nan, _ => .nan
  | _, .nan => .nan
  | .inf _, .inf _ => .nan
  | .inf s, .fin t _ => .inf (xor s t)
  | .fin s _, .inf t => .fin (xor s t) 0
  | .fin s N, .fin t M =>
    if M = 0 then (if N = 0 then .nan else .inf (xor s t))
    else round f (xor s t) (N * 2 ^ f.qneg) M

/-- The four operators of `scalar_floats::arithmetic`. -/
inductive ArithOp where
  | add | sub | mul | div
  deriving DecidableEq, Repr

def ArithOp.eval (f : Format) : ArithOp → Datum → Datum → Datum
  | .add => Ieee754.add f
  | .sub => Ieee754.sub f
  | .mul => Ieee754.mul f
  | .div => Ieee754.div f

/-! ### The special-case table -/

/-- **NaN propagates** through every operation, from either operand. -/
theorem nan_propagates (f : Format) (op : ArithOp) (a : Datum) :
    op.eval f .nan a = .nan ∧ op.eval f a .nan = .nan ∧ neg .nan = .nan := by
  cases op <;> cases a <;> simp [ArithOp.eval, add, sub, mul, div, neg]

/-- `inf - inf` (and `inf + -inf`) is NaN. -/
theorem inf_sub_inf (f : Format) (s : Bool) :
    sub f (.inf s) (.inf s) = .nan ∧ add f (.inf s) (.inf (!s)) = .nan := by
  cases s <;> simp [sub, add, neg]

/-- Infinities of one sign add to that infinity; an infinity absorbs any
finite addend. -/
theorem add_inf (f : Format) (s t : Bool) (N : Nat) :
    add f (.inf s) (.inf s) = .inf s ∧ add f (.inf s) (.fin t N) = .inf s ∧
      add f (.fin t N) (.inf s) = .inf s := by
  simp [add]

/-- `0 · inf` and `inf · 0` are NaN, whatever the signs. -/
theorem zero_mul_inf (f : Format) (s t : Bool) :
    mul f (.fin s 0) (.inf t) = .nan ∧ mul f (.inf t) (.fin s 0) = .nan := by
  simp [mul]

/-- `inf · x` for a nonzero `x` (or an infinity) is an infinity with the
exclusive or of the signs. -/
theorem inf_mul (f : Format) (s t : Bool) {N : Nat} (hN : N ≠ 0) :
    mul f (.inf s) (.fin t N) = .inf (xor s t) ∧ mul f (.fin t N) (.inf s) = .inf (xor t s) ∧
      mul f (.inf s) (.inf t) = .inf (xor s t) := by
  simp [mul, hN]

/-- `x / 0`: a signed infinity for a nonzero finite `x` (divideByZero), NaN
for `0 / 0`. -/
theorem div_by_zero (f : Format) (s t : Bool) {N : Nat} (hN : N ≠ 0) :
    div f (.fin s N) (.fin t 0) = .inf (xor s t) ∧ div f (.fin s 0) (.fin t 0) = .nan := by
  simp [div, hN]

/-- `inf / inf` is NaN; `inf / x` an infinity; `x / inf` a signed zero. -/
theorem div_inf (f : Format) (s t : Bool) (N : Nat) :
    div f (.inf s) (.inf t) = .nan ∧ div f (.inf s) (.fin t N) = .inf (xor s t) ∧
      div f (.fin s N) (.inf t) = .fin (xor s t) 0 := by
  simp [div]

/-- **Signed zero sums** (§6.3): `+0 + +0 = +0`, `-0 + -0 = -0`,
`+0 + -0 = -0 + +0 = +0`. -/
theorem add_zeros (f : Format) (s t : Bool) :
    add f (.fin s 0) (.fin t 0) = .fin (s && t) 0 := by
  simp [add, signed]

/-- **`x + (-x) = +0` and `x - x = +0`** for every finite `x`, `-0` included. -/
theorem add_neg_self (f : Format) (s : Bool) (N : Nat) :
    add f (.fin s N) (.fin (!s) N) = .fin false 0 ∧ sub f (.fin s N) (.fin s N) = .fin false 0 := by
  cases s <;> simp [add, sub, neg, signed]
  all_goals (intro h; exact absurd (by omega) h)

/-- `-0 - +0 = -0` (the difference of unlike-signed zeros keeps the first). -/
theorem neg_zero_sub_zero (f : Format) : sub f (.fin true 0) (.fin false 0) = .fin true 0 := by
  simp [sub, add, neg, signed]

theorem round_sign (f : Format) (s : Bool) (n d : Nat) :
    (∃ K, round f s n d = .fin s K) ∨ round f s n d = .inf s := by
  unfold round
  split
  · exact Or.inl ⟨_, rfl⟩
  · exact Or.inr rfl

/-- **The sign of a product or quotient** of finite values is the exclusive or
of the signs, for a zero result (an underflow included) and an overflow
alike. -/
theorem mul_div_sign (f : Format) (s t : Bool) (N M : Nat) (hM : M ≠ 0) :
    ((∃ K, mul f (.fin s N) (.fin t M) = .fin (xor s t) K) ∨
        mul f (.fin s N) (.fin t M) = .inf (xor s t)) ∧
      ((∃ K, div f (.fin s N) (.fin t M) = .fin (xor s t) K) ∨
        div f (.fin s N) (.fin t M) = .inf (xor s t)) := by
  simp only [mul, div, hM, ↓reduceIte]
  exact ⟨round_sign f _ _ _, round_sign f _ _ _⟩

/-- A zero times a finite value is a zero with the exclusive or of the signs. -/
theorem zero_mul (f : Format) (s t : Bool) (M : Nat) :
    mul f (.fin s 0) (.fin t M) = .fin (xor s t) 0 := by
  simp [mul, round, roundMag, roundUnb, rq, rr, ru, Format.maxN]

/-- Negation is an involution. -/
theorem neg_neg (a : Datum) : neg (neg a) = a := by
  cases a <;> simp [neg]

/-- Addition and multiplication are commutative, NaN for NaN. -/
theorem add_comm (f : Format) (a b : Datum) : add f a b = add f b a := by
  cases a <;> cases b <;> simp only [add]
  · rename_i s t; by_cases h : s = t <;> simp [h, eq_comm]
  · rename_i s N t M
    rw [Int.add_comm (signed s N), Bool.and_comm]

theorem mul_comm (f : Format) (a b : Datum) : mul f a b = mul f b a := by
  cases a <;> cases b <;> simp only [mul, Bool.xor_comm, Nat.mul_comm]

/-! ### Correct rounding of the operations -/

/-- A rounded result is the nearest finite value of the format to the exact
value, the even one at a tie — so is every finite result of `add`, `sub`,
`mul`, `div`, `ofInt` and `convert` below, each being a `round`. -/
theorem round_fin (f : Format) (hf : f.Valid) {s s' : Bool} {n d K : Nat} (hd : 0 < d)
    (h : round f s n d = .fin s' K) :
    s' = s ∧ Rep f K ∧ (∀ K', Rep f K' → dist n d K ≤ dist n d K') ∧
      (∀ K', Rep f K' → K' ≠ K → dist n d K' = dist n d K → encodeMag f K % 2 = 0) := by
  unfold round at h
  split at h
  · rename_i N hN
    cases h
    exact ⟨rfl, roundMag_rep f hN, fun _ h' => roundMag_nearest f hd hN h',
      fun _ h' hne heq => (roundMag_tie_even f hf hd hN h' hne heq).2⟩
  · cases h

/-- A rounded result is infinite exactly past the overflow threshold. -/
theorem round_inf_iff (f : Format) (hf : f.Valid) {s : Bool} {n d : Nat} (hd : 0 < d) :
    round f s n d = .inf s ↔ (f.maxN + 2 ^ f.p * 2 ^ f.kmax) * d ≤ 2 * n := by
  rw [← roundMag_none_iff f hf hd]
  unfold round
  split <;> simp_all

/-- **Exact results are exact.** When the exact sum of two finite values is a
finite value of the format, `add` returns it. -/
theorem add_exact (f : Format) {s t : Bool} {N M : Nat}
    (h0 : signed s N + signed t M ≠ 0) (hr : Rep f (signed s N + signed t M).natAbs) :
    add f (.fin s N) (.fin t M) =
      .fin (decide (signed s N + signed t M < 0)) (signed s N + signed t M).natAbs := by
  simp only [add, h0, ↓reduceIte, round]
  have := roundMag_exact f (d := 1) (by decide) hr
  rw [Nat.mul_one] at this
  rw [this]

/-! ## Comparisons -/

/-- A datum's position in the order of §5.11: the signed magnitude for a finite
value (both zeros at `0`), `±(maxN + 1)` for the infinities. -/
def rank (f : Format) : Datum → Int
  | .nan => 0
  | .inf s => if s then -((f.maxN : Int) + 1) else (f.maxN : Int) + 1
  | .fin s N => signed s N

/-- IEEE equality: false with a NaN operand; `-0 == +0`. -/
def feq (f : Format) (a b : Datum) : Bool :=
  !a.isNaN && !b.isNaN && decide (rank f a = rank f b)

/-- IEEE less-than: false with a NaN operand. -/
def flt (f : Format) (a b : Datum) : Bool :=
  !a.isNaN && !b.isNaN && decide (rank f a < rank f b)

/-- IEEE less-than-or-equal: false with a NaN operand. -/
def fle (f : Format) (a b : Datum) : Bool :=
  !a.isNaN && !b.isNaN && decide (rank f a ≤ rank f b)

/-- The six comparison operators of `.qr` (`scalar_types::Cmp`). -/
inductive Cmp where
  | eq | ne | lt | le | gt | ge
  deriving DecidableEq, Repr

/-- What each operator computes: the IEEE relations, `!=` the negation of
`==` (so true with a NaN operand), `>`/`>=` the converses. -/
def Cmp.eval (f : Format) : Cmp → Datum → Datum → Bool
  | .eq, a, b => feq f a b
  | .ne, a, b => !feq f a b
  | .lt, a, b => flt f a b
  | .le, a, b => fle f a b
  | .gt, a, b => flt f b a
  | .ge, a, b => fle f b a

/-- The effect each operator needs on a float operand (card
`W48-QR-FLOATING-POINT`, `QCore.FloatTypes.Effect`). -/
def Cmp.effect : Cmp → QCore.FloatTypes.Effect
  | .eq | .ne => .partialEq
  | .lt | .le | .gt | .ge => .partialOrd

/-- A finite value's rank lies strictly between the infinities'. -/
theorem rank_fin_bounds (f : Format) {s : Bool} {N : Nat} (h : Rep f N) :
    rank f (.inf true) < rank f (.fin s N) ∧ rank f (.fin s N) < rank f (.inf false) := by
  have := h.2
  cases s <;> simp [rank, signed] <;> omega

/-- The order on finite values is the order of their exact values
`signed s N · 2^(-qneg)`. -/
theorem flt_fin (f : Format) (s t : Bool) (N M : Nat) :
    flt f (.fin s N) (.fin t M) = decide (signed s N < signed t M) ∧
      feq f (.fin s N) (.fin t M) = decide (signed s N = signed t M) :=
  ⟨rfl, rfl⟩

/-- On non-NaN operands the comparisons are the order of the ranks. -/
theorem cmp_of_not_nan (f : Format) {a b : Datum} (ha : a.isNaN = false) (hb : b.isNaN = false) :
    flt f a b = decide (rank f a < rank f b) ∧ fle f a b = decide (rank f a ≤ rank f b) ∧
      feq f a b = decide (rank f a = rank f b) := by
  simp [flt, fle, feq, ha, hb]

/-- **NaN is unordered.** Every comparison with a NaN operand is false,
except `!=`, which is true. -/
theorem nan_unordered (f : Format) (c : Cmp) (a : Datum) :
    (c ≠ .ne → c.eval f .nan a = false ∧ c.eval f a .nan = false) ∧
      Cmp.ne.eval f .nan a = true ∧ Cmp.ne.eval f a .nan = true := by
  cases c <;> simp [Cmp.eval, feq, flt, fle, Datum.isNaN]

/-- **`x == x` is false exactly for a NaN.** -/
theorem feq_self (f : Format) (a : Datum) : feq f a a = !a.isNaN := by
  cases a <;> simp [feq, Datum.isNaN]

/-- **`-0 == +0`.** -/
theorem neg_zero_eq_zero (f : Format) : feq f (.fin true 0) (.fin false 0) = true := by
  simp [feq, rank, signed, Datum.isNaN]

/-- **Unordered exactly with a NaN.** Two operands are related by exactly one
of `<`, `==`, `>` when neither is a NaN, and by none when one is. -/
theorem trichotomy (f : Format) (a b : Datum) :
    (flt f a b || feq f a b || flt f b a) = (!a.isNaN && !b.isNaN) ∧
      (!(flt f a b && feq f a b) && !(flt f a b && flt f b a) &&
        !(feq f a b && flt f b a)) = true := by
  cases ha : a.isNaN
  · cases hb : b.isNaN
    · obtain ⟨l1, -, e1⟩ := cmp_of_not_nan f ha hb
      obtain ⟨l2, -, -⟩ := cmp_of_not_nan f hb ha
      rw [l1, e1, l2]
      generalize rank f a = x
      generalize rank f b = y
      by_cases h1 : x < y <;> by_cases h2 : x = y <;> simp [h1, h2] <;> omega
    · simp [flt, feq, ha, hb]
  · simp [flt, feq, ha]

/-- `<=` is `<` or `==`. -/
theorem fle_iff (f : Format) (a b : Datum) : fle f a b = (flt f a b || feq f a b) := by
  cases ha : a.isNaN
  · cases hb : b.isNaN
    · obtain ⟨l1, l2, e1⟩ := cmp_of_not_nan f ha hb
      rw [l1, l2, e1]
      generalize rank f a = x
      generalize rank f b = y
      by_cases h1 : x < y <;> by_cases h2 : x = y <;> simp [h1, h2] <;> omega
    · simp [fle, flt, feq, ha, hb]
  · simp [fle, flt, feq, ha]

/-- `==` is symmetric and transitive: a partial equivalence relation. -/
theorem feq_symm_trans (f : Format) (a b c : Datum) :
    feq f a b = feq f b a ∧ (feq f a b = true → feq f b c = true → feq f a c = true) := by
  constructor
  · unfold feq
    cases a.isNaN <;> cases b.isNaN <;> simp [eq_comm]
  · simp only [feq, Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq]
    intro ⟨⟨h1, _⟩, h3⟩ ⟨⟨_, h4⟩, h5⟩
    exact ⟨⟨h1, h4⟩, h3.trans h5⟩

/-- `<` is irreflexive and transitive: a strict partial order. -/
theorem flt_irrefl_trans (f : Format) (a b c : Datum) :
    flt f a a = false ∧ (flt f a b = true → flt f b c = true → flt f a c = true) := by
  constructor
  · simp [flt]
  · simp only [flt, Bool.and_eq_true, Bool.not_eq_true', decide_eq_true_eq]
    intro ⟨⟨h1, _⟩, h3⟩ ⟨⟨_, h4⟩, h5⟩
    exact ⟨⟨h1, h4⟩, Int.lt_trans h3 h5⟩

/-- **The gated comparisons mean exactly these relations.** The operators
`partial_eq` admits are `==` (IEEE equality) and `!=` (its negation); the
operators `partial_ord` admits are `<`, `<=`, `>`, `>=` (IEEE less-than,
less-or-equal and their converses). -/
theorem gated_relations (f : Format) (c : Cmp) :
    (c.effect = .partialEq ↔ c = .eq ∨ c = .ne) ∧
    (c.effect = .partialEq →
      (∀ a b, c.eval f a b = feq f a b) ∨ (∀ a b, c.eval f a b = !feq f a b)) ∧
    (c.effect = .partialOrd →
      (∀ a b, c.eval f a b = flt f a b) ∨ (∀ a b, c.eval f a b = fle f a b) ∨
        (∀ a b, c.eval f a b = flt f b a) ∨ (∀ a b, c.eval f a b = fle f b a)) := by
  cases c <;> simp [Cmp.effect, Cmp.eval]

/-- **Why they are gated.** IEEE equality is not reflexive and the IEEE order
is not total (a NaN), while on non-NaN operands `<=` is total and `==`
reflexive. -/
theorem partial_not_total (f : Format) :
    feq f .nan .nan = false ∧ fle f .nan (.fin false 0) = false ∧
      fle f (.fin false 0) .nan = false ∧
      ∀ a b : Datum, a.isNaN = false → b.isNaN = false →
        feq f a a = true ∧ (fle f a b || fle f b a) = true := by
  refine ⟨rfl, rfl, rfl, fun a b ha hb => ?_⟩
  obtain ⟨-, l1, -⟩ := cmp_of_not_nan f ha hb
  obtain ⟨-, l2, -⟩ := cmp_of_not_nan f hb ha
  rw [feq_self, ha, l1, l2]
  simp only [Bool.not_false, Bool.or_eq_true, decide_eq_true_eq, true_and]
  exact Int.le_total _ _

/-! ## Conversions -/

/-- Integer → float: the integer (`n · 2^qneg` units) rounded; `0` is `+0`. -/
def ofInt (f : Format) (n : Int) : Datum := round f (decide (n < 0)) (n.natAbs * 2 ^ f.qneg) 1

/-- Format → format: `f64 → f32` rounds, `f32 → f64` is exact
(`widen_exact`); NaN and the infinities carry over. -/
def convert (src dst : Format) : Datum → Datum
  | .nan => .nan
  | .inf s => .inf s
  | .fin s N => round dst s (N * 2 ^ dst.qneg) (2 ^ src.qneg)

/-- A datum's exact value as the float card's abstract `FValue`:
`signed s N · 2^(-qneg)`. -/
def Datum.toFValue (f : Format) : Datum → QCore.FloatTypes.FValue
  | .nan => .nan
  | .inf s => .inf s
  | .fin s N => .finite (signed s N) (-(f.qneg : Int))

/-- Float → integer: `QCore.FloatTypes.toInt` of the exact value — the
truncation toward zero, `none` (the typed `OutOfRange`) for NaN, ±inf and a
truncation that is not a value of the target. `bool` is no integer target: a
float `as bool` is no conversion (`scalar_types::convert` is `None`). -/
def toInt (f : Format) (t : Option Width) (ty : Ty) (d : Datum) : Option Int :=
  if ty = .bool then none else QCore.FloatTypes.toInt t ty (d.toFValue f)

/-- The ten integer targets, in the order of the vector tables. -/
def integerTargets : List Ty := [.i8, .i16, .i32, .i64, .isize, .u8, .u16, .u32, .u64, .usize]

/-- A float `as bool` converts nothing, as in the kernel. -/
theorem toInt_bool (f : Format) (t : Option Width) (d : Datum) : toInt f t .bool d = none := rfl

/-- The float card's `QCore.FloatTypes.toInt` does admit `.bool` (`0.5` →
`0`), unlike the kernel; recorded as `W48-QR-FLOAT-TOINT-BOOL-TARGET`, not
edited here. -/
theorem floatTypes_toInt_bool_differs :
    QCore.FloatTypes.toInt none .bool (.finite 1 (-1)) = some 0 ∧
      toInt binary64 none .bool (.fin false (2 ^ 1073)) = none := by
  decide +kernel

theorem roundUnb_scaled (f : Format) (a Q : Nat) :
    roundUnb f (a * 2 ^ Q) 1 = QCore.FloatTypes.roundNat f.p a * 2 ^ Q := by
  have hp := p_pos f
  by_cases ha : a = 0
  · subst ha
    simp [roundUnb, rq, rr, ru, rshift, ilog2_eq, QCore.FloatTypes.roundNat, Nat.log2_zero]
  have hlog : (a * 2 ^ Q).log2 = a.log2 + Q := log2_mul_pow Q ha
  have hs : rshift f (a * 2 ^ Q) 1 = (a.log2 + Q + 1) - f.p := by
    simp only [rshift, ilog2_eq]; rw [Nat.div_one, hlog]
  unfold roundUnb rq rr ru QCore.FloatTypes.roundNat
  rw [hs, Nat.one_mul]
  by_cases hsmall : a.log2 + 1 ≤ f.p
  · -- exact: the dropped bits are zero
    have hS : a.log2 + Q + 1 - f.p ≤ Q := by omega
    generalize a.log2 + Q + 1 - f.p = S at hS
    have hsplit : a * 2 ^ Q = a * 2 ^ (Q - S) * 2 ^ S := by
      rw [Nat.mul_assoc, ← Nat.pow_add, Nat.sub_add_cancel hS]
    rw [hsplit, Nat.mul_div_cancel _ (Nat.two_pow_pos _), Nat.mul_mod_left]
    simp [hsmall, (pow_pos_ne S).symm, ← hsplit]
  · -- `s0 ≥ 1` bits dropped, the same decision scaled by `2^Q`
    generalize hs0 : a.log2 + 1 - f.p = s0
    have hS : a.log2 + Q + 1 - f.p = s0 + Q := by omega
    have hs1 : 1 ≤ s0 := by omega
    rw [hS, Nat.pow_add, Nat.mul_div_mul_right _ _ (Nat.two_pow_pos _),
      Nat.mul_mod_mul_right]
    simp only [hsmall, ↓reduceIte]
    have hQ := Nat.two_pow_pos Q
    have hh : 2 ^ s0 = 2 * 2 ^ (s0 - 1) := by
      rw [← Nat.pow_succ']; congr 1; omega
    generalize a / 2 ^ s0 = q
    generalize hr : a % 2 ^ s0 = r
    generalize 2 ^ Q = P at hQ ⊢
    generalize 2 ^ (s0 - 1) = H at hh ⊢
    rw [hh]
    have e1 : 2 * H * P < 2 * (r * P) ↔ H < r := by
      rw [← Nat.mul_assoc, Nat.mul_lt_mul_right hQ]; omega
    have e2 : 2 * (r * P) = 2 * H * P ↔ r = H := by
      rw [← Nat.mul_assoc]
      constructor
      · intro h; have := Nat.eq_of_mul_eq_mul_right hQ h; omega
      · intro h; rw [h]
    by_cases hc : H < r ∨ (r = H ∧ q % 2 = 1)
    · have hc' : 2 * H * P < 2 * (r * P) ∨ (2 * (r * P) = 2 * H * P ∧ q % 2 = 1) := by
        rw [e1, e2]; exact hc
      simp only [eq_true hc, eq_true hc', ↓reduceIte]
      exact (Nat.mul_assoc _ _ _).symm
    · have hc' : ¬ (2 * H * P < 2 * (r * P) ∨ (2 * (r * P) = 2 * H * P ∧ q % 2 = 1)) := by
        rw [e1, e2]; exact hc
      simp only [eq_false hc, eq_false hc', ↓reduceIte]
      exact (Nat.mul_assoc _ _ _).symm

/-- **Integer → float agrees with the float card's rounding.** Absent
overflow, `ofInt` is `QCore.FloatTypes.roundNat` of the magnitude, the sign
kept: the value `QCore.FloatTypes.roundInt p n` (`ofInt_value`). -/
theorem ofInt_roundNat (f : Format) (n : Int)
    (h : QCore.FloatTypes.roundNat f.p n.natAbs * 2 ^ f.qneg ≤ f.maxN) :
    ofInt f n = .fin (decide (n < 0)) (QCore.FloatTypes.roundNat f.p n.natAbs * 2 ^ f.qneg) := by
  unfold ofInt round roundMag
  rw [roundUnb_scaled]
  simp [h]

theorem ofInt_value (f : Format) (n : Int) :
    signed (decide (n < 0)) (QCore.FloatTypes.roundNat f.p n.natAbs) =
      QCore.FloatTypes.roundInt f.p n := by
  unfold signed QCore.FloatTypes.roundInt
  by_cases h : n < 0 <;> simp [h]

/-- An integer of at most `p` bits converts exactly. -/
theorem ofInt_exact_small (f : Format) (n : Int) (h : n.natAbs < 2 ^ f.p)
    (hmax : n.natAbs * 2 ^ f.qneg ≤ f.maxN) :
    ofInt f n = .fin (decide (n < 0)) (n.natAbs * 2 ^ f.qneg) := by
  have hr : QCore.FloatTypes.roundNat f.p n.natAbs = n.natAbs :=
    QCore.FloatTypes.roundNat_exact_small (log2_lt_of_lt h (p_pos f))
  rw [ofInt_roundNat f n (by rw [hr]; exact hmax), hr]

theorem convert_fin (src dst : Format) (s : Bool) (N : Nat) :
    convert src dst (.fin s N) = round dst s (N * 2 ^ dst.qneg) (2 ^ src.qneg) := rfl

theorem round_of_some {f : Format} {s : Bool} {n d K : Nat} (h : roundMag f n d = some K) :
    round f s n d = .fin s K := by
  unfold round; rw [h]

set_option exponentiation.threshold 2048 in
/-- `f32 → f64` is exact: every binary32 value is the binary64 value
`N · 2^925` units (`2^-149 = 2^925 · 2^-1074`). -/
theorem widen_exact (s : Bool) {N : Nat} (h : Rep binary32 N) :
    convert binary32 binary64 (.fin s N) = .fin s (N * 2 ^ 925) := by
  have hrep : Rep binary64 (N * 2 ^ 925) := by
    obtain ⟨⟨M, k, hM, rfl⟩, hmax⟩ := h
    have hp : 2 ^ binary32.p ≤ 2 ^ binary64.p := by decide +kernel
    refine ⟨⟨M, k + 925, Nat.lt_of_lt_of_le hM hp, by rw [Nat.pow_add, Nat.mul_assoc]⟩, ?_⟩
    have : binary32.maxN * 2 ^ 925 ≤ binary64.maxN := by decide +kernel
    exact Nat.le_trans (Nat.mul_le_mul_right _ hmax) this
  have e : N * 2 ^ binary64.qneg = N * 2 ^ 925 * 2 ^ binary32.qneg := by
    rw [Nat.mul_assoc, ← Nat.pow_add, show 925 + binary32.qneg = binary64.qneg by decide]
  rw [convert_fin, e]
  exact round_of_some (roundMag_exact _ (Nat.two_pow_pos _) hrep)

/-- The float card's truncation of a finite datum's exact value is the
integer quotient `signed s N / 2^qneg`, rounded toward zero. -/
theorem truncDyadic_fin (f : Format) (hq : 0 < f.qneg) (s : Bool) (N : Nat) :
    QCore.FloatTypes.truncDyadic (signed s N) (-(f.qneg : Int)) =
      (signed s N).tdiv (2 ^ f.qneg) := by
  unfold QCore.FloatTypes.truncDyadic
  simp only [eq_false (show ¬ (0 ≤ -(f.qneg : Int)) by omega), ↓reduceIte]
  congr 2
  omega

/-- The float → integer conversion succeeds exactly when the datum is finite
and its truncation toward zero, `signed s N` divided by `2^qneg`, is a value
of the target. -/
theorem toInt_some_iff (f : Format) {t : Option Width} {ty : Ty} {d : Datum} {n : Int}
    (hq : 0 < f.qneg) (hty : ty ≠ .bool) :
    toInt f t ty d = some n ↔
      ∃ s N, d = .fin s N ∧ n = (signed s N).tdiv (2 ^ f.qneg) ∧ fits t ty n = true := by
  unfold toInt
  simp only [hty, ↓reduceIte]
  rw [QCore.FloatTypes.toInt_succeeds_iff]
  constructor
  · rintro ⟨m, e, hd, rfl, hfit⟩
    cases d with
    | nan => cases hd
    | inf _ => cases hd
    | fin s N =>
      simp only [Datum.toFValue, QCore.FloatTypes.FValue.finite.injEq] at hd
      obtain ⟨rfl, rfl⟩ := hd
      exact ⟨s, N, rfl, truncDyadic_fin f hq s N, hfit⟩
  · rintro ⟨s, N, rfl, rfl, hfit⟩
    exact ⟨_, _, rfl, (truncDyadic_fin f hq s N).symm, hfit⟩

/-- **The `OutOfRange` condition.** Float → integer fails exactly for NaN,
an infinity, or a finite value whose truncation is not a value of the
target. -/
theorem toInt_none_iff (f : Format) {t : Option Width} {ty : Ty} {d : Datum} (hq : 0 < f.qneg)
    (hty : ty ≠ .bool) :
    toInt f t ty d = none ↔
      d = .nan ∨ (∃ s, d = .inf s) ∨
        ∃ s N, d = .fin s N ∧ fits t ty ((signed s N).tdiv (2 ^ f.qneg)) = false := by
  cases d with
  | nan => simp [toInt, hty, Datum.toFValue, QCore.FloatTypes.toInt]
  | inf s => simp [toInt, hty, Datum.toFValue, QCore.FloatTypes.toInt]
  | fin s N =>
    simp only [toInt, hty, ↓reduceIte, Datum.toFValue, QCore.FloatTypes.toInt, truncDyadic_fin f hq,
      reduceCtorEq, false_or, exists_false, Datum.fin.injEq]
    cases s <;> cases hfit : fits t ty _ <;> simp [hfit]

/-! ## Patterns -/

/-- `scalar_floats::same_value`: equal patterns, or both NaN. -/
def sameValue (f : Format) (a b : Nat) : Bool :=
  a == b || ((decode f a).isNaN && (decode f b).isNaN)

/-- Unary minus on a pattern: the sign bit flipped (`scalar_floats::negate`). -/
def negBits (f : Format) (b : Nat) : Nat := if f.signBit ≤ b then b - f.signBit else b + f.signBit

/-- An operation on patterns: decode, operate, encode. -/
def arithBits (f : Format) (op : ArithOp) (a b : Nat) : Nat :=
  encode f (op.eval f (decode f a) (decode f b))

def cmpBits (f : Format) (c : Cmp) (a b : Nat) : Bool := c.eval f (decode f a) (decode f b)

def ofIntBits (f : Format) (n : Int) : Nat := encode f (ofInt f n)

def convertBits (src dst : Format) (a : Nat) : Nat := encode dst (convert src dst (decode src a))

def toIntBits (f : Format) (t : Option Width) (ty : Ty) (a : Nat) : Option Int :=
  toInt f t ty (decode f a)

/-- **Negation on patterns is negation on data**, NaN included. -/
theorem decode_negBits (f : Format) {b : Nat} (hb : b < 2 ^ f.width) :
    decode f (negBits f b) = neg (decode f b) := by
  rw [signBit_pow] at hb
  have hmod : negBits f b % f.signBit = b % f.signBit := by
    unfold negBits
    split
    · rename_i h
      rw [Nat.mod_eq_sub_mod h]
    · simp
  have hsign : decide (f.signBit ≤ negBits f b) = !decide (f.signBit ≤ b) := by
    unfold negBits
    split <;> rename_i h <;> simp [h] <;> omega
  unfold decode
  rw [hmod, hsign]
  split
  · simp [neg]
  · split <;> simp [neg]

/-- The negation of a valid pattern is a pattern of the format. -/
theorem negBits_lt (f : Format) {b : Nat} (hb : b < 2 ^ f.width) : negBits f b < 2 ^ f.width := by
  rw [signBit_pow] at hb ⊢
  unfold negBits
  split <;> omega

end QCore.Ieee754
