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

import QCore.RetentionPath
import QCore.LocalCondition
/-! S3 summary abstraction boundaries. Source/type/store extraction remains a
trust boundary. Keys include source/configuration/function scope; no candidate
site overlap supplies runtime equality. Endings across a call use only must
destroys: unordered removals cannot reconstruct incoming poisoning or order.
Symbolic paths denote the same object/value snapshot throughout a model trace.
Their extraction must establish that correspondence; source member stores,
unread call frames and unsupported exits withhold must endings and all release
prefixes. A custody-only declaration supplies no heap frame proof.
Rust: loop_lifetime_summary.rs and loop_lifetime_source.rs. -/
namespace QCore.LoopLifetimeSummary
inductive Root where
  | formal (scope index : Nat)
  | result (scope : Nat)
  | lifted (footprint : Nat)
  deriving DecidableEq, Repr
structure Path where
  root : Root
  fields : List Nat
  deriving DecidableEq, Repr
structure Registration where
  object : Path
  holder : Path
  poisoned : Bool
  deriving DecidableEq, Repr

def concrete (path : Path) : Bool :=
  match path.root with
  | .lifted _ => false
  | _ => true

def mayAlias (_left _right : Path) : Bool := true

def destroy (holder : Path) (rs : List Registration) : List Registration :=
  if concrete holder then rs.filter (fun r => r.holder != holder) else rs

/-- Every newly registered holder invalidates an earlier must-destroy,
whether the registration is local or imported from a callee's may effects. -/
def afterRegister (holder : Path) (destroyedBefore : List Path) : List Path :=
  destroyedBefore.filter fun p => p != holder

theorem a_new_registration_invalidates_its_earlier_destroy
    (holder : Path) (destroyedBefore : List Path) :
    holder ∉ afterRegister holder destroyedBefore := by
  simp [afterRegister]

structure ReleaseEffect where
  object : Path
  destroyedBefore : List Path
  deriving DecidableEq, Repr

/-- A may-release acts on incoming records before the callee's postconditions.
Only holders already destroyed before that release discharge it. -/
def releaseRefuses (effect : ReleaseEffect) (record : Registration) : Bool :=
  mayAlias effect.object record.object &&
    !(concrete record.holder && effect.destroyedBefore.contains record.holder)

def withoutHolderFrame (effect : ReleaseEffect) : ReleaseEffect :=
  { effect with destroyedBefore := [] }

theorem an_unproved_holder_frame_cannot_discharge_an_incoming_record
    (effect : ReleaseEffect) (record : Registration) :
    releaseRefuses (withoutHolderFrame effect) record = true := by
  simp [releaseRefuses, withoutHolderFrame, mayAlias]

theorem a_release_without_an_earlier_ending_refuses_every_incoming_record
    (object : Path) (record : Registration) :
    releaseRefuses ⟨object, []⟩ record = true := by
  simp [releaseRefuses, mayAlias]

theorem a_destroy_before_release_discharges_its_exact_holder
    (object : Path) (record : Registration) (knownHolder : concrete record.holder = true) :
    releaseRefuses ⟨object, [record.holder]⟩ record = false := by
  simp [releaseRefuses, knownHolder]

theorem a_later_destroy_does_not_retroactively_discharge_a_release
    (object _laterDestroy : Path) (record : Registration) :
    releaseRefuses ⟨object, []⟩ record = true := by
  simp [releaseRefuses, mayAlias]

theorem a_lifted_prefix_never_discharges_a_release
    (effect : ReleaseEffect) (record : Registration)
    (unknownHolder : concrete record.holder = false) : releaseRefuses effect record = true := by
  simp [releaseRefuses, mayAlias, unknownHolder]

/-- Proof presence is required on every branch where the record is present;
absence of a record differs from its unproved presence. Exact member paths
remain part of the record key. -/
def joinForwarded (leftRecords rightRecords leftProofs rightProofs : List Registration) : List Registration :=
  (leftProofs ++ rightProofs).filter fun record =>
    (!leftRecords.contains record || leftProofs.contains record) &&
    (!rightRecords.contains record || rightProofs.contains record)

