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

import QCore.VerdictSemigroup

/-!
# The integration semigroup across results

Card **AUDIT-COMPOSITION** (`docs/plans/pluggable-cards/README.md` seam G;
language specification §L21.2, "Composition across results"). This is the
model of `crates/qcore-schemas/src/composition.rs`.

A *component* is one producer's account of one published row. Here a
component is a record of a row identifier, a provenance identifier, an
opaque body (every other byte), a verdict and a list of named assumptions;
two components are identical exactly when every field is, which is the Rust
side's byte equality. A *contribution* is a finite set of components, kept as
a list read through membership (`Equiv`), and `⊕` is union (`union`, list
append read as a set). A row's published verdict is derived, never stored:
the join (`QCore.VerdictSemigroup`) over the row's components, joined with
`Unknown` when two of them share a provenance but differ (`conflicted`); its
assumptions are the union of theirs.

## What is proved

* **The algebra.** `⊕` is associative, commutative and idempotent with the
  empty contribution as its identity, and it erases nothing. Every derived
  quantity — the verdict, the conflict flag, the assumptions, presence —
  depends only on the set of components, so the derived rows inherit
  associativity, commutativity and idempotence, and a byte-identical
  duplicate changes nothing.
* **Monotonicity.** Adding a contribution never lowers a row's verdict, every
  component's verdict is below its row's, and adding a contribution never
  removes an assumption.
* **Conflict is kept, not chosen.** Two components of one row under one
  provenance with different bytes are both members of the union, and the row
  is at least `Unknown`; exactly `Unknown` unless some component is itself
  `Unsafe`, which the join never lowers.
* **Removal is a re-join.** Removing a contribution from a union of disjoint
  contributions restores exactly the other's set, hence its derivation.
* **Absence is refusal.** A row no component speaks about has no derivation,
  never the join's identity `Safe`.
* **Shared vectors.** `vectors` is decided here; the same list is parsed and
  checked against the Rust implementation by
  `crates/qcore-schemas/tests/audit_composition.rs`.

## What this is not

That the Rust `Contribution` computes these functions is pinned by that test
and the vectors; that a producer's component carries the right verdict is the
producer's own obligation (the trusted core, a checker plugin behind the
firewall) and is a premise here.
-/

namespace QCore.AuditComposition

open QCore.VerdictSemigroup

/-- One producer's account of one row. -/
structure Component where
  row : Nat
  prov : Nat
  body : Nat
  verdict : V
  assumptions : List Nat
  deriving DecidableEq, Repr

/-- A finite set of components, read through membership. -/
abbrev Contribution := List Component

/-- `⊕`: provenance-keyed union, read as a set. -/
def union (a b : Contribution) : Contribution := a ++ b

/-- Two contributions with the same members. -/
def Equiv (a b : Contribution) : Prop := ∀ c, c ∈ a ↔ c ∈ b

/-- The components of one row. -/
def ofRow (r : Nat) (a : Contribution) : List Component := a.filter (fun c => c.row == r)

/-- Two components of row `r` under one provenance whose bytes differ. -/
def conflicted (r : Nat) (a : Contribution) : Bool :=
  a.any fun c => a.any fun d =>
    c.row == r && d.row == r && c.prov == d.prov && decide (c ≠ d)

/-- The row's verdict: the join over its components, and `Unknown` on a
conflict. -/
def verdictOf (r : Nat) (a : Contribution) : V :=
  V.join (fold ((ofRow r a).map (·.verdict))) (if conflicted r a then .unknown else .safe)

/-- The row's assumptions: the union of its components'. -/
def assumptionsOf (r : Nat) (a : Contribution) : List Nat :=
  (ofRow r a).flatMap (·.assumptions)

/-- Some component speaks about row `r`. -/
def present (r : Nat) (a : Contribution) : Bool := a.any (fun c => c.row == r)

/-- The published row: `none` when no component speaks about it. -/
def derive (r : Nat) (a : Contribution) : Option V :=
  if present r a then some (verdictOf r a) else none

/-- The re-join without `b`: every component of `a` that is not one of `b`. -/
def remove (a b : Contribution) : Contribution := a.filter (fun c => !decide (c ∈ b))

/-! ## The algebra of `⊕` -/

theorem mem_union {a b : Contribution} {c : Component} : c ∈ union a b ↔ c ∈ a ∨ c ∈ b :=
  List.mem_append

theorem union_assoc (a b c : Contribution) : union (union a b) c = union a (union b c) :=
  List.append_assoc a b c

theorem union_comm (a b : Contribution) : Equiv (union a b) (union b a) := by
  intro c; simp only [mem_union]; exact Or.comm

