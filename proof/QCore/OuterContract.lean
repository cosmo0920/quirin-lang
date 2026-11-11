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
# Outer contracts and the runtime projection (card OUTER-CONTRACT, rule 31)

The abstract rule implemented by `crates/qcore-audit/src/outer_selection.rs`
(`contribution`, `runtime_verdict`, `project`), `crates/qcore-contract/src/outer_contract.rs`
(the loader) and `crates/qcore-c-gateway/src/outer_record_graph.rs` (the reach),
contract specification C04.20, at the user's decisions of 2026-10-17 (D1 "B:
loader envelope", D3 "Both from one run").

* **Facts.** A row rests on closed-world facts: conclusions the analysis drew
  because it saw every store or registration in the run. A member fact names a
  record (its tag); a premise names none.
* **Contracts.** An outer contract is in force or not (its pin), reaches a set
  of records (or every record), and covers some fact keys (its envelope).
* **Contribution.** A fact a contract cannot reach contributes nothing; a
  reached fact contributes `pending` when the contract is in force and covers
  it, and `unknown` otherwise. A premise is reached by every contract.
* **Runtime verdict.** The join (`safe < pending < unknown < unsafe`) of the
  static verdict and every contribution of every fact under every loaded
  contract. The static scope loads nothing.

Theorems:
* `static_projection_is_identity` (nothing loaded: the static verdict);
* `runtime_never_promotes` (`static ≤ runtime`);
* `a_row_without_closed_world_facts_is_unchanged`;
* `a_covered_fact_is_pending_with_its_contract`;
* `every_outer_pending_names_a_contract` (a `pending` contribution comes only
  from an in-force contract covering the fact, and a row moved from `safe` to
  `pending` names one);
* `an_uncovered_fact_is_never_safe_or_pending`;
* `runtime_is_the_join` (`join` is commutative and associative, and the
  projection of a concatenation is the projection of its parts, so the order
  facts are read in does not matter);
* `reachability_over_approximates` (every record reachable along the graph's
  edges from the binder is in any closed set holding the binder's records,
  which is what the reach computes, and a universal reach holds everything);
* `projection_vectors_hold` (the shared example of
  `crates/qcore-audit/tests/outer_projection.rs`).

## What this is not

A model over abstract keys and an abstract graph. That the Rust reads every
closed-world fact a row rests on from its dependency record (rule 13f: never
omitted), that the breadth-first search returns a closed set, and that Clang's
record graph has the edges the rule names, are review premises (trust
boundary 28), pinned by the Rust tests named above.
-/

namespace QCore.OuterContract

inductive Verdict where
  | safe
  | pending
  | unknown
  /-- `Unsafe` (`unsafe` is a Lean keyword). -/
  | violated
  deriving DecidableEq, Repr

namespace Verdict

def rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .violated => 3

/-- The composition: the larger in `safe < pending < unknown < unsafe`. -/
def join (a b : Verdict) : Verdict := if rank b ≤ rank a then a else b

theorem rank_injective {a b : Verdict} (h : rank a = rank b) : a = b := by
  cases a <;> cases b <;> simp_all [rank]

theorem le_join_left (a b : Verdict) : rank a ≤ rank (join a b) := by
  unfold join; split <;> omega

theorem le_join_right (a b : Verdict) : rank b ≤ rank (join a b) := by
  unfold join; split <;> omega

theorem join_comm (a b : Verdict) : join a b = join b a := by
  cases a <;> cases b <;> rfl

theorem join_assoc (a b c : Verdict) : join (join a b) c = join a (join b c) := by
  cases a <;> cases b <;> cases c <;> rfl

theorem join_safe (a : Verdict) : join a .safe = a := by
  cases a <;> rfl

end Verdict

open Verdict

/-- A closed-world fact: its key, and the record it is on (`none` for a
premise). -/
structure Fact where
  key : String
  tag : Option String
deriving DecidableEq, Repr

/-- An outer contract as a scenario reads it. -/
structure Contract where
  name : String
  inForce : Bool
  covers : String → Bool
  reaches : String → Bool

/-- What a fact contributes under one contract. -/
def contribution (c : Contract) (f : Fact) : Option Verdict :=
  let reached := match f.tag with
    | some t => c.reaches t
    | none => true
  if reached then
    some (if c.inForce && c.covers f.key then .pending else .unknown)
  else none

/-- Every contribution of every fact under every contract of the scenario. -/
def contributions (scenario : List Contract) (facts : List Fact) : List Verdict :=
  facts.flatMap (fun f => scenario.filterMap (fun c => contribution c f))

/-- The runtime verdict: the static verdict joined with every contribution. -/
def runtime (scenario : List Contract) (v : Verdict) (facts : List Fact) : Verdict :=
  (contributions scenario facts).foldl join v

theorem foldl_join_ge (l : List Verdict) (v : Verdict) : rank v ≤ rank (l.foldl join v) := by
  induction l generalizing v with
  | nil => simp
  | cons x xs ih => exact Nat.le_trans (le_join_left v x) (ih (join v x))

