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

/-!
# A checked measure for a counted loop (card W45-FB-ENGINE-PROGRESS-MEASURE)

The abstract rule implemented by `crates/qcore-c-gateway/src/loop_measure.rs`,
gateway rule 44. The rule records, beside the resource verdict, a bound fact
for a `for` loop whose counter `c` is set to a constant `c0`, tested `c < B`,
advanced by exactly one `c++` per iteration and not otherwise written. It
moves no verdict and states no progress property.

## The model

After the rule's checks (every conversion to the comparison's type preserves
values, `B` lies within the counter's type, nothing but the step writes the
counter), one iteration of an admitted loop is: the test `i < b` holds on
mathematical integers, the body leaves `i` unchanged, and the step sets `i` to
`i + 1`. The bound `b` is either fixed (a constant, or a stable local nothing
in the loop writes) or may change, staying at most the maximum of its type (a
narrow unsigned value); `bound k` is its value at the `k`-th test.

## What is proved

* **The measure** `measure b i = (b - i).toNat` is positive while the test
  holds (`measure_positive_while_running`), equals `b - i` there
  (`measure_is_the_distance`) and strictly decreases, by exactly one, per
  iteration (`measure_strictly_decreases`, `measure_decreases_by_one`).
* **No wrap.** After a true test with `b` at most the counter type's maximum,
  the step stays in the counter's range (`step_stays_in_counter_range`).
* **The iteration bound.** A run of `n` iterations from `c0` against bounds
  each at most `bmax` has `n ≤ (bmax - c0).toNat`
  (`run_length_bounded`); for a fixed bound (`fixed_bound_runs`), and when
  `c0 ≥ b` the loop runs no iteration (`no_iteration_when_init_reaches_bound`).
  An early `break`, `return` or `goto` only shortens a run: a prefix of a run is
  a run (`a_prefix_of_a_run_is_a_run`).
* **Value-preserving conversion** is the identity on in-range values
  (`in_range_conversion_is_identity`).
* **The decision.** `decide` mirrors the Rust rule's precedence; an admitted
  shape satisfies every premise the semantic theorems use
  (`an_admitted_shape_meets_the_premises`, `an_admitted_conjunct_meets_its_premises`),
  and each refusal shape is refused (`a_while_loop_is_refused`, …).
* **Refusals are necessary.** Each refused shape has a run that never ends:
  a counter that may wrap (`a_wrapping_counter_never_exits`), a bound written
  in the loop (`a_written_bound_never_exits`), a counter reset through an
  alias (`an_aliased_counter_reset_never_exits`), a double increment to the
  type's maximum (`a_double_increment_never_exits`) and a `<=` test against
  the type's maximum (`a_less_or_equal_test_never_exits`).
* **No verdict moves.** Recording the fact leaves the custody verdict
  (`recording_a_fact_keeps_the_verdict`).
* `decideVectors` is shared with the Rust table `DECIDE_VECTORS`
  (`decide_vectors_hold`).
-/

namespace QCore.CountedLoopMeasure

/-! ## The measure -/

/-- The measure of an admitted loop: the distance from the counter to the bound. -/
def measure (b i : Int) : Nat := (b - i).toNat

theorem measure_positive_while_running {b i : Int} (h : i < b) : 0 < measure b i := by
  unfold measure; omega

theorem measure_is_the_distance {b i : Int} (h : i ≤ b) : (measure b i : Int) = b - i := by
  unfold measure; omega

theorem measure_strictly_decreases {b i : Int} (h : i < b) :
    measure b (i + 1) < measure b i := by
  unfold measure; omega

theorem measure_decreases_by_one {b i : Int} (h : i < b) :
    measure b (i + 1) + 1 = measure b i := by
  unfold measure; omega

/-- The step after a true test stays within the counter's type `[lo, hi]` when
the bound is at most `hi`: `c++` neither wraps nor overflows. -/
theorem step_stays_in_counter_range {lo hi b i : Int} (hb : b ≤ hi) (hlo : lo ≤ i)
    (h : i < b) : lo ≤ i + 1 ∧ i + 1 ≤ hi := by
  omega

/-! ## Runs -/

/-- A run of `n` iterations from `c0`: at the `k`-th test the counter is
`c0 + k` (one step per iteration, no other write) and the test `c < bound k`
holds. -/
def Run (bound : Nat → Int) (c0 : Int) (n : Nat) : Prop :=
  ∀ k, k < n → c0 + k < bound k