theorem empty_union (a : Contribution) : union [] a = a := rfl

theorem union_empty (a : Contribution) : union a [] = a := List.append_nil a

theorem union_idem (a : Contribution) : Equiv (union a a) a := by
  intro c; simp only [mem_union, or_self]

/-- `⊕` erases nothing. -/
theorem union_keeps_left {a b : Contribution} {c : Component} (h : c ∈ a) : c ∈ union a b :=
  mem_union.mpr (Or.inl h)

theorem union_keeps_right {a b : Contribution} {c : Component} (h : c ∈ b) : c ∈ union a b :=
  mem_union.mpr (Or.inr h)

/-- A byte-identical component already present changes no member. -/
theorem union_duplicate {a : Contribution} {c : Component} (h : c ∈ a) :
    Equiv (union a [c]) a := by
  intro d
  simp only [mem_union, List.mem_singleton]
  constructor
  · rintro (m | rfl)
    · exact m
    · exact h
  · exact Or.inl

/-! ## Every derived quantity reads only the set -/

theorem mem_ofRow {r : Nat} {a : Contribution} {c : Component} :
    c ∈ ofRow r a ↔ c ∈ a ∧ c.row = r := by
  simp [ofRow]

theorem fold_safe_or_mem (xs : List V) : fold xs = .safe ∨ fold xs ∈ xs := by
  induction xs with
  | nil => exact Or.inl rfl
  | cons x xs ih =>
    show V.join x (fold xs) = .safe ∨ V.join x (fold xs) ∈ x :: xs
    unfold V.join
    split
    · rcases ih with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_cons_of_mem x h)
    · exact Or.inr List.mem_cons_self

theorem fold_rank_le {xs ys : List V} (sub : ∀ x, x ∈ xs → x ∈ ys) :
    (fold xs).rank ≤ (fold ys).rank := by
  rcases fold_safe_or_mem xs with h | h
  · rw [h]; exact Nat.zero_le _
  · exact every_link_is_below_the_fold (sub _ h)

theorem fold_congr {xs ys : List V} (same : ∀ x, x ∈ xs ↔ x ∈ ys) : fold xs = fold ys :=
  V.rank_injective (Nat.le_antisymm (fold_rank_le (fun x => (same x).mp))
    (fold_rank_le (fun x => (same x).mpr)))

theorem conflicted_iff {r : Nat} {a : Contribution} :
    conflicted r a = true ↔
      ∃ c ∈ a, ∃ d ∈ a, c.row = r ∧ d.row = r ∧ c.prov = d.prov ∧ c ≠ d := by
  simp [conflicted, List.any_eq_true, Bool.and_eq_true, beq_iff_eq, and_assoc]

theorem present_iff {r : Nat} {a : Contribution} : present r a = true ↔ ∃ c ∈ a, c.row = r := by
  simp [present, List.any_eq_true]

theorem mem_assumptionsOf {r : Nat} {a : Contribution} {x : Nat} :
    x ∈ assumptionsOf r a ↔ ∃ c ∈ a, c.row = r ∧ x ∈ c.assumptions := by
  simp only [assumptionsOf, List.mem_flatMap, mem_ofRow]
  constructor
  · rintro ⟨c, ⟨m, hr⟩, hx⟩; exact ⟨c, m, hr, hx⟩
  · rintro ⟨c, m, hr, hx⟩; exact ⟨c, ⟨m, hr⟩, hx⟩

theorem conflicted_congr {a b : Contribution} (same : Equiv a b) (r : Nat) :
    conflicted r a = conflicted r b := by
  apply Bool.eq_iff_iff.mpr
  simp only [conflicted_iff]
  constructor
  · rintro ⟨c, mc, d, md, h⟩; exact ⟨c, (same c).mp mc, d, (same d).mp md, h⟩
  · rintro ⟨c, mc, d, md, h⟩; exact ⟨c, (same c).mpr mc, d, (same d).mpr md, h⟩

theorem verdictOf_congr {a b : Contribution} (same : Equiv a b) (r : Nat) :
    verdictOf r a = verdictOf r b := by
  unfold verdictOf
  rw [conflicted_congr same r]
  congr 1
  apply fold_congr
  intro v
  simp only [List.mem_map, mem_ofRow]
  constructor
  · rintro ⟨c, ⟨m, hr⟩, hv⟩; exact ⟨c, ⟨(same c).mp m, hr⟩, hv⟩
  · rintro ⟨c, ⟨m, hr⟩, hv⟩; exact ⟨c, ⟨(same c).mpr m, hr⟩, hv⟩

