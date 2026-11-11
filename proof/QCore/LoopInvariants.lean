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

import QCore.ContextualCustody
import QCore.CountedUsers
import QCore.VerdictSemigroup

/-!
# Loops, recursion and budgets (card CG27-LOOPS-RECURSION-BUDGETS)

Gateway rule 11x. Four parts, each the model of one Rust rule.

**Invariant induction** (`Induction`). An invariant that holds on entry and is
preserved by every body step holds after any number of iterations; a body step
that leaves it is a counterexample after one iteration, so a check that finds
one must refuse, never assume. A settled post-fixed point of a monotone
transfer is such an invariant, which is what the custody work list's settled
loop-head state is.

**The counted-user loop rule** (`CountedLoop`, `legacy_counted_loops.rs`). A
loop whose every body operation is count-neutral leaves a path's net count and
its pending release check exactly as if it ran zero times, for any number of
iterations: the invariant "the counts equal their head values" is preserved by
every body path. A body with an addition, a drop, a check or an opaque call is
refused, and one iteration of an addition already changes the count.

**Widening** (`Widening`). The custody join widens an alias set above
`MAX_ALIAS_TARGETS` to the unknown target. Every strict growth of a set lowers
a natural potential, so a chain of strict growths is finite, and the widened
value is absorbing and reported, never `Safe`.

**Signed threshold rank** (`SignedThreshold`, `legacy_signed_rank.rs`). The
control is partitioned at a literal threshold `c`: base `n <= c`, step `n > c`,
rank `max(n - c, 0)`. A step actual `n - k` (`k >= 1`) or the literal `0` has a
strictly smaller rank, so the step relation is well founded and publication is
strong induction over the rank, reusing
`QCore.UnsignedCountdown.publish_by_mutual_rank_induction`. On a 32-bit `int`
with `0 <= c`, `k <= 1024` the actual is representable. The unsigned route
keeps `n - 1` only: `n - 2` from `n = 1` wraps to the top of the range.

**Budgets** (`Budget`). A budget-exhausted leaf is `Unknown`, so the function's
fold is never `Safe`, whatever its other leaves are.

## What this is not

Path enumeration, the flow-insensitive neutrality reading of a loop's tokens,
the guard and actual extraction and the Rust correspondence are trust boundary
28's premises, pinned by `crates/qcore-c-gateway/tests/loops_recursion_budgets.rs`
and the shared vectors there. No statement here is about the termination of a C
loop: the counted-user rule needs none, because its invariant is preserved by
any number of iterations, including all of an infinite run's prefixes.
-/

namespace QCore.LoopInvariants

/-! ## Invariant induction -/

namespace Induction

/-- `n` iterations of a body step. -/
def iterate {σ : Type} (body : σ → σ) : Nat → σ → σ
  | 0, state => state
  | n + 1, state => iterate body n (body state)

/-- Holds on entry and preserved by the body: holds after every iteration. -/
theorem invariant_holds_after_every_iteration {σ : Type} (invariant : σ → Prop)
    (body : σ → σ) (preserved : ∀ state, invariant state → invariant (body state)) :
    ∀ n state, invariant state → invariant (iterate body n state) := by
  intro n
  induction n with
  | zero => intro state holds; exact holds
  | succ n ih => intro state holds; exact ih (body state) (preserved state holds)

/-- A nondeterministic body (one step per body path) preserves an invariant
when every path does; any interleaving of paths keeps it. -/
def iteratePaths {σ : Type} : List (σ → σ) → σ → σ
  | [], state => state
  | step :: steps, state => iteratePaths steps (step state)

theorem invariant_holds_along_every_path_sequence {σ : Type} (invariant : σ → Prop)
    (paths : List (σ → σ))
    (preserved : ∀ step ∈ paths, ∀ state, invariant state → invariant (step state)) :
    ∀ (run : List (σ → σ)), (∀ step ∈ run, step ∈ paths) →
      ∀ state, invariant state → invariant (iteratePaths run state) := by
  intro run
  induction run with
  | nil => intro _ state holds; exact holds
  | cons step steps ih =>
    intro member state holds
    have inPaths : step ∈ paths := member step List.mem_cons_self
    exact ih (fun s hs => member s (List.mem_cons_of_mem _ hs)) (step state)
      (preserved step inPaths state holds)

/-- The decidable induction check over a finite list of observed body
transitions: every transition that starts inside the invariant ends inside it. -/
def checkInduction {σ : Type} (invariant : σ → Bool) (transitions : List (σ × σ)) : Bool :=
  transitions.all fun pair => !invariant pair.1 || invariant pair.2