theorem a_join_forwarded_on_two_present_arms_has_both_proofs
    (leftRecords rightRecords leftProofs rightProofs : List Registration)
    (record : Registration) (leftPresent : record ∈ leftRecords)
    (rightPresent : record ∈ rightRecords)
    (forwarded : record ∈ joinForwarded leftRecords rightRecords leftProofs rightProofs) :
    record ∈ leftProofs ∧ record ∈ rightProofs := by
  have gates := (List.mem_filter.mp forwarded).2
  simpa [leftPresent, rightPresent, List.contains_iff_mem] using gates

def exitOwed (records proofs : List Registration) : List Registration :=
  records.filter fun record => !proofs.contains record

def exitForwarded (records proofs : List Registration) : List Registration :=
  records.filter fun record => proofs.contains record

theorem every_retained_record_remains_owed_or_explicitly_forwarded
    (records proofs : List Registration) (record : Registration) (present : record ∈ records) :
    record ∈ exitOwed records proofs ∨ record ∈ exitForwarded records proofs := by
  by_cases proven : record ∈ proofs
  · right; simp [exitForwarded, present, proven]
  · left; simp [exitOwed, present, proven]

theorem forwarding_does_not_invent_or_remove_its_retained_record
    (records proofs : List Registration) (record : Registration)
    (forwarded : record ∈ exitForwarded records proofs) : record ∈ records := by
  exact (List.mem_filter.mp forwarded).1

theorem an_unproved_record_is_still_an_exit_obligation
    (records proofs : List Registration) (record : Registration)
    (present : record ∈ records) (unproved : record ∉ proofs) :
    record ∈ exitOwed records proofs := by simp [exitOwed, present, unproved]

def mayJoin (left right : List Registration) : List Registration := left ++ right

def mustJoin (left right : List Path) : List Path :=
  left.filter (fun p => right.contains p)

def admitted (openReasons obligations : List Nat) : Bool :=
  openReasons.isEmpty && obligations.isEmpty

def demote (old discovered : List Nat) : List Nat := old ++ discovered

def appendPath (base : Path) (fields : List Nat) : Path :=
  { root := base.root, fields := base.fields ++ fields }

