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

import QCore.BodyClauseStanding

/-!
# A refinement conflict settled by the declaration's own entry check (card DECLARED-ASSUMPTION-BODY-CHECKS, rules 13m–13n)

The abstract rules implemented by
`crates/qcore-c-gateway/src/legacy_declared_entry_check.rs` (which bodies are
checked) and the evidence a unit records for one examined function-map
declaration in force in its derivation
(`crates/qcore-c-gateway/src/contract_dependencies.rs`, `unit_evidence`).

* **Candidates (rule 13m).** Only a declaration in force in the derivation
  exactly as declared, that is its own custody projection (no container or
  counted-user clause), not refuted (C06.4), not withdrawn by the mode gate
  (a duty contradiction), and not a withdrawn checked specification (C06.3),
  and whose W4 summary conflicts with it.
* **Recording.** A conflict is a failure unless the candidate's declared-entry
  check passed (then `13m` evidence and no failure); an equal W4 summary is
  `W4` evidence; otherwise nothing. A refutation, a mode withdrawal and a
  withdrawn checked specification stay failures, and a failure wins in its
  unit (`QCore.ContractDependencies.a_failure_wins_in_its_unit`).
* **Coverage.** `13m` covers exactly what `entry` covers: the custody
  projection (`QCore.BodyClauseStanding.covers`).