/-- A transition that leaves the invariant fails the check: such an invariant is
refused, never assumed. -/
theorem a_broken_transition_is_refused {σ : Type} (invariant : σ → Bool)
    (transitions : List (σ × σ)) (pre post : σ) (member : (pre, post) ∈ transitions)
    (holds : invariant pre = true) (broken : invariant post = false) :
    checkInduction invariant transitions = false := by
  unfold checkInduction
  rw [List.all_eq_false]
  exact ⟨(pre, post), member, by simp [holds, broken]⟩

/-- A passing check is exactly preservation on every listed transition. -/
theorem a_passing_check_preserves {σ : Type} (invariant : σ → Bool)
    (transitions : List (σ × σ)) (passes : checkInduction invariant transitions = true) :
    ∀ pair ∈ transitions, invariant pair.1 = true → invariant pair.2 = true := by
  intro pair member holds
  unfold checkInduction at passes
  rw [List.all_eq_true] at passes
  have := passes pair member
  simp [holds] at this
  exact this

/-- A post-fixed point of a monotone transfer above the entry is an inductive
invariant: every iterate from the entry stays below it. The custody work list's
settled loop-head state is such a point. -/
theorem postfixpoint_bounds_every_iterate {σ : Type} (le : σ → σ → Prop)
    (transfer : σ → σ) (entry bound : σ)
    (monotone : ∀ a b, le a b → le (transfer a) (transfer b))
    (trans : ∀ a b c, le a b → le b c → le a c)
    (above : le entry bound) (post : le (transfer bound) bound) :
    ∀ n, le (iterate transfer n entry) bound := by
  have shift : ∀ n state, iterate transfer (n + 1) state = iterate transfer n (transfer state) := by
    intro n state; rfl
  have commute : ∀ n state, iterate transfer n (transfer state) = transfer (iterate transfer n state) := by
    intro n
    induction n with
    | zero => intro state; rfl
    | succ n ih => intro state; exact ih (transfer state)
  intro n
  induction n with
  | zero => exact above
  | succ n ih =>
    rw [shift, commute]
    exact trans _ _ _ (monotone _ _ ih) post

end Induction

/-! ## The counted-user loop rule -/

namespace CountedLoop
open QCore.CountedUsers

/-- The loop rule admits a body only when every operation on every body path is
count-neutral. -/
def admitted (body : List (List UOp)) : Bool :=
  body.all fun path => path.all fun op => op == .neutral

/-- Any sequence of iterations of an admitted body. -/
def iterations (run : List (List UOp)) : List UOp := run.flatten

theorem neutral_ops_have_no_effect_on_delta (ops rest : List UOp)
    (neutral : ∀ op ∈ ops, op = .neutral) : delta (ops ++ rest) = delta rest := by
  induction ops with
  | nil => rfl
  | cons op ops ih =>
    have hop : op = .neutral := neutral op List.mem_cons_self
    have hrest : ∀ o ∈ ops, o = .neutral := fun o m => neutral o (List.mem_cons_of_mem _ m)
    subst hop
    simp only [List.cons_append, delta, stepOf, ih hrest]
    cases delta rest <;> simp

theorem neutral_ops_have_no_effect_on_pending (ops rest : List UOp) (count : Int) (owed : Bool)
    (neutral : ∀ op ∈ ops, op = .neutral) :
    pendingFrom count owed (ops ++ rest) = pendingFrom count owed rest := by
  induction ops with
  | nil => rfl
  | cons op ops ih =>
    have hop : op = .neutral := neutral op List.mem_cons_self
    have hrest : ∀ o ∈ ops, o = .neutral := fun o m => neutral o (List.mem_cons_of_mem _ m)
    subst hop
    simp only [List.cons_append, pendingFrom]
    exact ih hrest

theorem admitted_iterations_are_neutral (body run : List (List UOp))
    (ok : admitted body = true) (drawn : ∀ path ∈ run, path ∈ body) :
    ∀ op ∈ iterations run, op = .neutral := by
  intro op member
  unfold iterations at member
  rw [List.mem_flatten] at member
  obtain ⟨path, inRun, inPath⟩ := member
  unfold admitted at ok
  rw [List.all_eq_true] at ok
  have := ok path (drawn path inRun)
  rw [List.all_eq_true] at this
  simpa using this op inPath

