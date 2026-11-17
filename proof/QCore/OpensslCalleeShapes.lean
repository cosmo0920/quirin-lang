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
# The OpenSSL user crate's documented shapes and the dependency fold

Card **OPENSSL-PROVIDER-REMAINING-CALLEES**. Two things are modelled.

**The checker's documented shapes.** `quirin-openssl-check`
(`providers/quirin-provider-openssl/src/check.rs`, `expected_clauses`)
derives from each function's documented shape (`documented.rs`) the exact
clause set its declaration must state, and refuses any other. Here a shape
and its premises determine `expected`; a declaration is accepted when it is a
permutation of it (the Rust sorts both and compares). Proved:

* two accepted declarations of one documented function state the same
  clauses (`accepted_declarations_agree`);
* a reference-counted object's release or constructor states no final release
  (`counted_never_released`), and a returned reference carries the reference
  kind (`returned_reference_is_counted`);
* a borrowed result owns nothing and releases nothing
  (`borrowed_result_owns_nothing`);
* a retention keeps the retained position borrowed and states the retention,
  never a release of it (`retention_keeps_item_borrowed`): never the
  borrowed-and-forgotten reading;
* a plain call states no custody clause (`plain_takes_no_custody`);
* every premise the documentation needs (a null pointer constant at a
  position, the registered-callbacks premise) is in every accepted
  declaration (`premise_is_stated`), so a declaration without it is refused.

**The dependency fold.** The audit publishes, for a function applying bound
user contracts, the join of its own status and every binding's verdict
(`QCore.VerdictSemigroup`). The card found that the fold read only the
declarations its control-flow graph lowered to `ContractCall` nodes, so a
scalar `return f();` over a bound declaration published `Safe` over an
uncertified claim; the fix folds every bound declaration the row rests on
(`contract_dependencies.declarations`). Proved: the fold never lowers the
function's own status (`published_never_lowers`); resting on a `Pending`
binding is never `Safe` (`resting_on_pending_is_never_safe`); and folding the
dependencies is at least folding any subset of them, the `ContractCall`
callees included (`dependencies_cover_applied`).

## What this is not

That the Rust checker's lexical reading of a module is this clause list, that
the documentation says what `documented.rs` records, and that the analysis's
dependency record lists every declaration a row rests on (rule 13f) are
premises, pinned by the crate's tests and
`crates/qcore-audit/tests/user_contract_plugins.rs`.
-/

namespace QCore.OpensslCalleeShapes

open QCore.VerdictSemigroup
open QCore.VerdictSemigroup.V

/-- A position's documented nullability. -/
inductive Null where
  | nullable
  | nonnull
  deriving DecidableEq, Repr

/-- The custody shape a page documents (`documented::Documented`). -/
inductive Shape where
  | plain
  | countedRelease (p : Nat)
  | countedConstructor
  | constructor
  | release (p : Nat) (n : Null)
  | borrowedResult (p : Nat) (n : Null)
  | retains (item holder : Nat)
  deriving DecidableEq, Repr

/-- A premise bounding where a declaration is believed. -/
inductive Premise where
  | nullArgument (p : Nat)
  | registeredCallbacks
  deriving DecidableEq, Repr

/-- The clauses (less the selector, revision and range, common to all). -/
inductive Clause where
  | borrowed (p : Nat)
  | released (p : Nat) (n : Null)
  | referenceDropped (p : Nat)
  | returnNone
  | returnOwned (n : Null)
  | returnKindReference
  | returnBorrowedFrom (p : Nat) (n : Null)
  | retainedBy (item holder : Nat)
  | nullPremise (p : Nat)
  | callbacksPremise
  deriving DecidableEq, Repr

def premiseClause : Premise → Clause
  | .nullArgument p => .nullPremise p
  | .registeredCallbacks => .callbacksPremise

/-- Every parameter is borrowed except a non-counted release's own. -/
def parameterClause : Shape → Nat → Clause
  | .release p n, i => if i = p then .released p n else .borrowed i
  | _, i => .borrowed i

