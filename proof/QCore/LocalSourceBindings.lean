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

import QCore.CjsonRepresentationPremises
import QCore.LibcReadingSiteEvidence

/-! Finite source-local refinement only. Native allocation/execution, Clang
projection and Rust/Lean correspondence are explicit external prerequisites.
No theorem converts a finite scope into an opaque universal graph proposition. -/
namespace QCore.LocalSourceBindings
open CjsonRepresentationPremises

/-- The Rust adapter derives these equations from retained native object fields.
`ownedChild` in this finite view means exactly the selected sequence, not a
claim about arbitrary caller custody. The rank is the sequence index. -/
structure EntryCheck (e : Environment) (rank : Nat → Nat) : Prop where
  parentLive : e.live e.parent
  exactDomain : ∀ x, e.ownedChild x ↔ x ∈ e.children
  childrenLive : ∀ x ∈ e.children, e.live x
  head : e.child = e.children.head?
  adjacent : ∀ before a b suffix,
    e.children = before ++ a :: b :: suffix → e.next a = some b
  nextRank : ∀ a b, e.next a = some b → rank a < rank b
  unique : e.children.Nodup
  emptyHead : e.child = none ↔ e.children = []
  first : firstPrev e
  last : lastNext e
  targets : linkTargets e
  outside : parentOutside e
  previous : predecessor e

theorem next_path_increases_rank (next : Nat → Option Nat) (rank : Nat → Nat)
    (grows : ∀ a b, next a = some b → rank a < rank b)
    {a b : Nat} (path : NextPath next a b) : rank a < rank b := by
  induction path with
  | step edge => exact grows _ _ edge
  | trans _ _ first second => exact Nat.lt_trans first second

/-- A finite checked index ranking supplies the nontrivial acyclicity part of
chain. Eight entry propositions follow; noWriter is deliberately absent. -/
theorem checked_entry_has_eight_finite_equations (e : Environment) (rank : Nat → Nat)
    (checked : EntryCheck e rank) :
    chain e ∧ distinct e ∧ empty e ∧ firstPrev e ∧ lastNext e ∧
      linkTargets e ∧ parentOutside e ∧ predecessor e := by
  refine ⟨?_, checked.unique, checked.emptyHead, checked.first, checked.last,
    checked.targets, checked.outside, checked.previous⟩
  refine ⟨checked.parentLive, checked.exactDomain, checked.childrenLive,
    checked.head, checked.adjacent, ?_⟩
  intro x _ path
  exact (Nat.lt_irrefl (rank x))
    (next_path_increases_rank e.next rank checked.nextRank path)

/-- Native single-thread samples do not establish arbitrary interference. -/
theorem a_foreign_link_write_still_refutes_writer_guard (e : Environment) (s : Step)
    (seen : s ∈ e.trace) (foreign : s.actor ≠ e.callActor)
    (writes : s.writesLinks = true) : ¬ noWriter e := by
  exact a_writing_step_refutes_no_writer e s seen foreign writes

/-- A source-local native domain. The driver must bind these fields to its
retained native snapshots; the model supplies no execution attestation. -/
structure NativeDomain where
  entries : List Environment
  rank : Environment → Nat → Nat
  postPredicates : List Bool
  caseKeys : List (Nat × Option Nat)

def canonicalCases : List (Nat × Option Nat) :=
  [(0, none), (1, some 0), (2, some 0), (2, some 1),
   (3, some 0), (3, some 1), (3, some 2),
   (4, some 0), (4, some 1), (4, some 2), (4, some 3)]

def finiteEntryEquations (d : NativeDomain) : Prop :=
  ∀ e ∈ d.entries, chain e ∧ distinct e ∧ empty e ∧ firstPrev e ∧
    lastNext e ∧ linkTargets e ∧ parentOutside e ∧ predecessor e

def finiteDetachPost (d : NativeDomain) : Prop :=
  ∀ predicate ∈ d.postPredicates, predicate = true

def incompleteFixedCaseDomain (d : NativeDomain) : Prop :=
  ¬ d.caseKeys.Perm canonicalCases

theorem native_entry_checks_refine_finite_equations (d : NativeDomain)
    (allEntries : ∀ e ∈ d.entries, EntryCheck e (d.rank e)) :
    finiteEntryEquations d := by
  intro e member
  exact checked_entry_has_eight_finite_equations e (d.rank e) (allEntries e member)

theorem complete_case_domain_covers_each_case (d : NativeDomain)
    (complete : d.caseKeys.Perm canonicalCases) (key : Nat × Option Nat)
    (member : key ∈ canonicalCases) : key ∈ d.caseKeys := by
  exact complete.mem_iff.mpr member

theorem checked_post_predicate_is_true (d : NativeDomain)
    (allTrue : d.postPredicates.all id = true) (predicate : Bool)
    (member : predicate ∈ d.postPredicates) : predicate = true := by
  exact List.all_eq_true.mp allTrue predicate member

/-- Full finite recorded-domain equality, not selected membership. -/
def domainAccepts (held presented fresh : List S) (predicate : S → Prop) : Prop :=
  presented = held ∧ fresh = held ∧ ∀ s ∈ fresh, predicate s

theorem complete_recorded_domain_covers_each_held_site (held presented fresh : List S)
    (predicate : S → Prop) (accepted : domainAccepts held presented fresh predicate)
    (s : S) (member : s ∈ held) : predicate s := by
  apply accepted.2.2 s
  simpa [accepted.2.1] using member

theorem truncated_domain_refused (held presented fresh : List S) (predicate : S → Prop)
    (s : S) (freshMember : s ∈ fresh) (omitted : s ∉ presented) :
    ¬ domainAccepts held presented fresh predicate := by
  intro accepted
  apply omitted
  simpa [accepted.1, accepted.2.1] using freshMember

/-- A scalar route supplies the same allowed-route disjunction independently
of nested-route truth; the checker never conjoins both alternatives. -/
theorem scalar_site_preserves_alternative (nested scalar : Bool) (h : scalar = true) :
    nested = true ∨ scalar = true := by
  exact Or.inr h

/-- Exact Context remains a prerequisite even after local source binding.
Dependency records here are supplied by the actual consultation closure. -/
structure Row where
  id : Nat
  context : LibcReadingSiteEvidence.Context
  dependencies : List Nat

def affected (ctx : LibcReadingSiteEvidence.Context) (atom : Nat) (row : Row) : Prop :=
  row.context = ctx ∧ atom ∈ row.dependencies

theorem another_context_is_unaffected (ctx : LibcReadingSiteEvidence.Context)
    (atom : Nat) (row : Row) (foreign : row.context ≠ ctx) : ¬ affected ctx atom row := by
  intro h
  exact foreign h.1

theorem unrelated_dependency_is_unaffected (ctx : LibcReadingSiteEvidence.Context)
    (atom : Nat) (row : Row) (absent : atom ∉ row.dependencies) : ¬ affected ctx atom row := by
  intro h
  exact absent h.2

end QCore.LocalSourceBindings