/-- The loop's summary: any number of iterations of an admitted body, spliced
anywhere into a path, leaves the path's net count as if the loop ran zero
times. -/
theorem an_admitted_loop_preserves_delta (body run : List (List UOp)) (before after : List UOp)
    (ok : admitted body = true) (drawn : ∀ path ∈ run, path ∈ body)
    (known : ∃ a, delta before = some a) :
    delta (before ++ iterations run ++ after) = delta (before ++ after) := by
  obtain ⟨a, ha⟩ := known
  have neutral := admitted_iterations_are_neutral body run ok drawn
  have middle : delta (iterations run ++ after) = delta after :=
    neutral_ops_have_no_effect_on_delta _ _ neutral
  cases hafter : delta after with
  | none =>
    -- An unknown suffix stays unknown either way.
    have unknown : ∀ (pre post : List UOp), delta post = none → delta (pre ++ post) = none := by
      intro pre post hpost
      induction pre with
      | nil => exact hpost
      | cons op ops ih =>
        simp only [List.cons_append, delta, ih]
        cases stepOf op <;> rfl
    rw [List.append_assoc, unknown before _ (by rw [middle, hafter]), unknown before _ hafter]
  | some b =>
    rw [List.append_assoc, delta_append before _ a b ha (by rw [middle, hafter]),
      delta_append before after a b ha hafter]

/-- And its pending release check, from any count. -/
theorem an_admitted_loop_preserves_pending (body run : List (List UOp)) (after : List UOp)
    (count : Int) (owed : Bool) (ok : admitted body = true)
    (drawn : ∀ path ∈ run, path ∈ body) :
    pendingFrom count owed (iterations run ++ after) = pendingFrom count owed after :=
  neutral_ops_have_no_effect_on_pending _ _ count owed
    (admitted_iterations_are_neutral body run ok drawn)

/-- A body path with an addition is refused. -/
theorem a_counting_body_is_refused (body : List (List UOp)) (path : List UOp)
    (member : path ∈ body) (adds : UOp.add ∈ path) : admitted body = false := by
  unfold admitted
  apply Bool.eq_false_iff.mpr
  intro all
  rw [List.all_eq_true] at all
  have every := all path member
  rw [List.all_eq_true] at every
  exact absurd (every .add adds) (by decide)

/-- An opaque call in the body is refused. -/
theorem an_opaque_body_is_refused (body : List (List UOp)) (path : List UOp)
    (member : path ∈ body) (unknown : UOp.opaque ∈ path) : admitted body = false := by
  unfold admitted
  apply Bool.eq_false_iff.mpr
  intro all
  rw [List.all_eq_true] at all
  have every := all path member
  rw [List.all_eq_true] at every
  exact absurd (every .opaque unknown) (by decide)

/-- Non-vacuity: assuming the "count unchanged" invariant of a counting body
would be wrong after one iteration. -/
theorem one_counting_iteration_changes_the_count :
    delta (iterations [[.add]]) = some 1 ∧ delta [] = some 0 := by
  constructor <;> rfl

end CountedLoop

/-! ## Widening -/

namespace Widening

/-- An alias value: a target set of the given size, or the widened unknown. -/
inductive Alias where
  | set (size : Nat)
  | unknown
  deriving DecidableEq, Repr

/-- The join with a set of `incoming` targets, widened above `bound`. -/
def grow (bound : Nat) : Alias → Nat → Alias
  | .unknown, _ => .unknown
  | .set size, incoming =>
    let joined := max size incoming
    if joined > bound then .unknown else .set joined

/-- What is left to grow before widening. -/
def potential (bound : Nat) : Alias → Nat
  | .set size => bound + 1 - size
  | .unknown => 0

/-- Every strict change lowers the potential, so a chain of strict changes has
at most `bound + 1` steps. -/
theorem a_strict_growth_lowers_the_potential (bound : Nat) (value : Alias) (incoming : Nat)
    (within : ∀ size, value = .set size → size ≤ bound)
    (changed : grow bound value incoming ≠ value) :
    potential bound (grow bound value incoming) < potential bound value := by
  cases value with
  | unknown => simp [grow] at changed
  | set size =>
    have hs := within size rfl
    unfold grow at changed ⊢
    by_cases h : max size incoming > bound
    · simp only [h, ↓reduceIte, potential]; omega
    · simp only [h, ↓reduceIte] at changed ⊢
      simp only [potential]
      have grew : max size incoming ≠ size := fun e => changed (by rw [e])
      omega

