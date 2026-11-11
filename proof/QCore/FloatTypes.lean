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

/-! Floating-point values of `.qr` sources (card `W48-QR-FLOATING-POINT`).

The Lean statement of the parts of `crates/qcore-core/src/scalar_floats.rs` and
of `source.scalar-calls.v1`'s effect check that the card proves:

* **The comparison effects.** A function's float comparisons need
  `partial_eq` (`==`, `!=`) or `partial_ord` (the orderings) in its declared
  row; they are never inferred, and a caller declares every effect its callees
  derive. `no_undeclared_float_comparison`: an admitted function whose row
  lacks an effect reaches no float comparison needing it, through any chain of
  calls. `caller_declares_callee_effects`: effects propagate through calls.
* **Float → integer.** `toInt` truncates a finite value toward zero and fails
  exactly when the truncation is not a value of the target; NaN and ±inf always
  fail (`toInt_succeeds_iff`).
* **Integer → float.** `roundInt p n` rounds to `p` significant bits, to
  nearest, ties to even (`roundNat_error_le_half`, `roundInt_exact_small`).

**What is modelled, and what is not.** A float is modelled abstractly as NaN,
a signed infinity or a finite dyadic `m · 2^e` (`FValue`); nothing here is an
IEEE formalisation. IEEE arithmetic, comparison and `f64 → f32` are the host's
operations in the Rust reference, and they are compared with the emitted C by
native execution on the card's boundary table — observation, not proof. The
two tables `roundingVectors` and `truncationVectors` below bind these
definitions to the Rust kernel: the Rust test
`lean_float_vectors_match_the_rust_kernel` parses and replays both against
`qcore_core::scalar_types::convert`. Neither is an extraction or refinement
theorem. No project axiom is used. -/
namespace QCore.FloatTypes

open QCore.ScalarTypes (Ty Width fits fits_bounds fits_of_bounds)

/-! ## The comparison effects -/

/-- The two comparison effects. `partialOrd` does not imply `partialEq`. -/
inductive Effect where
  | partialEq | partialOrd
  deriving DecidableEq, Repr

/-- One site of a function body the effect check reads: a float comparison
needing an effect, an integer or `bool` comparison (ungated), or a call. -/
inductive Site where
  | floatCmp (e : Effect)
  | intCmp
  | call (g : Nat)
  deriving DecidableEq, Repr

/-- A function: its declared row's comparison effects, and its body's sites. -/
structure Fn where
  declared : List Effect
  body : List Site

/-- What a function reaches, as the least relation: a float comparison in its
own body, or one a function it calls reaches. This is the semantic property
the effect row is about. -/
inductive Reaches (p : List Fn) : Nat → Effect → Prop where
  | here {i : Nat} {f : Fn} {e : Effect} :
      p[i]? = some f → Site.floatCmp e ∈ f.body → Reaches p i e
  | through {i g : Nat} {f : Fn} {e : Effect} :
      p[i]? = some f → Site.call g ∈ f.body → Reaches p g e → Reaches p i e

/-- The checker's rule, as the Rust route applies it: every float comparison
of a function's own body is declared by its row (checked while the body is
typed), and every effect a callee reaches is declared by the caller's row
(checked in call-graph order, callees first). -/
def Admitted (p : List Fn) : Prop :=
  ∀ (i : Nat) (f : Fn), p[i]? = some f →
    (∀ e : Effect, Site.floatCmp e ∈ f.body → e ∈ f.declared) ∧
    (∀ (g : Nat) (e : Effect), Site.call g ∈ f.body → Reaches p g e → e ∈ f.declared)

/-- Every effect an admitted function reaches is declared by its row. -/
theorem reached_is_declared {p : List Fn} (h : Admitted p) {i : Nat} {e : Effect}
    (r : Reaches p i e) : ∀ f, p[i]? = some f → e ∈ f.declared := by
  cases r with
  | here hf hs =>
    intro f' hf'
    rw [hf] at hf'
    cases hf'
    exact (h _ _ hf).1 _ hs
  | through hf hs hg =>
    intro f' hf'
    rw [hf] at hf'
    cases hf'
    exact (h _ _ hf).2 _ _ hs hg

