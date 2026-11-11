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

import Std

namespace QCore.Cycle18CallbackObligations

/-! Rule model for per-callback Cycle 18 obligation accounting.

`qcore_verification::cycle18_callback_obligations` measures the seven
obligations the bounded aggregate leaves unresolved, for each selected Monkey
callback and each of its imported source paths, and publishes one verdict per
callback. This module models the decision rule and the three source-bound
dischargers that rule consumes:

* the verdict rule: `composed` only with a joined evidence family, no
  blocking provisional-Unsafe scenario and every relevant obligation
  discharged;
* initialization reachability: a constant callee result refutes a guard that
  disagrees with it, and nothing else does;
* activation-flag dependence: a callback whose paths read the flags only
  through single-bit guards covering every combination selects a path for
  every flags value, and a flags-independent callback selects the same paths
  for every flags value;
* `cb_timeout` path composition: the abstract custody effect of its three
  source path shapes, including the refused close-failure and liveness cases.

Correspondence with the Rust implementation is maintained by review; it is
not an extraction theorem, and faithful C import remains a premise. -/

inductive Obligation where
  | initializationReachability
  | callerNullHandling
  | callerSizeBounds
  | allocationAuthorityProvenance
  | registrationLiveness
  | naturalActivationReachability
  | allPathExecutionCoverage
  deriving DecidableEq, Repr

inductive Status where
  | discharged
  | pending
  deriving DecidableEq, Repr

structure Relevant where
  obligation : Obligation
  paths : List Nat
  status : Status
  deriving DecidableEq, Repr

structure Row where
  familyRefused : Bool
  obligations : List Relevant
  blocking : List String
  deriving DecidableEq, Repr

/-- An obligation holds a callback open only when it governs some path. -/
def Relevant.holdsOpen (relevant : Relevant) : Bool :=
  !relevant.paths.isEmpty && relevant.status != .discharged

def openObligations (row : Row) : List Obligation :=
  (row.obligations.filter Relevant.holdsOpen).map Relevant.obligation

inductive Verdict where
  | composed
  | unknown (openObligations : List Obligation) (blocking : List String)
      (familyRefused : Bool)
  deriving DecidableEq, Repr

def verdict (row : Row) : Verdict :=
  if !row.familyRefused && (openObligations row).isEmpty && row.blocking.isEmpty then
    .composed
  else
    .unknown (openObligations row) row.blocking row.familyRefused

private theorem composed_facts {row : Row} (result : verdict row = .composed) :
    row.familyRefused = false ∧ openObligations row = [] ∧ row.blocking = [] := by
  unfold verdict at result
  split at result
  · rename_i accepted
    simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_iff] at accepted
    exact ⟨accepted.1.1, accepted.1.2, accepted.2⟩
  · contradiction

/-- A composed callback has every relevant obligation discharged. -/
theorem composed_discharges_every_relevant_obligation
    {row : Row} (result : verdict row = .composed) :
    ∀ relevant ∈ row.obligations, relevant.paths ≠ [] →
      relevant.status = .discharged := by
  intro relevant member nonempty
  have none := (composed_facts result).2.1
  unfold openObligations at none
  have filtered : row.obligations.filter Relevant.holdsOpen = [] :=
    List.map_eq_nil_iff.mp none
  have notOpen : relevant.holdsOpen = false := by
    cases h : relevant.holdsOpen
    · rfl
    · have present : relevant ∈ row.obligations.filter Relevant.holdsOpen :=
        List.mem_filter.mpr ⟨member, h⟩
      rw [filtered] at present
      simp at present
  unfold Relevant.holdsOpen at notOpen
  cases paths : relevant.paths with
  | nil => exact absurd paths nonempty
  | cons head tail =>
    rw [paths] at notOpen
    simpa using notOpen

/-- A composed callback has no blocking provisional-Unsafe scenario. -/
theorem composed_has_no_blocking_scenario
    {row : Row} (result : verdict row = .composed) : row.blocking = [] :=
  (composed_facts result).2.2