/-- The widened value is absorbing. -/
theorem unknown_is_absorbing (bound incoming : Nat) :
    grow bound .unknown incoming = .unknown := rfl

/-- Growth stays inside the bound until it widens. -/
theorem growth_stays_within (bound : Nat) (value : Alias) (incoming size : Nat)
    (result : grow bound value incoming = .set size) : size ≤ bound := by
  cases value with
  | unknown => simp [grow] at result
  | set old =>
    unfold grow at result
    by_cases h : max old incoming > bound
    · simp [h] at result
    · simp only [h, ↓reduceIte, Alias.set.injEq] at result; omega

/-- A widened alias is reported: the function carrying it is never `Safe`. -/
def leafOf : Alias → QCore.VerdictSemigroup.V
  | .unknown => .unknown
  | .set _ => .safe

theorem a_widened_alias_is_never_safe (bound : Nat) (value : Alias) (incoming : Nat)
    (widened : grow bound value incoming = .unknown) (others : List QCore.VerdictSemigroup.V) :
    QCore.VerdictSemigroup.fold (leafOf (grow bound value incoming) :: others) ≠
      QCore.VerdictSemigroup.V.safe := by
  rw [widened]
  intro h
  have := QCore.VerdictSemigroup.every_link_is_below_the_fold
    (x := .unknown) (xs := .unknown :: others) List.mem_cons_self
  simp only [leafOf] at h
  rw [h] at this
  simp [QCore.VerdictSemigroup.V.rank] at this

end Widening

/-! ## Signed threshold rank -/

namespace SignedThreshold

/-- The rank of a control value against the threshold. -/
def rank (threshold value : Int) : Nat := (value - threshold).toNat

theorem the_base_context_has_rank_zero (threshold value : Int) (base : value ≤ threshold) :
    rank threshold value = 0 := by
  unfold rank; omega

theorem the_step_context_has_a_positive_rank (threshold value : Int) (step : threshold < value) :
    0 < rank threshold value := by
  unfold rank; omega

/-- An admitted actual: `n - k` with `k >= 1`, or the literal `0`. -/
inductive Actual where
  | decrement (step : Nat)
  | zero
  deriving DecidableEq, Repr

def Actual.admitted : Actual → Bool
  | .decrement step => 1 ≤ step && step ≤ 1024
  | .zero => true

def Actual.value (control : Int) : Actual → Int
  | .decrement step => control - step
  | .zero => 0

/-- Every admitted actual from the step context has a strictly smaller rank,
for a nonnegative threshold. -/
theorem an_admitted_actual_decreases (threshold control : Int) (actual : Actual)
    (nonnegative : 0 ≤ threshold) (step : threshold < control)
    (ok : actual.admitted = true) :
    rank threshold (actual.value control) < rank threshold control := by
  cases actual with
  | decrement k =>
    simp only [Actual.admitted, Bool.and_eq_true, decide_eq_true_eq] at ok
    simp only [rank, Actual.value]; omega
  | zero => simp only [rank, Actual.value]; omega

/-- Several ranks at once: every admitted actual of a multi-rank body decreases. -/
theorem every_multi_rank_actual_decreases (threshold control : Int) (actuals : List Actual)
    (nonnegative : 0 ≤ threshold) (step : threshold < control)
    (ok : ∀ actual ∈ actuals, actual.admitted = true) :
    ∀ actual ∈ actuals, rank threshold (actual.value control) < rank threshold control :=
  fun actual member => an_admitted_actual_decreases threshold control actual nonnegative step
    (ok actual member)

structure Invocation where
  function : Nat
  control : Int
  deriving DecidableEq, Repr

/-- A recursive edge: only from the step context, with an admitted actual. -/
def Calls (threshold : Int) (callee caller : Invocation) : Prop :=
  threshold < caller.control ∧ ∃ actual : Actual, actual.admitted = true ∧
    callee.control = actual.value caller.control

theorem threshold_calls_are_well_founded (threshold : Int) (nonnegative : 0 ≤ threshold) :
    WellFounded (Calls threshold) := by
  apply Subrelation.wf (r := (measure fun invocation : Invocation =>
    rank threshold invocation.control).rel)
  · intro callee caller call
    obtain ⟨step, actual, ok, same⟩ := call
    show rank threshold callee.control < rank threshold caller.control
    rw [same]
    exact an_admitted_actual_decreases threshold caller.control actual nonnegative step ok
  · exact (measure _).wf

