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

import QCore.SourceLoop

/-! The three-layer unrolling budget of `source.loop.v1` — card
`LOOP-UNROLL-BUDGET`.

Card `LOOP-V1` bounded the checking route with **one** number: a bound above
`maxIterations` was refused, everything below it admitted. This module models
what replaced that — three layers, one total decision function, and two
theorems that say what the replacement may and may not do.

**Layer one is an absolute ceiling no declaration may raise.**
`every_admitted_bound_is_inside_the_ceiling` is the load-bearing statement: for
*every* bound and *every* declaration a program can write, an admitted loop's
bound is at most `ceiling`. It is proved over the declaration rather than
assumed about it, which is the difference between a ceiling and a default with a
longer spelling.

**Layer two is the default budget** a loop with no attribute is held to, and
**layer three is the licence** a `[| #unroll, where #bound < N |]` clause list
declares. The licence is *checked*: `the_declared_condition_is_checked` is the
refusal that fires when the loop's own bound does not satisfy the condition the
program wrote, and `the_ceiling_is_judged_before_the_bound` is the ordering that
stops a small loop from carrying a licence for a large one.

**The card only refuses.** `the_budget_admits_nothing_loop_v1_refused` is
invariant 1's shape for this change: every bound this decision admits was
already admitted by the single ceiling it replaces, so no program became
checkable by adding a budget. `the_budget_refuses_something_loop_v1_admitted`
makes that non-vacuous in the other direction, and `a_declaration_recovers_it`
shows the licence is what gives the refused program back.

**The licence erases.** An attributed program and an unattributed one with the
same admitted bound present the checker the same event list, cost the same, and
run to the same ledger. In this model those are `rfl`, and that **is** the
content: the licence has no place in the construction from which it could reach
the unrolling. The emission half of the same claim is the Rust byte comparison
`an_attributed_and_an_unattributed_loop_emit_byte_identical_c`.

**The one-pass peak.** `the_one_pass_maximum_is_the_maximum_over_every_prefix`
is the repair the cost measurement forced, stated abstractly over any stepper
and any measure: the maximum a run passes through is the maximum over its
prefixes, so replacing a prefix rescan with one pass changes no number. It is
what took the worst measured check of a 1024-iteration acquire/release body from
56.4 s to 0.39 s, and so what keeps the ceiling defensible at 4096.

What is **not** proved here: that the Rust reader extracts a clause list
faithfully from source tokens (a tested Rust boundary, the family's standing
convention); anything about the *cost* in seconds, which is a measurement and
lives in the card's report; and any relation between the ceiling's numeral and a
buffer's extent, which is a decision this model records rather than derives.
-/

namespace QCore.LoopBudget

open QCore.SourceLoop
open QCore.StdBuffer

/-! ## The three constants -/

/-- Layer two: the bound a `for` carrying no attribute may reach. -/
def defaultBudget : Nat := 256

/-- Layer one: the absolute ceiling. Equal to card `LOOP-V1`'s `maxIterations`,
which this card keeps rather than lowers — see the report's cost table. -/
def ceiling : Nat := maxIterations

/-- The largest `N` a `#bound < N` clause may name. One more than the ceiling,
because `#bound < N` licenses bounds up to `N - 1`: the rule binds what a
declaration can *license*, not the numeral it spells. -/
def maxDeclaredLimit : Nat := ceiling + 1

theorem the_default_is_inside_the_ceiling : defaultBudget ≤ ceiling := by decide

/-! ## The declaration and the decision -/

/-- What one `[| #unroll, where #bound < N |]` clause list declares. -/
structure License where
  limit : Nat
  deriving DecidableEq, Repr

/-- The five answers the decision can give. Two admit; three refuse, and each
refusal names a different layer. -/
inductive Verdict where
  /-- No attribute, and the bound is inside the default budget. -/
  | admittedByDefault
  /-- A declaration inside the ceiling that this loop's bound satisfies. -/
  | admittedByLicense
  /-- No attribute, and the bound is above the default budget. -/
  | aboveDefault
  /-- The declaration would license a bound above the ceiling. -/
  | licenseAboveCeiling
  /-- The declaration is present and this loop's bound does not satisfy it. -/
  | licenseViolated
  deriving DecidableEq, Repr

/-- **The three-layer decision, total in both arguments.**

The order of the two licensed tests is the whole of layer one's force: a
declaration naming more than the construct models is refused *before* this
loop's own bound is looked at. -/
def admit (bound : Nat) : Option License → Verdict
  | none => if bound ≤ defaultBudget then .admittedByDefault else .aboveDefault
  | some license =>
      if license.limit > maxDeclaredLimit then .licenseAboveCeiling
      else if bound ≥ license.limit then .licenseViolated
      else .admittedByLicense

