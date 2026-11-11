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

/-! IEEE 754 binary interchange formats (card `W48-QR-FLOAT-IEEE-LEAN-MODEL`).

The encoding half of the Lean model of `f32` and `f64` (IEEE 754-2019 §3.4,
binary32 and binary64): bit-pattern decoding and encoding with sign, biased
exponent and trailing significand fields, subnormals, both zeros, both
infinities and NaN (whose payload and sign the model does not specify).

**Units.** Every finite value of a format `f` is an integer multiple of the
least subnormal `2^(-f.qneg)` (`2^-149` for binary32, `2^-1074` for binary64).
A finite datum therefore carries its sign and its **magnitude in those units**,
a natural number `N`: its value is `(-1)^neg · N · 2^(-qneg)`. The magnitudes a
format represents are exactly the `N = M · 2^k` with `M < 2^p` and `N ≤ maxN`
(`Rep`); `encodeMag`/`decodeMag` are the bijection between them and the
magnitude bits `E · 2^tw + T` of the pattern.

Proved here: `decode_encode` (every valid datum, a NaN included, survives
encoding), `encode_decode` (every non-NaN pattern survives decoding),
`decode_valid`, `encode_lt`, and the class partition `classify_spec` (the five
classes read from the fields are exactly the decoded value's classes).
`QCore.Ieee754Round` rounds exact values to these formats, and
`QCore.Ieee754Ops` defines the operations. No project axiom is used. -/
namespace QCore.Ieee754

/-- A binary interchange format: `ew` exponent bits and `tw` trailing
significand bits (IEEE 754's `w` and `t`), precision `p = tw + 1`. -/
structure Format where
  ew : Nat
  tw : Nat
  deriving DecidableEq, Repr

/-- binary32, `f32`: 8 exponent bits, 23 trailing significand bits. -/
def binary32 : Format := ⟨8, 23⟩

/-- binary64, `f64`: 11 exponent bits, 52 trailing significand bits. -/
def binary64 : Format := ⟨11, 52⟩

namespace Format

/-- The precision `p`, significant bits of a normal value. -/
def p (f : Format) : Nat := f.tw + 1

/-- The width of a pattern: sign, exponent and trailing significand. -/
def width (f : Format) : Nat := 1 + f.ew + f.tw

/-- The sign bit's weight, `2^(ew + tw)`. -/
def signBit (f : Format) : Nat := 2 ^ (f.ew + f.tw)

/-- The all-ones biased exponent field (infinities and NaNs). -/
def ones (f : Format) : Nat := 2 ^ f.ew - 1

/-- The magnitude bits of an infinity: exponent all ones, fraction zero. -/
def infMag (f : Format) : Nat := f.ones * 2 ^ f.tw

/-- The largest scale `k` of a finite magnitude `M · 2^k`: the largest biased
exponent `2^ew - 2`, less one. -/
def kmax (f : Format) : Nat := 2 ^ f.ew - 3

/-- The largest finite magnitude, `(2^p - 1) · 2^kmax` units. -/
def maxN (f : Format) : Nat := (2 ^ f.p - 1) * 2 ^ f.kmax

/-- The least subnormal is `2^(-qneg)`: `qneg = bias + tw - 1` with
`bias = 2^(ew-1) - 1`. -/
def qneg (f : Format) : Nat := 2 ^ (f.ew - 1) + f.tw - 2

/-- The formats the model speaks about: at least two exponent bits and one
trailing significand bit (binary32 and binary64 are). -/
structure Valid (f : Format) : Prop where
  ew_ge : 2 ≤ f.ew
  tw_ge : 1 ≤ f.tw

end Format

theorem binary32_valid : binary32.Valid := ⟨by decide, by decide⟩
theorem binary64_valid : binary64.Valid := ⟨by decide, by decide⟩

/-- binary32's constants: 32 bits, precision 24, least subnormal `2^-149`,
largest finite value `(2^24 - 1) · 2^104`. -/
theorem binary32_constants :
    binary32.width = 32 ∧ binary32.p = 24 ∧ binary32.qneg = 149 ∧
      binary32.maxN = (2 ^ 24 - 1) * 2 ^ 253 := by decide +kernel

/-- binary64's constants: 64 bits, precision 53, least subnormal `2^-1074`,
largest finite value `(2^53 - 1) · 2^971`. -/
theorem binary64_constants :
    binary64.width = 64 ∧ binary64.p = 53 ∧ binary64.qneg = 1074 ∧
      binary64.maxN = (2 ^ 53 - 1) * 2 ^ 2045 := by decide +kernel

/-! ## Arithmetic helpers -/

theorem log2_mul_pow {M : Nat} (k : Nat) (h : M ≠ 0) : (M * 2 ^ k).log2 = M.log2 + k := by
  have hk : 2 ^ k ≠ 0 := Nat.pos_iff_ne_zero.mp (Nat.two_pow_pos k)
  rw [Nat.log2_eq_iff (Nat.mul_ne_zero h hk)]
  have h1 := Nat.log2_self_le h
  have h2 : M < 2 ^ (M.log2 + 1) := Nat.lt_log2_self
  constructor
  · rw [Nat.pow_add]; exact Nat.mul_le_mul_right _ h1
  · rw [show M.log2 + k + 1 = (M.log2 + 1) + k by omega, Nat.pow_add]
    exact Nat.mul_lt_mul_of_pos_right h2 (Nat.two_pow_pos k)

theorem log2_lt_of_lt {N k : Nat} (h : N < 2 ^ k) (hk : 0 < k) : N.log2 < k := by
  by_cases h0 : N = 0
  · subst h0; simp [Nat.log2_zero]; omega
  · exact (Nat.log2_lt h0).mpr h

theorem pow_pos_ne (k : Nat) : 2 ^ k ≠ 0 := Nat.pos_iff_ne_zero.mp (Nat.two_pow_pos k)

/-! ## A logarithm the kernel computes

`Nat.log2` is not one of the kernel's accelerated operations, so evaluating
it on the magnitudes of binary64 arithmetic (up to about `2^4200`) unfolds
thousands of recursive steps. `ilog2` finds the same number by a 16-step
binary search over `2^k ≤ n` (powers and comparisons are accelerated) and is
proved equal to `Nat.log2` (`ilog2_eq`); the model's definitions use it, and
every proof rewrites it away first. -/

/-- The binary search: with `2^acc ≤ n` (or `acc = 0`) and `n < 2^(acc + 2^k)`,
the largest `r` with `2^r ≤ n`. -/
def log2Search : Nat → Nat → Nat → Nat
  | 0, _, acc => acc
  | k + 1, n, acc =>
    if 2 ^ (acc + 2 ^ k) ≤ n then log2Search k n (acc + 2 ^ k) else log2Search k n acc

/-- `Nat.log2`, computed by `log2Search` below `2^65536`. -/
def ilog2 (n : Nat) : Nat := if n < 2 ^ 65536 then log2Search 16 n 0 else n.log2

theorem log2Search_spec (n : Nat) :
    ∀ k acc, (acc = 0 ∨ 2 ^ acc ≤ n) → n < 2 ^ (acc + 2 ^ k) →
      (log2Search k n acc = 0 ∨ 2 ^ log2Search k n acc ≤ n) ∧
        n < 2 ^ (log2Search k n acc + 1) := by
  intro k
  induction k with
  | zero => intro acc h1 h2; simpa [log2Search] using ⟨h1, h2⟩
  | succ k ih =>
    intro acc h1 h2
    have hk : 2 ^ (k + 1) = 2 ^ k + 2 ^ k := by rw [Nat.pow_succ]; omega
    simp only [log2Search]
    split
    · rename_i h
      exact ih _ (Or.inr h) (by rw [Nat.add_assoc, ← hk]; exact h2)
    · rename_i h
      exact ih _ h1 (Nat.not_le.mp h)

theorem ilog2_eq (n : Nat) : ilog2 n = n.log2 := by
  unfold ilog2
  split
  · rename_i hn
    obtain ⟨h1, h2⟩ := log2Search_spec n 16 0 (Or.inl rfl)
      (by rw [show 0 + 2 ^ 16 = 65536 from rfl]; exact hn)
    generalize log2Search 16 n 0 = r at h1 h2
    by_cases h0 : n = 0
    · subst h0
      rcases h1 with h1 | h1
      · rw [h1, Nat.log2_zero]
      · exact absurd h1 (by have := Nat.two_pow_pos r; omega)
    · rw [eq_comm, Nat.log2_eq_iff h0]
      rcases h1 with h1 | h1
      · subst h1; exact ⟨by omega, h2⟩
      · exact ⟨h1, h2⟩
  · rfl

/-! ## Magnitudes -/

/-- The scale of a magnitude's canonical form: `log2 N - tw`, zero for a
subnormal or the least binade. -/
def shift (f : Format) (N : Nat) : Nat := ilog2 N - f.tw

/-- The canonical significand `N / 2^shift`, the integer significand IEEE
754 writes as `1.T` (or `0.T` for a subnormal) scaled by `2^tw`. -/
def sig (f : Format) (N : Nat) : Nat := N / 2 ^ shift f N

/-- The magnitude bits `E · 2^tw + T` of a magnitude: `shift · 2^tw + sig`
(the implicit bit of a normal significand carries into the exponent field). -/
def encodeMag (f : Format) (N : Nat) : Nat := shift f N * 2 ^ f.tw + sig f N

/-- The magnitude a pattern's magnitude bits denote: `T` for a subnormal or
zero (`E = 0`), `(2^tw + T) · 2^(E-1)` for a normal value. -/
def decodeMag (f : Format) (m : Nat) : Nat :=
  if m / 2 ^ f.tw = 0 then m % 2 ^ f.tw
  else (2 ^ f.tw + m % 2 ^ f.tw) * 2 ^ (m / 2 ^ f.tw - 1)

/-- A magnitude with at most `p` significant bits, at any scale: the values
of the format with an unbounded exponent range. -/
def UnbRep (f : Format) (N : Nat) : Prop := ∃ M k, M < 2 ^ f.p ∧ N = M * 2 ^ k

/-- A magnitude of a finite value of the format. -/
def Rep (f : Format) (N : Nat) : Prop := UnbRep f N ∧ N ≤ f.maxN

theorem sig_lt (f : Format) (N : Nat) : sig f N < 2 ^ f.p := by
  simp only [sig, shift, ilog2_eq, Format.p]
  rw [Nat.div_lt_iff_lt_mul (Nat.two_pow_pos _), ← Nat.pow_add]
  exact Nat.lt_of_lt_of_le Nat.lt_log2_self (Nat.pow_le_pow_right (by decide) (by omega))

theorem sig_ge (f : Format) {N : Nat} (hN : N ≠ 0) (h : f.tw ≤ N.log2) :
    2 ^ f.tw ≤ sig f N := by
  simp only [sig, shift, ilog2_eq]
  rw [Nat.le_div_iff_mul_le (Nat.two_pow_pos _), ← Nat.pow_add,
    show f.tw + (N.log2 - f.tw) = N.log2 by omega]
  exact Nat.log2_self_le hN

theorem shift_of_lt (f : Format) {N : Nat} (h : N < 2 ^ f.p) : shift f N = 0 := by
  have := log2_lt_of_lt h (by unfold Format.p; omega)
  simp only [shift, ilog2_eq]; unfold Format.p at this; omega

theorem sig_of_lt (f : Format) {N : Nat} (h : N < 2 ^ f.p) : sig f N = N := by
  unfold sig; rw [shift_of_lt f h]; simp

/-- The canonical form of a representable magnitude is exact. -/
theorem sig_mul_shift {f : Format} {N : Nat} (h : UnbRep f N) :
    sig f N * 2 ^ shift f N = N := by
  obtain ⟨M, k, hM, rfl⟩ := h
  by_cases h0 : M = 0
  · subst h0; simp [sig, shift, ilog2_eq, Nat.log2_zero]
  · have hlog := log2_lt_of_lt hM (by unfold Format.p; omega)
    have hs : shift f (M * 2 ^ k) ≤ k := by
      simp only [shift, ilog2_eq]; rw [log2_mul_pow k h0]; unfold Format.p at hlog; omega
    unfold sig
    generalize shift f (M * 2 ^ k) = S at hs ⊢
    have hsplit : M * 2 ^ k = M * 2 ^ (k - S) * 2 ^ S := by
      rw [Nat.mul_assoc, ← Nat.pow_add, Nat.sub_add_cancel hs]
    rw [hsplit, Nat.mul_div_cancel _ (Nat.two_pow_pos _)]

theorem unbRep_iff {f : Format} {N : Nat} : UnbRep f N ↔ sig f N * 2 ^ shift f N = N :=
  ⟨sig_mul_shift, fun h => ⟨_, _, sig_lt f N, h.symm⟩⟩

/-- Every representable magnitude has scale at most `kmax`. -/
theorem shift_le_kmax {f : Format} {N : Nat} (h : Rep f N) : shift f N ≤ f.kmax := by
  by_cases h0 : N = 0
  · subst h0; simp [shift, ilog2_eq, Nat.log2_zero]
  · have hlt : N < 2 ^ (f.p + f.kmax) := by
      have := h.2
      unfold Format.maxN at this
      rw [Nat.pow_add]
      have hp : 0 < 2 ^ f.p := Nat.two_pow_pos _
      have : (2 ^ f.p - 1) * 2 ^ f.kmax < 2 ^ f.p * 2 ^ f.kmax :=
        Nat.mul_lt_mul_of_pos_right (by omega) (Nat.two_pow_pos _)
      omega
    have := (Nat.log2_lt h0).mpr hlt
    simp only [shift, ilog2_eq]; unfold Format.p at this; omega

theorem decodeMag_encodeMag {f : Format} {N : Nat} (h : UnbRep f N) :
    decodeMag f (encodeMag f N) = N := by
  have hc := sig_mul_shift h
  have hlt := sig_lt f N
  have hP : 0 < 2 ^ f.tw := Nat.two_pow_pos _
  have hp2 : 2 ^ f.p = 2 * 2 ^ f.tw := by unfold Format.p; rw [Nat.pow_succ]; omega
  unfold decodeMag encodeMag
  rw [Nat.add_comm (shift f N * 2 ^ f.tw), Nat.add_mul_div_right _ _ hP,
    Nat.add_mul_mod_self_right]
  by_cases hs : sig f N < 2 ^ f.tw
  · -- a subnormal, zero, or the first binade's significand: scale zero
    have hsh : shift f N = 0 := by
      by_cases h0 : N = 0
      · subst h0; simp [shift, ilog2_eq, Nat.log2_zero]
      · by_cases hl : f.tw ≤ N.log2
        · exact absurd (sig_ge f h0 hl) (by omega)
        · simp only [shift, ilog2_eq]; omega
    rw [Nat.div_eq_of_lt hs, Nat.mod_eq_of_lt hs, hsh]
    rw [hsh] at hc; simpa using hc
  · have hd : sig f N / 2 ^ f.tw = 1 := by
      apply Nat.le_antisymm
      · have : sig f N / 2 ^ f.tw < 2 := (Nat.div_lt_iff_lt_mul hP).mpr (by omega)
        omega
      · exact (Nat.le_div_iff_mul_le hP).mpr (by omega)
    have hm : sig f N % 2 ^ f.tw = sig f N - 2 ^ f.tw := by
      rw [Nat.mod_eq_sub_mod (by omega), Nat.mod_eq_of_lt (by omega)]
    rw [hd, hm]
    simp only [eq_false (show ¬ (1 + shift f N = 0) by omega), ↓reduceIte,
      show 1 + shift f N - 1 = shift f N by omega,
      show 2 ^ f.tw + (sig f N - 2 ^ f.tw) = sig f N by omega]
    exact hc

theorem decodeMag_rep {f : Format} (hf : f.Valid) {m : Nat} (hm : m < f.infMag) :
    Rep f (decodeMag f m) := by
  have hP : 0 < 2 ^ f.tw := Nat.two_pow_pos _
  have hp2 : 2 ^ f.p = 2 * 2 ^ f.tw := by unfold Format.p; rw [Nat.pow_succ]; omega
  have hT : m % 2 ^ f.tw < 2 ^ f.tw := Nat.mod_lt _ hP
  have hE : m / 2 ^ f.tw < f.ones := by
    rw [Nat.div_lt_iff_lt_mul hP]; exact hm
  have hones : f.ones = f.kmax + 2 := by
    unfold Format.ones Format.kmax
    have : 4 ≤ 2 ^ f.ew := by
      have := Nat.pow_le_pow_right (n := 2) (by decide) hf.ew_ge; simpa using this
    omega
  have hmax : 2 ^ f.p - 1 ≤ f.maxN := by
    unfold Format.maxN
    exact Nat.le_mul_of_pos_right _ (Nat.two_pow_pos _)
  unfold decodeMag
  split
  · exact ⟨⟨m % 2 ^ f.tw, 0, by omega, by simp⟩, by omega⟩
  · refine ⟨⟨2 ^ f.tw + m % 2 ^ f.tw, m / 2 ^ f.tw - 1, by omega, rfl⟩, ?_⟩
    unfold Format.maxN
    exact Nat.mul_le_mul (by omega) (Nat.pow_le_pow_right (by decide) (by omega))

theorem encodeMag_decodeMag (f : Format) (m : Nat) :
    encodeMag f (decodeMag f m) = m := by
  have hP : 0 < 2 ^ f.tw := Nat.two_pow_pos _
  have hT : m % 2 ^ f.tw < 2 ^ f.tw := Nat.mod_lt _ hP
  have hdm := Nat.div_add_mod m (2 ^ f.tw)
  unfold decodeMag
  split
  · rename_i hE
    have hp : m % 2 ^ f.tw < 2 ^ f.p :=
      Nat.lt_of_lt_of_le hT (Nat.pow_le_pow_right (by decide) (by unfold Format.p; omega))
    unfold encodeMag
    rw [shift_of_lt f hp, sig_of_lt f hp]
    rw [hE] at hdm; simp at hdm; simp [hdm]
  · rename_i hE
    have hlog : (2 ^ f.tw + m % 2 ^ f.tw).log2 = f.tw := by
      rw [Nat.log2_eq_iff (by omega)]
      constructor
      · omega
      · rw [Nat.pow_succ]; omega
    have hsh : shift f ((2 ^ f.tw + m % 2 ^ f.tw) * 2 ^ (m / 2 ^ f.tw - 1)) = m / 2 ^ f.tw - 1 := by
      simp only [shift, ilog2_eq]; rw [log2_mul_pow _ (by omega), hlog]; omega
    unfold encodeMag sig
    rw [hsh, Nat.mul_div_cancel _ (Nat.two_pow_pos _)]
    have hE1 : 1 ≤ m / 2 ^ f.tw := Nat.pos_of_ne_zero hE
    have : (m / 2 ^ f.tw - 1) * 2 ^ f.tw + 2 ^ f.tw = m / 2 ^ f.tw * 2 ^ f.tw := by
      rw [← Nat.succ_mul, Nat.succ_eq_add_one, Nat.sub_add_cancel hE1]
    rw [← Nat.add_assoc, this, Nat.mul_comm]
    exact hdm

theorem encodeMag_lt {f : Format} (hf : f.Valid) {N : Nat} (h : Rep f N) :
    encodeMag f N < f.infMag := by
  have hP : 0 < 2 ^ f.tw := Nat.two_pow_pos _
  have hk := shift_le_kmax h
  have hlt := sig_lt f N
  have hp2 : 2 ^ f.p = 2 * 2 ^ f.tw := by unfold Format.p; rw [Nat.pow_succ]; omega
  have hones : f.ones = f.kmax + 2 := by
    unfold Format.ones Format.kmax
    have : 4 ≤ 2 ^ f.ew := by
      have := Nat.pow_le_pow_right (n := 2) (by decide) hf.ew_ge; simpa using this
    omega
  unfold encodeMag Format.infMag
  rw [hones]
  have : shift f N * 2 ^ f.tw ≤ f.kmax * 2 ^ f.tw := Nat.mul_le_mul_right _ hk
  rw [Nat.add_mul]
  omega

/-! ## Data and patterns -/

/-- An IEEE datum: a NaN (payload and sign unspecified), a signed infinity, or
a signed finite magnitude in units of the least subnormal (both zeros are
`fin _ 0`). -/
inductive Datum where
  | nan
  | inf (neg : Bool)
  | fin (neg : Bool) (mag : Nat)
  deriving DecidableEq, Repr

/-- A datum of the format: a finite magnitude must be representable. -/
def Datum.Valid (f : Format) : Datum → Prop
  | .nan => True
  | .inf _ => True
  | .fin _ N => Rep f N

/-- The sign bit's contribution to a pattern. -/
def signPart (f : Format) (neg : Bool) : Nat := if neg then f.signBit else 0

/-- The quiet NaN the model encodes every NaN as (exponent all ones, top
fraction bit set, sign clear): `0x7FC00000` and `0x7FF8000000000000`. -/
def quietNaN (f : Format) : Nat := f.infMag + 2 ^ (f.tw - 1)

/-- The pattern of a datum. -/
def encode (f : Format) : Datum → Nat
  | .nan => quietNaN f
  | .inf s => signPart f s + f.infMag
  | .fin s N => signPart f s + encodeMag f N

/-- The datum a pattern denotes (the sign is bit `ew + tw`; the magnitude bits
are the rest). -/
def decode (f : Format) (b : Nat) : Datum :=
  if b % f.signBit < f.infMag then .fin (decide (f.signBit ≤ b)) (decodeMag f (b % f.signBit))
  else if b % f.signBit = f.infMag then .inf (decide (f.signBit ≤ b))
  else .nan

theorem quietNaN_binary32 : quietNaN binary32 = 0x7FC00000 := by decide
theorem quietNaN_binary64 : quietNaN binary64 = 0x7FF8000000000000 := by decide

theorem infMag_lt_signBit (f : Format) : f.infMag < f.signBit := by
  unfold Format.infMag Format.signBit Format.ones
  rw [Nat.pow_add]
  exact Nat.mul_lt_mul_of_pos_right (by have := Nat.two_pow_pos f.ew; omega) (Nat.two_pow_pos _)

theorem quietNaN_lt_signBit {f : Format} (hf : f.Valid) : quietNaN f < f.signBit := by
  have htw : 2 ^ f.tw = 2 * 2 ^ (f.tw - 1) := by
    rw [← Nat.pow_succ']; congr 1; have := hf.tw_ge; omega
  have h1 : 1 ≤ 2 ^ f.ew := Nat.two_pow_pos _
  unfold quietNaN Format.infMag Format.signBit Format.ones
  rw [Nat.pow_add, Nat.sub_mul, Nat.one_mul]
  have : 2 ^ f.tw ≤ 2 ^ f.ew * 2 ^ f.tw := Nat.le_mul_of_pos_left _ h1
  have := Nat.two_pow_pos (f.tw - 1)
  omega

theorem signBit_pow (f : Format) : 2 ^ f.width = 2 * f.signBit := by
  unfold Format.width Format.signBit
  rw [show 1 + f.ew + f.tw = (f.ew + f.tw) + 1 by omega, Nat.pow_succ, Nat.mul_comm]

/-- A pattern below the sign bit plus a sign decodes to that sign and those
magnitude bits. -/
theorem decode_signPart (f : Format) (s : Bool) {m : Nat} (hm : m < f.signBit) :
    (signPart f s + m) % f.signBit = m ∧ decide (f.signBit ≤ signPart f s + m) = s := by
  cases s <;> simp [signPart, Nat.mod_eq_of_lt hm, Nat.not_le.mpr hm]

/-- **Decode after encode.** Every valid datum is recovered from its pattern;
a NaN is recovered as a NaN. -/
theorem decode_encode {f : Format} (hf : f.Valid) {d : Datum} (hd : d.Valid f) :
    decode f (encode f d) = d := by
  have hinf := infMag_lt_signBit f
  cases d with
  | nan =>
    have hq := quietNaN_lt_signBit hf
    have : f.infMag < quietNaN f := by unfold quietNaN; have := Nat.two_pow_pos (f.tw - 1); omega
    simp only [decode, encode, Nat.mod_eq_of_lt hq, Nat.not_lt.mpr (Nat.le_of_lt this),
      Nat.ne_of_gt this, ↓reduceIte]
  | inf s =>
    obtain ⟨h1, h2⟩ := decode_signPart f s hinf
    unfold decode encode
    simp [h1, h2]
  | fin s N =>
    have hlt := encodeMag_lt hf hd
    obtain ⟨h1, h2⟩ := decode_signPart f s (Nat.lt_trans hlt hinf)
    unfold decode encode
    simp [h1, h2, hlt, decodeMag_encodeMag hd.1]

/-- **Encode after decode.** Every pattern of the format's width that is not
a NaN is its datum's encoding. -/
theorem encode_decode {f : Format} {b : Nat} (hb : b < 2 ^ f.width) (hn : decode f b ≠ .nan) :
    encode f (decode f b) = b := by
  rw [signBit_pow] at hb
  have hsp : signPart f (decide (f.signBit ≤ b)) + b % f.signBit = b := by
    by_cases hs : f.signBit ≤ b
    · have : b % f.signBit = b - f.signBit := by
        rw [Nat.mod_eq_sub_mod hs, Nat.mod_eq_of_lt (by omega)]
      simp [signPart, hs, this] <;> omega
    · simp [signPart, hs, Nat.mod_eq_of_lt (Nat.not_le.mp hs)]
  unfold decode at hn ⊢
  split
  · rename_i hm
    simp only [encode]; rw [encodeMag_decodeMag]; exact hsp
  · split
    · rename_i _ he
      simp only [encode]; rw [← he]; exact hsp
    · simp_all

/-- Every decoded datum is valid. -/
theorem decode_valid {f : Format} (hf : f.Valid) (b : Nat) : (decode f b).Valid f := by
  unfold decode
  split
  · exact decodeMag_rep hf (by assumption)
  · split <;> trivial

/-- Every valid datum's pattern has the format's width. -/
theorem encode_lt {f : Format} (hf : f.Valid) {d : Datum} (hd : d.Valid f) :
    encode f d < 2 ^ f.width := by
  rw [signBit_pow]
  have hinf := infMag_lt_signBit f
  cases d with
  | nan => simp only [encode]; have := quietNaN_lt_signBit hf; omega
  | inf s => cases s <;> simp [encode, signPart] <;> omega
  | fin s N =>
    have := encodeMag_lt hf hd
    cases s <;> simp [encode, signPart] <;> omega

/-! ## Classes -/

/-- The five classes of IEEE 754 §3.4 a pattern's fields select. -/
inductive FClass where
  | zero | subnormal | normal | infinite | nan
  deriving DecidableEq, Repr

/-- The biased exponent field. -/
def expField (f : Format) (b : Nat) : Nat := b % f.signBit / 2 ^ f.tw

/-- The trailing significand field. -/
def fracField (f : Format) (b : Nat) : Nat := b % 2 ^ f.tw

/-- The class of a pattern, read from its fields alone. -/
def classify (f : Format) (b : Nat) : FClass :=
  if expField f b = 0 then (if fracField f b = 0 then .zero else .subnormal)
  else if expField f b = f.ones then (if fracField f b = 0 then .infinite else .nan)
  else .normal

theorem fracField_eq (f : Format) (b : Nat) : fracField f b = b % f.signBit % 2 ^ f.tw := by
  unfold fracField Format.signBit
  rw [Nat.mod_mod_of_dvd _ (Nat.pow_dvd_pow 2 (by omega))]

theorem decodeMag_zero_iff (f : Format) (m : Nat) :
    decodeMag f m = 0 ↔ m / 2 ^ f.tw = 0 ∧ m % 2 ^ f.tw = 0 := by
  unfold decodeMag
  split
  · simp_all
  · rename_i h
    simp only [h, false_and, iff_false]
    exact Nat.mul_ne_zero (by have := Nat.two_pow_pos f.tw; omega) (pow_pos_ne _)

theorem decodeMag_lt_iff (f : Format) (m : Nat) :
    decodeMag f m < 2 ^ f.tw ↔ m / 2 ^ f.tw = 0 := by
  unfold decodeMag
  split
  · simp_all [Nat.mod_lt _ (Nat.two_pow_pos _)]
  · rename_i h
    simp only [h, iff_false, Nat.not_lt]
    exact Nat.le_trans (Nat.le_add_right _ _) (Nat.le_mul_of_pos_right _ (Nat.two_pow_pos _))

/-- **The class partition.** The class a pattern's fields select is exactly
the class of the datum it decodes to: zero, subnormal (`0 < N < 2^tw`),
normal (`2^tw ≤ N`), infinite, or NaN. `classify` is a function, so every
pattern has exactly one class. -/
theorem classify_spec {f : Format} (hf : f.Valid) (b : Nat) :
    (classify f b = .zero ↔ ∃ s, decode f b = .fin s 0) ∧
    (classify f b = .subnormal ↔ ∃ s N, decode f b = .fin s N ∧ 0 < N ∧ N < 2 ^ f.tw) ∧
    (classify f b = .normal ↔ ∃ s N, decode f b = .fin s N ∧ 2 ^ f.tw ≤ N) ∧
    (classify f b = .infinite ↔ ∃ s, decode f b = .inf s) ∧
    (classify f b = .nan ↔ decode f b = .nan) := by
  have hP : 0 < 2 ^ f.tw := Nat.two_pow_pos _
  have hdm := Nat.div_add_mod (b % f.signBit) (2 ^ f.tw)
  have hones : 3 ≤ f.ones := by
    unfold Format.ones
    have := Nat.pow_le_pow_right (n := 2) (by decide) hf.ew_ge; simp at this; omega
  have hlt : b % f.signBit < f.infMag ↔ expField f b < f.ones := by
    unfold expField Format.infMag
    rw [Nat.div_lt_iff_lt_mul hP]
  have heq : b % f.signBit = f.infMag ↔ expField f b = f.ones ∧ fracField f b = 0 := by
    rw [fracField_eq]
    unfold expField Format.infMag
    constructor
    · intro h; rw [h, Nat.mul_div_cancel _ hP, Nat.mul_mod_left]; exact ⟨rfl, rfl⟩
    · rintro ⟨h1, h2⟩; rw [h1, h2] at hdm; rw [← hdm]; simp [Nat.mul_comm]
  have hbound : expField f b ≤ f.ones := by
    unfold expField Format.ones
    have := Nat.mod_lt b (Nat.two_pow_pos (f.ew + f.tw))
    unfold Format.signBit at *
    have : b % 2 ^ (f.ew + f.tw) / 2 ^ f.tw < 2 ^ f.ew := by
      rw [Nat.div_lt_iff_lt_mul hP, ← Nat.pow_add]; exact this
    omega
  have hz := decodeMag_zero_iff f (b % f.signBit)
  have hs := decodeMag_lt_iff f (b % f.signBit)
  rw [← fracField_eq] at hz
  change _ ↔ expField f b = 0 at hs
  change _ ↔ expField f b = 0 ∧ _ at hz
  unfold classify decode
  generalize expField f b = E at *
  generalize fracField f b = F at *
  generalize decodeMag f (b % f.signBit) = N at *
  generalize decide (f.signBit ≤ b) = s
  generalize b % f.signBit = m at *
  have ho : f.ones ≠ 0 := by omega
  by_cases hE0 : E = 0
  · have hm : m < f.infMag := hlt.mpr (by omega)
    by_cases hF : F = 0
    · have hN : N = 0 := hz.mpr ⟨hE0, hF⟩
      subst hE0 hF hN
      simp [hm, hP]
    · have hN : N ≠ 0 := fun h => hF (hz.mp h).2
      have hN2 : N < 2 ^ f.tw := hs.mpr hE0
      subst hE0
      simp [hm, hF, hN, hN2]
      cases s <;> simp [Nat.pos_of_ne_zero hN, hN2]
  · by_cases hEo : E = f.ones
    · have hm : ¬ m < f.infMag := fun h => by have := hlt.mp h; omega
      by_cases hF : F = 0
      · have he : m = f.infMag := heq.mpr ⟨hEo, hF⟩
        simp [hEo, hF, he, ho]
      · have he : m ≠ f.infMag := fun h => hF (heq.mp h).2
        simp [hEo, hF, he, hm, ho]
    · have hm : m < f.infMag := hlt.mpr (by omega)
      have hN : N ≠ 0 := fun h => hE0 (hz.mp h).1
      have hge : 2 ^ f.tw ≤ N := Nat.not_lt.mp (fun h => hE0 (hs.mp h))
      simp [hE0, hEo, hm, hN, hge, Nat.pos_of_ne_zero hN]
      cases s <;> simp [hge]

end QCore.Ieee754