/-- **No undeclared float comparison.** An admitted function whose row does not
declare an effect reaches no float comparison needing it, in its own body or
through any chain of calls. -/
theorem no_undeclared_float_comparison {p : List Fn} (h : Admitted p) {i : Nat} {f : Fn}
    {e : Effect} (hf : p[i]? = some f) (hn : e ∉ f.declared) : ¬ Reaches p i e :=
  fun r => hn (reached_is_declared h r f hf)

/-- **Propagation through calls.** A caller declares every effect a function
it calls reaches. -/
theorem caller_declares_callee_effects {p : List Fn} (h : Admitted p) {i g : Nat} {f : Fn}
    {e : Effect} (hf : p[i]? = some f) (hc : Site.call g ∈ f.body) (hr : Reaches p g e) :
    e ∈ f.declared :=
  (h _ _ hf).2 _ _ hc hr

/-- A function without `partial_eq` in an admitted program contains no float
`==`/`!=` of its own. -/
theorem no_equality_without_partial_eq {p : List Fn} (h : Admitted p) {i : Nat} {f : Fn}
    (hf : p[i]? = some f) (hn : Effect.partialEq ∉ f.declared) :
    Site.floatCmp .partialEq ∉ f.body :=
  fun hs => no_undeclared_float_comparison h hf hn (.here hf hs)

/-- A function without `partial_ord` in an admitted program contains no float
ordering of its own. -/
theorem no_ordering_without_partial_ord {p : List Fn} (h : Admitted p) {i : Nat} {f : Fn}
    (hf : p[i]? = some f) (hn : Effect.partialOrd ∉ f.declared) :
    Site.floatCmp .partialOrd ∉ f.body :=
  fun hs => no_undeclared_float_comparison h hf hn (.here hf hs)

/-- `partial_ord` does not admit `==`: a function declaring only `partial_ord`
whose body compares floats for equality is not admitted. -/
theorem partial_ord_does_not_admit_equality :
    ¬ Admitted [{ declared := [.partialOrd], body := [.floatCmp .partialEq] }] := by
  intro h
  have := (h 0 _ rfl).1 .partialEq (by simp)
  simp at this

/-- A caller that does not declare its callee's effect is not admitted. -/
theorem undeclared_callee_effect_is_refused :
    ¬ Admitted [{ declared := [], body := [.call 1] },
                { declared := [.partialOrd], body := [.floatCmp .partialOrd] }] := by
  intro h
  have := (h 0 _ rfl).2 1 .partialOrd (by simp) (.here rfl (by simp))
  simp at this

/-- Integer and `bool` comparisons are ungated: a program of them, with empty
rows and calls between its functions, is admitted. -/
theorem integer_comparisons_need_no_effect :
    Admitted [{ declared := [], body := [.intCmp, .call 1] },
              { declared := [], body := [.intCmp] }] := by
  have one : ∀ e, ¬ Reaches [{ declared := [], body := [.intCmp, .call 1] },
      { declared := [], body := [.intCmp] }] 1 e := by
    intro e r
    cases r with
    | here hf hs =>
      simp at hf
      subst hf
      simp at hs
    | through hf hs _ =>
      simp at hf
      subst hf
      simp at hs
  intro i f hf
  constructor
  · intro e hs
    match i, hf with
    | 0, hf => simp at hf; subst hf; simp at hs
    | 1, hf => simp at hf; subst hf; simp at hs
    | _ + 2, hf => simp at hf
  · intro g e hs hr
    match i, hf with
    | 0, hf =>
      simp at hf
      subst hf
      simp at hs
      subst hs
      exact absurd hr (one e)
    | 1, hf => simp at hf; subst hf; simp at hs
    | _ + 2, hf => simp at hf

/-! ## Float → integer -/

/-- A float value, abstractly: NaN, a signed infinity, or a finite `m · 2^e`
(every finite binary32/binary64 value is one, signed zeros both `0 · 2^e`). -/
inductive FValue where
  | nan
  | inf (negative : Bool)
  | finite (m : Int) (e : Int)

/-- `m · 2^e` truncated toward zero: exact when `e ≥ 0`, `Int.tdiv` otherwise. -/
def truncDyadic (m e : Int) : Int :=
  if 0 ≤ e then m * 2 ^ e.toNat else m.tdiv (2 ^ (-e).toNat)