def substitute (scope : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (footprint : Option Nat) (path : Path) : Option Path :=
  let liftOr (base : Option Path) :=
    match base with
    | some p => some (appendPath p path.fields)
    | none => footprint.map fun f => { root := .lifted f, fields := path.fields }
  match path.root with
  | .formal owner i => if owner == scope then
      match actuals[i]? with
      | some actual => liftOr actual
      | none => none
    else none
  | .result owner => if owner == scope then liftOr resultPath else none
  | .lifted _ => some path

theorem a_may_join_keeps_every_left_registration
    (left right : List Registration) (r : Registration) (h : r ∈ left) :
    r ∈ mayJoin left right := by simp [mayJoin, h]

theorem a_may_join_keeps_every_right_registration
    (left right : List Registration) (r : Registration) (h : r ∈ right) :
    r ∈ mayJoin left right := by simp [mayJoin, h]

theorem a_must_ending_stands_on_both_paths
    (left right : List Path) (p : Path) (h : p ∈ mustJoin left right) :
    p ∈ left ∧ p ∈ right := by
  simpa [mustJoin, List.mem_filter, List.contains_iff_mem] using h

theorem a_destroy_ends_every_registration_at_its_holder
    (rs : List Registration) (h : Path) (r : Registration)
    (atHolder : r.holder = h) (knownHolder : concrete h = true) : r ∉ destroy h rs := by
  simp [destroy, knownHolder, List.mem_filter, atHolder]

theorem a_destroy_keeps_other_holders
    (rs : List Registration) (h : Path) (r : Registration)
    (present : r ∈ rs) (other : r.holder ≠ h) : r ∈ destroy h rs := by
  simp only [destroy]
  split <;> simp_all [List.mem_filter]

theorem one_lifted_holder_never_ends_a_whole_footprint
    (rs : List Registration) (footprint : Nat) (fields : List Nat) :
    destroy ⟨.lifted footprint, fields⟩ rs = rs := by simp [destroy, concrete]

theorem formal_and_result_roots_remain_possible_aliases
    (owner scope index : Nat) (left right : List Nat) :
    mayAlias ⟨.formal owner index, left⟩ ⟨.result scope, right⟩ = true := rfl

theorem the_fold_only_demotes (old discovered : List Nat) (n : Nat)
    (h : n ∈ old) : n ∈ demote old discovered := by simp [demote, h]

theorem an_open_summary_is_never_admitted (reason : Nat)
    (rest obligations : List Nat) : admitted (reason :: rest) obligations = false := by
  simp [admitted]

theorem an_undischarged_precondition_is_never_admitted
    (reasons : List Nat) (obligation : Nat) (rest : List Nat) :
    admitted reasons (obligation :: rest) = false := by simp [admitted]

theorem a_foreign_function_formal_never_substitutes
    (scope owner index : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (footprint : Option Nat) (fields : List Nat)
    (different : owner ≠ scope) :
    substitute scope actuals resultPath footprint ⟨.formal owner index, fields⟩ = none := by
  simp [substitute, different]

theorem a_known_actual_preserves_its_own_root
    (scope index : Nat) (actuals : List (Option Path)) (actual : Path)
    (resultPath : Option Path) (footprint : Option Nat) (fields : List Nat)
    (known : actuals[index]? = some (some actual)) :
    substitute scope actuals resultPath footprint ⟨.formal scope index, fields⟩ =
      some (appendPath actual fields) := by simp [substitute, known]

theorem an_unresolved_actual_without_a_footprint_never_substitutes
    (scope index : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (fields : List Nat)
    (unknownActual : actuals[index]? = some none) :
    substitute scope actuals resultPath none ⟨.formal scope index, fields⟩ = none := by
  simp [substitute, unknownActual]

theorem a_closed_footprint_keeps_an_unnameable_registration_root
    (scope index footprint : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (fields : List Nat)
    (unknownActual : actuals[index]? = some none) :
    substitute scope actuals resultPath (some footprint) ⟨.formal scope index, fields⟩ =
      some ⟨.lifted footprint, fields⟩ := by
  simp [substitute, unknownActual]

/-- Shared with Rust's shared_destroy_vectors; source identity/canonicalization
is an extraction premise, not established by these abstract identifiers. -/
def destroyVectors : List (Bool × Bool × Bool × Bool) := [
  (false, true, false, false), (true, true, false, false),
  (false, false, false, true), (true, false, false, true),
  (false, true, true, true), (true, true, true, true)]

def destroyVector (poisoned sameHolder lifted : Bool) : Bool :=
  let holder : Path := ⟨if lifted then .lifted 0 else .formal 0 0, [1]⟩
  let selected : Path := if sameHolder then holder else ⟨.formal 0 1, [1]⟩
  let record : Registration := ⟨⟨.formal 0 0, [0]⟩, holder, poisoned⟩
  !(destroy selected [record]).isEmpty

theorem shared_destroy_vectors_hold :
    destroyVectors.all (fun (p, s, l, expected) => destroyVector p s l == expected) = true := by
  decide

/-- The single-body retention machine's poisoning rule remains authoritative;
S3 never reinterprets an unmatched removal as an identity operation. -/
theorem the_ordered_poisoning_boundary
    (events : List QCore.RetentionPath.Event) (record : QCore.RetentionPath.Rec)
    (records : QCore.RetentionPath.Records) (present : record ∈ records)
    (poisoned : record.poisoned = true)
    (notEnded : ∀ event ∈ events, QCore.RetentionPath.ends event record.holder = false) :
    ∀ after, QCore.RetentionPath.run records events = some after → record ∈ after :=
  QCore.RetentionPath.a_poisoned_record_ends_only_by_its_loop_destroy
    events record records present poisoned notEnded

end QCore.LoopLifetimeSummary
