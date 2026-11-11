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

/-! Typed bounds and indices of the counted loop (card `W48-QR-SCALAR-TYPES-R5`,
`W48-QR-TYPES-LOOP-BOUNDS`).

The Lean statement of `qcore_core::source_loop::typed_bound` and
`index_admits`: `for NAME in 0..BOUND` where `BOUND` is the check-time number
`n` of a §G02t type `ty`. The low end `0` is a literal and takes the bound's
type, so the range has one type and the index has it too.

The decision refuses a `bool` bound, an `isize`/`usize` bound with no recorded
target, a negative bound, a zero bound, a bound above the 4096-iteration
ceiling, and a count that is not a value of the index type. What the theorems
say about an **admitted** bound is what makes the loop sound at every width:

* every index value `0 … count-1` is a value of the index type
  (`every_index_is_a_value`);
* the step `i + 1` computed from every index value is the checked addition of
  `QCore.ScalarTypes`, and it never faults (`the_step_never_faults`) — so the
  emitted `i += 1` neither overflows nor, on a narrow type, truncates;
* the loop, run with that checked step, **terminates after exactly `count`
  iterations** and visits exactly `0, 1, …, count-1` in order
  (`the_loop_runs_exactly_count_times`);
* a `u64` near its maximum and a signed bound near `MIN` or `MAX` are decided
  (refused by the ceiling or the sign rule, or admitted with every index inside
  the type), by `decide` on the boundary rows;
* on `i32` the decision is exactly card `LOOP-V1`'s (`i32_is_loop_v1`), so the
  widening moves no existing loop.

The binding to Rust is the vector table `loopAgreement`, replayed through
`qcore_core::source_loop::typed_bound` by the Rust test
`lean_loop_bound_vectors_match_the_decision`, and the native runs of the
emitted loops; neither is an extraction theorem. -/
namespace QCore.ScalarLoopBounds

open QCore.ScalarTypes

/-- The iteration ceiling, `source_loop::MAX_ITERATIONS`. -/
def ceiling : Int := 4096

/-- Why a bound is refused, as `source_loop::BoundRefusal`'s typed arms. -/
inductive Refusal where
  | notAnInteger
  | needsRecordedTarget
  | negative
  | zero
  | tooLarge
  | countOutsideIndex
  deriving DecidableEq, Repr

/-- The decision: a count of iterations, or the refusal. -/
inductive Decision where
  | count (c : Nat)
  | refused (r : Refusal)
  deriving DecidableEq, Repr

def targetWidth : Ty → Bool
  | .isize | .usize => true
  | _ => false