theorem the_base_context_makes_no_admitted_call (threshold : Int) (callee caller : Invocation)
    (base : caller.control ≤ threshold) : ¬ Calls threshold callee caller := by
  intro call
  exact absurd call.1 (by omega)

/-- Publication: the base validates at rank zero with no recursive summary, the
step validates at rank `r` reading only ranks below `r`; then every rank holds. -/
theorem publish_by_threshold_induction (threshold : Int) (valid : Nat → Int → Prop)
    (base : ∀ function value, value ≤ threshold → valid function value)
    (step : ∀ value, threshold < value →
      (∀ smaller, rank threshold smaller < rank threshold value → ∀ function, valid function smaller) →
      ∀ function, valid function value) :
    ∀ value function, valid function value := by
  have byRank := QCore.UnsignedCountdown.publish_by_mutual_rank_induction
    (fun function r => ∀ value, rank threshold value = r → valid function value)
    (by
      intro function value zero
      exact base function value (by unfold rank at zero; omega))
    (by
      intro r positive lower function value hr
      have above : threshold < value := by unfold rank at hr; omega
      exact step value above (fun smaller hsmaller g =>
        lower (rank threshold smaller) (by omega) g smaller rfl) function)
  intro value function
  exact byRank (rank threshold value) function value rfl

/-- A sign guard `n OP literal`, normalised so the control is on the left (the
Rust reader mirrors `literal OP n` first). -/
inductive Comparison where
  | le
  | lt
  | gt
  | ge
  deriving DecidableEq, Repr

/-- The guard's truth at a control value. -/
def holds (comparison : Comparison) (literal : Nat) (value : Int) : Bool :=
  match comparison with
  | .le => decide (value ≤ literal)
  | .lt => decide (value < literal)
  | .gt => decide (value > literal)
  | .ge => decide (value ≥ literal)

/-- The reading `legacy_signed_rank::sign_guard` makes: the threshold and
whether the guard is true in the base context, or a refusal (a negative
threshold, or one above `MAX_THRESHOLD = 1024`). -/
def readGuard (comparison : Comparison) (literal : Nat) : Option (Nat × Bool) :=
  let reading : Option (Nat × Bool) := match comparison with
    | .le => some (literal, true)
    | .lt => if literal = 0 then none else some (literal - 1, true)
    | .gt => some (literal, false)
    | .ge => if literal = 0 then none else some (literal - 1, false)
  match reading with
  | some (threshold, base) => if threshold ≤ 1024 then some (threshold, base) else none
  | none => none

/-- The respelling is sound: a read guard is true exactly in the base context
(`n <= c`) when it is a base guard, and exactly in the step context otherwise,
at every control value. So deciding it by the context is deciding it. -/
theorem a_read_guard_is_decided_by_its_context (comparison : Comparison) (literal : Nat)
    (threshold : Nat) (base : Bool) (read : readGuard comparison literal = some (threshold, base))
    (value : Int) :
    holds comparison literal value = (decide (value ≤ threshold) == base) := by
  cases comparison <;> simp only [readGuard] at read
  · by_cases h : literal ≤ 1024
    · simp only [h, ↓reduceIte, Option.some.injEq, Prod.mk.injEq] at read
      obtain ⟨rfl, rfl⟩ := read
      simp [holds]
    · simp [h] at read
  · by_cases zero : literal = 0
    · simp [zero] at read
    · by_cases h : literal - 1 ≤ 1024
      · simp only [zero, h, ↓reduceIte, Option.some.injEq, Prod.mk.injEq] at read
        obtain ⟨rfl, rfl⟩ := read
        simp only [holds]
        by_cases v : value < (literal : Int)
        · have : value ≤ ((literal - 1 : Nat) : Int) := by omega
          simp [v, this]
        · have : ¬ value ≤ ((literal - 1 : Nat) : Int) := by omega
          simp [v, this]
      · simp [zero, h] at read
  · by_cases h : literal ≤ 1024
    · simp only [h, ↓reduceIte, Option.some.injEq, Prod.mk.injEq] at read
      obtain ⟨rfl, rfl⟩ := read
      simp only [holds]
      by_cases v : value > (literal : Int)
      · have : ¬ value ≤ (literal : Int) := by omega
        simp [v, this]
      · have : value ≤ (literal : Int) := by omega
        simp [v, this]
    · simp [h] at read
  · by_cases zero : literal = 0
    · simp [zero] at read
    · by_cases h : literal - 1 ≤ 1024
      · simp only [zero, h, ↓reduceIte, Option.some.injEq, Prod.mk.injEq] at read
        obtain ⟨rfl, rfl⟩ := read
        simp only [holds]
        by_cases v : value ≥ (literal : Int)
        · have : ¬ value ≤ ((literal - 1 : Nat) : Int) := by omega
          simp [v, this]
        · have : value ≤ ((literal - 1 : Nat) : Int) := by omega
          simp [v, this]
      · simp [zero, h] at read

