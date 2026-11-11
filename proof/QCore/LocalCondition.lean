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

/-! S3 scoped conditions: a written prerequisite is an obligation of its caller,
not an assumed proof. Unknown conditions preserve both feasible branches.
Facts retain exact binder/object identity; joining paths drops one-sided facts.
Executable counterpart: crates/qcore-core/src/local_condition.rs. -/
namespace QCore.LocalCondition
/-- Guarded declarations live in the explicit S3 table, never the raw table
that unaware effect prepasses read. Canonical source replay checks partition. -/
def visibleUnaware (hasBinding hasBlock : Bool) : Bool := !(hasBinding || hasBlock)

theorem a_bound_condition_is_not_visible_to_an_unaware_consumer (hasBlock : Bool) :
    visibleUnaware true hasBlock = false := by simp [visibleUnaware]

theorem a_condition_block_is_not_visible_to_an_unaware_consumer (hasBinding : Bool) :
    visibleUnaware hasBinding true = false := by simp [visibleUnaware]

def ordinaryPublicationAllowed (guarded discharged : Bool) : Bool := !guarded && discharged

theorem a_guarded_interface_never_publishes_an_ordinary_summary (discharged : Bool) :
    ordinaryPublicationAllowed true discharged = false := by simp [ordinaryPublicationAllowed]

def publicationVectors : List (Bool × Bool × Bool) :=
  [(false,false,false),(false,true,true),(true,false,false),(true,true,false)]

theorem shared_publication_vectors_hold :
    publicationVectors.all (fun (guarded,discharged,expected) => ordinaryPublicationAllowed guarded discharged == expected) = true := by decide

inductive Truth where
  | yes | no | unknown
  deriving DecidableEq, Repr
structure Fact where
  binder : Nat
  root : Nat
  value : Bool
  deriving DecidableEq, Repr
structure Guard where
  binder : Nat
  root : Nat
  negated : Bool
  deriving DecidableEq, Repr

def prerequisite : Truth → Bool → Bool
  | .no, _ => true
  | _, discharged => discharged

def exact (g : Guard) (f : Fact) : Bool :=
  g.binder == f.binder && g.root == f.root

def matching (g : Guard) (facts : List Fact) : List Fact :=
  facts.filter (exact g)

def reading (g : Guard) (facts : List Fact) : Truth :=
  match matching g facts with
  | [] => .unknown
  | f :: rest =>
    if rest.any (fun x => x.value != f.value) then .unknown
    else if f.value != g.negated then .yes else .no

def join (left right : List Fact) : List Fact :=
  left.filter (fun f => right.contains f)

def enterScope (outer : List Fact) (localFact : Fact) : List Fact :=
  outer ++ [localFact]

theorem unknown_never_waives_a_prerequisite (discharged : Bool) :
    prerequisite .unknown discharged = discharged := rfl

theorem active_never_assumes_a_prerequisite (discharged : Bool) :
    prerequisite .yes discharged = discharged := rfl

theorem inactive_waives_only_its_own_prerequisite (discharged : Bool) :
    prerequisite .no discharged = true := rfl

theorem unknown_covers_every_concrete_branch (discharged : Bool) :
    prerequisite .unknown discharged =
      (prerequisite .yes discharged && prerequisite .no discharged) := by
  cases discharged <;> rfl

theorem an_unknown_undischarged_prerequisite_refuses :
    prerequisite .unknown false = false := rfl

theorem a_join_fact_was_established_on_both_paths
    (left right : List Fact) (f : Fact) (h : f ∈ join left right) :
    f ∈ left ∧ f ∈ right := by
  simpa [join, List.mem_filter, List.contains_iff_mem] using h

theorem no_common_fact_leaves_no_join_facts (left right : List Fact)
    (h : ∀ f, f ∈ left → f ∉ right) : join left right = [] := by
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro f hf
  obtain ⟨hl, hr⟩ := a_join_fact_was_established_on_both_paths left right f hf
  exact h f hl hr

theorem an_unmatched_instance_is_unresolved (g : Guard) (facts : List Fact)
    (h : ∀ f ∈ facts, f.root ≠ g.root) : reading g facts = .unknown := by
  have empty : matching g facts = [] := by
    apply List.eq_nil_iff_forall_not_mem.mpr
    intro f hf
    have hf' := List.mem_filter.mp hf
    have matched := hf'.2
    simp only [exact, Bool.and_eq_true, beq_iff_eq] at matched
    exact h f hf'.1 matched.2.symm
  simp [reading, empty]

theorem contradictory_facts_never_discharge_a_condition (binder root : Nat) (negated : Bool) :
    reading ⟨binder, root, negated⟩ [⟨binder, root, true⟩, ⟨binder, root, false⟩] = .unknown := by
  simp [reading, matching, exact]

def leaveScope (outer : List Fact) (_inner : List Fact) : List Fact := outer

theorem leaving_scope_never_exports_a_fresh_fact
    (outer : List Fact) (localFact : Fact) (fresh : localFact ∉ outer) :
    localFact ∉ leaveScope outer (enterScope outer localFact) := by
  simpa [leaveScope] using fresh

/-- Ordinary Hoare entry reasoning: the source adapter must separately prove
exact condition/predicate identity and entry-to-call snapshot correspondence.
An annotation at the external caller supplies no such proof. -/
theorem a_matching_conditional_entry_precondition_discharges_the_body_use
    (entryCondition currentCondition required : Bool)
    (sameSnapshot : entryCondition = currentCondition)
    (entryRequires : entryCondition = true → required = true) :
    currentCondition = true → required = true := by
  intro active
  exact entryRequires (sameSnapshot.trans active)

def vectors : List (Truth × Bool × Bool) := [
  (.yes, false, false), (.yes, true, true),
  (.no, false, true), (.no, true, true),
  (.unknown, false, false), (.unknown, true, true)]
theorem shared_vectors_hold :
    vectors.all (fun (t, d, expected) => prerequisite t d == expected) = true := by decide
end QCore.LocalCondition
