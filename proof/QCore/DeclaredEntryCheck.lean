import QCore.BodyClauseStanding

/-!
# The declared-entry check (card DECLARED-ASSUMPTION-BODY-CHECKS, rules 13m–13o)

The abstract rules implemented by
`crates/qcore-c-gateway/src/legacy_declared_entry_check.rs` (which bodies are
checked, and under which rule) and the evidence a unit records for one
examined function-map declaration
(`crates/qcore-c-gateway/src/contract_dependencies.rs`, `unit_evidence`).

* **Candidates.** Only a function-map declaration in force in the derivation
  exactly as declared, that is its own custody projection (no container or
  counted-user clause), not refuted (C06.4), not withdrawn by the mode gate,
  and not a withdrawn checked specification (C06.3). Under rule 13n when W4's
  summary conflicts with it; under rule 13m when W4 published nothing and the
  unit does not report the body.
* **Recording.** A conflict is a failure unless the candidate's declared-entry
  check passed (then `13n` evidence); an equal W4 summary is `W4` evidence; a
  scope-only body with no summary is `13m` evidence when its check passed;
  otherwise nothing. A refutation, a mode withdrawal and a withdrawn checked
  specification stay failures, and a failure wins in its unit
  (`QCore.ContractDependencies.a_failure_wins_in_its_unit`).
* **Coverage.** `13m` and `13n` cover exactly what `entry` covers: the custody
  projection (`QCore.BodyClauseStanding.covers`).

Theorems:
* `new_evidence_needs_the_entry_check`, `refinement_evidence_needs_a_conflict`,
  `scope_evidence_needs_an_unreported_body_without_a_summary`;
* `a_container_or_counted_user_declaration_is_never_checked`,
  `a_withdrawn_or_refuted_declaration_is_never_checked`;
* `a_failed_check_keeps_the_conflict_a_failure`,
  `a_contradiction_still_fails`, `without_a_passing_check_nothing_changes`;
* `the_new_evidence_covers_only_custody`,
  `the_new_evidence_never_covers_a_text_clause`;
* the shared vectors `recording_vectors_hold`, bound to the Rust by
  `crates/qcore-c-gateway/tests/declared_entry_check.rs`.

## What this is not

A proof that the gateway's entry analysis is exact for custody. The check
is the one W4 validates every summary with (`observations_match_contract`
over the body analysed under the entry custody); that the Rust performs it on
the CFG the reported run builds, with every refusal of the row route, is a
review premise (trust boundary 28), pinned by the Rust controls named above.
-/

namespace QCore.DeclaredEntryCheck

open QCore.BodyClauseStanding (Evidence ClauseKind covers allKinds)
open QCore.ContractDependencies (Outcome unitOutcome)

/-- What the unit knows about one examined function-map declaration. -/
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
  /-- The unit reports the function as a row. -/
  row : Bool
  /-- The declared-entry check passed on this body. -/
  entryHolds : Bool
  deriving DecidableEq, Repr

inductive Rule where
  | scope | refinement
  deriving DecidableEq, Repr

/-- Whether the declaration may be checked at all. -/
def Body.eligible (b : Body) : Bool :=
  b.inForce && b.ownProjection && !b.refuted && !b.modeWithdrawn && !b.specificationWithdrawn

/-- The rule a body is checked under, if any (`candidate`). -/
def candidate (b : Body) : Option Rule :=
  if !b.eligible then none
  else if b.conflict then some .refinement
  else if !b.summary && !b.row then some .scope
  else none

/-- The declared-entry check established the declaration under rule `r`. -/
def established (b : Body) (r : Rule) : Bool :=
  candidate b == some r && b.entryHolds

/-- The failures the unit records for the declaration. -/
def failed (b : Body) : Bool :=
  b.refuted || b.modeWithdrawn || b.specificationWithdrawn ||
    (b.conflict && !established b .refinement)

/-- The evidence the unit records for the declaration (`unit_evidence`). -/
def evidence (b : Body) : List Evidence :=
  if b.conflict then (if established b .refinement then [.r13n] else [])
  else if b.summary then [.w4]
  else if established b .scope then [.r13m]
  else []

/-- The unit's outcome for the declaration. -/
def outcome (b : Body) : Outcome :=
  unitOutcome (!(evidence b).isEmpty) (failed b)