theorem a_prefix_of_a_run_is_a_run {bound : Nat → Int} {c0 : Int} {n m : Nat}
    (h : Run bound c0 n) (hm : m ≤ n) : Run bound c0 m := by
  intro k hk
  exact h k (by omega)

/-- Every run against bounds at most `bmax` has at most `(bmax - c0).toNat`
iterations. -/
theorem run_length_bounded {bound : Nat → Int} {bmax c0 : Int} {n : Nat}
    (h : Run bound c0 n) (hmax : ∀ k, bound k ≤ bmax) : n ≤ (bmax - c0).toNat := by
  cases n with
  | zero => omega
  | succ m =>
    have hlast := h m (by omega)
    have hb := hmax m
    omega

theorem fixed_bound_runs {b c0 : Int} {n : Nat} (h : Run (fun _ => b) c0 n) :
    n ≤ (b - c0).toNat :=
  run_length_bounded h (fun _ => Int.le_refl b)

theorem no_iteration_when_init_reaches_bound {b c0 : Int} {n : Nat}
    (hinit : b ≤ c0) (h : Run (fun _ => b) c0 n) : n = 0 := by
  have := fixed_bound_runs h
  omega

/-- The measure at the `k`-th test of a run against a fixed bound is the
initial measure less `k`. -/
theorem measure_along_a_run {b c0 : Int} {n k : Nat} (h : Run (fun _ => b) c0 n)
    (hk : k < n) : measure b (c0 + k) + k = measure b c0 := by
  have : c0 + k < b := h k hk
  unfold measure; omega

/-! ## Conversions -/

/-- Conversion to an unsigned type of width `w`: reduction modulo `2^w`. -/
def toUnsigned (w : Nat) (x : Int) : Int := x % (2 ^ w : Nat)

theorem in_range_conversion_is_identity {w : Nat} {x : Int} (h0 : 0 ≤ x)
    (h : x < (2 ^ w : Nat)) : toUnsigned w x = x := by
  unfold toUnsigned
  exact Int.emod_eq_of_lt h0 h

/-! ## Refusals are necessary: runs that never end -/

/-- `f` applied `n` times. -/
def iter (f : Int → Int) : Nat → Int → Int
  | 0, x => x
  | n + 1, x => f (iter f n x)

/-- `unsigned char c; for (c = 0; c < 300; c++)`: `c` wraps at 256, so the test
holds at every iteration. -/
def wrapStep (i : Int) : Int := (i + 1) % 256

theorem wrap_step_in_range (i : Int) : 0 ≤ wrapStep i ∧ wrapStep i < 256 := by
  unfold wrapStep; omega

theorem a_wrapping_counter_never_exits (n : Nat) : iter wrapStep n 0 < 300 := by
  cases n with
  | zero => simp [iter]
  | succ m =>
    have := wrap_step_in_range (iter wrapStep m 0)
    simp only [iter]
    omega

/-- `for (c = 0; c < b; c++) b++;`: the bound moves with the counter. -/
theorem a_written_bound_never_exits {c0 b : Int} (h : c0 < b) (k : Nat) :
    c0 + k < b + k := by
  omega

/-- A counter reset to `c0` through an alias in the body: whatever the step
does, the test sees `c0` again at every iteration. -/
theorem an_aliased_counter_reset_never_exits {c0 b : Int} (h : c0 < b) (n : Nat) :
    iter (fun _ => c0) n c0 < b := by
  cases n with
  | zero => simpa [iter] using h
  | succ m => simpa [iter] using h

/-- `unsigned char c; for (c = 0; c < 255; c++) c++;`: the counter advances
by two, stays even, wraps at 256 and never reaches 255. -/
def doubleStep (i : Int) : Int := (i + 2) % 256

theorem double_step_even (i : Int) (h : i % 2 = 0) :
    doubleStep i % 2 = 0 ∧ 0 ≤ doubleStep i ∧ doubleStep i < 256 := by
  unfold doubleStep; omega

theorem a_double_increment_never_exits (n : Nat) :
    iter doubleStep n 0 % 2 = 0 ∧ 0 ≤ iter doubleStep n 0 ∧ iter doubleStep n 0 < 255 := by
  induction n with
  | zero => simp [iter]
  | succ m ih =>
    have := double_step_even (iter doubleStep m 0) ih.1
    simp only [iter]
    omega

/-- `unsigned char c; for (c = 0; c <= 255; c++)`: every value of the type
passes the test. -/
theorem a_less_or_equal_test_never_exits (n : Nat) : iter wrapStep n 0 ≤ 255 := by
  cases n with
  | zero => simp [iter]
  | succ m =>
    have := wrap_step_in_range (iter wrapStep m 0)
    simp only [iter]
    omega

