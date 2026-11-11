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

import QCore.ContractRevisionMode

/-!
# Tip identities of pinned contracts

Card **CONTRACT-PIN-TIP-MODE** (contract spec §C06.2, tip identities). A
pinned declaration's lock entry holds either **reviewed identities** (the
function's sites at named, reviewed revisions; `QCore.ContractRevisionMode`)
or **one tip identity**: a repository and a ref (a branch) whose current
commit the audit resolves. A tip entry is in force exactly where the contract
text is the pinned one and the sites read from the resolved commit's blobs are
the audited source's sites. An unresolvable tip — the repository not located,
the ref missing, a tip that does not descend from the reviewed commit, an
unreadable blob, or a tip with no site of the function — is never in force.

This is the model of `crates/qcore-audit/src/contract_tip.rs` (the decision
`tip_decision`, whose table `TIP_VECTORS` is bound to `tipVectors` below by
`contract_tip::tests::tip_lean_vectors_match`) and of its use in
`crates/qcore-audit/src/contract_revision.rs` (`pin_standing`).

## What is proved

* **A reviewed pin means what it meant**: an entry with no tip decides
  exactly as `QCore.ContractRevisionMode` decides it and carries no notice.
* **In force exactly at the tip's sites**: a tracked tip is in force iff the
  text matches and the tip resolved to the audited identity; an unresolved tip
  is never in force, and neither is a tip whose sites differ (a moved tip).
* **Held, never decided**: a verdict resting only on out-of-force tips is
  `Unknown` with reason `contractPinMismatch` (the run's held-call rule then
  makes it `Pending`), never `Safe` or `Unsafe`.
* **A tip decides as a reviewed pin at its sites**: replacing every tip entry
  by the reviewed pin whose only identity is what the tip resolved to leaves
  every verdict unchanged.
* **The notice decides nothing**: a tip entry's notice is `info` exactly when
  it is in force and `warning` otherwise; a reviewed pin has none; the free-text
  release label changes no verdict and no notice, and erasing every tip (so
  every notice) leaves every verdict as it was.
* **A named assumption**: every tip entry in force is listed among the run's
  tip assumptions, and no reviewed pin is.
* **The decision table**: `tipVectors` is `decision` on every input, and
  `decision` is the model's in-force flag and notice severity.

## What this is not

That the resolved commit is the ref's current commit, that its blobs are read
without touching a working tree, and that site digests are a faithful
identity are the scanner's and Git's boundary (`crates/qcore-audit/src/
c_identity.rs`, `contract_tip.rs`), pinned by
`crates/qcore-audit/tests/contract_tip_mode.rs`. A tip in force tracks a
revision no human may have reviewed; the model records that only as the
listed assumption, which is the point.
-/

namespace QCore.ContractTipMode

open QCore.ReasonCategory
open QCore.ContractRevisionMode

/-- How a tracked tip resolved on the auditing host: the sites read from the
blobs of the ref's current commit, or nothing. -/
inductive Resolution (I : Type) where
  | unresolved
  | resolved (sites : I)
  deriving DecidableEq, Repr

namespace Resolution

/-- The identities a resolution contributes to the pin evaluation. -/
def identities {I : Type} : Resolution I → List I
  | .unresolved => []
  | .resolved s => [s]

/-- Whether the tip resolved. -/
def isResolved {I : Type} : Resolution I → Bool
  | .unresolved => false
  | .resolved _ => true

end Resolution

/-- A notice's severity. -/
inductive Severity where
  | info
  | warning
  deriving DecidableEq, Repr

/-- A declaration with its lock entry: `tip = none` is a reviewed pin (its
`base.reviewed` identities decide); `tip = some r` tracks a tip that resolved
to `r`, and its entry records no reviewed identity, so `base.reviewed` is not
read. `label` is the entry's free-text release label. -/
structure TipDeclaration (O I : Type) where
  base : Declaration O I
  tip : Option (Resolution I)
  label : Option String

namespace TipDeclaration

/-- The declaration the pin evaluation decides on: a reviewed pin as it is; a
tip entry with the tip's sites as its only identity, or none. -/
def effective {O I : Type} (d : TipDeclaration O I) : Declaration O I :=
  match d.tip with
  | none => d.base
  | some r => { d.base with reviewed := r.identities }