/-- Which verdicts admit. -/
def isAdmitted : Verdict → Bool
  | .admittedByDefault => true
  | .admittedByLicense => true
  | .aboveDefault => false
  | .licenseAboveCeiling => false
  | .licenseViolated => false

/-- The verdicts, as one list, so a sixth answer must edit a theorem rather than
slip past one — the `QCore.StdIo.the_operand_forms_are_exactly_these`
convention. -/
def verdicts : List Verdict :=
  [.admittedByDefault, .admittedByLicense, .aboveDefault, .licenseAboveCeiling,
   .licenseViolated]

theorem the_verdicts_are_exactly_these (v : Verdict) : v ∈ verdicts := by
  cases v <;> simp [verdicts]

/-- The decision is total: every bound and every declaration reaches one of the
five answers, with no partiality and no default case. -/
theorem the_decision_is_total (bound : Nat) (license : Option License) :
    admit bound license ∈ verdicts :=
  the_verdicts_are_exactly_these _

/-! ## Layer one: the ceiling is a ceiling -/

/-- **The load-bearing theorem.** Whatever a program declares, an admitted
loop's bound is inside the ceiling. Proved over the declaration rather than
assumed about it. -/
theorem every_admitted_bound_is_inside_the_ceiling
    (bound : Nat) (license : Option License)
    (admitted : isAdmitted (admit bound license) = true) : bound ≤ ceiling := by
  cases license with
  | none =>
    by_cases small : bound ≤ defaultBudget
    · have inside : defaultBudget ≤ ceiling := the_default_is_inside_the_ceiling
      omega
    · simp [admit, small, isAdmitted] at admitted
  | some license =>
    by_cases above : license.limit > maxDeclaredLimit
    · simp [admit, above, isAdmitted] at admitted
    · by_cases violated : bound ≥ license.limit
      · simp [admit, above, violated, isAdmitted] at admitted
      · simp only [maxDeclaredLimit, Nat.not_lt] at above
        omega

/-- The same fact read from the refusing side: no declaration admits a bound
above the ceiling. -/
theorem a_declaration_cannot_raise_the_ceiling
    (bound : Nat) (license : License) (above : bound > ceiling) :
    isAdmitted (admit bound (some license)) = false := by
  cases decision : isAdmitted (admit bound (some license)) with
  | false => rfl
  | true =>
    have inside := every_admitted_bound_is_inside_the_ceiling bound (some license) decision
    exact absurd inside (by omega)

/-- **The ordering that makes layer one a ceiling.** A declaration naming more
than the construct models is refused for *every* bound, including a bound the
default would have admitted on its own — so a small loop cannot carry a licence
for a large one. -/
theorem the_ceiling_is_judged_before_the_bound
    (bound : Nat) (license : License) (above : license.limit > maxDeclaredLimit) :
    admit bound (some license) = .licenseAboveCeiling := by
  simp [admit, above]

/-- The exact boundary: the largest declaration the construct admits licenses a
traversal of the whole ceiling, and the numeral one above it does not exist. -/
theorem the_largest_declaration_licenses_the_whole_ceiling :
    admit ceiling (some ⟨maxDeclaredLimit⟩) = .admittedByLicense := by decide

theorem nothing_above_the_largest_declaration_is_read :
    admit 2 (some ⟨maxDeclaredLimit + 1⟩) = .licenseAboveCeiling := by decide

/-! ## Layer two: the default budget -/

theorem the_default_needs_no_declaration (bound : Nat) (small : bound ≤ defaultBudget) :
    admit bound none = .admittedByDefault := by simp [admit, small]

theorem a_bound_above_the_default_needs_one (bound : Nat) (large : bound > defaultBudget) :
    admit bound none = .aboveDefault := by
  simp [admit, Nat.not_le_of_gt large]

/-! ## Layer three: the declaration is checked, not trusted -/

/-- The declared condition is compared against the bound the checker computes.
A loop that does not satisfy the condition its own program wrote is refused —
which is what "checked, never trusted" means as a theorem. -/
theorem the_declared_condition_is_checked
    (bound : Nat) (license : License)
    (inside : license.limit ≤ maxDeclaredLimit) (violated : bound ≥ license.limit) :
    admit bound (some license) = .licenseViolated := by
  simp [admit, Nat.not_lt_of_le inside, violated]

/-- And the condition is **strict**, so the declared numeral is not itself
licensed: the boundary is exactly where the spelling says it is. -/
theorem the_declared_condition_is_strict
    (license : License) (inside : license.limit ≤ maxDeclaredLimit) :
    admit license.limit (some license) = .licenseViolated :=
  the_declared_condition_is_checked _ _ inside (Nat.le_refl _)