/-- A composed callback's evidence family joined. -/
theorem composed_has_joined_family
    {row : Row} (result : verdict row = .composed) : row.familyRefused = false :=
  (composed_facts result).1

/-- One open relevant obligation keeps the callback Unknown. -/
theorem open_relevant_obligation_is_unknown
    {row : Row} {relevant : Relevant} (member : relevant ∈ row.obligations)
    (nonempty : relevant.paths ≠ []) (pending : relevant.status = .pending) :
    verdict row ≠ .composed := by
  intro result
  have discharged :=
    composed_discharges_every_relevant_obligation result relevant member nonempty
  rw [pending] at discharged
  cases discharged

/-- One blocking scenario keeps the callback Unknown. -/
theorem blocking_scenario_is_unknown
    {row : Row} {scenario : String} (member : scenario ∈ row.blocking) :
    verdict row ≠ .composed := by
  intro result
  rw [composed_has_no_blocking_scenario result] at member
  cases member

def allDischarged : List Relevant :=
  [ ⟨.initializationReachability, [0], .discharged⟩,
    ⟨.callerNullHandling, [0], .discharged⟩,
    ⟨.registrationLiveness, [0], .discharged⟩,
    ⟨.naturalActivationReachability, [0], .discharged⟩,
    ⟨.allPathExecutionCoverage, [0], .discharged⟩ ]

/-- The Composed arm is reachable: the rule is not vacuously Unknown. -/
theorem fully_discharged_row_composes :
    verdict ⟨false, allDischarged, []⟩ = .composed := by
  decide

/-- An obligation with no relevant path does not hold a callback open. -/
theorem irrelevant_pending_obligation_does_not_block :
    verdict ⟨false, ⟨.callerSizeBounds, [], .pending⟩ :: allDischarged, []⟩ =
      .composed := by
  decide

/-- The measured after-state of `cb_wait_2_timeout` is Unknown: caller null
handling and registration liveness have no discharger, and its registration
context carries provisional-Unsafe scenarios. -/
theorem measured_wait_callback_is_unknown :
    verdict ⟨false,
      [ ⟨.initializationReachability, [0], .discharged⟩,
        ⟨.callerNullHandling, [0], .pending⟩,
        ⟨.registrationLiveness, [0], .pending⟩,
        ⟨.naturalActivationReachability, [0], .discharged⟩,
        ⟨.allPathExecutionCoverage, [0], .discharged⟩ ],
      ["base_creation_failure", "fired_allocation_failure",
        "timer_creation_failure"]⟩ =
      .unknown [.callerNullHandling, .registrationLiveness]
        ["base_creation_failure", "fired_allocation_failure",
          "timer_creation_failure"] false := by
  decide

/-! ### Initialization reachability -/

/-- Each callee path is either one constant return or something else. -/
def constantReturn (callee : List (Option Int)) : Option Int :=
  match callee.head? with
  | some (some value) => if callee.all (· == some value) then some value else none
  | _ => none

inductive Comparison where
  | ne
  | eq
  deriving DecidableEq, Repr

def Comparison.holds : Comparison → Int → Int → Bool
  | .ne, left, right => left != right
  | .eq, left, right => left == right

structure GuardedPath where
  comparison : Comparison
  compared : Int
  taken : Bool
  /-- The path returns before any allocation: the initialization-failure branch. -/
  failureBranch : Bool
  deriving DecidableEq, Repr

def GuardedPath.infeasibleUnder (path : GuardedPath) (result : Int) : Bool :=
  path.comparison.holds result path.compared != path.taken

def refutesFailure (callee : List (Option Int)) (paths : List GuardedPath) : Bool :=
  match constantReturn callee with
  | none => false
  | some value =>
      paths.any GuardedPath.failureBranch &&
        paths.all (fun path => !path.failureBranch || path.infeasibleUnder value) &&
        paths.any (fun path => !path.infeasibleUnder value)

