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
# Premised member facts (card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER, rules 14x, 14z)

The abstract rule implemented by `crates/qcore-c-gateway/src/null_deref/scope.rs`
(`member_fact_premises`, `premises_of`), `null_deref.rs` (`attach_premises`) and
`crates/qcore-audit/src/null_fold.rs` (`add_dependencies`), contract
specification C09.3 and C04.18, at the user's decision of 2026-10-15 ("Only
nonnull_member* need it").

* **Facts.** A stable member (`nonnull_member_stable`) or a scope entry
  (`nonnull_member_scope`) is a member fact a null row lists under one
  dependency key (`stable:TAG.FIELD`, or the declared function's name). Its
  declaration may name premises (`requires resource_premise(NAME, "…")`):
  the fact is in force only while every one of them holds.
* **Listing.** A row lists exactly the premises of the facts its dependency
  set names, and names each in its dependencies as `premise:NAME`. A
  dependency added later (the run fold's travel) lists its premises too.
* **Evidence only.** Listing changes no verdict.

Theorems:
* `a_resting_row_lists_the_premise`, `a_listed_premise_is_a_dependency`;
* `a_listed_premise_comes_from_a_named_fact` (nothing spurious);
* `a_row_on_no_premised_fact_lists_none`;
* `listed_premises_put_every_fact_in_force` (a member fact the row rests on is
  in force exactly under the premises the row lists);
* `listing_changes_no_verdict`;
* `more_dependencies_never_drop_a_premise` (travel is monotone);
* `premise_vectors_hold` (the shared example of
  `crates/qcore-c-gateway/tests/null_member_premise.rs`).

## What this is not

A model over abstract names. That the walk's consulted names are what this
calls the dependency set, and the correspondence with the compiled Rust, are
review premises (trust boundary 28), pinned by the Rust tests named above.
-/

namespace QCore.NullMemberPremise

/-- A member fact: the dependency key a row lists it under and the premises
its declaration names. -/
structure Fact where
  key : String
  premises : List String
deriving DecidableEq, Repr

/-- The verdict is abstract here: listing never reads it. -/
structure Row (V : Type) where
  verdict : V
  dependencies : List String
  premises : List String

/-- The premises of the facts `deps` names. -/
def premisesOf (env : List Fact) (deps : List String) : List String :=
  (env.filter (fun fact => decide (fact.key ∈ deps))).flatMap Fact.premises

/-- `attach_premises`: list the premises and name each as a dependency. -/
def attach {V : Type} (env : List Fact) (row : Row V) : Row V :=
  { verdict := row.verdict
    dependencies := row.dependencies ++ (premisesOf env row.dependencies).map ("premise:" ++ ·)
    premises := premisesOf env row.dependencies }

theorem mem_premisesOf {env : List Fact} {deps : List String} {p : String} :
    p ∈ premisesOf env deps ↔ ∃ fact ∈ env, fact.key ∈ deps ∧ p ∈ fact.premises := by
  unfold premisesOf
  simp [List.mem_flatMap, List.mem_filter, and_assoc]

theorem a_resting_row_lists_the_premise {V : Type} (env : List Fact) (row : Row V)
    (fact : Fact) (p : String) (hf : fact ∈ env) (hk : fact.key ∈ row.dependencies)
    (hp : p ∈ fact.premises) : p ∈ (attach env row).premises := by
  show p ∈ premisesOf env row.dependencies
  exact mem_premisesOf.mpr ⟨fact, hf, hk, hp⟩

theorem a_listed_premise_is_a_dependency {V : Type} (env : List Fact) (row : Row V)
    (p : String) (hp : p ∈ (attach env row).premises) :
    ("premise:" ++ p) ∈ (attach env row).dependencies := by
  show ("premise:" ++ p) ∈
    row.dependencies ++ (premisesOf env row.dependencies).map ("premise:" ++ ·)
  exact List.mem_append_right _ (List.mem_map.mpr ⟨p, hp, rfl⟩)

theorem a_listed_premise_comes_from_a_named_fact {V : Type} (env : List Fact) (row : Row V)
    (p : String) (hp : p ∈ (attach env row).premises) :
    ∃ fact ∈ env, fact.key ∈ row.dependencies ∧ p ∈ fact.premises :=
  mem_premisesOf.mp hp

theorem a_row_on_no_premised_fact_lists_none {V : Type} (env : List Fact) (row : Row V)
    (h : ∀ fact ∈ env, fact.key ∈ row.dependencies → fact.premises = []) :
    (attach env row).premises = [] := by
  show premisesOf env row.dependencies = []
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro p hp
  obtain ⟨fact, hf, hk, hmem⟩ := mem_premisesOf.mp hp
  rw [h fact hf hk] at hmem
  exact List.not_mem_nil hmem

/-- A fact is in force under `holds` when every premise it names holds. -/
def inForce (holds : String → Prop) (fact : Fact) : Prop :=
  ∀ p ∈ fact.premises, holds p

/-- A member fact the row rests on is in force under exactly the premises the
row lists: whoever discharges the listed premises discharges every fact's. -/
theorem listed_premises_put_every_fact_in_force {V : Type} (env : List Fact) (row : Row V)
    (holds : String → Prop) (h : ∀ p ∈ (attach env row).premises, holds p) :
    ∀ fact ∈ env, fact.key ∈ row.dependencies → inForce holds fact := by
  intro fact hf hk p hp
  exact h p (a_resting_row_lists_the_premise env row fact p hf hk hp)

theorem listing_changes_no_verdict {V : Type} (env : List Fact) (row : Row V) :
    (attach env row).verdict = row.verdict ∧
      ∀ d ∈ row.dependencies, d ∈ (attach env row).dependencies := by
  refine ⟨rfl, ?_⟩
  intro d hd
  exact List.mem_append_left _ hd

theorem more_dependencies_never_drop_a_premise (env : List Fact) (deps more : List String)
    (h : ∀ d ∈ deps, d ∈ more) (p : String) (hp : p ∈ premisesOf env deps) :
    p ∈ premisesOf env more := by
  obtain ⟨fact, hf, hk, hmem⟩ := mem_premisesOf.mp hp
  exact mem_premisesOf.mpr ⟨fact, hf, h _ hk, hmem⟩

/-- The shared example: a premised stable member and scope entry, and a
`quant null-deref` declaration (no fact, so no key). -/
def exampleEnv : List Fact :=
  [{ key := "stable:S.next", premises := ["no_plugin"] },
   { key := "scoped", premises := ["no_plugin"] }]

theorem premise_vectors_hold :
    premisesOf exampleEnv ["stable:S.next"] = ["no_plugin"] ∧
    premisesOf exampleEnv ["scoped"] = ["no_plugin"] ∧
    premisesOf exampleEnv ["deref"] = [] ∧
    premisesOf exampleEnv [] = [] := by
  decide

end QCore.NullMemberPremise