theorem foldl_join_ge_mem (l : List Verdict) (v x : Verdict) (h : x ∈ l) :
    rank x ≤ rank (l.foldl join v) := by
  induction l generalizing v with
  | nil => simp at h
  | cons y ys ih =>
    simp only [List.foldl_cons]
    rcases List.mem_cons.mp h with rfl | hm
    · exact Nat.le_trans (le_join_right v x) (foldl_join_ge ys (join v x))
    · exact ih (join v y) hm

/-- Nothing is loaded in the static scope: the projection is the identity. -/
theorem static_projection_is_identity (v : Verdict) (facts : List Fact) :
    runtime [] v facts = v := by
  unfold runtime contributions
  have empty : facts.flatMap
      (fun f => ([] : List Contract).filterMap (fun c => contribution c f)) = [] := by
    induction facts with
    | nil => rfl
    | cons x xs ih => simp [List.flatMap_cons]
  rw [empty]
  rfl

theorem runtime_never_promotes (s : List Contract) (v : Verdict) (facts : List Fact) :
    rank v ≤ rank (runtime s v facts) :=
  foldl_join_ge _ v

theorem a_row_without_closed_world_facts_is_unchanged (s : List Contract) (v : Verdict) :
    runtime s v [] = v := by
  simp [runtime, contributions]

theorem a_covered_fact_is_pending_with_its_contract (c : Contract) (f : Fact)
    (reached : ∀ t, f.tag = some t → c.reaches t = true)
    (inForce : c.inForce = true) (covered : c.covers f.key = true) :
    contribution c f = some .pending := by
  unfold contribution
  cases h : f.tag with
  | none => simp [inForce, covered]
  | some t => simp [reached t h, inForce, covered]

/-- A `pending` contribution comes only from an in-force contract covering
the fact. -/
theorem a_pending_contribution_names_its_contract (c : Contract) (f : Fact)
    (h : contribution c f = some .pending) : c.inForce = true ∧ c.covers f.key = true := by
  by_cases hc : (c.inForce && c.covers f.key) = true
  · simpa using hc
  · exfalso
    unfold contribution at h
    split at h <;> simp_all

theorem mem_contributions {s : List Contract} {facts : List Fact} {x : Verdict} :
    x ∈ contributions s facts ↔ ∃ f ∈ facts, ∃ c ∈ s, contribution c f = some x := by
  unfold contributions
  simp [List.mem_flatMap, List.mem_filterMap]

theorem foldl_join_mem_or_init (l : List Verdict) (v : Verdict) :
    l.foldl join v = v ∨ l.foldl join v ∈ l := by
  induction l generalizing v with
  | nil => simp
  | cons x xs ih =>
    simp only [List.foldl_cons]
    rcases ih (join v x) with h | h
    · rw [h]; unfold join; split
      · exact Or.inl rfl
      · exact Or.inr (List.mem_cons_self ..)
    · exact Or.inr (List.mem_cons_of_mem _ h)

/-- A row the projection moves from `safe` to `pending` names an in-force
contract covering one of its facts: an outer `Pending` always has its named,
human-resolvable reason (hard invariant 2). -/
theorem every_outer_pending_names_a_contract (s : List Contract) (facts : List Fact)
    (h : runtime s .safe facts = .pending) :
    ∃ f ∈ facts, ∃ c ∈ s, contribution c f = some .pending ∧
      c.inForce = true ∧ c.covers f.key = true := by
  unfold runtime at h
  rcases foldl_join_mem_or_init (contributions s facts) .safe with hv | hm
  · rw [hv] at h; cases h
  · rw [h] at hm
    obtain ⟨f, hf, c, hc, hcon⟩ := mem_contributions.mp hm
    exact ⟨f, hf, c, hc, hcon, a_pending_contribution_names_its_contract c f hcon⟩

/-- A fact a loaded contract reaches but does not cover (or covers while not
in force) leaves the row at least `unknown`: never `safe`, never `pending`. -/
theorem an_uncovered_fact_is_never_safe_or_pending (s : List Contract) (v : Verdict)
    (facts : List Fact) (f : Fact) (hf : f ∈ facts) (c : Contract) (hc : c ∈ s)
    (reached : ∀ t, f.tag = some t → c.reaches t = true)
    (uncovered : (c.inForce && c.covers f.key) = false) :
    rank .unknown ≤ rank (runtime s v facts) := by
  have hcon : contribution c f = some .unknown := by
    unfold contribution
    cases h : f.tag with
    | none => simp [uncovered]
    | some t => simp [reached t h, uncovered]
  exact foldl_join_ge_mem _ v .unknown (mem_contributions.mpr ⟨f, hf, c, hc, hcon⟩)

theorem foldl_join_append (l m : List Verdict) (v : Verdict) :
    (l ++ m).foldl join v = m.foldl join (l.foldl join v) := by
  simp [List.foldl_append]