/-- `x as ty` for a float `x`: the truncation when it is a value of `ty`;
`none` — the typed `OutOfRange` — for NaN, ±inf and a truncation that does not
fit. -/
def toInt (t : Option Width) (ty : Ty) : FValue → Option Int
  | .finite m e => if fits t ty (truncDyadic m e) = true then some (truncDyadic m e) else none
  | _ => none

/-- **Float → integer succeeds exactly when the truncated value fits.** -/
theorem toInt_succeeds_iff {t : Option Width} {ty : Ty} {x : FValue} {n : Int} :
    toInt t ty x = some n ↔
      ∃ m e, x = .finite m e ∧ n = truncDyadic m e ∧ fits t ty n = true := by
  constructor
  · intro h
    cases x with
    | nan => simp [toInt] at h
    | inf _ => simp [toInt] at h
    | finite m e =>
      simp only [toInt] at h
      split at h
      · rename_i hfit
        cases h
        exact ⟨m, e, rfl, rfl, hfit⟩
      · contradiction
  · rintro ⟨m, e, rfl, rfl, hfit⟩
    simp [toInt, hfit]

/-- NaN never converts to an integer. -/
theorem toInt_nan (t : Option Width) (ty : Ty) : toInt t ty .nan = none := rfl

/-- Neither infinity converts to an integer. -/
theorem toInt_inf (t : Option Width) (ty : Ty) (negative : Bool) :
    toInt t ty (.inf negative) = none := rfl

/-- A successful conversion's result is a value of the target. -/
theorem toInt_in_range {t : Option Width} {ty : Ty} {x : FValue} {n : Int}
    (h : toInt t ty x = some n) : ∃ lo hi, ty.range t = some (lo, hi) ∧ lo ≤ n ∧ n ≤ hi := by
  obtain ⟨_, _, _, _, hfit⟩ := toInt_succeeds_iff.mp h
  exact fits_bounds hfit

/-- A value with a non-negative exponent is an integer, converted exactly. -/
theorem truncDyadic_exact {m e : Int} (h : 0 ≤ e) : truncDyadic m e = m * 2 ^ e.toNat := by
  simp [truncDyadic, h]

/-- Without a recorded target no float converts to `isize` or `usize`. -/
theorem toInt_needs_target (x : FValue) :
    toInt none .isize x = none ∧ toInt none .usize x = none := by
  cases x <;> simp [toInt, fits, Ty.range]

/-! ## Integer → float -/

/-- `n` rounded to `p` significant bits, to nearest, ties to even (`p ≥ 1`):
exact when `n` has at most `p` bits; otherwise the top `p` bits `q`, plus one
when the dropped part `r` exceeds half a unit of the last place, or equals it
and `q` is odd. -/
def roundNat (p n : Nat) : Nat :=
  if n.log2 + 1 ≤ p then n
  else
    let s := n.log2 + 1 - p
    let q := n / 2 ^ s
    let r := n % 2 ^ s
    if 2 ^ (s - 1) < r ∨ (r = 2 ^ (s - 1) ∧ q % 2 = 1) then (q + 1) * 2 ^ s else q * 2 ^ s

/-- An integer rounded to `p` significant bits, the sign kept. -/
def roundInt (p : Nat) (n : Int) : Int :=
  if n < 0 then -((roundNat p n.natAbs : Nat) : Int) else ((roundNat p n.natAbs : Nat) : Int)

/-- An integer of at most `p` bits is converted exactly. -/
theorem roundNat_exact_small {p n : Nat} (h : n.log2 + 1 ≤ p) : roundNat p n = n := by
  simp [roundNat, h]