def shapeClauses : Shape → List Clause
  | .plain => [.returnNone]
  | .countedRelease p => [.referenceDropped p, .returnNone]
  | .countedConstructor => [.returnOwned .nullable, .returnKindReference]
  | .constructor => [.returnOwned .nullable]
  | .release _ _ => [.returnNone]
  | .borrowedResult p n => [.returnBorrowedFrom p n]
  | .retains item holder => [.retainedBy item holder, .returnNone]

/-- `expected_clauses`: the exact clause list of a declaration of `arity`
parameters stating `s` under `ps`. -/
def expected (s : Shape) (arity : Nat) (ps : List Premise) : List Clause :=
  ps.map premiseClause ++ (List.range arity).map (parameterClause s) ++ shapeClauses s

/-- The checker accepts exactly the declarations stating the expected
clauses, in any order. -/
def accepts (s : Shape) (arity : Nat) (ps : List Premise) (decl : List Clause) : Prop :=
  decl.Perm (expected s arity ps)

theorem accepted_declarations_agree {s : Shape} {arity : Nat} {ps : List Premise}
    {d₁ d₂ : List Clause} (h₁ : accepts s arity ps d₁) (h₂ : accepts s arity ps d₂) :
    d₁.Perm d₂ :=
  h₁.trans h₂.symm

theorem mem_expected {s : Shape} {arity : Nat} {ps : List Premise} {c : Clause} :
    c ∈ expected s arity ps ↔
      (∃ q ∈ ps, premiseClause q = c) ∨
      (∃ i, i < arity ∧ parameterClause s i = c) ∨ c ∈ shapeClauses s := by
  simp [expected, List.mem_append, List.mem_map, List.mem_range]

theorem premise_never_custody (q : Premise) (p : Nat) (n : Null) :
    premiseClause q ≠ .released p n ∧ premiseClause q ≠ .returnOwned n := by
  cases q <;> simp [premiseClause]

/-- A reference-counted object's release or constructor states no final
release of anything. -/
theorem counted_never_released (s : Shape)
    (hs : s = .countedConstructor ∨ ∃ p, s = .countedRelease p)
    (arity : Nat) (ps : List Premise) (p : Nat) (n : Null) :
    Clause.released p n ∉ expected s arity ps := by
  intro h
  rcases mem_expected.mp h with ⟨q, _, hq⟩ | ⟨i, _, hi⟩ | hc
  · exact (premise_never_custody q p n).1 hq
  · rcases hs with rfl | ⟨r, rfl⟩ <;> simp [parameterClause] at hi
  · rcases hs with rfl | ⟨r, rfl⟩ <;> simp [shapeClauses] at hc

theorem returned_reference_is_counted (arity : Nat) (ps : List Premise) :
    Clause.returnKindReference ∈ expected .countedConstructor arity ps :=
  mem_expected.mpr (Or.inr (Or.inr (by simp [shapeClauses])))

/-- A borrowed result owns nothing and releases nothing. -/
theorem borrowed_result_owns_nothing (p : Nat) (m : Null) (arity : Nat) (ps : List Premise)
    (n : Null) (q : Nat) :
    Clause.returnOwned n ∉ expected (.borrowedResult p m) arity ps ∧
      Clause.released q n ∉ expected (.borrowedResult p m) arity ps := by
  constructor
  · intro h
    rcases mem_expected.mp h with ⟨r, _, hr⟩ | ⟨i, _, hi⟩ | hc
    · exact (premise_never_custody r 0 n).2 hr
    · simp [parameterClause] at hi
    · simp [shapeClauses] at hc
  · intro h
    rcases mem_expected.mp h with ⟨r, _, hr⟩ | ⟨i, _, hi⟩ | hc
    · exact (premise_never_custody r q n).1 hr
    · simp [parameterClause] at hi
    · simp [shapeClauses] at hc

/-- A retention keeps the retained position borrowed, states the retention,
and never releases it: never borrowed and forgotten. -/
theorem retention_keeps_item_borrowed (item holder arity : Nat) (ps : List Premise)
    (h : item < arity) (n : Null) :
    Clause.borrowed item ∈ expected (.retains item holder) arity ps ∧
      Clause.retainedBy item holder ∈ expected (.retains item holder) arity ps ∧
      Clause.released item n ∉ expected (.retains item holder) arity ps := by
  refine ⟨mem_expected.mpr (Or.inr (Or.inl ⟨item, h, by simp [parameterClause]⟩)),
    mem_expected.mpr (Or.inr (Or.inr (by simp [shapeClauses]))), ?_⟩
  intro hm
  rcases mem_expected.mp hm with ⟨r, _, hr⟩ | ⟨i, _, hi⟩ | hc
  · exact (premise_never_custody r item n).1 hr
  · simp [parameterClause] at hi
  · simp [shapeClauses] at hc