* **Evidence only (rule 13n).** No verdict reads the record (the previous
  cards' `publishing_the_set_changes_no_verdict`); this module states only
  what is recorded.

Theorems:
* `new_evidence_needs_the_entry_check`, `new_evidence_needs_a_conflict`;
* `a_container_or_counted_user_declaration_is_never_checked`,
  `a_withdrawn_or_refuted_declaration_is_never_checked`;
* `a_failed_check_keeps_the_conflict_a_failure`,
  `a_contradiction_still_fails`, `without_a_passing_check_nothing_changes`,
  `only_a_conflict_changes`;
* `the_new_evidence_covers_only_custody`,
  `the_new_evidence_never_covers_a_clause_entry_does_not`;
* the shared vectors `recording_vectors_hold`, bound to the Rust by the unit
  test of `legacy_declared_entry_check.rs`.

## What this is not

A proof that the gateway's entry analysis is exact for custody. The check is
the one W4 validates every summary with (`observations_match_contract` over
the body analysed under the entry custody); that the Rust performs it on the
CFG the reported run builds, with every refusal of the row route, is a review
premise (trust boundary 28), pinned by the Rust controls of
`crates/qcore-c-gateway/tests/declared_entry_check.rs`.
-/

namespace QCore.DeclaredEntryCheck

open QCore.BodyClauseStanding (Evidence ClauseKind covers)
open QCore.ContractDependencies (Outcome unitOutcome)

/-- What the unit knows about one examined function-map declaration in force
in its derivation. -/
structure Body where
  /-- In force in the derivation exactly as declared. -/
  inForce : Bool
  /-- The declaration is its own custody projection. -/
  ownProjection : Bool
  /-- Refuted by a body (C06.4). -/
  refuted : Bool
  /-- Withdrawn by the mode gate (a duty contradiction). -/
  modeWithdrawn : Bool
  /-- A withdrawn checked specification (C06.3). -/
  specificationWithdrawn : Bool
  /-- W4's validated summary differs from the declaration. -/
  conflict : Bool
  /-- W4 published a validated summary (equal or not). -/
  summary : Bool
  /-- The declared-entry check passed on this body. -/
  entryHolds : Bool
  deriving DecidableEq, Repr

/-- Whether the declaration may be checked at all. -/
def Body.eligible (b : Body) : Bool :=
  b.inForce && b.ownProjection && !b.refuted && !b.modeWithdrawn && !b.specificationWithdrawn

/-- The body is checked (`checked`). -/
def candidate (b : Body) : Bool :=
  b.eligible && b.conflict

/-- The declared-entry check established the declaration. -/
def established (b : Body) : Bool :=
  candidate b && b.entryHolds

/-- The failures the unit records for the declaration. -/
def failed (b : Body) : Bool :=
  b.refuted || b.modeWithdrawn || b.specificationWithdrawn ||
    (b.conflict && !established b)

/-- The evidence the unit records for the declaration (`recorded`). -/
def evidence (b : Body) : List Evidence :=
  if b.conflict then (if established b then [.r13m] else [])
  else if b.summary then [.w4]
  else []

/-- The unit's outcome for the declaration. -/
def outcome (b : Body) : Outcome :=
  unitOutcome (!(evidence b).isEmpty) (failed b)

/-- The previous card's recording. -/
def previousFailed (b : Body) : Bool :=
  b.refuted || b.modeWithdrawn || b.specificationWithdrawn || b.conflict

def previousEvidence (b : Body) : List Evidence :=
  if b.conflict then [] else if b.summary then [.w4] else []

/-! ## The new evidence needs the check -/

theorem new_evidence_needs_the_entry_check (b : Body) (he : Evidence.r13m ∈ evidence b) :
    b.entryHolds = true := by
  unfold evidence established at he
  cases hc : b.conflict <;> cases hs : b.summary <;> cases hh : b.entryHolds <;>
    simp_all

theorem new_evidence_needs_a_conflict (b : Body) (he : Evidence.r13m ∈ evidence b) :
    b.conflict = true ∧ b.eligible = true := by
  unfold evidence established candidate at he
  cases hc : b.conflict <;> cases hs : b.summary <;> cases hl : b.eligible <;> simp_all

/-! ## Who is never checked -/

theorem a_container_or_counted_user_declaration_is_never_checked (b : Body)
    (h : b.ownProjection = false) : candidate b = false := by
  simp [candidate, Body.eligible, h]

theorem a_withdrawn_or_refuted_declaration_is_never_checked (b : Body)
    (h : b.refuted = true ∨ b.modeWithdrawn = true ∨ b.specificationWithdrawn = true) :
    candidate b = false := by
  rcases h with h | h | h <;> simp [candidate, Body.eligible, h]

/-! ## Failures stay failures -/

theorem a_failed_check_keeps_the_conflict_a_failure (b : Body)
    (hc : b.conflict = true) (hh : b.entryHolds = false) : outcome b = .failed := by
  simp [outcome, failed, established, hc, hh, unitOutcome]

theorem a_contradiction_still_fails (b : Body)
    (h : b.refuted = true ∨ b.modeWithdrawn = true ∨ b.specificationWithdrawn = true) :
    outcome b = .failed := by
  rcases h with h | h | h <;> simp [outcome, failed, h, unitOutcome]

/-- Where the check does not pass, the unit records exactly what it recorded
before the card. -/
theorem without_a_passing_check_nothing_changes (b : Body) (hh : b.entryHolds = false) :
    failed b = previousFailed b ∧ evidence b = previousEvidence b := by
  constructor
  · simp [failed, previousFailed, established, hh]
  · unfold evidence previousEvidence established
    cases b.conflict <;> cases b.summary <;> simp [hh]

/-- Without a conflict nothing changes either: the card reads only conflicts. -/
theorem only_a_conflict_changes (b : Body) (hc : b.conflict = false) :
    failed b = previousFailed b ∧ evidence b = previousEvidence b := by
  constructor
  · simp [failed, previousFailed, hc]
  · simp [evidence, previousEvidence, hc]

/-! ## Coverage -/

theorem the_new_evidence_covers_only_custody (k : ClauseKind) :
    covers .r13m k = k.custody ∧ covers .r13m k = covers .entry k := by
  simp [covers]

theorem the_new_evidence_never_covers_a_clause_entry_does_not :
    covers .r13m .premise = false ∧ covers .r13m .variadicTail = false ∧
      covers .r13m .noReturn = false ∧ covers .r13m .countedUser = false ∧
      covers .r13m .container = false ∧ covers .r13m .retention = false := by
  decide

/-! ## Shared vectors

Bound to the Rust (`checked` and `recorded`) by the unit test of
`crates/qcore-c-gateway/src/legacy_declared_entry_check.rs`: (conflict,
summary, eligible, entryHolds, the evidence spelling or "failed" or "none"). -/

def bodyOf (conflict summary eligible entryHolds : Bool) : Body :=
  ⟨true, eligible, false, false, false, conflict, summary, entryHolds⟩

def Body.spelling (b : Body) : String :=
  match outcome b, evidence b with
  | .failed, _ => "failed"
  | _, [e] => e.spelling
  | _, _ => "none"

def recordingVectors : List (Bool × Bool × Bool × Bool × String) := [
  -- a nullable release wrapper: W4 conflicts, the declaration's entry holds
  (true, true, true, true, "13m"),
  -- a nullable formal handed to a non-null release: the check fails
  (true, true, true, false, "failed"),
  -- an equal W4 summary
  (false, true, true, false, "W4"),
  -- an equal W4 summary; the check is not run
  (false, true, true, true, "W4"),
  -- no summary: nothing is recorded, whatever an entry check would say
  (false, false, true, true, "none"),
  -- a counted-user declaration is never checked; its conflict stays a failure
  (true, true, false, true, "failed")
]

theorem recording_vectors_hold :
    recordingVectors.all (fun v =>
      (bodyOf v.1 v.2.1 v.2.2.1 v.2.2.2.1).spelling == v.2.2.2.2) = true := by
  decide

end QCore.DeclaredEntryCheck