/-- The projection is the join: reading the facts in two parts is reading
them at once, and `join` is commutative and associative, so neither the order
of the facts nor of the contracts changes the verdict. -/
theorem runtime_is_the_join (s : List Contract) (v : Verdict) (f g : List Fact) :
    runtime s v (f ++ g) = runtime s (runtime s v f) g ∧
      (∀ a b : Verdict, join a b = join b a) ∧
      (∀ a b c : Verdict, join (join a b) c = join a (join b c)) := by
  refine ⟨?_, join_comm, join_assoc⟩
  unfold runtime contributions
  rw [List.flatMap_append, foldl_join_append]

/-! ## Reachability -/

/-- A record graph: each record's edges (typed pointers, embedded records and
the reverse `container_of` edges are all edges here). -/
structure Graph where
  edges : String → List String

/-- What code holding the binder can reach along the graph. -/
inductive Reachable (g : Graph) (start : List String) : String → Prop where
  | base {r} : r ∈ start → Reachable g start r
  | step {r s} : Reachable g start r → s ∈ g.edges r → Reachable g start s

/-- The reach the Rust computes: every record, or a set holding the binder's
records and closed under the edges (the breadth-first search's result). -/
inductive Reach where
  | all
  | records (s : String → Prop)

def Reach.contains : Reach → String → Prop
  | .all, _ => True
  | .records s, r => s r

def closedOver (g : Graph) (start : List String) (s : String → Prop) : Prop :=
  (∀ r ∈ start, s r) ∧ ∀ r, s r → ∀ t ∈ g.edges r, s t

/-- Over-approximation: a closed set holding the binder's records holds every
record the binder reaches, and `all` holds everything. So a fact on a record
loaded code can reach is never left out of the projection. -/
theorem reachability_over_approximates (g : Graph) (start : List String) (reach : Reach)
    (computed : reach = .all ∨ ∃ s, reach = .records s ∧ closedOver g start s)
    (r : String) (h : Reachable g start r) : reach.contains r := by
  rcases computed with rfl | ⟨s, rfl, hstart, hclosed⟩
  · trivial
  · show s r
    induction h with
    | base hr => exact hstart _ hr
    | step _ hedge ih => exact hclosed _ ih _ hedge

/-- The record-graph reader distinguishes primitive pointers from a pointer
whose typedef pointee Clang has not classified. The latter must reach all. -/
inductive PointerPointee where
  | primitive
  | record (tag : String)
  | unclassified

def PointerPointee.reach : PointerPointee → Reach
  | .primitive => .records (fun _ => False)
  | .record tag => .records (fun t => t = tag)
  | .unclassified => .all

theorem an_unclassified_pointer_reaches_every_record (tag : String) :
    (PointerPointee.reach .unclassified).contains tag := by
  trivial

theorem a_primitive_pointer_reaches_no_record (tag : String) :
    ¬ (PointerPointee.reach .primitive).contains tag := by
  simp [PointerPointee.reach, Reach.contains]

/-! ## The shared example -/

/-- `crates/qcore-audit/tests/outer_projection.rs`: the contract covers
`instance.p`, a run set `collector.cb` and the premise `app_closed`; it reaches
every record. -/
def exampleContract (inForce : Bool) : Contract where
  name := "app_plugin"
  inForce := inForce
  covers := fun key => key == "instance.p" || key == "collector.cb" || key == "app_closed"
  reaches := fun _ => true

def stableP : Fact := { key := "instance.p", tag := some "instance" }
def stableQ : Fact := { key := "instance.q", tag := some "instance" }

theorem projection_vectors_hold :
    runtime [exampleContract true] .safe [stableP] = .pending ∧
    runtime [exampleContract true] .safe [stableP, stableQ] = .unknown ∧
    runtime [exampleContract true] .violated [stableP] = .violated ∧
    runtime [exampleContract false] .safe [stableP] = .unknown ∧
    runtime [exampleContract true] .pending [] = .pending ∧
    runtime [] .safe [stableP, stableQ] = .safe := by
  decide

/-! ## RK-PLUGIN-LIBRARY-OUTER-ENVELOPE

The second envelope and the strengthened Fluent Bit envelope each cover the
callback-closure premise. A single unreviewed outside-code condition cannot
cover the other loaded class, so the all-loaded scenario needs both. -/

def rkCallbackFact : Fact := { key := "rk_plugin_callback_closure", tag := none }

def rkLibraryContract : Contract where
  name := "rk_plugin_library"
  inForce := true
  covers := fun key => key == "rk_plugin_callback_closure"
  reaches := fun _ => true

def fluentBitContract : Contract where
  name := "flb_external_plugin"
  inForce := true
  covers := fun key => key == "flb_input_instance.p" || key == "rk_plugin_callback_closure"
  reaches := fun _ => true

theorem rk_callback_envelope_is_pending_with_both_loaded :
    runtime [rkLibraryContract] .safe [rkCallbackFact] = .pending ∧
    runtime [fluentBitContract] .safe [rkCallbackFact] = .pending ∧
    runtime [rkLibraryContract, fluentBitContract] .safe [rkCallbackFact] = .pending := by
  decide

end QCore.OuterContract