/-- **Round to nearest.** The rounded value is within half a unit of the last
kept place of `n`: `|roundNat p n - n| ≤ 2^(s-1)` when `s` bits are dropped. -/
theorem roundNat_error_le_half {p n s : Nat} (hp : p < n.log2 + 1) (hs : n.log2 + 1 - p = s) :
    roundNat p n ≤ n + 2 ^ (s - 1) ∧ n ≤ roundNat p n + 2 ^ (s - 1) := by
  have h1 : 1 ≤ s := by omega
  have hpow : 2 ^ s = 2 * 2 ^ (s - 1) := by
    rw [← Nat.pow_succ']
    congr 1
    omega
  have hpos : 0 < 2 ^ s := Nat.two_pow_pos s
  have hdiv := Nat.div_add_mod n (2 ^ s)
  have hlt : n % 2 ^ s < 2 ^ s := Nat.mod_lt _ hpos
  have hnot : ¬ (n.log2 + 1 ≤ p) := by omega
  simp only [roundNat, hnot, ite_false, hs]
  generalize n / 2 ^ s = q at *
  generalize n % 2 ^ s = r at *
  generalize 2 ^ (s - 1) = half at *
  generalize 2 ^ s = S at *
  have hmul : (q + 1) * S = S * q + S := by rw [Nat.succ_mul, Nat.mul_comm]
  split
  · rw [hmul]
    constructor <;> omega
  · rw [Nat.mul_comm q S]
    constructor <;> omega

/-- An integer of at most `p` bits keeps its value, sign included. -/
theorem roundInt_exact_small {p : Nat} {n : Int} (h : n.natAbs.log2 + 1 ≤ p) :
    roundInt p n = n := by
  unfold roundInt
  rw [roundNat_exact_small h]
  split <;> omega

/-- Rounding is symmetric: `-n` rounds to the negation of `n`'s rounding. -/
theorem roundInt_neg (p : Nat) (n : Int) (h : n ≠ 0) : roundInt p (-n) = -(roundInt p n) := by
  unfold roundInt
  rw [Int.natAbs_neg]
  split <;> split <;> omega

/-! ## The vectors the Rust kernel replays -/

/-- `(precision, n, roundInt precision n)`: binary32 (24) and binary64 (53)
around `2^24`, `2^25`, `2^53` and the 64-bit limits, both signs, ties to even
in both directions. -/
def roundingVectors : List (Nat × Int × Int) := [
  (24, 16777215, 16777215),
  (24, 16777216, 16777216),
  (24, 16777217, 16777216),
  (24, 16777219, 16777220),
  (24, -16777217, -16777216),
  (24, -16777219, -16777220),
  (24, 33554434, 33554432),
  (24, 33554435, 33554436),
  (24, 33554438, 33554440),
  (24, 2147483647, 2147483648),
  (24, -2147483648, -2147483648),
  (24, 18446744073709551615, 18446744073709551616),
  (53, 9007199254740991, 9007199254740991),
  (53, 9007199254740993, 9007199254740992),
  (53, 9007199254740995, 9007199254740996),
  (53, -9007199254740993, -9007199254740992),
  (53, 18014398509481986, 18014398509481984),
  (53, 18014398509481990, 18014398509481992),
  (53, 9223372036854775807, 9223372036854775808),
  (53, -9223372036854775808, -9223372036854775808),
  (53, 18446744073709551615, 18446744073709551616)
]

theorem rounding_vectors_hold :
    roundingVectors.all (fun (p, n, r) => roundInt p n == r) = true := by decide

/-- `(m, e, target, fits, result)`: `toInt none target (m · 2^e)`, with
`fits = false` (and result `0`) for the typed `OutOfRange`. -/
def truncationVectors : List (Int × Int × Ty × Bool × Int) := [
  (3, -1, .i8, true, 1),
  (-3, -1, .i8, true, -1),
  (5, -2, .i32, true, 1),
  (-7, -2, .i32, true, -1),
  (255, 0, .u8, true, 255),
  (256, 0, .u8, false, 0),
  (-1, 0, .u8, false, 0),
  (-1, -1, .u8, true, 0),
  (257, -1, .i8, false, 0),
  (-257, -1, .i8, true, -128),
  (1, 63, .i64, false, 0),
  (-1, 63, .i64, true, -9223372036854775808),
  (1, 64, .u64, false, 0),
  (9007199254740991, 11, .u64, true, 18446744073709549568),
  (1, 31, .i32, false, 0),
  (-1, 31, .i32, true, -2147483648),
  (1, -60, .i32, true, 0)
]

theorem truncation_vectors_hold :
    truncationVectors.all (fun (m, e, ty, ok, n) =>
      toInt none ty (.finite m e) == (if ok then some n else none)) = true := by decide

end QCore.FloatTypes
