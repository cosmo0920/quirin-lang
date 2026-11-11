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

import QCore.NullTriage

/-!
# Exact and inexact path readings (card NULL-TRIAGE-UNRESOLVED-PATHS, gateway rules 15q–15u)

The abstract rules of `crates/qcore-core/src/null_paths.rs` and of the path
search `crates/qcore-c-gateway/src/null_deref/witness.rs`, on top of
`QCore.NullTriage`.

* **No free widening** (`a_call_result_never_contributes`,
  `a_witness_holds_whatever_the_callees_return`). A value is an *entry*
  value (a formal, a memory value read since the last call or store, free by
  the function's entry premise) or a *call result*. A witness path may take
  an undecided arm only over an entry value; a call result's condition is
  taken only when its bound decides it. So a validated witness never chooses
  what a callee returns: every valuation that agrees with the witness on the
  entry values takes every arm, whatever the call results are.
* **A bound decides soundly** (`a_bound_excludes_its_outside_arm`): a value
  inside its bound (the C type's range, narrowed by a checked result-range
  contract) never takes an arm whose set misses the bound.
* **Inexact exploration proves infeasibility** (`exhaustive_search_is_sound`).
  A program is a tree of branches over a valuation. The search follows a
  branch's one arm only when its facts decide the condition (soundly), and
  otherwise both arms, whether the condition is exact or not. When no
  explored leaf reaches the dereference NULL, no valuation consistent with
  the facts does.
* **Difference and sum arms are exact** (`difference_attains`,
  `sum_attains`): over two independent hole-free intervals every value of
  the difference (sum) interval is attained, so an arm whose set meets it
  has an execution.
* **A monotone counter keeps its least start** (`counter_stays_above`): the
  body check of a result-range contract.

## What this is not

A model of C or of the gateway's reading of Clang's AST: that the search's
facts are sound for the C values (decided conditions, cached memory values
forgotten at every call and at every assignment of a variable they spell)
is a review premise (trust boundary 28), pinned by the controls in
`crates/qcore-c-gateway/tests/null_triage_unresolved_paths.rs`.
-/

namespace QCore.NullTriagePaths

/-! ## Readings and the exploration step -/

inductive Reading where
  | decided (value : Bool)
  | exact
  | inexact
  deriving DecidableEq, Repr

inductive StepOutcome where
  | taken | infeasible | takenInexact
  deriving DecidableEq, Repr

/-- `null_paths::explore`. -/
def explore : Reading → Bool → StepOutcome
  | .decided v, arm => if v = arm then .taken else .infeasible
  | .exact, _ => .taken
  | .inexact, _ => .takenInexact

/-- An inexact reading never keeps a path exact. -/
theorem an_inexact_arm_is_never_exact (arm : Bool) : explore .inexact arm ≠ .taken := by
  simp [explore]

/-! ## No free widening: call results never contribute to a witness -/

/-- Where a value comes from. -/
inductive Origin where
  | entry
  | callResult
  deriving DecidableEq, Repr

open QCore.NullTriage (Step validated takes choose)

/-- A witness the search accepts: validated (`QCore.NullTriage`), and every
undecided step reads an entry value. -/
def exactWitness (origin : Nat → Origin) (path : List Step) : Prop :=
  validated path ∧ ∀ s ∈ path, s.decided = none → origin s.atom = .entry

/-- A value *contributes* to a path when an undecided step reads it. -/
def contributes (path : List Step) (a : Nat) : Prop :=
  ∃ s ∈ path, s.decided = none ∧ s.atom = a

/-- **A call result never contributes a satisfying assignment.** -/
theorem a_call_result_never_contributes (origin : Nat → Origin) (path : List Step)
    (h : exactWitness origin path) (a : Nat) (ha : origin a = .callResult) :
    ¬ contributes path a := by
  rintro ⟨s, hs, hd, rfl⟩
  have := h.2 s hs hd
  rw [ha] at this
  cases this

/-- **A witness holds whatever the callees return**: any valuation that
agrees with the witness's choice on the entry values takes every arm. -/
theorem a_witness_holds_whatever_the_callees_return (origin : Nat → Origin)
    (path : List Step) (h : exactWitness origin path) (σ : Nat → Bool)
    (hσ : ∀ a, origin a = .entry → σ a = choose path a) :
    ∀ s ∈ path, takes σ s := by
  intro s hs
  have hsat := QCore.NullTriage.a_validated_path_is_satisfiable path h.1 s hs
  unfold takes at hsat ⊢
  cases hd : s.decided with
  | some v => simpa [hd] using hsat
  | none =>
      have hentry := h.2 s hs hd
      simp only [hd] at hsat ⊢
      rw [hσ s.atom hentry]
      exact hsat

/-! ## A bound decides soundly -/

/-- An arm's set as an interval `[lo, hi]`. -/
structure Bound where
  lo : Int
  hi : Int
  deriving DecidableEq, Repr

def Bound.mem (b : Bound) (v : Int) : Prop := b.lo ≤ v ∧ v ≤ b.hi

/-- Two intervals meet. -/
def meets (a b : Bound) : Bool := decide (max a.lo b.lo ≤ min a.hi b.hi)

/-- **A bound excludes an arm it does not meet**: a value inside its bound is
never in an arm's set that misses the bound, so the other arm is decided. -/
theorem a_bound_excludes_its_outside_arm (bound arm : Bound) (v : Int)
    (hv : bound.mem v) (hmiss : meets bound arm = false) : ¬ arm.mem v := by
  intro ha
  simp only [meets, decide_eq_false_iff_not, Bound.mem] at hmiss hv ha
  omega

/-! ## Inexact exploration proves infeasibility -/

/-- A program: a tree of branches over a valuation of integer values, whose
leaves say whether they reach the dereference with the pointer NULL. -/
inductive Prog where
  | leaf (reaches : Bool)
  | branch (cond : (Nat → Int) → Bool) (yes no : Prog)

/-- The concrete run. -/
def run (σ : Nat → Int) : Prog → Bool
  | .leaf r => r
  | .branch c y n => if c σ then run σ y else run σ n

/-- The facts a path collected: each condition with the arm it took. -/
def sat (facts : List (((Nat → Int) → Bool) × Bool)) (σ : Nat → Int) : Prop :=
  ∀ f ∈ facts, f.1 σ = f.2

/-- A decider is sound when what it decides holds for every valuation the
facts admit. -/
def SoundDecider (dec : List (((Nat → Int) → Bool) × Bool) → ((Nat → Int) → Bool) → Option Bool) :
    Prop :=
  ∀ fs c v, dec fs c = some v → ∀ σ, sat fs σ → c σ = v

/-- The search: a decided branch follows its arm; any other branch, exact or
inexact, is explored on both arms. The result over-approximates whether
some leaf is reached. -/
def search (dec : List (((Nat → Int) → Bool) × Bool) → ((Nat → Int) → Bool) → Option Bool) :
    List (((Nat → Int) → Bool) × Bool) → Prog → Bool
  | _, .leaf r => r
  | fs, .branch c y n =>
      match dec fs c with
      | some true => search dec ((c, true) :: fs) y
      | some false => search dec ((c, false) :: fs) n
      | none => search dec ((c, true) :: fs) y || search dec ((c, false) :: fs) n

/-- **Exhaustive search is sound**: when the search reaches no leaf, no
valuation the facts admit does. -/
theorem exhaustive_search_is_sound (dec) (hdec : SoundDecider dec) :
    ∀ (p : Prog) (fs : List (((Nat → Int) → Bool) × Bool)),
      search dec fs p = false → ∀ σ, sat fs σ → run σ p = false := by
  intro p
  induction p with
  | leaf r => intro fs h σ _; simpa [search, run] using h
  | branch c y n ihy ihn =>
      intro fs h σ hσ
      have extend : ∀ b, c σ = b → sat ((c, b) :: fs) σ := by
        intro b hb f hf
        rcases List.mem_cons.mp hf with rfl | hf
        · exact hb
        · exact hσ f hf
      unfold search at h
      unfold run
      cases hd : dec fs c with
      | some v =>
          have hv := hdec fs c v hd σ hσ
          cases v
          · rw [hd] at h; simp only at h
            simp [hv]; exact ihn _ h σ (extend false hv)
          · rw [hd] at h; simp only at h
            simp [hv]; exact ihy _ h σ (extend true hv)
      | none =>
          rw [hd] at h
          simp only [Bool.or_eq_false_iff] at h
          cases hc : c σ
          · simp; exact ihn _ h.2 σ (extend false hc)
          · simp; exact ihy _ h.1 σ (extend true hc)

/-! ## Difference and sum arms -/

/-- **Every value of the difference interval is attained.** -/
theorem difference_attains (al ah bl bh t : Int) (ha : al ≤ ah) (hb : bl ≤ bh)
    (hlo : al - bh ≤ t) (hhi : t ≤ ah - bl) :
    ∃ a b, al ≤ a ∧ a ≤ ah ∧ bl ≤ b ∧ b ≤ bh ∧ a - b = t := by
  by_cases h : t + bh ≤ ah
  · exact ⟨t + bh, bh, by omega, h, hb, Int.le_refl _, by omega⟩
  · exact ⟨ah, ah - t, ha, Int.le_refl _, by omega, by omega, by omega⟩

/-- **Every value of the sum interval is attained.** -/
theorem sum_attains (al ah bl bh t : Int) (ha : al ≤ ah) (hb : bl ≤ bh)
    (hlo : al + bl ≤ t) (hhi : t ≤ ah + bh) :
    ∃ a b, al ≤ a ∧ a ≤ ah ∧ bl ≤ b ∧ b ≤ bh ∧ a + b = t := by
  by_cases h : t - bl ≤ ah
  · exact ⟨t - bl, bl, by omega, h, Int.le_refl _, hb, by omega⟩
  · exact ⟨ah, t - ah, ha, Int.le_refl _, by omega, by omega, by omega⟩

/-- `null_paths::relation_arms` over the difference (sum) interval `[lo, hi]`:
`(true arm, false arm)` of `d OP c`; `none` for an unread operator. -/
def arms (lo hi : Int) (op : String) (c : Int) : Option (Bool × Bool) :=
  let equal := decide (lo ≤ c ∧ c ≤ hi)
  let other := decide (lo ≠ hi ∨ lo ≠ c)
  match op with
  | "==" => some (equal, other)
  | "!=" => some (other, equal)
  | "<" => some (decide (lo < c), decide (hi ≥ c))
  | ">=" => some (decide (hi ≥ c), decide (lo < c))
  | "<=" => some (decide (lo ≤ c), decide (hi > c))
  | ">" => some (decide (hi > c), decide (lo ≤ c))
  | _ => none

/-- **A feasible `<` arm has a witness value in the interval.** -/
theorem lt_arm_is_attained (lo hi c : Int) (h : lo ≤ hi) :
    arms lo hi "<" c = some (true, true) → ∃ d₁ d₂, lo ≤ d₁ ∧ d₁ ≤ hi ∧ d₁ < c ∧
      lo ≤ d₂ ∧ d₂ ≤ hi ∧ ¬ d₂ < c := by
  intro hr
  simp only [arms, Option.some.injEq, Prod.mk.injEq, decide_eq_true_eq] at hr
  exact ⟨lo, hi, Int.le_refl _, h, hr.1, h, Int.le_refl _, by omega⟩

/-- **An infeasible arm has no value**: when the interval misses the arm's
set, no value in it takes the arm (`<`, `==`). -/
theorem an_unmet_arm_has_no_value (lo hi c d : Int) (hd : lo ≤ d ∧ d ≤ hi) :
    (arms lo hi "<" c).map Prod.fst = some false → ¬ d < c := by
  intro hr
  simp only [arms, Option.map_some, Option.some.injEq, decide_eq_false_iff_not] at hr
  omega

/-! ## The counter range of a body -/

/-- An update of a counter: set to a constant, or increased by one. -/
inductive Update where
  | set (k : Int)
  | add (k : Int)
  deriving DecidableEq, Repr

def apply (v : Int) : Update → Int
  | .set k => k
  | .add k => v + k

def final (v : Int) : List Update → Int
  | [] => v
  | u :: rest => final (apply v u) rest

/-- **A counter that starts at or above `lo`, is set only to constants at or
above `lo` and grows only by non-negative constants stays at or above `lo`**
(the mathematical value; a defined C execution has no signed overflow). -/
theorem counter_stays_above (lo : Int) :
    ∀ (us : List Update) (v : Int), lo ≤ v →
      (∀ u ∈ us, match u with | .set k => lo ≤ k | .add k => 0 ≤ k) →
      lo ≤ final v us := by
  intro us
  induction us with
  | nil => intro v hv _; simpa [final] using hv
  | cons u rest ih =>
      intro v hv hall
      simp only [final]
      apply ih
      · have hu := hall u (List.mem_cons_self ..)
        cases u with
        | set k => simpa [apply] using hu
        | add k => simp only [apply]; simp at hu; omega
      · intro w hw; exact hall w (List.mem_cons_of_mem _ hw)

/-! ## Vectors bound to the Rust -/

def stepName : StepOutcome → String
  | .taken => "taken" | .infeasible => "infeasible" | .takenInexact => "taken-inexact"

def readingOf : String → Reading
  | "d1" => .decided true
  | "d0" => .decided false
  | "exact" => .exact
  | _ => .inexact

/-- Reading, arm, step: `EXPLORE_VECTORS`. -/
def exploreVectors : List (String × Bool × String) := [
  ("d1", true, "taken"),
  ("d1", false, "infeasible"),
  ("d0", false, "taken"),
  ("exact", true, "taken"),
  ("exact", false, "taken"),
  ("inexact", true, "taken-inexact"),
  ("inexact", false, "taken-inexact")
]

theorem explore_vectors_hold :
    exploreVectors.all (fun (r, a, o) => stepName (explore (readingOf r) a) == o) = true := by
  decide

/-- `null_paths::conclude`. -/
def conclude (exactReach inexactReach complete : Bool) : String :=
  if exactReach then "finding"
  else if inexactReach || !complete then "unresolved"
  else "infeasible"

/-- **Only an exact path is a finding.** -/
theorem a_finding_needs_an_exact_path (i c : Bool) : conclude false i c ≠ "finding" := by
  cases i <;> cases c <;> decide

/-- **Infeasible only after a complete search that reached nothing.** -/
theorem infeasible_needs_a_complete_empty_search (e i c : Bool)
    (h : conclude e i c = "infeasible") : e = false ∧ i = false ∧ c = true := by
  cases e <;> cases i <;> cases c <;> simp_all [conclude]

/-- Exact reach, inexact reach, complete, conclusion: `CONCLUDE_VECTORS`. -/
def concludeVectors : List (Bool × Bool × Bool × String) := [
  (true, true, false, "finding"),
  (true, false, true, "finding"),
  (false, true, true, "unresolved"),
  (false, false, false, "unresolved"),
  (false, false, true, "infeasible")
]

theorem conclude_vectors_hold :
    concludeVectors.all (fun (e, i, c, o) => conclude e i c == o) = true := by
  decide

/-- Interval, operator, constant, arms: `ARMS_VECTORS` (the Rust reads the
difference or sum interval of its two values first). -/
def armsVectors : List (Int × Int × String × Int × Bool × Bool) := [
  (0, 10, "<", 0, false, true),
  (-5, 10, "<", 0, true, true),
  (3, 3, "==", 3, true, false),
  (3, 3, "!=", 3, false, true),
  (0, 10, "==", 11, false, true),
  (0, 10, ">", 10, false, true),
  (0, 10, "<=", 10, true, false),
  (1, 10, ">=", 1, true, false)
]

theorem arms_vectors_hold :
    armsVectors.all (fun (lo, hi, op, c, t, f) => arms lo hi op c == some (t, f)) = true := by
  decide

end QCore.NullTriagePaths