/-! ## The decision -/

inductive Refusal where
  | noCounter
  | stepNotIncrement
  | counterNotLocalInteger
  | counterAddressTaken
  | opaqueControl
  | labelInLoop
  | counterWrittenInLoop
  | initNotConstant
  | testNotLessThan
  | comparisonConverts
  | counterMayWrap
  | boundWrittenInLoop
  | boundAddressTaken
  | boundNotStable
  deriving DecidableEq, Repr

/-- The codes the Rust report spells (`Refusal::code`). -/
def Refusal.code : Refusal → String
  | .noCounter => "no-counter"
  | .stepNotIncrement => "step-not-increment"
  | .counterNotLocalInteger => "counter-not-local-integer"
  | .counterAddressTaken => "counter-address-taken"
  | .opaqueControl => "opaque-control"
  | .labelInLoop => "label-in-loop"
  | .counterWrittenInLoop => "counter-written-in-loop"
  | .initNotConstant => "init-not-constant"
  | .testNotLessThan => "test-not-less-than"
  | .comparisonConverts => "comparison-converts"
  | .counterMayWrap => "counter-may-wrap"
  | .boundWrittenInLoop => "bound-written-in-loop"
  | .boundAddressTaken => "bound-address-taken"
  | .boundNotStable => "bound-not-stable"

inductive BoundKind where
  | constant
  | stableLocal
  | typeRange
  deriving DecidableEq, Repr

/-- One conjunct `c < B`, abstracted as the Rust `Conjunct`. -/
structure Conjunct where
  preserving : Bool
  constant : Bool
  isLocal : Bool
  written : Bool
  addressTaken : Bool
  narrow : Bool
  fitsCounter : Bool
  deriving DecidableEq, Repr

/-- The loop, abstracted as the Rust `Shape`. -/
structure Shape where
  isFor : Bool
  stepIncrement : Bool
  counterLocalInteger : Bool
  counterAddressTaken : Bool
  opaqueControl : Bool
  labelInLoop : Bool
  counterWritten : Bool
  initConstant : Bool
  conjuncts : List Conjunct
  deriving DecidableEq, Repr

def conjunctKind (c : Conjunct) : Except Refusal BoundKind :=
  if !c.preserving then .error .comparisonConverts
  else if c.constant then
    (if c.fitsCounter then .ok .constant else .error .counterMayWrap)
  else if c.isLocal && !c.written && !c.addressTaken then
    (if c.fitsCounter then .ok .stableLocal else .error .counterMayWrap)
  else if c.narrow then
    (if c.fitsCounter then .ok .typeRange else .error .counterMayWrap)
  else if c.isLocal && c.written then .error .boundWrittenInLoop
  else if c.isLocal && c.addressTaken then .error .boundAddressTaken
  else .error .boundNotStable

def admits (c : Conjunct) : Bool :=
  match conjunctKind c with
  | .ok _ => true
  | .error _ => false

def firstRefusal (c : Conjunct) : Option Refusal :=
  match conjunctKind c with
  | .ok _ => none
  | .error r => some r

/-- `none`: admitted; otherwise the first refusal in the Rust precedence. -/
def decide (s : Shape) : Option Refusal :=
  if !s.isFor then some .noCounter
  else if !s.stepIncrement then some .stepNotIncrement
  else if !s.counterLocalInteger then some .counterNotLocalInteger
  else if s.counterAddressTaken then some .counterAddressTaken
  else if s.opaqueControl then some .opaqueControl
  else if s.labelInLoop then some .labelInLoop
  else if s.counterWritten then some .counterWrittenInLoop
  else if !s.initConstant then some .initNotConstant
  else match s.conjuncts with
    | [] => some .testNotLessThan
    | c :: rest => if (c :: rest).any admits then none else firstRefusal c

theorem an_admitted_conjunct_meets_its_premises {c : Conjunct} {k : BoundKind}
    (h : conjunctKind c = .ok k) :
    c.preserving = true ∧ c.fitsCounter = true ∧
      (k = .constant → c.constant = true) ∧
      (k = .stableLocal → c.isLocal = true ∧ c.written = false ∧ c.addressTaken = false) ∧
      (k = .typeRange → c.narrow = true) := by
  unfold conjunctKind at h
  cases c with
  | mk p con loc w a nar fits =>
    cases p <;> cases con <;> cases loc <;> cases w <;> cases a <;> cases nar <;>
      cases fits <;> simp at h <;> subst h <;> simp