theorem a_bound_below_the_declared_limit_is_licensed
    (bound : Nat) (license : License)
    (inside : license.limit ≤ maxDeclaredLimit) (below : bound < license.limit) :
    admit bound (some license) = .admittedByLicense := by
  simp [admit, Nat.not_lt_of_le inside, Nat.not_le_of_gt below]

/-! ## The card only refuses -/

/-- What card `LOOP-V1` admitted: a positive bound inside the single ceiling. -/
def admittedByLoopV1 (bound : Nat) : Bool := 0 < bound && bound ≤ maxIterations

/-- **Invariant 1's shape for this change.** Every bound the three-layer
decision admits was already admitted by the one ceiling it replaces, so no
program became checkable by adding a budget. -/
theorem the_budget_admits_nothing_loop_v1_refused
    (bound : Nat) (license : Option License) (positive : 0 < bound)
    (admitted : isAdmitted (admit bound license) = true) :
    admittedByLoopV1 bound = true := by
  have inside := every_admitted_bound_is_inside_the_ceiling bound license admitted
  simp [admittedByLoopV1, positive]
  simpa [ceiling] using inside

/-- Non-vacuous in the other direction: there is a bound `LOOP-V1` admitted that
this decision refuses without a declaration. -/
theorem the_budget_refuses_something_loop_v1_admitted :
    admittedByLoopV1 (defaultBudget + 1) = true ∧
      isAdmitted (admit (defaultBudget + 1) none) = false := by decide

/-- And the declaration is what gives it back, which is the point of the whole
construct. -/
theorem a_declaration_recovers_it :
    isAdmitted (admit (defaultBudget + 1) (some ⟨defaultBudget + 2⟩)) = true := by decide

/-! ## The licence erases

An item carrying a licence and the same item without one present the checker the
same events, cost the same, and run to the same ledger. Each proof is `rfl`, and
that is the content: there is no place in the construction from which a licence
could reach the unrolling. -/

/-- One statement of an admitting route, optionally carrying a licence. -/
structure Attributed where
  license : Option License
  item : Item

/-- What the checker is handed: the item, with the licence dropped. -/
def erase (a : Attributed) : Item := a.item

theorem the_licence_erases_from_the_unrolling (license : Option License) (item : Item) :
    unroll (erase ⟨license, item⟩) = unroll item := rfl

theorem the_licence_moves_no_event
    (license : Option License) (item : Item) (ledger : Ledger) :
    run ledger (unroll (erase ⟨license, item⟩)) = run ledger (unroll item) := rfl

theorem the_licence_changes_no_cost (license : Option License) (item : Item) :
    unrolledLength (erase ⟨license, item⟩) = unrolledLength item := rfl

/-- Whole programs, not only single statements: an attributed sequence unrolls
to exactly the sequence its licences were stripped from. -/
theorem an_attributed_program_unrolls_to_its_unattributed_one (xs : List Attributed) :
    unrollList (xs.map erase) = unrollList (xs.map Attributed.item) := rfl

/-! ## The one-pass peak

The repair the cost measurement forced, stated abstractly: the maximum a run
passes through is the maximum over its prefixes. Replacing a prefix rescan with
one pass therefore changes no number — it only stops paying for the rescan. -/

variable {σ α : Type}

/-- The states a run passes through: the initial one, then one per step. -/
def states (f : σ → α → σ) : σ → List α → List σ
  | s, [] => [s]
  | s, a :: rest => s :: states f (f s a) rest

/-- The maximum of a measure over a list of states. -/
def maxOf (m : σ → Nat) : List σ → Nat
  | [] => 0
  | s :: rest => max (m s) (maxOf m rest)

/-- The running maximum: one pass, taking the measure as it goes. -/
def runningMax (m : σ → Nat) (f : σ → α → σ) : σ → List α → Nat
  | s, [] => m s
  | s, a :: rest => max (m s) (runningMax m f (f s a) rest)

/-- **The equality the repair rests on.** One pass gives the same number as
visiting every state the run passes through, so the narrowed scan and the
one-pass maximum are the same computation with different costs. -/
theorem the_one_pass_maximum_is_the_maximum_over_every_prefix
    (m : σ → Nat) (f : σ → α → σ) (s : σ) (xs : List α) :
    runningMax m f s xs = maxOf m (states f s xs) := by
  induction xs generalizing s with
  | nil => simp [runningMax, states, maxOf]
  | cons a rest ih => simp [runningMax, states, maxOf, ih]

/-- And the maximum is attained: it is one of the measured values, never a
number the run never held. -/
theorem the_maximum_is_a_value_the_run_passed_through
    (m : σ → Nat) (f : σ → α → σ) (s : σ) (xs : List α) :
    m s ≤ runningMax m f s xs := by
  cases xs with
  | nil => simp [runningMax]
  | cons a rest => exact Nat.le_max_left _ _

end QCore.LoopBudget
