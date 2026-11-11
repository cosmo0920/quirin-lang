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
# Guarded retention clauses

Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rule 12w; contract specification
§C04.14.1). This module models `crates/qcore-schemas/src/retained_by.rs`'s
`RetentionGuard::decide` and `in_force`, the loader's partition test
(`crates/qcore-contract/src/contract_retained_by.rs`, `guard_partitions`)
and the in-unit body check (`crates/qcore-c-gateway/src/legacy_retention_guard.rs`,
`validated`).

USER DECISION 2026-10-05: a clause conditioned on an integer argument's value
applies only where the caller's actual is decided to satisfy the guard; an
undecided actual gets the cautious reading, the union of every guarded
clause, never the favourable one; guards partition or cover, an uncovered
value getting the cautious reading; where the body is available, each guarded
clause is validated under its guard.

## What is proved

* **The guard semantics**: a decided value in range is `holds` or `fails` by
  the comparison over the integers; anything else is undecided; only a
  decided failure takes a clause out of force.
* **The cautious join**: every reading is a subset of the cautious reading
  (all clauses); an undecided actual gives exactly the cautious reading.
* **Never a favourable undecided reading**: for any verdict that more
  retention never improves (`Monotone`), the verdict of a reading is bounded
  by the cautious one; so an undecided guard yields `Safe` only where the
  cautious reading does.
* **Partition**: the loader's test admits a guard only when some value of the
  parameter's range satisfies it and some value does not.
* **The body check**: if every use that can retain `P` lies in a region that
  implies the guard, no such use runs when the guard fails.
* **Controls**: the Rust unit tests' decision vectors.
-/

namespace QCore.RetentionGuard

/-! ## Guards -/

inductive Rel where
  | eq | lt | gt | le | ge
  deriving DecidableEq, Repr

def Rel.holds : Rel → Int → Int → Bool
  | .eq, v, k => v == k
  | .lt, v, k => decide (v < k)
  | .gt, v, k => decide (v > k)
  | .le, v, k => decide (v ≤ k)
  | .ge, v, k => decide (v ≥ k)

structure Guard where
  rel : Rel
  k : Int
  lo : Int
  hi : Int
  deriving Repr

inductive Decision where
  | holds | fails | undecided
  deriving DecidableEq, Repr

/-- `RetentionGuard::decide`. -/
def Guard.decide (g : Guard) : Option Int → Decision
  | some v =>
    if g.lo ≤ v ∧ v ≤ g.hi then (if g.rel.holds v g.k then .holds else .fails)
    else .undecided
  | none => .undecided

/-- `RetentionGuard::in_force`. -/
def inForce : Decision → Bool
  | .fails => false
  | _ => true

theorem only_a_decided_failure_is_out_of_force (d : Decision) :
    inForce d = false ↔ d = .fails := by
  cases d <;> simp [inForce]

theorem a_failure_is_a_decided_value_in_range_that_fails (g : Guard) (a : Option Int)
    (h : g.decide a = .fails) :
    ∃ v, a = some v ∧ g.lo ≤ v ∧ v ≤ g.hi ∧ g.rel.holds v g.k = false := by
  cases a with
  | none => simp [Guard.decide] at h
  | some v =>
    simp only [Guard.decide] at h
    split at h
    · rename_i range
      split at h
      · cases h
      · rename_i fails
        exact ⟨v, rfl, range.1, range.2, by simpa using fails⟩
    · cases h

theorem an_undecided_actual_keeps_the_clause (g : Guard) : inForce (g.decide none) = true := rfl

/-! ## Readings and the cautious join -/

/-- A guarded clause: its guard (`none`: unguarded) and the identity of what
it retains. -/
structure Clause where
  id : Nat
  guard : Option Guard

def Clause.inForce (c : Clause) (actual : Option Int) : Bool :=
  match c.guard with
  | none => true
  | some g => QCore.RetentionGuard.inForce (g.decide actual)

/-- The clauses in force at one call, each read with its own actual. -/
def reading (cs : List Clause) (actual : Clause → Option Int) : List Clause :=
  cs.filter (fun c => c.inForce (actual c))

/-- The cautious reading: every clause. -/
def cautious (cs : List Clause) : List Clause := cs

theorem every_reading_is_within_the_cautious_one (cs : List Clause)
    (actual : Clause → Option Int) : ∀ c ∈ reading cs actual, c ∈ cautious cs := by
  intro c h
  exact (List.mem_filter.mp h).1

theorem an_undecided_call_reads_the_cautious_union (cs : List Clause) :
    reading cs (fun _ => none) = cautious cs := by
  unfold reading cautious
  apply List.filter_eq_self.mpr
  intro c _
  cases h : c.guard <;> simp [Clause.inForce, h, an_undecided_actual_keeps_the_clause]

/-! ## Never a favourable undecided reading -/

inductive Verdict where
  | safe | pending | unknown | «unsafe»
  deriving DecidableEq, Repr