theorem present_congr {a b : Contribution} (same : Equiv a b) (r : Nat) :
    present r a = present r b := by
  apply Bool.eq_iff_iff.mpr
  simp only [present_iff]
  constructor
  · rintro ⟨c, m, h⟩; exact ⟨c, (same c).mp m, h⟩
  · rintro ⟨c, m, h⟩; exact ⟨c, (same c).mpr m, h⟩

theorem derive_congr {a b : Contribution} (same : Equiv a b) (r : Nat) :
    derive r a = derive r b := by
  simp only [derive, present_congr same r, verdictOf_congr same r]

theorem assumptions_congr {a b : Contribution} (same : Equiv a b) (r x : Nat) :
    x ∈ assumptionsOf r a ↔ x ∈ assumptionsOf r b := by
  simp only [mem_assumptionsOf]
  constructor
  · rintro ⟨c, m, h⟩; exact ⟨c, (same c).mp m, h⟩
  · rintro ⟨c, m, h⟩; exact ⟨c, (same c).mpr m, h⟩

/-! ## The derived rows inherit the algebra -/

theorem derive_assoc (a b c : Contribution) (r : Nat) :
    derive r (union (union a b) c) = derive r (union a (union b c)) := by
  rw [union_assoc]

theorem derive_comm (a b : Contribution) (r : Nat) :
    derive r (union a b) = derive r (union b a) :=
  derive_congr (union_comm a b) r

theorem derive_identity (a : Contribution) (r : Nat) :
    derive r (union [] a) = derive r a ∧ derive r (union a []) = derive r a := by
  rw [empty_union, union_empty]; exact ⟨rfl, rfl⟩

theorem derive_idem (a : Contribution) (r : Nat) : derive r (union a a) = derive r a :=
  derive_congr (union_idem a) r

/-- A byte-identical component changes no derivation and makes no conflict. -/
theorem derive_duplicate {a : Contribution} {c : Component} (h : c ∈ a) (r : Nat) :
    derive r (union a [c]) = derive r a ∧ conflicted r (union a [c]) = conflicted r a :=
  ⟨derive_congr (union_duplicate h) r, conflicted_congr (union_duplicate h) r⟩

/-! ## Monotonicity -/

theorem join_rank_mono {a b c d : V} (ac : a.rank ≤ c.rank) (bd : b.rank ≤ d.rank) :
    (V.join a b).rank ≤ (V.join c d).rank := by
  rw [join_rank, join_rank]
  exact Nat.max_le.mpr ⟨Nat.le_trans ac (Nat.le_max_left _ _),
    Nat.le_trans bd (Nat.le_max_right _ _)⟩

theorem conflicted_mono {a b : Contribution} {r : Nat} (h : conflicted r a = true) :
    conflicted r (union a b) = true := by
  rw [conflicted_iff] at h ⊢
  obtain ⟨c, mc, d, md, rest⟩ := h
  exact ⟨c, union_keeps_left mc, d, union_keeps_left md, rest⟩

/-- Adding a contribution never lowers a row's verdict. -/
theorem the_join_never_lowers (a b : Contribution) (r : Nat) :
    (verdictOf r a).rank ≤ (verdictOf r (union a b)).rank := by
  unfold verdictOf
  apply join_rank_mono
  · apply fold_rank_le
    intro v
    simp only [List.mem_map, mem_ofRow]
    rintro ⟨c, ⟨m, hr⟩, hv⟩
    exact ⟨c, ⟨union_keeps_left m, hr⟩, hv⟩
  · cases h : conflicted r a
    · simp [V.rank]
    · simp [conflicted_mono h, V.rank]

/-- Every component's verdict is below its row's. -/
theorem every_component_is_below_its_row {a : Contribution} {c : Component} {r : Nat}
    (m : c ∈ a) (hr : c.row = r) : c.verdict.rank ≤ (verdictOf r a).rank := by
  unfold verdictOf
  refine Nat.le_trans ?_ (le_join_left _ _)
  apply every_link_is_below_the_fold
  exact List.mem_map.mpr ⟨c, mem_ofRow.mpr ⟨m, hr⟩, rfl⟩

/-- Adding a contribution never removes an assumption. -/
theorem adding_never_removes_an_assumption (a b : Contribution) (r x : Nat)
    (h : x ∈ assumptionsOf r a) : x ∈ assumptionsOf r (union a b) := by
  rw [mem_assumptionsOf] at h ⊢
  obtain ⟨c, m, rest⟩ := h
  exact ⟨c, union_keeps_left m, rest⟩