private theorem constant_return_covers
    {callee : List (Option Int)} {value : Int}
    (constant : constantReturn callee = some value) :
    ∀ result, some result ∈ callee → result = value := by
  intro result member
  unfold constantReturn at constant
  cases head : callee.head? with
  | none => simp [head] at constant
  | some entry =>
    cases entry with
    | none => simp [head] at constant
    | some first =>
      simp only [head] at constant
      split at constant
      · rename_i all
        have same : first = value := Option.some.inj constant
        have covered := List.all_eq_true.mp all _ member
        simp only [beq_iff_eq, Option.some.injEq] at covered
        rw [covered, same]
      · contradiction

/-- Soundness of the discharge: whatever the callee returns on any of its
paths, every refuted initialization-failure path disagrees with its guard, so
no execution can take it. -/
theorem refuted_failure_branch_is_never_taken
    {callee : List (Option Int)} {paths : List GuardedPath}
    (refuted : refutesFailure callee paths = true) :
    ∀ result, some result ∈ callee →
      ∀ path ∈ paths, path.failureBranch = true →
        path.comparison.holds result path.compared ≠ path.taken := by
  intro result produced path member failure
  unfold refutesFailure at refuted
  split at refuted
  · contradiction
  · rename_i value constant
    simp only [Bool.and_eq_true] at refuted
    have all := List.all_eq_true.mp refuted.1.2 path member
    rw [failure] at all
    simp only [Bool.not_true, Bool.false_or] at all
    rw [constant_return_covers constant result produced]
    unfold GuardedPath.infeasibleUnder at all
    intro equal
    rw [equal] at all
    simp at all

/-- A callee with a second, different return value refutes nothing. -/
theorem varying_callee_does_not_refute (paths : List GuardedPath) :
    refutesFailure [some 0, some (-1)] paths = false := by
  simp [refutesFailure, constantReturn]

/-- A callee path that is not a constant return refutes nothing. -/
theorem opaque_callee_path_does_not_refute
    (rest : List (Option Int)) (paths : List GuardedPath) :
    refutesFailure (none :: rest) paths = false := by
  simp [refutesFailure, constantReturn]

def pinnedLoopCreate (compared : Int) : List GuardedPath :=
  [ ⟨.ne, compared, true, true⟩,
    ⟨.ne, compared, false, false⟩,
    ⟨.ne, compared, false, false⟩,
    ⟨.ne, compared, false, false⟩ ]

/-- The pinned shape: `_mk_event_init` returns 0 and loop creation tests
`!= 0`, so the failure branch is refuted. -/
theorem pinned_initialization_branch_is_refuted :
    refutesFailure [some 0] (pinnedLoopCreate 0) = true := by
  decide

/-- Comparing against a value the constant satisfies refutes nothing. -/
theorem satisfied_guard_does_not_refute :
    refutesFailure [some 0] (pinnedLoopCreate 1) = false := by
  decide

/-! ### Activation-flag dependence -/

/-- A path class records the tested bit positions and their taken outcomes. -/
abbrev PathClass := List (Nat × Bool)

def classMatches (flags : Nat) (pathClass : PathClass) : Bool :=
  pathClass.all fun (bit, taken) => flags.testBit bit == taken

def selected (classes : List PathClass) (flags : Nat) : List PathClass :=
  classes.filter (classMatches flags)

/-- A callback whose paths test no flag bit selects the same paths for every
flags value, natural or controlled. -/
theorem independent_selection_ignores_flags
    (classes : List PathClass) (untested : ∀ pathClass ∈ classes, pathClass = [])
    (first second : Nat) :
    selected classes first = selected classes second := by
  unfold selected
  apply List.filter_congr
  intro pathClass member
  simp [untested pathClass member, classMatches]

/-- `cb_event`'s four classes over bits 1 (`EV_READ`) and 2 (`EV_WRITE`). -/
def cbEventClasses : List PathClass :=
  [ [(1, true), (2, true)], [(1, true), (2, false)],
    [(1, false), (2, true)], [(1, false), (2, false)] ]

/-- Every flags value, including every natural activation, selects exactly
one `cb_event` path class. -/
theorem cb_event_classes_partition_every_flags_value (flags : Nat) :
    (selected cbEventClasses flags).length = 1 := by
  cases first : flags.testBit 1 <;> cases second : flags.testBit 2 <;>
    simp [selected, cbEventClasses, classMatches, first, second]