/-- The previous card's recording, before rules 13m and 13n. -/
def previousFailed (b : Body) : Bool :=
  b.refuted || b.modeWithdrawn || b.specificationWithdrawn || b.conflict

def previousEvidence (b : Body) : List Evidence :=
  if b.conflict then [] else if b.summary then [.w4] else []

/-! ## The new evidence needs the check -/

theorem new_evidence_needs_the_entry_check (b : Body) (e : Evidence)
    (he : e ∈ evidence b) (new : e = .r13m ∨ e = .r13n) : b.entryHolds = true := by
  unfold evidence established at he
  cases hc : b.conflict <;> cases hs : b.summary <;> cases hh : b.entryHolds <;>
    simp_all

theorem refinement_evidence_needs_a_conflict (b : Body) (he : Evidence.r13n ∈ evidence b) :
    b.conflict = true ∧ b.eligible = true := by
  unfold evidence established candidate at he
  cases hc : b.conflict <;> cases hs : b.summary <;> cases hl : b.eligible <;>
    cases hr : b.row <;> simp_all

theorem scope_evidence_needs_an_unreported_body_without_a_summary (b : Body)
    (he : Evidence.r13m ∈ evidence b) :
    b.conflict = false ∧ b.summary = false ∧ b.row = false ∧ b.eligible = true := by
  unfold evidence established candidate at he
  cases hc : b.conflict <;> cases hs : b.summary <;> cases hl : b.eligible <;>
    cases hr : b.row <;> simp_all

/-! ## Who is never checked -/

theorem a_container_or_counted_user_declaration_is_never_checked (b : Body)
    (h : b.ownProjection = false) : candidate b = none := by
  simp [candidate, Body.eligible, h]

theorem a_withdrawn_or_refuted_declaration_is_never_checked (b : Body)
    (h : b.refuted = true ∨ b.modeWithdrawn = true ∨ b.specificationWithdrawn = true) :
    candidate b = none := by
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

/-! ## Coverage -/

theorem the_new_evidence_covers_only_custody (k : ClauseKind) :
    covers .r13m k = k.custody ∧ covers .r13n k = k.custody ∧
      covers .r13m k = covers .entry k := by
  simp [covers]

theorem the_new_evidence_never_covers_a_text_clause :
    covers .r13m .premise = false ∧ covers .r13n .premise = false ∧
      covers .r13m .variadicTail = false ∧ covers .r13n .variadicTail = false ∧
      covers .r13m .noReturn = false ∧ covers .r13n .noReturn = false ∧
      covers .r13m .countedUser = false ∧ covers .r13n .container = false := by
  decide

/-! ## Shared vectors

Bound to the Rust controls of `crates/qcore-c-gateway/tests/declared_entry_check.rs`:
(conflict, summary, row, eligible, entryHolds, the evidence spelling or
"failed" or "none"). -/

def bodyOf (conflict summary row eligible entryHolds : Bool) : Body :=
  ⟨true, eligible, false, false, false, conflict, summary, row, entryHolds⟩

def Body.spelling (b : Body) : String :=
  match outcome b, evidence b with
  | .failed, _ => "failed"
  | _, [e] => e.spelling
  | _, _ => "none"

def recordingVectors : List (Bool × Bool × Bool × Bool × Bool × String) := [
  -- a nullable release wrapper: W4 conflicts, the declaration's entry holds
  (true, true, false, true, true, "13n"),
  -- a nullable formal handed to a non-null release: the check fails
  (true, true, false, true, false, "failed"),
  -- an equal W4 summary
  (false, true, false, true, false, "W4"),
  -- a scope-only body with no summary whose declaration holds
  (false, false, false, true, true, "13m"),
  -- the same body reported as a row: the row route's `entry` decides
  (false, false, true, true, true, "none"),
  -- a scope-only body whose check fails
  (false, false, false, true, false, "none"),
  -- a counted-user declaration is never checked; its conflict stays a failure
  (true, true, false, false, true, "failed")
]

theorem recording_vectors_hold :
    recordingVectors.all (fun v =>
      (bodyOf v.1 v.2.1 v.2.2.1 v.2.2.2.1 v.2.2.2.2.1).spelling == v.2.2.2.2.2) = true := by
  decide

end QCore.DeclaredEntryCheck