def Verdict.rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- A verdict over retention sets that more retention never improves. -/
def Monotone (verdict : List Clause → Verdict) : Prop :=
  ∀ r r' : List Clause, (∀ c ∈ r, c ∈ r') → (verdict r).rank ≤ (verdict r').rank

theorem the_cautious_reading_bounds_every_reading (verdict : List Clause → Verdict)
    (mono : Monotone verdict) (cs : List Clause) (actual : Clause → Option Int) :
    (verdict (reading cs actual)).rank ≤ (verdict (cautious cs)).rank :=
  mono _ _ (every_reading_is_within_the_cautious_one cs actual)

/-- The user's requirement: an undecided guard never yields `Safe` where the
cautious reading would not. -/
theorem an_undecided_guard_is_safe_only_where_the_cautious_reading_is
    (verdict : List Clause → Verdict) (cs : List Clause)
    (h : verdict (reading cs (fun _ => none)) = .safe) : verdict (cautious cs) = .safe := by
  rw [an_undecided_call_reads_the_cautious_union] at h
  exact h

/-! ## Partition -/

/-- The loader's `guard_partitions`. -/
def partitions (g : Guard) : Bool :=
  decide (g.lo ≤ g.k ∧ g.k ≤ g.hi) &&
    match g.rel with
    | .eq => decide (g.lo < g.hi)
    | .lt | .ge => decide (g.lo < g.k)
    | .gt | .le => decide (g.k < g.hi)

theorem an_admitted_guard_partitions_its_range (g : Guard) (h : partitions g = true) :
    (∃ v, g.lo ≤ v ∧ v ≤ g.hi ∧ g.rel.holds v g.k = true) ∧
      (∃ w, g.lo ≤ w ∧ w ≤ g.hi ∧ g.rel.holds w g.k = false) := by
  obtain ⟨rel, k, lo, hi⟩ := g
  cases rel <;> simp only [partitions, Bool.and_eq_true, decide_eq_true_eq] at h <;>
    obtain ⟨⟨lok, khi⟩, rest⟩ := h <;> dsimp only
  · refine ⟨⟨k, lok, khi, by simp [Rel.holds]⟩, ?_⟩
    by_cases hk : k = lo
    · exact ⟨hi, by omega, Int.le_refl _, by simp [Rel.holds] <;> omega⟩
    · exact ⟨lo, Int.le_refl _, by omega, by simp [Rel.holds] <;> omega⟩
  · exact ⟨⟨lo, Int.le_refl _, by omega, by simp [Rel.holds] <;> omega⟩,
      ⟨k, lok, khi, by simp [Rel.holds]⟩⟩
  · exact ⟨⟨hi, by omega, Int.le_refl _, by simp [Rel.holds] <;> omega⟩,
      ⟨k, lok, khi, by simp [Rel.holds]⟩⟩
  · exact ⟨⟨k, lok, khi, by simp [Rel.holds]⟩,
      ⟨hi, by omega, Int.le_refl _, by simp [Rel.holds] <;> omega⟩⟩
  · exact ⟨⟨k, lok, khi, by simp [Rel.holds]⟩,
      ⟨lo, Int.le_refl _, by omega, by simp [Rel.holds] <;> omega⟩⟩

/-! ## The body check -/

/-- One mention of the retained parameter in the body: whether it can keep
the pointer, and the region it lies in (`none`: outside every guard region;
`some r`: a region entered only for values satisfying `r`). -/
structure Use where
  retains : Bool
  region : Option (Int → Bool)

/-- `validated`: every use that can keep the pointer lies in a region that
implies the guard. -/
def Validated (g : Guard) (body : List Use) : Prop :=
  ∀ u ∈ body, u.retains = true →
    ∃ r, u.region = some r ∧ ∀ v, r v = true → g.rel.holds v g.k = true

/-- The uses that run when the guard parameter's value is `v`. -/
def runs (v : Int) (u : Use) : Bool :=
  match u.region with
  | none => true
  | some r => r v

theorem a_validated_body_keeps_nothing_when_the_guard_fails (g : Guard) (body : List Use)
    (valid : Validated g body) (v : Int) (fails : g.rel.holds v g.k = false) :
    ∀ u ∈ body, runs v u = true → u.retains = false := by
  intro u mem run
  cases hr : u.retains
  · rfl
  · obtain ⟨r, hreg, implies⟩ := valid u mem hr
    simp only [runs, hreg] at run
    have := implies v run
    rw [fails] at this
    cases this

/-! ## Controls: the Rust unit tests' vectors -/

/-- `kind == 2` over `u32`. -/
def kindIsPtr : Guard := ⟨.eq, 2, 0, 4294967295⟩

theorem decision_controls :
    kindIsPtr.decide (some 2) = .holds ∧ kindIsPtr.decide (some 1) = .fails ∧
    kindIsPtr.decide none = .undecided ∧ kindIsPtr.decide (some (-1)) = .undecided := by
  decide

theorem partition_controls :
    partitions kindIsPtr = true ∧ partitions ⟨.ge, 0, 0, 4294967295⟩ = false ∧
    partitions ⟨.lt, 0, 0, 4294967295⟩ = false ∧ partitions ⟨.eq, -1, 0, 4294967295⟩ = false ∧
    partitions ⟨.le, 127, -128, 127⟩ = false ∧ partitions ⟨.ge, -4, -128, 127⟩ = true := by
  decide

end QCore.RetentionGuard