/-- `index_admits`: a loop of `c` iterations may have an index of type `ty`
exactly when `c` itself is a value of `ty` (the index's values and the final
step's value all lie in `0 … c`). -/
def indexAdmits (t : Option Width) (ty : Ty) (c : Nat) : Option Refusal :=
  if ty = .bool then some .countOutsideIndex
  else if targetWidth ty = true ∧ t = none then some .needsRecordedTarget
  else if fits t ty c = true then none
  else some .countOutsideIndex

/-- `typed_bound`, in its order. -/
def typedBound (t : Option Width) (ty : Ty) (n : Int) : Decision :=
  if ty = .bool then .refused .notAnInteger
  else if targetWidth ty = true ∧ t = none then .refused .needsRecordedTarget
  else if n < 0 then .refused .negative
  else if n = 0 then .refused .zero
  else if ceiling < n then .refused .tooLarge
  else match indexAdmits t ty n.toNat with
    | none => .count n.toNat
    | some r => .refused r

/-! ## What an admitted bound is -/

theorem admitted_facts {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) :
    ty ≠ .bool ∧ (c : Int) = n ∧ 1 ≤ c ∧ c ≤ 4096 ∧ fits t ty c = true := by
  unfold typedBound at h
  split at h
  · cases h
  · rename_i hb
    split at h
    · cases h
    · rename_i ht
      split at h
      · cases h
      · rename_i hn
        split at h
        · cases h
        · rename_i hz
          split at h
          · cases h
          · rename_i hc
            split at h
            · rename_i heq
              cases h
              have hf : fits t ty n.toNat = true := by
                have h0 : ((n.toNat : Nat) : Int) = n := Int.toNat_of_nonneg (by omega)
                simp [indexAdmits, hb, ht] at heq
                rw [h0]
                simpa [show max n 0 = n by omega] using heq
              have h0 : 0 ≤ n := by omega
              have hcn : ((n.toNat : Nat) : Int) = n := Int.toNat_of_nonneg h0
              refine ⟨hb, hcn, ?_, ?_, hf⟩
              · omega
              · simp [ceiling] at hc; omega
            · cases h

/-- Every integer type's range contains `0`. -/
theorem zero_in_range {t : Option Width} {ty : Ty} {lo hi : Int} (hb : ty ≠ .bool)
    (hr : ty.range t = some (lo, hi)) : lo ≤ 0 := by
  cases ty <;> cases t <;> (try cases ‹Width›) <;> simp_all [Ty.range, signedRange, unsignedRange] <;> omega

/-- **Every index value is a value of the index type.** -/
theorem every_index_is_a_value {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) (i : Nat) (hi : i ≤ c) : fits t ty i = true := by
  obtain ⟨hb, _, _, _, hf⟩ := admitted_facts h
  obtain ⟨lo, hi', hr, hl, hh⟩ := fits_bounds hf
  have := zero_in_range hb hr
  exact fits_of_bounds hr (by omega) (by omega)

theorem checked_add_of_fits {t : Option Width} {ty : Ty} {a b : Int} (hb : ty ≠ .bool)
    (ha : fits t ty a = true) (hb' : fits t ty b = true) (hs : fits t ty (a + b) = true) :
    checked t .add ty a b = some (a + b) := by
  obtain ⟨lo, hi, hr, _, _⟩ := fits_bounds ha
  simp [checked, hb, hr, ha, hb', raw, exact, narrow, hs]

/-- **The step never faults.** From every index value `i < count`, the checked
addition `i + 1` at the index type succeeds with `i + 1`: the emitted `+= 1`
never overflows and never truncates. -/
theorem the_step_never_faults {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) (i : Nat) (hi : i < c) :
    checked t .add ty i 1 = some ((i : Int) + 1) := by
  obtain ⟨hb, _, h1, _, _⟩ := admitted_facts h
  have hi1 : fits t ty (1 : Nat) = true := every_index_is_a_value h 1 h1
  have hs : fits t ty ((i + 1 : Nat)) = true := every_index_is_a_value h (i + 1) hi
  apply checked_add_of_fits hb (every_index_is_a_value h i (Nat.le_of_lt hi))
  · simpa using hi1
  · simpa using hs

/-! ## The loop runs exactly `count` times

The emitted loop, `for (T i = 0; i < BOUND; i += 1)`, with the step as the
checked addition: a faulting step would end the run with `none`, and so would
running out of `fuel`. -/

def run (t : Option Width) (ty : Ty) (bound : Int) : Nat → Int → Option (List Int)
  | 0, _ => none
  | fuel + 1, i =>
    if i < bound then
      match checked t .add ty i 1 with
      | some j => (run t ty bound fuel j).map (i :: ·)
      | none => none
    else some []

theorem run_succ (t : Option Width) (ty : Ty) (bound : Int) (fuel : Nat) (i : Int) :
    run t ty bound (fuel + 1) i =
      if i < bound then
        match checked t .add ty i 1 with
        | some j => (run t ty bound fuel j).map (i :: ·)
        | none => none
      else some [] := rfl

theorem run_from {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) :
    ∀ d m : Nat, m + d = c →
      run t ty c (d + 1) m = some ((List.range' m d).map (fun k : Nat => (k : Int))) := by
  intro d
  induction d with
  | zero =>
    intro m hm
    simp [run, show ¬ ((m : Int) < (c : Int)) by omega]
  | succ d ih =>
    intro m hm
    have hlt : m < c := by omega
    have hstep := the_step_never_faults h m hlt
    have hrest := ih (m + 1) (by omega)
    rw [run_succ]
    simp only [show (m : Int) < (c : Int) from by omega, ↓reduceIte, hstep]
    have : ((m : Int) + 1) = ((m + 1 : Nat) : Int) := by push_cast; rfl
    rw [this, hrest]
    simp [List.range'_succ]

/-- **The headline.** An admitted loop, run with the checked step, terminates
after exactly `count` iterations and visits exactly `0, 1, …, count-1`, at
every type and width the decision admits. -/
theorem the_loop_runs_exactly_count_times {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) :
    run t ty n (c + 1) 0 = some ((List.range c).map (fun k : Nat => (k : Int))) := by
  obtain ⟨_, hcn, _, _, _⟩ := admitted_facts h
  rw [← hcn]
  simpa [List.range_eq_range'] using run_from h c 0 (by omega)

/-- The count is the number of iterations. -/
theorem the_count_is_the_iteration_count {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) :
    ((run t ty n (c + 1) 0).map List.length) = some c := by
  rw [the_loop_runs_exactly_count_times h]
  simp

/-! ## Refusals -/

theorem a_bool_is_not_a_bound (t : Option Width) (n : Int) :
    typedBound t .bool n = .refused .notAnInteger := by
  simp [typedBound]

theorem a_target_width_bound_needs_the_target (n : Int) :
    typedBound none .usize n = .refused .needsRecordedTarget ∧
      typedBound none .isize n = .refused .needsRecordedTarget := by
  simp [typedBound, targetWidth]

/-- Every admitted count is inside the ceiling, whatever the type's width. -/
theorem admitted_counts_are_inside_the_ceiling {t : Option Width} {ty : Ty} {n : Int} {c : Nat}
    (h : typedBound t ty n = .count c) : 1 ≤ c ∧ c ≤ 4096 :=
  let ⟨_, _, h1, h2, _⟩ := admitted_facts h
  ⟨h1, h2⟩

/-- On `i32` the decision is exactly card `LOOP-V1`'s: a positive bound up to
the ceiling, counted as itself. -/
theorem i32_is_loop_v1 (t : Option Width) (n : Int) (c : Nat) :
    typedBound t .i32 n = .count c ↔ (0 < n ∧ n ≤ 4096 ∧ (c : Int) = n) := by
  constructor
  · intro h
    obtain ⟨_, hcn, h1, h2, _⟩ := admitted_facts h
    omega
  · rintro ⟨h0, h1, hc⟩
    subst hc
    have hf : fits t .i32 (c : Int) = true := by
      simp [fits, Ty.range]; omega
    simp [typedBound, indexAdmits, targetWidth, ceiling, hf,
      show ¬ (c : Int) < 0 by omega, show c ≠ 0 by omega,
      show ¬ (4096 : Int) < (c : Int) by omega]

/-! ## Boundary rows, decided -/

/-- `(type, bound, admitted count)` at the 64-bit target; replayed through the
Rust decision by `lean_loop_bound_vectors_match_the_decision`. The rows cover
each type's `MIN`, `MIN+1`, `-1`, `0`, `1`, `MAX-1` and `MAX` where the
ceiling or the sign decides them, and the ceiling itself. -/
def loopAgreement : List (Ty × Int × Option Nat) := [
  (.u8, 1, some 1), (.u8, 254, some 254), (.u8, 255, some 255), (.u8, 0, none),
  (.i8, 127, some 127), (.i8, 126, some 126), (.i8, -128, none), (.i8, -1, none),
  (.u16, 4096, some 4096), (.u16, 4097, none), (.u16, 65535, none),
  (.i16, 32767, none), (.i16, -32768, none), (.i16, 4096, some 4096),
  (.i32, 2147483647, none), (.i32, -2147483648, none), (.i32, 256, some 256),
  (.u32, 4294967295, none), (.u32, 1, some 1),
  (.i64, 9223372036854775807, none), (.i64, 9223372036854775806, none),
  (.i64, -9223372036854775808, none), (.i64, -9223372036854775807, none), (.i64, 4095, some 4095),
  (.u64, 18446744073709551615, none), (.u64, 18446744073709551614, none), (.u64, 4096, some 4096),
  (.isize, 9223372036854775807, none), (.isize, 7, some 7),
  (.usize, 18446744073709551615, none), (.usize, 4096, some 4096),
  (.bool, 1, none)]

def admittedCount : Decision → Option Nat
  | .count c => some c
  | .refused _ => none

theorem loopAgreement_holds :
    loopAgreement.all (fun (ty, n, c) => admittedCount (typedBound (some .w64) ty n) == c) = true := by
  decide

/-- `u64` at its maximum and one below it: refused by the ceiling, before any
index exists. -/
theorem u64_near_its_maximum_is_refused (t : Option Width) :
    typedBound t .u64 18446744073709551615 = .refused .tooLarge ∧
      typedBound t .u64 18446744073709551614 = .refused .tooLarge := by
  constructor <;> simp [typedBound, targetWidth, ceiling]

/-- Signed bounds near `MIN` are refused by the sign rule, near `MAX` by the
ceiling. -/
theorem signed_bounds_near_min_and_max_are_refused (t : Option Width) :
    typedBound t .i64 (-9223372036854775808) = .refused .negative ∧
      typedBound t .i64 9223372036854775807 = .refused .tooLarge ∧
      typedBound t .i8 (-128) = .refused .negative := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [typedBound, targetWidth, ceiling]

/-- `u8`'s maximum is admitted, and its last step reaches exactly `255`, a
value of `u8`: the narrowest unsigned loop that can reach the top of its type. -/
theorem u8_reaches_its_maximum_without_fault (t : Option Width) :
    typedBound t .u8 255 = .count 255 ∧ checked t .add .u8 254 1 = some 255 := by
  constructor
  · simp [typedBound, indexAdmits, targetWidth, ceiling, fits, Ty.range]
  · simp [checked, Ty.range, fits, raw, exact, narrow]

end QCore.ScalarLoopBounds
