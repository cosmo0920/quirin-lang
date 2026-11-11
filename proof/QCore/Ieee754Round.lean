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

import QCore.Ieee754

/-! Round to nearest, ties to even (card `W48-QR-FLOAT-IEEE-LEAN-MODEL`).

IEEE 754-2019 §4.3.1 `roundTiesToEven` of an exact non-negative rational
`x = n / d` (in units of the format's least subnormal, `d > 0`) to a format,
with subnormal rounding (below `2^p` units the grid is every unit) and the
overflow rule of §7.4 (a result that, rounded with an unbounded exponent range,
exceeds the largest finite magnitude becomes infinity).

Distances are measured exactly: `dist n d N = |n - N·d| = |x - N| · d`, so
comparing two candidates' `dist` for the same `x` compares their distances to
`x`. Proved:

* `roundMag_rep` — the result is a finite value of the format;
* `roundMag_nearest` — no finite value of the format is nearer to `x`;
* `roundMag_tie_even` — at a tie the result's significand is even, and so is
  its pattern (`encodeMag_even`);
* `roundMag_none_iff` — overflow to infinity exactly when
  `x ≥ maxN + 2^(kmax-1)`, half a unit in the last place above the largest
  finite value;
* `roundMag_mono`, `roundMag_mono_none` — rounding is monotone;
* `roundMag_exact` — a representable `x` rounds to itself.

The same statements for the unbounded exponent range (`roundUnb_*`) carry the
proofs. No project axiom is used. -/
namespace QCore.Ieee754

/-- `|n - N·d|`: the distance from `x = n/d` to `N`, times `d`. -/
def dist (n d N : Nat) : Nat := if N * d ≤ n then n - N * d else N * d - n

/-- The scale of `x`'s rounding: `log2 ⌊x⌋ + 1 - p`, zero below `2^p` units
(the subnormal and least normal binades share the unit grid). -/
def rshift (f : Format) (n d : Nat) : Nat := (ilog2 (n / d) + 1) - f.p

/-- The unit in the last place of `x`'s rounding, times `d`: `d · 2^s`. -/
def ru (f : Format) (n d : Nat) : Nat := d * 2 ^ rshift f n d

/-- The truncated quotient `q = ⌊x / 2^s⌋`. -/
def rq (f : Format) (n d : Nat) : Nat := n / ru f n d

/-- The remainder `n - q · d · 2^s`. -/
def rr (f : Format) (n d : Nat) : Nat := n % ru f n d

/-- `x = n/d` rounded to nearest, ties to even, to `p` significant bits with
an unbounded exponent range: `q · 2^s` or `(q + 1) · 2^s`, whichever is
nearer, the even `q` at a tie. -/
def roundUnb (f : Format) (n d : Nat) : Nat :=
  if ru f n d < 2 * rr f n d ∨ (2 * rr f n d = ru f n d ∧ rq f n d % 2 = 1) then
    (rq f n d + 1) * 2 ^ rshift f n d
  else rq f n d * 2 ^ rshift f n d

/-- `roundTiesToEven` to the format: the finite magnitude, or `none` for an
overflow to infinity. -/
def roundMag (f : Format) (n d : Nat) : Option Nat :=
  if roundUnb f n d ≤ f.maxN then some (roundUnb f n d) else none

/-- The signed rounding: `(-1)^neg · x` rounded, `±inf` on overflow. A zero
result keeps `neg` (IEEE 754 §6.3: an underflow to zero keeps the sign). -/
def round (f : Format) (neg : Bool) (n d : Nat) : Datum :=
  match roundMag f n d with
  | some N => .fin neg N
  | none => .inf neg

/-! ## The grid around `x` -/

theorem p_pos (f : Format) : 1 ≤ f.p := by unfold Format.p; omega

theorem pow_p (f : Format) : 2 ^ f.p = 2 * 2 ^ (f.p - 1) := by
  rw [← Nat.pow_succ']; rfl

theorem dist_of_le {n d N : Nat} (h : N * d ≤ n) : dist n d N = n - N * d := by
  simp [dist, h]

theorem dist_of_lt {n d N : Nat} (h : n < N * d) : dist n d N = N * d - n := by
  simp [dist, Nat.not_le.mpr h]

/-- `dist` is the integer distance `|n - N·d|`, i.e. `|x - N| · d` for
`x = n / d`. -/
theorem dist_eq_natAbs (n d N : Nat) : dist n d N = ((n : Int) - (N * d : Nat)).natAbs := by
  unfold dist
  split <;> omega

/-- The quotient `q = ⌊x / 2^s⌋` has at most `p` bits, and exactly `p` when
the scale is positive. -/
theorem quotient_bounds (f : Format) (n d : Nat) :
    rq f n d < 2 ^ f.p ∧ (rshift f n d = 0 ∨ 2 ^ (f.p - 1) ≤ rq f n d) := by
  unfold rq ru
  rw [← Nat.div_div_eq_div_mul]
  have hp := p_pos f
  generalize hi : n / d = i
  have hs : rshift f n d = (i.log2 + 1) - f.p := by simp only [rshift, ilog2_eq]; rw [hi]
  rw [hs]
  constructor
  · rw [Nat.div_lt_iff_lt_mul (Nat.two_pow_pos _), ← Nat.pow_add]
    exact Nat.lt_of_lt_of_le Nat.lt_log2_self (Nat.pow_le_pow_right (by decide) (by omega))
  · by_cases h0 : (i.log2 + 1) - f.p = 0
    · exact Or.inl h0
    · right
      have hi0 : i ≠ 0 := by
        intro h; subst h; rw [Nat.log2_zero] at h0; omega
      rw [Nat.le_div_iff_mul_le (Nat.two_pow_pos _), ← Nat.pow_add,
        show f.p - 1 + (i.log2 + 1 - f.p) = i.log2 by omega]
      exact Nat.log2_self_le hi0

/-- **No value between grid neighbours.** For a `c` of exactly `p` bits (or
any `c` on the unit grid), no magnitude of `p` significant bits lies strictly
between `c · 2^s` and `(c + 1) · 2^s`. -/
theorem gap (f : Format) {c s N : Nat} (hn : s = 0 ∨ 2 ^ (f.p - 1) ≤ c)
    (h : UnbRep f N) : N ≤ c * 2 ^ s ∨ (c + 1) * 2 ^ s ≤ N := by
  rcases hn with hs | hn
  · subst hs; simp only [Nat.pow_zero, Nat.mul_one]; omega
  · obtain ⟨M, k, hM, rfl⟩ := h
    by_cases hk : s ≤ k
    · have hsplit : M * 2 ^ k = M * 2 ^ (k - s) * 2 ^ s := by
        rw [Nat.mul_assoc, ← Nat.pow_add, Nat.sub_add_cancel hk]
      rw [hsplit]
      by_cases hc' : M * 2 ^ (k - s) ≤ c
      · exact Or.inl (Nat.mul_le_mul_right _ hc')
      · exact Or.inr (Nat.mul_le_mul_right _ (by omega))
    · left
      have hp := p_pos f
      have h1 : M * 2 ^ k ≤ M * 2 ^ (s - 1) :=
        Nat.mul_le_mul_left _ (Nat.pow_le_pow_right (by decide) (by omega))
      have h2 : M * 2 ^ (s - 1) < 2 ^ f.p * 2 ^ (s - 1) :=
        Nat.mul_lt_mul_of_pos_right hM (Nat.two_pow_pos _)
      have h3 : 2 ^ f.p * 2 ^ (s - 1) = 2 ^ (f.p - 1) * 2 ^ s := by
        rw [← Nat.pow_add, ← Nat.pow_add]; congr 1; omega
      have h4 : 2 ^ (f.p - 1) * 2 ^ s ≤ c * 2 ^ s := Nat.mul_le_mul_right _ hn
      omega

/-- The significand of a grid point `c · 2^s` is `c`. -/
theorem sig_grid (f : Format) {c s : Nat} (hc : c < 2 ^ f.p) (hn : s = 0 ∨ 2 ^ (f.p - 1) ≤ c) :
    sig f (c * 2 ^ s) = c := by
  rcases hn with hs | hn
  · subst hs; simp [sig_of_lt f hc]
  · have hc0 : c ≠ 0 := by have := Nat.two_pow_pos (f.p - 1); omega
    have hlog : c.log2 = f.tw := by
      rw [Nat.log2_eq_iff hc0]
      have := pow_p f
      unfold Format.p at *
      simp only [Nat.add_sub_cancel] at hn
      exact ⟨hn, by omega⟩
    simp only [sig, shift, ilog2_eq]
    rw [log2_mul_pow _ hc0, hlog, show f.tw + s - f.tw = s by omega,
      Nat.mul_div_cancel _ (Nat.two_pow_pos _)]

/-- The significand of the power of two `2^p · 2^s` is `2^(p-1)`. -/
theorem sig_top (f : Format) (s : Nat) : sig f (2 ^ f.p * 2 ^ s) = 2 ^ (f.p - 1) := by
  have h : 2 ^ f.p * 2 ^ s = 2 ^ (f.p - 1) * 2 ^ (s + 1) := by
    rw [pow_p, Nat.pow_succ]; ac_rfl
  rw [h]
  exact sig_grid f (by rw [pow_p f]; have := Nat.two_pow_pos (f.p - 1); omega)
    (Or.inr (Nat.le_refl _))

/-- The pattern of a finite magnitude is even exactly when its significand is:
IEEE 754's "even" (least significant significand bit zero) is the pattern's. -/
theorem encodeMag_even (f : Format) (hf : f.Valid) (N : Nat) :
    encodeMag f N % 2 = sig f N % 2 := by
  unfold encodeMag
  have : 2 ^ f.tw = 2 * 2 ^ (f.tw - 1) := by
    rw [← Nat.pow_succ']; congr 1; have := hf.tw_ge; omega
  rw [this, Nat.add_mod, show shift f N * (2 * 2 ^ (f.tw - 1)) = 2 * (shift f N * 2 ^ (f.tw - 1)) by
    rw [Nat.mul_left_comm], Nat.mul_mod_right]
  simp

/-! ## The unbounded rounding -/

/-- The two candidates `lo = q·2^s` and `hi = (q+1)·2^s` around `x`, scaled
by `d`, against `n = q·u + r`. -/
theorem candidates (f : Format) {n d : Nat} (hd : 0 < d) :
    rq f n d * 2 ^ rshift f n d * d = rq f n d * ru f n d ∧
    (rq f n d + 1) * 2 ^ rshift f n d * d = rq f n d * ru f n d + ru f n d ∧
    rq f n d * ru f n d + rr f n d = n ∧ rr f n d < ru f n d ∧ 0 < ru f n d := by
  have hu : 0 < ru f n d := Nat.mul_pos hd (Nat.two_pow_pos _)
  refine ⟨?_, ?_, ?_, Nat.mod_lt _ hu, hu⟩
  · unfold ru; rw [Nat.mul_assoc, Nat.mul_comm (2 ^ _) d]
  · unfold ru; rw [Nat.mul_assoc, Nat.mul_comm (2 ^ _) d, Nat.succ_mul]
  · unfold rq rr; rw [Nat.mul_comm]; exact Nat.div_add_mod n _

theorem roundUnb_cases (f : Format) (n d : Nat) :
    (roundUnb f n d = (rq f n d + 1) * 2 ^ rshift f n d ∧
      (ru f n d < 2 * rr f n d ∨ (2 * rr f n d = ru f n d ∧ rq f n d % 2 = 1))) ∨
    (roundUnb f n d = rq f n d * 2 ^ rshift f n d ∧ 2 * rr f n d ≤ ru f n d ∧
      ¬ (2 * rr f n d = ru f n d ∧ rq f n d % 2 = 1)) := by
  unfold roundUnb
  split
  · exact Or.inl ⟨rfl, by assumption⟩
  · rename_i h; exact Or.inr ⟨rfl, by omega, fun h' => h (Or.inr h')⟩

/-- The unbounded rounding is a magnitude of `p` significant bits. -/
theorem roundUnb_unbRep (f : Format) (n d : Nat) : UnbRep f (roundUnb f n d) := by
  obtain ⟨hq, -⟩ := quotient_bounds f n d
  rcases roundUnb_cases f n d with ⟨h, -⟩ | ⟨h, -⟩ <;> rw [h]
  · generalize rq f n d = q at hq
    by_cases hq1 : q + 1 < 2 ^ f.p
    · exact ⟨_, _, hq1, rfl⟩
    · refine ⟨2 ^ (f.p - 1), rshift f n d + 1, ?_, ?_⟩
      · rw [pow_p f]; have := Nat.two_pow_pos (f.p - 1); omega
      · rw [show q + 1 = 2 * 2 ^ (f.p - 1) by rw [← pow_p]; omega, Nat.pow_succ]; ac_rfl
  · exact ⟨_, _, hq, rfl⟩

/-- A competitor of `p` significant bits lies at or below `lo`, or at or
above `hi`, and is at least as far from `x` as that candidate. -/
theorem competitor (f : Format) {n d N : Nat} (hd : 0 < d) (h : UnbRep f N) :
    (N * d ≤ rq f n d * ru f n d ∧ dist n d N = n - N * d ∧ rr f n d ≤ dist n d N ∧
        (N * d = rq f n d * ru f n d → N = rq f n d * 2 ^ rshift f n d)) ∨
      (rq f n d * ru f n d + ru f n d ≤ N * d ∧ dist n d N = N * d - n ∧
        ru f n d - rr f n d ≤ dist n d N ∧
        (N * d = rq f n d * ru f n d + ru f n d → N = (rq f n d + 1) * 2 ^ rshift f n d)) := by
  obtain ⟨hlo, hhi, hn, hr, hu⟩ := candidates f (n := n) hd
  obtain ⟨-, hq'⟩ := quotient_bounds f n d
  rcases gap f hq' h with hN | hN
  · left
    have h1 : N * d ≤ rq f n d * 2 ^ rshift f n d * d := Nat.mul_le_mul_right _ hN
    have hdist : dist n d N = n - N * d := dist_of_le (by omega)
    exact ⟨by omega, hdist, by omega, fun _ => Nat.eq_of_mul_eq_mul_right hd (by omega)⟩
  · right
    have h1 : (rq f n d + 1) * 2 ^ rshift f n d * d ≤ N * d := Nat.mul_le_mul_right _ hN
    have hdist : dist n d N = N * d - n := dist_of_lt (by omega)
    exact ⟨by omega, hdist, by omega, fun _ => Nat.eq_of_mul_eq_mul_right hd (by omega)⟩

/-- **Nearest, unbounded.** No magnitude of `p` significant bits is nearer to
`x` than its rounding. -/
theorem roundUnb_nearest (f : Format) {n d N : Nat} (hd : 0 < d) (h : UnbRep f N) :
    dist n d (roundUnb f n d) ≤ dist n d N := by
  obtain ⟨hlo, hhi, hn, hr, hu⟩ := candidates f (n := n) hd
  have hc := competitor f (n := n) hd h
  rcases roundUnb_cases f n d with ⟨h, hup⟩ | ⟨h, hdn, -⟩ <;> rw [h]
  · rw [dist_of_lt (by omega)]; omega
  · rw [dist_of_le (by omega)]; omega

/-- **Ties to even, unbounded.** A different magnitude of `p` significant bits
exactly as near to `x` as the rounding has an odd significand, and the
rounding an even one. -/
theorem roundUnb_tie (f : Format) (hf : f.Valid) {n d N : Nat} (hd : 0 < d) (h : UnbRep f N)
    (hne : N ≠ roundUnb f n d) (heq : dist n d N = dist n d (roundUnb f n d)) :
    sig f (roundUnb f n d) % 2 = 0 ∧ sig f N % 2 = 1 := by
  obtain ⟨hlo, hhi, hn, hr, hu⟩ := candidates f (n := n) hd
  obtain ⟨hq, hq'⟩ := quotient_bounds f n d
  have hc := competitor f (n := n) hd h
  have hp1 : 2 ≤ f.p := by have := hf.tw_ge; unfold Format.p; omega
  have htop : 2 ^ (f.p - 1) % 2 = 0 := by
    rw [show f.p - 1 = (f.p - 2) + 1 by omega, Nat.pow_succ]; simp
  have hsiglo : sig f (rq f n d * 2 ^ rshift f n d) = rq f n d := sig_grid f hq hq'
  -- `hi`'s significand has `q + 1`'s parity, also when `q + 1 = 2^p`
  have hsighi : sig f ((rq f n d + 1) * 2 ^ rshift f n d) % 2 = (rq f n d + 1) % 2 := by
    by_cases h1 : rq f n d + 1 < 2 ^ f.p
    · rw [sig_grid f h1 (by omega)]
    · rw [show rq f n d + 1 = 2 ^ f.p by omega, sig_top, htop, pow_p f]; simp
  rcases roundUnb_cases f n d with ⟨hR, hup⟩ | ⟨hR, hdn, hnt⟩ <;> rw [hR] at hne heq ⊢
  · have hdR : dist n d ((rq f n d + 1) * 2 ^ rshift f n d) = ru f n d - rr f n d := by
      rw [dist_of_lt (by omega)]; omega
    rcases hc with ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩
    · have hN : N = rq f n d * 2 ^ rshift f n d := h4 (by omega)
      rw [hN, hsiglo]; omega
    · exact absurd (h4 (by omega)) hne
  · have hdR : dist n d (rq f n d * 2 ^ rshift f n d) = rr f n d := by
      rw [dist_of_le (by omega)]; omega
    rcases hc with ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩
    · exact absurd (h4 (by omega)) hne
    · have hN : N = (rq f n d + 1) * 2 ^ rshift f n d := h4 (by omega)
      rw [hN, hsiglo]; omega

/-- A magnitude of `p` significant bits rounds to itself. -/
theorem roundUnb_exact (f : Format) {N d : Nat} (hd : 0 < d) (h : UnbRep f N) :
    roundUnb f (N * d) d = N := by
  have hn := roundUnb_nearest f (n := N * d) hd h
  rw [dist_of_le (Nat.le_refl _), Nat.sub_self] at hn
  apply Nat.eq_of_mul_eq_mul_right hd
  unfold dist at hn
  split at hn <;> omega

theorem midpoint_of_nearer {n d A B : Nat} (hAB : B * d < A * d)
    (h : dist n d A ≤ dist n d B) : A * d + B * d ≤ 2 * n := by
  unfold dist at h
  split at h <;> split at h <;> omega

theorem midpoint_of_nearer' {n d A B : Nat} (hAB : B * d < A * d)
    (h : dist n d B ≤ dist n d A) : 2 * n ≤ A * d + B * d := by
  unfold dist at h
  split at h <;> split at h <;> omega

/-- **Monotone, unbounded.** `x₁ ≤ x₂` rounds to `R₁ ≤ R₂`. -/
theorem roundUnb_mono (f : Format) (hf : f.Valid) {n₁ d₁ n₂ d₂ : Nat} (h₁ : 0 < d₁)
    (h₂ : 0 < d₂) (hx : n₁ * d₂ ≤ n₂ * d₁) : roundUnb f n₁ d₁ ≤ roundUnb f n₂ d₂ := by
  have u₁ := roundUnb_unbRep f n₁ d₁
  have u₂ := roundUnb_unbRep f n₂ d₂
  have near₁ := roundUnb_nearest f (n := n₁) h₁ u₂
  have near₂ := roundUnb_nearest f (n := n₂) h₂ u₁
  have e₁ := fun hne heq => roundUnb_tie f hf (n := n₁) h₁ u₂ hne heq
  have e₂ := fun hne heq => roundUnb_tie f hf (n := n₂) h₂ u₁ hne heq
  generalize roundUnb f n₁ d₁ = R₁ at *
  generalize roundUnb f n₂ d₂ = R₂ at *
  apply Nat.le_of_not_lt
  intro hlt
  have l₁ : R₂ * d₁ < R₁ * d₁ := Nat.mul_lt_mul_of_pos_right hlt h₁
  have l₂ : R₂ * d₂ < R₁ * d₂ := Nat.mul_lt_mul_of_pos_right hlt h₂
  have A := midpoint_of_nearer l₁ near₁
  have B := midpoint_of_nearer' l₂ near₂
  -- chain the two midpoint bounds through `x₁ ≤ x₂`: both are equalities
  have A' : (R₁ * d₁ + R₂ * d₁) * d₂ ≤ 2 * n₁ * d₂ := Nat.mul_le_mul_right _ A
  have B' : 2 * n₂ * d₁ ≤ (R₁ * d₂ + R₂ * d₂) * d₁ := Nat.mul_le_mul_right _ B
  have X : 2 * n₁ * d₂ ≤ 2 * n₂ * d₁ := by
    rw [Nat.mul_assoc, Nat.mul_assoc]; exact Nat.mul_le_mul_left _ hx
  have E : (R₁ * d₁ + R₂ * d₁) * d₂ = (R₁ * d₂ + R₂ * d₂) * d₁ := by
    rw [Nat.add_mul, Nat.add_mul, Nat.mul_right_comm R₁ d₁, Nat.mul_right_comm R₂ d₁]
  have eA : R₁ * d₁ + R₂ * d₁ = 2 * n₁ := Nat.eq_of_mul_eq_mul_right h₂ (by omega)
  have eB : 2 * n₂ = R₁ * d₂ + R₂ * d₂ := Nat.eq_of_mul_eq_mul_right h₁ (by omega)
  -- so both are ties, and each rounding is the even one of the pair
  have t₁ : dist n₁ d₁ R₂ = dist n₁ d₁ R₁ := by unfold dist; split <;> split <;> omega
  have t₂ : dist n₂ d₂ R₁ = dist n₂ d₂ R₂ := by unfold dist; split <;> split <;> omega
  have := e₁ (Nat.ne_of_lt hlt) t₁
  have := e₂ (Nat.ne_of_gt hlt) t₂
  omega

/-! ## The format's rounding -/

theorem roundMag_some {f : Format} {n d N : Nat} (h : roundMag f n d = some N) :
    roundUnb f n d = N ∧ N ≤ f.maxN := by
  unfold roundMag at h
  split at h
  · cases h; exact ⟨rfl, by assumption⟩
  · cases h

/-- The result is a finite value of the format. -/
theorem roundMag_rep (f : Format) {n d N : Nat} (h : roundMag f n d = some N) : Rep f N := by
  obtain ⟨h1, h2⟩ := roundMag_some h
  exact ⟨h1 ▸ roundUnb_unbRep f n d, h2⟩

/-- **Nearest.** No finite value of the format is nearer to `x`. -/
theorem roundMag_nearest (f : Format) {n d N N' : Nat} (hd : 0 < d)
    (h : roundMag f n d = some N) (h' : Rep f N') : dist n d N ≤ dist n d N' := by
  obtain ⟨h1, -⟩ := roundMag_some h
  exact h1 ▸ roundUnb_nearest f hd h'.1

/-- **Ties to even.** When another finite value is exactly as near to `x`,
the result's significand — and its pattern — is even. -/
theorem roundMag_tie_even (f : Format) (hf : f.Valid) {n d N N' : Nat} (hd : 0 < d)
    (h : roundMag f n d = some N) (h' : Rep f N') (hne : N' ≠ N)
    (heq : dist n d N' = dist n d N) :
    sig f N % 2 = 0 ∧ encodeMag f N % 2 = 0 := by
  obtain ⟨h1, -⟩ := roundMag_some h
  subst h1
  have := (roundUnb_tie f hf hd h'.1 hne heq).1
  exact ⟨this, by rw [encodeMag_even f hf]; exact this⟩

/-- The largest finite magnitude's significand is `2^p - 1`, odd. -/
theorem sig_maxN (f : Format) : sig f f.maxN = 2 ^ f.p - 1 := by
  unfold Format.maxN
  have := pow_p f
  have := Nat.two_pow_pos (f.p - 1)
  exact sig_grid f (by omega) (Or.inr (by omega))

theorem maxN_unbRep (f : Format) : UnbRep f f.maxN :=
  ⟨_, _, by have := Nat.two_pow_pos f.p; omega, rfl⟩

theorem top_unbRep (f : Format) : UnbRep f (2 ^ f.p * 2 ^ f.kmax) :=
  ⟨2 ^ (f.p - 1), f.kmax + 1, by rw [pow_p f]; have := Nat.two_pow_pos (f.p - 1); omega, by
    rw [pow_p f, Nat.pow_succ]; ac_rfl⟩

/-- **Overflow.** `x` rounds to infinity exactly when it is at least the
midpoint of the largest finite magnitude `(2^p - 1)·2^kmax` and
`2^p · 2^kmax`: half a unit in the last place above the largest finite value,
the tie going to infinity (`2^p - 1` is odd). -/
theorem roundMag_none_iff (f : Format) (hf : f.Valid) {n d : Nat} (hd : 0 < d) :
    roundMag f n d = none ↔ (f.maxN + 2 ^ f.p * 2 ^ f.kmax) * d ≤ 2 * n := by
  have hmx : f.maxN + 2 ^ f.kmax = 2 ^ f.p * 2 ^ f.kmax := by
    unfold Format.maxN; rw [Nat.sub_mul, Nat.one_mul]
    have : 2 ^ f.kmax ≤ 2 ^ f.p * 2 ^ f.kmax := Nat.le_mul_of_pos_left _ (Nat.two_pow_pos _)
    omega
  have hkd : 0 < 2 ^ f.kmax * d := Nat.mul_pos (Nat.two_pow_pos _) hd
  have hlt : f.maxN * d + 2 ^ f.kmax * d = 2 ^ f.p * 2 ^ f.kmax * d := by
    rw [← hmx, Nat.add_mul]
  have hsigM : sig f f.maxN % 2 = 1 := by
    rw [sig_maxN, pow_p]; have := Nat.two_pow_pos (f.p - 1); omega
  have uR := roundUnb_unbRep f n d
  have hgR : roundUnb f n d ≤ f.maxN ∨ 2 ^ f.p * 2 ^ f.kmax ≤ roundUnb f n d := by
    have := gap f (c := 2 ^ f.p - 1) (s := f.kmax)
      (Or.inr (by rw [pow_p f]; have := Nat.two_pow_pos (f.p - 1); omega)) uR
    rwa [Nat.sub_add_cancel (Nat.two_pow_pos _)] at this
  have nearM := roundUnb_nearest f (n := n) hd (maxN_unbRep f)
  have nearT := roundUnb_nearest f (n := n) hd (top_unbRep f)
  have tieT := fun hne heq => roundUnb_tie f hf (n := n) hd (top_unbRep f) hne heq
  unfold roundMag
  generalize roundUnb f n d = R at *
  generalize 2 ^ f.p * 2 ^ f.kmax = T at *
  rw [Nat.add_mul]
  constructor
  · intro hnone
    have hbig : ¬ R ≤ f.maxN := by intro h; simp [h] at hnone
    have h1 : T * d ≤ R * d := Nat.mul_le_mul_right _ (by omega)
    unfold dist at nearM
    split at nearM <;> split at nearM <;> omega
  · intro hx
    by_cases hle : R ≤ f.maxN
    · exfalso
      have h1 : R * d ≤ f.maxN * d := Nat.mul_le_mul_right _ hle
      by_cases hTn : T * d ≤ n
      · rw [dist_of_le (by omega), dist_of_le hTn] at nearT; omega
      · rw [dist_of_le (by omega), dist_of_lt (by omega)] at nearT
        have hRM : R = f.maxN := Nat.eq_of_mul_eq_mul_right hd (by omega)
        have hne : T ≠ R := fun h => by rw [h] at hlt; omega
        have := tieT hne (by
          rw [dist_of_lt (by omega : n < T * d), dist_of_le (by omega : R * d ≤ n)]; omega)
        rw [hRM] at this; omega
    · simp [hle]

/-- **Monotone.** Two finite results are ordered as their exact values. -/
theorem roundMag_mono (f : Format) (hf : f.Valid) {n₁ d₁ n₂ d₂ N₁ N₂ : Nat} (h₁ : 0 < d₁)
    (h₂ : 0 < d₂) (hx : n₁ * d₂ ≤ n₂ * d₁) (r₁ : roundMag f n₁ d₁ = some N₁)
    (r₂ : roundMag f n₂ d₂ = some N₂) : N₁ ≤ N₂ := by
  obtain ⟨e₁, -⟩ := roundMag_some r₁
  obtain ⟨e₂, -⟩ := roundMag_some r₂
  rw [← e₁, ← e₂]
  exact roundUnb_mono f hf h₁ h₂ hx

/-- **Monotone overflow.** A value above one that overflows overflows too. -/
theorem roundMag_mono_none (f : Format) (hf : f.Valid) {n₁ d₁ n₂ d₂ : Nat} (h₁ : 0 < d₁)
    (h₂ : 0 < d₂) (hx : n₁ * d₂ ≤ n₂ * d₁) (r₁ : roundMag f n₁ d₁ = none) :
    roundMag f n₂ d₂ = none := by
  rw [roundMag_none_iff f hf h₁] at r₁
  rw [roundMag_none_iff f hf h₂]
  generalize f.maxN + 2 ^ f.p * 2 ^ f.kmax = K at *
  have a : K * d₁ * d₂ ≤ 2 * n₁ * d₂ := Nat.mul_le_mul_right _ r₁
  have b : 2 * n₁ * d₂ ≤ 2 * n₂ * d₁ := by
    rw [Nat.mul_assoc, Nat.mul_assoc]; exact Nat.mul_le_mul_left _ hx
  have c : K * d₁ * d₂ = K * d₂ * d₁ := Nat.mul_right_comm _ _ _
  exact Nat.le_of_mul_le_mul_right (by omega : K * d₂ * d₁ ≤ 2 * n₂ * d₁) h₁

/-- **Exact.** A finite value of the format rounds to itself. -/
theorem roundMag_exact (f : Format) {N d : Nat} (hd : 0 < d) (h : Rep f N) :
    roundMag f (N * d) d = some N := by
  unfold roundMag
  rw [roundUnb_exact f hd h.1]
  simp [h.2]

/-- **Subnormal rounding.** Below `2^p` units (every subnormal and the least
normal binade) the grid is every unit: the scale is zero, so `x` rounds to
the nearer integer, ties to the even one. -/
theorem rshift_small (f : Format) {n d : Nat} (h : n / d < 2 ^ f.p) : rshift f n d = 0 := by
  simp only [rshift, ilog2_eq]
  have := log2_lt_of_lt h (p_pos f)
  omega

end QCore.Ieee754