/-- Every component's assumptions are the row's. -/
theorem the_row_carries_every_component_assumption {a : Contribution} {c : Component}
    {r x : Nat} (m : c ∈ a) (hr : c.row = r) (hx : x ∈ c.assumptions) :
    x ∈ assumptionsOf r a :=
  mem_assumptionsOf.mpr ⟨c, m, hr, hx⟩

/-! ## Conflict -/

/-- A conflicted row is never below `Unknown`. -/
theorem a_conflict_is_at_least_unknown {a : Contribution} {r : Nat}
    (h : conflicted r a = true) : V.unknown.rank ≤ (verdictOf r a).rank := by
  unfold verdictOf
  rw [h]
  exact le_join_right _ _

/-- A conflicted row is exactly `Unknown` unless some component is `Unsafe`. -/
theorem a_conflict_without_unsafe_is_unknown {a : Contribution} {r : Nat}
    (h : conflicted r a = true)
    (bounded : ∀ c ∈ a, c.row = r → c.verdict.rank ≤ V.unknown.rank) :
    verdictOf r a = .unknown := by
  apply V.rank_injective
  apply Nat.le_antisymm
  · unfold verdictOf
    rw [h, join_rank]
    apply Nat.max_le.mpr
    refine ⟨?_, Nat.le_refl _⟩
    rcases fold_safe_or_mem ((ofRow r a).map (·.verdict)) with s | s
    · rw [s]; exact Nat.zero_le _
    · obtain ⟨c, mc, hv⟩ := List.mem_map.mp s
      rw [← hv]
      exact bounded c (mem_ofRow.mp mc).1 (mem_ofRow.mp mc).2
  · exact a_conflict_is_at_least_unknown h

/-- Two components of one row under one provenance with different bytes: both
are kept by `⊕`, the row is conflicted, and it is at least `Unknown`. -/
theorem a_conflict_keeps_both {a b : Contribution} {c d : Component}
    (mc : c ∈ a) (md : d ∈ b) (row : c.row = d.row) (prov : c.prov = d.prov)
    (differ : c ≠ d) :
    c ∈ union a b ∧ d ∈ union a b ∧ conflicted c.row (union a b) = true ∧
      V.unknown.rank ≤ (verdictOf c.row (union a b)).rank := by
  have conflict : conflicted c.row (union a b) = true :=
    conflicted_iff.mpr ⟨c, union_keeps_left mc, d, union_keeps_right md, rfl, row.symm, prov,
      differ⟩
  exact ⟨union_keeps_left mc, union_keeps_right md, conflict,
    a_conflict_is_at_least_unknown conflict⟩

/-- Components that differ only in provenance never conflict. -/
theorem distinct_provenances_never_conflict {r : Nat} {a : Contribution}
    (distinct : ∀ c ∈ a, ∀ d ∈ a, c.row = r → d.row = r → c.prov = d.prov → c = d) :
    conflicted r a = false := by
  cases h : conflicted r a
  · rfl
  · obtain ⟨c, mc, d, md, hc, hd, hp, ne⟩ := conflicted_iff.mp h
    exact absurd (distinct c mc d md hc hd hp) ne

/-! ## Removal -/

theorem mem_remove {a b : Contribution} {c : Component} : c ∈ remove a b ↔ c ∈ a ∧ c ∉ b := by
  simp [remove]

/-- Removing a contribution from a union of disjoint contributions restores the
other's set. -/
theorem removal_restores_the_set {a b : Contribution} (disjoint : ∀ c ∈ a, c ∉ b) :
    Equiv (remove (union a b) b) a := by
  intro c
  rw [mem_remove, mem_union]
  constructor
  · rintro ⟨ma | mb, nb⟩
    · exact ma
    · exact absurd mb nb
  · intro ma; exact ⟨Or.inl ma, disjoint c ma⟩

/-- ... and so its derivation, conflict flag and assumptions. -/
theorem removal_restores_the_derivation {a b : Contribution} (disjoint : ∀ c ∈ a, c ∉ b)
    (r : Nat) :
    derive r (remove (union a b) b) = derive r a ∧
      conflicted r (remove (union a b) b) = conflicted r a ∧
      ∀ x, x ∈ assumptionsOf r (remove (union a b) b) ↔ x ∈ assumptionsOf r a :=
  ⟨derive_congr (removal_restores_the_set disjoint) r,
    conflicted_congr (removal_restores_the_set disjoint) r,
    assumptions_congr (removal_restores_the_set disjoint) r⟩

/-! ## Absence is refusal -/