/-- The conjunct stage admits only when some conjunct admits. -/
theorem conjunct_stage_admits {cs : List Conjunct}
    (h : (match cs with
      | [] => some Refusal.testNotLessThan
      | c :: rest => if (c :: rest).any admits then none else firstRefusal c) = none) :
    ∃ c ∈ cs, admits c = true := by
  cases cs with
  | nil => simp at h
  | cons c rest =>
    by_cases ha : (c :: rest).any admits = true
    · exact List.any_eq_true.mp ha
    · exfalso
      simp only [ha] at h
      have hc : admits c = false := by
        cases hk : admits c with
        | false => rfl
        | true => exact absurd (List.any_eq_true.mpr ⟨c, List.mem_cons_self, hk⟩) ha
      unfold admits at hc
      unfold firstRefusal at h
      cases hk : conjunctKind c with
      | ok k => simp [hk] at hc
      | error r => simp [hk] at h

theorem an_admitted_shape_meets_the_premises {s : Shape} (h : decide s = none) :
    s.isFor = true ∧ s.stepIncrement = true ∧ s.counterLocalInteger = true ∧
      s.counterAddressTaken = false ∧ s.opaqueControl = false ∧ s.labelInLoop = false ∧
      s.counterWritten = false ∧ s.initConstant = true ∧
      ∃ c ∈ s.conjuncts, admits c = true := by
  obtain ⟨f, st, li, ca, oc, ll, cw, ic, cs⟩ := s
  unfold decide at h
  cases f <;> cases st <;> cases li <;> cases ca <;> cases oc <;> cases ll <;>
    cases cw <;> cases ic <;> simp only [Bool.not_true, Bool.not_false, Bool.false_eq_true,
      ite_false, ite_true, reduceCtorEq] at h
  exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, conjunct_stage_admits h⟩

/-- The base shape the refusal theorems vary: a `for` loop with every gate
passed and the given conjuncts. -/
def gated (cs : List Conjunct) : Shape :=
  { isFor := true, stepIncrement := true, counterLocalInteger := true,
    counterAddressTaken := false, opaqueControl := false, labelInLoop := false,
    counterWritten := false, initConstant := true, conjuncts := cs }

theorem a_while_loop_is_refused (s : Shape) (h : s.isFor = false) :
    decide s = some .noCounter := by
  simp [decide, h]

theorem a_non_increment_step_is_refused (s : Shape) (hf : s.isFor = true)
    (h : s.stepIncrement = false) : decide s = some .stepNotIncrement := by
  simp [decide, hf, h]

theorem an_address_taken_counter_is_refused (s : Shape) (hf : s.isFor = true)
    (hs : s.stepIncrement = true) (hl : s.counterLocalInteger = true)
    (h : s.counterAddressTaken = true) : decide s = some .counterAddressTaken := by
  simp [decide, hf, hs, hl, h]

theorem a_double_increment_is_refused (cs : List Conjunct) :
    decide { gated cs with counterWritten := true } = some .counterWrittenInLoop := by
  simp [decide, gated]

theorem a_non_less_than_test_is_refused :
    decide (gated []) = some .testNotLessThan := by
  simp [decide, gated]

theorem a_wrapping_bound_is_refused (c : Conjunct) (hp : c.preserving = true)
    (hc : c.constant = true) (hf : c.fitsCounter = false) :
    decide (gated [c]) = some .counterMayWrap := by
  cases c with
  | mk p con loc w a nar fits =>
    simp at hp hc hf
    subst hp hc hf
    simp [decide, gated, admits, firstRefusal, conjunctKind]

theorem a_written_bound_is_refused (c : Conjunct) (hp : c.preserving = true)
    (hc : c.constant = false) (hl : c.isLocal = true) (hw : c.written = true)
    (hn : c.narrow = false) : decide (gated [c]) = some .boundWrittenInLoop := by
  cases c with
  | mk p con loc w a nar fits =>
    simp at hp hc hl hw hn
    subst hp hc hl hw hn
    simp [decide, gated, admits, firstRefusal, conjunctKind]

theorem an_address_taken_wide_bound_is_refused (c : Conjunct) (hp : c.preserving = true)
    (hc : c.constant = false) (hl : c.isLocal = true) (hw : c.written = false)
    (ha : c.addressTaken = true) (hn : c.narrow = false) :
    decide (gated [c]) = some .boundAddressTaken := by
  cases c with
  | mk p con loc w a nar fits =>
    simp at hp hc hl hw ha hn
    subst hp hc hl hw ha hn
    simp [decide, gated, admits, firstRefusal, conjunctKind]