/-- The clauses that take, release, keep or return custody. -/
def custody : Clause → Bool
  | .released .. | .referenceDropped _ | .returnOwned _ | .returnKindReference
  | .returnBorrowedFrom .. | .retainedBy .. => true
  | _ => false

theorem plain_takes_no_custody (arity : Nat) (ps : List Premise) (c : Clause)
    (h : c ∈ expected .plain arity ps) : custody c = false := by
  rcases mem_expected.mp h with ⟨q, _, rfl⟩ | ⟨i, _, rfl⟩ | hc
  · cases q <;> rfl
  · rfl
  · simp [shapeClauses] at hc; subst hc; rfl

/-- Every documented premise is stated by every accepted declaration, so a
declaration that drops one is refused. -/
theorem premise_is_stated {s : Shape} {arity : Nat} {ps : List Premise} {decl : List Clause}
    (h : accepts s arity ps decl) {q : Premise} (hq : q ∈ ps) :
    premiseClause q ∈ decl :=
  h.mem_iff.mpr (mem_expected.mpr (Or.inl ⟨q, hq, rfl⟩))

/-! ## The dependency fold -/

/-- A function row's published status: its own, joined with every binding's
verdict of every bound declaration in `deps`. -/
def published {α : Type} (own : V) (links : α → List V) (deps : List α) : V :=
  join own (fold (deps.flatMap links))

theorem published_never_lowers {α : Type} (own : V) (links : α → List V) (deps : List α) :
    own.rank ≤ (published own links deps).rank :=
  le_join_left _ _

theorem resting_on_pending_is_never_safe {α : Type} (own : V) (links : α → List V)
    {deps : List α} {d : α} (hd : d ∈ deps) (hl : V.pending ∈ links d) :
    published own links deps ≠ .safe := by
  intro hp
  have hmem : V.pending ∈ deps.flatMap links := List.mem_flatMap.mpr ⟨d, hd, hl⟩
  have hle := every_link_is_below_the_fold hmem
  have hj := le_join_right own (fold (deps.flatMap links))
  have : (V.pending).rank ≤ (published own links deps).rank := Nat.le_trans hle hj
  rw [hp] at this
  exact absurd this (by decide)

theorem fold_le_of_subset {xs ys : List V} (h : ∀ x ∈ xs, x ∈ ys) :
    (fold xs).rank ≤ (fold ys).rank := by
  induction xs with
  | nil => simp [fold, rank]
  | cons x xs ih =>
    have hx := every_link_is_below_the_fold (h x (List.mem_cons_self ..))
    have hxs := ih (fun y hy => h y (List.mem_cons_of_mem _ hy))
    show (join x (fold xs)).rank ≤ _
    rw [join_rank]
    exact Nat.max_le.mpr ⟨hx, hxs⟩

/-- Folding every dependency is at least folding the `ContractCall` callees
the earlier fold read, whenever those are among the dependencies. -/
theorem dependencies_cover_applied {α : Type} (own : V) (links : α → List V)
    {applied deps : List α} (h : ∀ a ∈ applied, a ∈ deps) :
    (published own links applied).rank ≤ (published own links deps).rank := by
  unfold published
  rw [join_rank, join_rank]
  have hsub : ∀ x ∈ applied.flatMap links, x ∈ deps.flatMap links := by
    intro x hx
    rcases List.mem_flatMap.mp hx with ⟨a, ha, hxa⟩
    exact List.mem_flatMap.mpr ⟨a, h a ha, hxa⟩
  exact Nat.max_le.mpr ⟨Nat.le_max_left _ _, Nat.le_trans (fold_le_of_subset hsub) (Nat.le_max_right _ _)⟩

end QCore.OpensslCalleeShapes