/-! ### `cb_timeout` path composition -/

inductive Endpoint where
  | live
  | closed
  | indeterminate
  deriving DecidableEq, Repr

structure Custody where
  endpoint : Endpoint
  eventLive : Bool
  deleted : Bool
  mapLive : Bool
  deriving DecidableEq, Repr

inductive Step where
  | send
  | effectFree
  | closeWrite (succeeds : Bool)
  | deleteSelf
  | freeSelf
  | dropMap
  deriving DecidableEq, Repr

/-- Abstract custody effect of one source-derived step inside a running
activation; `none` is a refusal. -/
def Step.apply : Step → Custody → Option Custody
  | .send, custody =>
      if custody.endpoint = .live ∧ custody.mapLive then some custody else none
  | .effectFree, custody => some custody
  | .closeWrite succeeds, custody =>
      if custody.endpoint = .live then
        some { custody with endpoint := if succeeds then .closed else .indeterminate }
      else none
  | .deleteSelf, custody =>
      if custody.eventLive ∧ ¬custody.deleted then some { custody with deleted := true }
      else none
  | .freeSelf, custody =>
      if custody.eventLive ∧ custody.deleted then some { custody with eventLive := false }
      else none
  | .dropMap, custody =>
      if custody.mapLive ∧ ¬custody.eventLive then some { custody with mapLive := false }
      else none

def run : List Step → Custody → Option Custody
  | [], custody => some custody
  | step :: rest, custody => (step.apply custody).bind (run rest)

/-- An activation needs a live, undeleted event and live map. -/
def activate (steps : List Step) (custody : Custody) : Option Custody :=
  if custody.eventLive ∧ ¬custody.deleted ∧ custody.mapLive then run steps custody
  else none

def registered : Custody := ⟨.live, true, false, true⟩

def released (custody : Custody) : Bool :=
  custody.endpoint == .closed && !custody.eventLive && !custody.mapLive

def teardownPath (withPerror : Bool) (closeSucceeds : Bool) : List Step :=
  [.send, .effectFree] ++ (if withPerror then [.effectFree] else []) ++
    [.closeWrite closeSucceeds, .deleteSelf, .freeSelf, .dropMap]

def successPath : List Step := [.send]

/-- Both teardown paths release the endpoint, event and map exactly when the
ignored close succeeds. -/
theorem teardown_paths_release_everything (withPerror : Bool) :
    ((activate (teardownPath withPerror true) registered).map released) = some true := by
  cases withPerror <;> decide

/-- The non-teardown path leaves the registered custody unchanged. -/
theorem success_path_preserves_registration :
    activate successPath registered = some registered := by
  decide

/-- A non-teardown activation followed by a teardown activation releases
everything: all three source paths compose on the close-success branch. -/
theorem success_then_teardown_releases_everything (withPerror : Bool) :
    ((activate successPath registered).bind (activate (teardownPath withPerror true))).map
      released = some true := by
  cases withPerror <;> decide

/-- No activation is admitted after a teardown. -/
theorem no_activation_after_teardown (withPerror : Bool) (steps : List Step) :
    ((activate (teardownPath withPerror true) registered).bind (activate steps)) = none := by
  cases withPerror <;> simp [activate, teardownPath, run, Step.apply, registered]

/-- A failed close leaves the endpoint indeterminate, so the close-failure
branch does not reach a released state. -/
theorem close_failure_branch_is_not_released (withPerror : Bool) :
    ((activate (teardownPath withPerror false) registered).map released) = some false := by
  cases withPerror <;> decide

/-- Only the non-teardown path before loop exit leaves custody live: release
needs a later teardown activation (the liveness gap). -/
theorem success_path_alone_is_not_released :
    ((activate successPath registered).map released) = some false := by
  decide

/-- Dropping the map before the event is freed is refused. -/
theorem reordered_teardown_is_refused :
    activate [.send, .closeWrite true, .dropMap, .deleteSelf, .freeSelf] registered = none := by
  decide

end QCore.Cycle18CallbackObligations