theorem a_converting_comparison_is_refused (c : Conjunct) (hp : c.preserving = false) :
    decide (gated [c]) = some .comparisonConverts := by
  cases c with
  | mk p con loc w a nar fits =>
    simp at hp
    subst hp
    simp [decide, gated, admits, firstRefusal, conjunctKind]

/-! ## No verdict moves -/

/-- The four published verdicts; `accused` is `Unsafe` (a Lean keyword). -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | accused
  deriving DecidableEq, Repr

/-- A function's published row: its custody verdict and the loop facts
recorded beside it. -/
structure Row where
  verdict : Verdict
  facts : List (Option Refusal)

def record (r : Row) (fact : Option Refusal) : Row :=
  { r with facts := r.facts ++ [fact] }

theorem recording_a_fact_keeps_the_verdict (r : Row) (fact : Option Refusal) :
    (record r fact).verdict = r.verdict := by
  simp [record]

theorem recording_facts_keeps_the_verdict (r : Row) (facts : List (Option Refusal)) :
    (facts.foldl record r).verdict = r.verdict := by
  induction facts generalizing r with
  | nil => simp
  | cons f rest ih => simp only [List.foldl]; rw [ih]; exact recording_a_fact_keeps_the_verdict r f

/-! ## Shared vectors -/

def conjunctOf : List Bool → Conjunct
  | [p, con, loc, w, a, nar, fits] => ⟨p, con, loc, w, a, nar, fits⟩
  | _ => ⟨false, false, false, false, false, false, false⟩

def shapeOf (gates : List Bool) (cs : List (List Bool)) : Shape :=
  match gates with
  | [f, st, li, ca, oc, ll, cw, ic] => ⟨f, st, li, ca, oc, ll, cw, ic, cs.map conjunctOf⟩
  | _ => ⟨false, false, false, false, false, false, false, false, []⟩

def outcomeCode : Option Refusal → String
  | none => "admitted"
  | some r => r.code

/-- Identical to the Rust `DECIDE_VECTORS`. -/
def decideVectors : List (List Bool × List (List Bool) × String) := [
  ([true, true, true, false, false, false, false, true], [[true, true, false, false, false, false, true]], "admitted"),
  ([true, true, true, false, false, false, false, true], [[true, false, true, false, false, true, true]], "admitted"),
  ([true, true, true, false, false, false, false, true], [[true, false, true, false, true, true, true]], "admitted"),
  ([true, true, true, false, false, false, false, true], [[true, false, true, false, true, false, true]], "bound-address-taken"),
  ([false, false, false, false, false, false, false, false], [], "no-counter"),
  ([true, false, true, false, false, false, false, true], [[true, true, false, false, false, false, true]], "step-not-increment"),
  ([true, true, false, false, false, false, false, true], [], "counter-not-local-integer"),
  ([true, true, true, true, false, false, false, true], [[true, true, false, false, false, false, true]], "counter-address-taken"),
  ([true, true, true, false, true, false, false, true], [[true, true, false, false, false, false, true]], "opaque-control"),
  ([true, true, true, false, false, true, false, true], [[true, true, false, false, false, false, true]], "label-in-loop"),
  ([true, true, true, false, false, false, true, true], [[true, true, false, false, false, false, true]], "counter-written-in-loop"),
  ([true, true, true, false, false, false, false, false], [[true, true, false, false, false, false, true]], "init-not-constant"),
  ([true, true, true, false, false, false, false, true], [], "test-not-less-than"),
  ([true, true, true, false, false, false, false, true], [[false, false, true, false, false, false, true]], "comparison-converts"),
  ([true, true, true, false, false, false, false, true], [[true, true, false, false, false, false, false]], "counter-may-wrap"),
  ([true, true, true, false, false, false, false, true], [[true, false, true, true, false, false, true]], "bound-written-in-loop"),
  ([true, true, true, false, false, false, false, true], [[true, false, false, false, false, false, true]], "bound-not-stable"),
  ([true, true, true, false, false, false, false, true], [[true, false, false, false, false, false, true], [true, true, false, false, false, false, true]], "admitted")
]

theorem decide_vectors_hold :
    decideVectors.all (fun (g, cs, code) => outcomeCode (decide (shapeOf g cs)) == code) = true := by
  decide

end QCore.CountedLoopMeasure