/-- In force, as the pin evaluation decides it. -/
def inForce {O I : Type} [DecidableEq I] (d : TipDeclaration O I) : Bool :=
  d.effective.inForce

/-- The notice: none for a reviewed pin; for a tip entry, `info` exactly when
it is in force and `warning` otherwise. -/
def notice {O I : Type} [DecidableEq I] (d : TipDeclaration O I) : Option Severity :=
  match d.tip with
  | none => none
  | some _ => some (if d.inForce then .info else .warning)

/-- The same entry as a reviewed pin whose identities are what the tip
resolved to (no tip, so no notice). -/
def asReviewed {O I : Type} (d : TipDeclaration O I) : TipDeclaration O I :=
  { d with base := d.effective, tip := none }

end TipDeclaration

/-- The verdict of one obligation: the revision-mode verdict over the
declarations the pin evaluation decides on. -/
def verdictTip {O I : Type} [DecidableEq I] (env : List (TipDeclaration O I)) (o : O) :
    Verdict :=
  verdict (env.map TipDeclaration.effective) o

/-- The run's tip assumptions: every tip entry in force. -/
def tipAssumptions {O I : Type} [DecidableEq I] (env : List (TipDeclaration O I)) :
    List (TipDeclaration O I) :=
  env.filter (fun d => d.tip.isSome && d.inForce)

/-! ## A reviewed pin means what it meant -/

/-- The effective declaration has the entry's own mode. -/
theorem effective_mode {O I : Type} (d : TipDeclaration O I) :
    d.effective.mode = d.base.mode := by
  cases d with
  | mk base tip label => cases tip <;> rfl