/-- Shared vectors, read by `crates/qcore-c-gateway/src/legacy_signed_rank.rs`'s
test `lean_vectors_match_the_rust_reader`: `(comparison, literal, reading)`. -/
def guardVectors : List (Comparison × Nat × Option (Nat × Bool)) :=
  [ (.le, 0, some (0, true)), (.le, 1, some (1, true)), (.lt, 1, some (0, true)),
    (.lt, 0, none), (.gt, 0, some (0, false)), (.ge, 1, some (0, false)),
    (.ge, 0, none), (.ge, 2, some (1, false)), (.le, 1024, some (1024, true)),
    (.le, 1025, none), (.gt, 1025, none), (.lt, 1025, some (1024, true)) ]

theorem guard_vectors_hold :
    guardVectors.all (fun vector => readGuard vector.1 vector.2.1 == vector.2.2) = true := by
  decide

/-- Shared vectors for the step actual `n - k`: `(k, admitted)`. -/
def stepVectors : List (Nat × Bool) :=
  [(0, false), (1, true), (2, true), (1024, true), (1025, false)]

theorem step_vectors_hold :
    stepVectors.all (fun vector => (Actual.decrement vector.1).admitted == vector.2) = true := by
  decide

/-- Representability on a 32-bit `int`: from the step context with a threshold
in `[0, 1024]` and a step in `[1, 1024]`, the actual stays above `INT_MIN`. -/
theorem the_actual_is_representable (threshold control : Int) (step : Nat)
    (nonnegative : 0 ≤ threshold) (above : threshold < control)
    (small : step ≤ 1024) (top : control ≤ 2147483647) :
    -2147483648 ≤ control - step ∧ control - step ≤ 2147483647 := by
  omega

/-- Why the unsigned route keeps `n - 1` alone: `n - 2` from `n = 1` wraps to the
top of a 32-bit range, a larger rank. -/
theorem an_unsigned_double_step_can_increase :
    (1 + 4294967296 - 2) % 4294967296 = 4294967295 ∧ 1 < 4294967295 := by decide

/-- Why a signed control needs a threshold, not the zero/nonzero partition:
the nonzero context contains `-1`, whose decrement moves away from zero. -/
theorem a_signed_nonzero_decrement_moves_away_from_zero :
    ((-1 : Int) ≠ 0) ∧ (-1 : Int) - 1 < -1 ∧ rank 0 (-1) = 0 := by decide

end SignedThreshold

/-! ## Budgets -/

namespace Budget
open QCore.VerdictSemigroup

/-- One leaf of a function: decided, or a budget the analysis exhausted. -/
inductive Leaf where
  | decided (verdict : V)
  | exhausted
  deriving DecidableEq, Repr

def leafVerdict : Leaf → V
  | .decided verdict => verdict
  | .exhausted => .unknown

def functionVerdict (leaves : List Leaf) : V := fold (leaves.map leafVerdict)

/-- A budget-exhausted leaf is coverage, reported `Unknown` at least: the
function is never `Safe` (and never `Pending`), whatever its other leaves. -/
theorem an_exhausted_budget_is_never_safe (leaves : List Leaf)
    (member : Leaf.exhausted ∈ leaves) :
    functionVerdict leaves ≠ .safe ∧ 2 ≤ (functionVerdict leaves).rank := by
  have link : V.unknown ∈ leaves.map leafVerdict :=
    List.mem_map.mpr ⟨.exhausted, member, rfl⟩
  have bound := every_link_is_below_the_fold link
  refine ⟨?_, bound⟩
  intro h
  unfold functionVerdict at h
  rw [h] at bound
  simp [V.rank] at bound

/-- Non-vacuity: without the exhausted leaf the same function is `Safe`. -/
theorem the_same_function_without_exhaustion_is_safe :
    functionVerdict [.decided .safe] = .safe ∧
      functionVerdict [.decided .safe, .exhausted] = .unknown := by
  constructor <;> rfl

end Budget

end QCore.LoopInvariants