theorem derive_none_iff {r : Nat} {a : Contribution} :
    derive r a = none ↔ ∀ c ∈ a, c.row ≠ r := by
  unfold derive
  cases h : present r a
  · simp only [Bool.false_eq_true, ↓reduceIte, true_iff]
    intro c m hr
    have : present r a = true := present_iff.mpr ⟨c, m, hr⟩
    rw [h] at this; cases this
  · simp only [↓reduceIte, reduceCtorEq, false_iff]
    intro none
    obtain ⟨c, m, hr⟩ := present_iff.mp h
    exact none c m hr

/-- The empty contribution publishes no row, never `Safe`. -/
theorem the_empty_contribution_publishes_nothing (r : Nat) : derive r [] = none := rfl

/-- A present row's derivation is its verdict. -/
theorem derive_some_of_mem {a : Contribution} {c : Component} (m : c ∈ a) :
    derive c.row a = some (verdictOf c.row a) := by
  simp [derive, present_iff.mpr ⟨c, m, rfl⟩]

/-! ## Shared vectors

Each vector is a contribution, a row, the expected derivation, the expected
conflict flag and the expected assumption set. The list below is decided by
`vectors_hold`, and `crates/qcore-schemas/tests/audit_composition.rs` parses
the same lines and checks the Rust `Contribution` against them. Keep one
vector per line in exactly this form. -/

structure Vector where
  components : List Component
  row : Nat
  expected : Option V
  conflict : Bool
  assumptions : List Nat
  deriving Repr

def Vector.holds (v : Vector) : Bool :=
  derive v.row v.components == v.expected &&
    conflicted v.row v.components == v.conflict &&
    (assumptionsOf v.row v.components).all (· ∈ v.assumptions) &&
    v.assumptions.all (· ∈ assumptionsOf v.row v.components)

def vectors : List Vector := [
  ⟨[], 0, none, false, []⟩,
  ⟨[⟨0, 0, 0, .safe, [1]⟩], 0, some .safe, false, [1]⟩,
  ⟨[⟨0, 0, 0, .safe, [1]⟩, ⟨0, 1, 0, .unknown, [2]⟩], 0, some .unknown, false, [1, 2]⟩,
  ⟨[⟨0, 0, 0, .pending, [3]⟩, ⟨0, 1, 0, .safe, []⟩], 0, some .pending, false, [3]⟩,
  ⟨[⟨0, 0, 0, .provisionalUnsafe, []⟩, ⟨0, 1, 0, .unknown, [1]⟩], 0, some .provisionalUnsafe, false, [1]⟩,
  ⟨[⟨0, 0, 0, .safe, [1]⟩, ⟨0, 0, 0, .safe, [1]⟩], 0, some .safe, false, [1]⟩,
  ⟨[⟨0, 0, 0, .safe, [1]⟩, ⟨0, 0, 1, .safe, [2]⟩], 0, some .unknown, true, [1, 2]⟩,
  ⟨[⟨0, 0, 0, .provisionalUnsafe, []⟩, ⟨0, 0, 1, .safe, []⟩], 0, some .provisionalUnsafe, true, []⟩,
  ⟨[⟨0, 0, 0, .safe, []⟩, ⟨0, 0, 0, .pending, []⟩], 0, some .unknown, true, []⟩,
  ⟨[⟨1, 0, 0, .provisionalUnsafe, [3]⟩, ⟨0, 0, 0, .safe, [1]⟩], 0, some .safe, false, [1]⟩,
  ⟨[⟨1, 0, 0, .safe, []⟩], 0, none, false, []⟩,
  ⟨[⟨1, 0, 0, .safe, []⟩, ⟨1, 0, 1, .safe, []⟩, ⟨0, 0, 0, .pending, [4]⟩], 0, some .pending, false, [4]⟩,
  ⟨[⟨1, 0, 0, .safe, []⟩, ⟨1, 0, 1, .safe, []⟩, ⟨0, 0, 0, .pending, [4]⟩], 1, some .unknown, true, []⟩,
  ⟨[⟨0, 0, 0, .pending, [1]⟩, ⟨0, 1, 1, .pending, [2]⟩, ⟨0, 2, 2, .safe, [1, 3]⟩], 0, some .pending, false, [1, 2, 3]⟩
]

theorem vectors_hold : vectors.all Vector.holds = true := by decide

/-- The vectors are not vacuous: they reach every verdict, a conflict, an
absent row and a duplicate. -/
theorem vectors_reach_every_case :
    (vectors.map (·.expected)).contains none ∧
      (vectors.map (·.expected)).contains (some .safe) ∧
      (vectors.map (·.expected)).contains (some .pending) ∧
      (vectors.map (·.expected)).contains (some .unknown) ∧
      (vectors.map (·.expected)).contains (some .provisionalUnsafe) ∧
      (vectors.map (·.conflict)).contains true := by decide

end QCore.AuditComposition