/-- An entry with no tip is decided exactly as the revision-mode model
decides its declaration, and carries no notice. -/
theorem a_reviewed_pin_means_what_it_meant {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (reviewed : d.tip = none) :
    d.effective = d.base ∧ d.inForce = d.base.inForce ∧ d.notice = none := by
  simp [TipDeclaration.inForce, TipDeclaration.effective, TipDeclaration.notice, reviewed]

/-! ## In force exactly at the tip's sites -/

/-- **A tracked tip is in force iff the text matches and the tip resolved to
the audited identity.** -/
theorem a_tip_is_in_force_iff_its_sites_are_the_audited_ones {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (r : Resolution I) (tracked : d.tip = some r)
    (pinned : d.base.mode = .pinned) :
    d.inForce = true ↔ d.base.textMatches = true ∧ r = .resolved d.base.audited := by
  have mode : d.effective.mode = .pinned := by rw [effective_mode]; exact pinned
  have eff : d.effective = { d.base with reviewed := r.identities } := by
    simp [TipDeclaration.effective, tracked]
  unfold TipDeclaration.inForce
  rw [a_pin_is_in_force_iff_its_text_and_some_identity_match _ mode, eff]
  cases r with
  | unresolved => simp [Resolution.identities]
  | resolved s =>
    simp only [Resolution.identities, List.mem_singleton, Resolution.resolved.injEq]
    exact ⟨fun h => ⟨h.1, h.2.symm⟩, fun h => ⟨h.1, h.2.symm⟩⟩

/-- **An unresolved tip is never in force**: it applies nothing, it is a pin
mismatch, and its notice is a warning. -/
theorem an_unresolved_tip_is_never_in_force {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (tracked : d.tip = some .unresolved)
    (pinned : d.base.mode = .pinned) :
    d.inForce = false ∧ d.effective.applied = none ∧ d.effective.mismatched = true ∧
      d.notice = some .warning := by
  have off : d.inForce = false := by
    cases h : d.inForce
    · rfl
    · have := ((a_tip_is_in_force_iff_its_sites_are_the_audited_ones d _ tracked pinned).mp h).2
      cases this
  have mode : d.effective.mode = .pinned := by rw [effective_mode]; exact pinned
  have effOff : d.effective.inForce = false := off
  refine ⟨off, by simp [Declaration.applied, effOff], by simp [Declaration.mismatched, mode, effOff],
    by simp [TipDeclaration.notice, tracked, off]⟩

/-- **A moved tip whose sites differ from the audited ones is held**, whatever
the contract text. -/
theorem a_tip_whose_sites_differ_is_held {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (s : I) (tracked : d.tip = some (.resolved s))
    (differs : s ≠ d.base.audited) (pinned : d.base.mode = .pinned) :
    d.inForce = false ∧ d.effective.mismatched = true ∧ d.notice = some .warning := by
  have off : d.inForce = false := by
    cases h : d.inForce
    · rfl
    · have := ((a_tip_is_in_force_iff_its_sites_are_the_audited_ones d _ tracked pinned).mp h).2
      exact absurd (Resolution.resolved.inj this) differs
  have mode : d.effective.mode = .pinned := by rw [effective_mode]; exact pinned
  have effOff : d.effective.inForce = false := off
  exact ⟨off, by simp [Declaration.mismatched, mode, effOff],
    by simp [TipDeclaration.notice, tracked, off]⟩

/-- At the reviewed tip the entry is in force with an `info` notice. -/
theorem a_tip_at_the_audited_sites_is_in_force {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (tracked : d.tip = some (.resolved d.base.audited))
    (text : d.base.textMatches = true) (pinned : d.base.mode = .pinned) :
    d.inForce = true ∧ d.notice = some .info := by
  have on : d.inForce = true :=
    (a_tip_is_in_force_iff_its_sites_are_the_audited_ones d _ tracked pinned).mpr ⟨text, rfl⟩
  exact ⟨on, by simp [TipDeclaration.notice, tracked, on]⟩

/-! ## Held, never decided -/

/-- **A verdict resting only on out-of-force entries is `Unknown` with reason
`contractPinMismatch`**, tips included. -/
theorem a_verdict_resting_on_an_out_of_force_tip_is_unknown {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (o : O)
    (only : ∀ d ∈ env, d.effective.touches o = true → d.effective.mismatched = true)
    (held : ∃ d ∈ env, d.effective.mismatched = true ∧ d.effective.touches o = true) :
    verdictTip env o = .unknown (some .contractPinMismatch) := by
  unfold verdictTip
  apply a_verdict_resting_on_a_mismatched_pin_is_unknown
  · intro e member touched
    obtain ⟨d, dMember, same⟩ := List.mem_map.mp member
    subst same
    exact only d dMember touched
  · obtain ⟨d, dMember, mismatched, touched⟩ := held
    exact ⟨d.effective, List.mem_map.mpr ⟨d, dMember, rfl⟩, mismatched, touched⟩

/-- The corollary: neither `Safe` nor `Unsafe`. -/
theorem an_out_of_force_tip_dependency_is_neither_safe_nor_unsafe {O I : Type}
    [DecidableEq I] (env : List (TipDeclaration O I)) (o : O)
    (only : ∀ d ∈ env, d.effective.touches o = true → d.effective.mismatched = true)
    (held : ∃ d ∈ env, d.effective.mismatched = true ∧ d.effective.touches o = true) :
    verdictTip env o ≠ .safe ∧ verdictTip env o ≠ .provisionalUnsafe := by
  rw [a_verdict_resting_on_an_out_of_force_tip_is_unknown env o only held]
  exact ⟨by simp, by simp⟩

/-! ## A tip decides as a reviewed pin at its sites -/

/-- Replacing every entry by the reviewed pin at what it resolved to leaves
every verdict unchanged. -/
theorem a_tip_decides_as_a_reviewed_pin_at_its_sites {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (o : O) :
    verdictTip (env.map TipDeclaration.asReviewed) o = verdictTip env o := by
  unfold verdictTip
  rw [List.map_map]
  congr 1

/-! ## The notice decides nothing -/

/-- A tip entry's notice is `info` exactly when it is in force, and `warning`
exactly when it is not. -/
theorem the_notice_is_info_exactly_when_in_force {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (r : Resolution I) (tracked : d.tip = some r) :
    (d.notice = some .info ↔ d.inForce = true) ∧
      (d.notice = some .warning ↔ d.inForce = false) := by
  cases h : d.inForce <;> simp [TipDeclaration.notice, tracked, h]

/-- The free-text release label changes no verdict and no notice. -/
theorem the_release_label_decides_nothing {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (o : O) (relabel : TipDeclaration O I → Option String) :
    verdictTip (env.map fun d => { d with label := relabel d }) o = verdictTip env o ∧
      ∀ d : TipDeclaration O I, ({ d with label := relabel d } : TipDeclaration O I).notice =
        d.notice := by
  refine ⟨?_, fun d => rfl⟩
  unfold verdictTip
  rw [List.map_map]
  congr 1

/-- **Erasing the notices changes no verdict**: a tip entry carries a notice
and its reviewed-pin form carries none, yet every verdict is the same. -/
theorem the_notice_changes_no_verdict {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (o : O) (d : TipDeclaration O I) (r : Resolution I)
    (tracked : d.tip = some r) :
    d.notice ≠ none ∧ d.asReviewed.notice = none ∧
      verdictTip (env.map TipDeclaration.asReviewed) o = verdictTip env o := by
  refine ⟨by simp [TipDeclaration.notice, tracked], rfl,
    a_tip_decides_as_a_reviewed_pin_at_its_sites env o⟩

/-! ## A named assumption -/

/-- Every tip entry in force is listed among the run's tip assumptions. -/
theorem an_in_force_tip_is_a_listed_assumption {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (d : TipDeclaration O I) (member : d ∈ env)
    (r : Resolution I) (tracked : d.tip = some r) (on : d.inForce = true) :
    d ∈ tipAssumptions env := by
  simp [tipAssumptions, List.mem_filter, member, tracked, on]

/-- No reviewed pin is a tip assumption. -/
theorem a_reviewed_pin_is_never_a_tip_assumption {O I : Type} [DecidableEq I]
    (env : List (TipDeclaration O I)) (d : TipDeclaration O I) (reviewed : d.tip = none) :
    d ∉ tipAssumptions env := by
  simp [tipAssumptions, List.mem_filter, reviewed]

/-! ## The decision table -/

/-- The pin evaluation's decision for one tip entry (Rust `tip_decision`):
whether the contract text matches, whether the tip resolved, and whether its
sites equal the audited ones, to (in force, notice severity). -/
def decision (textMatches resolved sitesEqual : Bool) : Bool × Severity :=
  if textMatches && resolved && sitesEqual then (true, .info) else (false, .warning)

/-- `tipVectors` = `TIP_VECTORS`: `(text matches, resolved, sites equal, in
force, severity)`. -/
def tipVectors : List (Bool × Bool × Bool × Bool × Severity) := [
  (false, false, false, false, .warning),
  (false, false, true, false, .warning),
  (false, true, false, false, .warning),
  (false, true, true, false, .warning),
  (true, false, false, false, .warning),
  (true, false, true, false, .warning),
  (true, true, false, false, .warning),
  (true, true, true, true, .info)
]

theorem tip_vectors_hold :
    tipVectors.all (fun v => decision v.1 v.2.1 v.2.2.1 == (v.2.2.2.1, v.2.2.2.2)) = true := by
  decide

/-- The decision is the model's: for a tracked tip of a pinned declaration,
its in-force flag and its notice are `decision` of the text match, the
resolution and the site equality. -/
theorem the_decision_is_the_model {O I : Type} [DecidableEq I]
    (d : TipDeclaration O I) (r : Resolution I) (tracked : d.tip = some r)
    (pinned : d.base.mode = .pinned) :
    d.inForce =
        (decision d.base.textMatches r.isResolved (decide (r = .resolved d.base.audited))).1 ∧
      d.notice =
        some (decision d.base.textMatches r.isResolved
          (decide (r = .resolved d.base.audited))).2 := by
  have iff := a_tip_is_in_force_iff_its_sites_are_the_audited_ones d r tracked pinned
  have flag : d.inForce =
      (decision d.base.textMatches r.isResolved (decide (r = .resolved d.base.audited))).1 := by
    apply Bool.eq_iff_iff.mpr
    rw [iff]
    cases r with
    | unresolved => simp [decision, Resolution.isResolved]
    | resolved s =>
      by_cases e : s = d.base.audited <;> cases t : d.base.textMatches <;>
        simp [decision, Resolution.isResolved, e]
  have severity : ∀ a b c : Bool,
      (decision a b c).2 = if (decision a b c).1 then .info else .warning := by
    intro a b c
    unfold decision
    split <;> rfl
  refine ⟨flag, ?_⟩
  rw [severity, ← flag]
  simp [TipDeclaration.notice, tracked]

end QCore.ContractTipMode
