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

namespace QCore.LegacyCAnalyzer

abbrev ResourceId := Nat

inductive ResourceStatus where
  | live
  | released
  | escaped
  deriving DecidableEq

inductive Instruction where
  | acquire (resource : ResourceId)
  | release (resource : ResourceId)
  | escape (resource : ResourceId)
  /-- A statement the importer models as carrying no custody effect. The
      importer decides which statements qualify; see `QCore.TypedFieldAccess`
      for the typed direct scalar-field classification that produces one. -/
  | noop
  | unsupported
  deriving DecidableEq

inductive Diagnostic where
  | leak (resource : ResourceId)
  | doubleRelease (resource : ResourceId)
  | releaseAfterEscape (resource : ResourceId)
  | unknown
  deriving DecidableEq

structure AbstractState where
  ledger : List (ResourceId × ResourceStatus)
  diagnostics : List Diagnostic
  precise : Bool
  deriving DecidableEq

def initial : AbstractState := {
  ledger := []
  diagnostics := []
  precise := true }

def lookup (resource : ResourceId) :
    List (ResourceId × ResourceStatus) → Option ResourceStatus
  | [] => none
  | (candidate, status) :: rest =>
      if candidate = resource then some status else lookup resource rest

def update (resource : ResourceId) (status : ResourceStatus) :
    List (ResourceId × ResourceStatus) → List (ResourceId × ResourceStatus)
  | [] => [(resource, status)]
  | (candidate, oldStatus) :: rest =>
      if candidate = resource then
        (candidate, status) :: rest
      else
        (candidate, oldStatus) :: update resource status rest

def transfer (state : AbstractState) : Instruction → AbstractState
  | .acquire resource =>
      match lookup resource state.ledger with
      | none => { state with ledger := update resource .live state.ledger }
      | some _ => {
          state with
          diagnostics := .unknown :: state.diagnostics
          precise := false }
  | .release resource =>
      match lookup resource state.ledger with
      | some .live => { state with ledger := update resource .released state.ledger }
      | some .released => {
          state with diagnostics := .doubleRelease resource :: state.diagnostics }
      | some .escaped => {
          state with diagnostics := .releaseAfterEscape resource :: state.diagnostics }
      | none => {
          state with
          diagnostics := .unknown :: state.diagnostics
          precise := false }
  | .escape resource =>
      match lookup resource state.ledger with
      | some .live => { state with ledger := update resource .escaped state.ledger }
      | some _ => {
          state with
          diagnostics := .unknown :: state.diagnostics
          precise := false }
      | none => {
          state with
          diagnostics := .unknown :: state.diagnostics
          precise := false }
  | .noop => state
  | .unsupported => {
      state with
      diagnostics := .unknown :: state.diagnostics
      precise := false }

def run (program : List Instruction) : AbstractState :=
  program.foldl transfer initial

def close (state : AbstractState) : AbstractState :=
  state.ledger.foldl (fun current entry =>
    if entry.2 = .live then
      { current with diagnostics := .leak entry.1 :: current.diagnostics }
    else current) state

def isSafetyIssue : Diagnostic → Bool
  | .leak _ | .doubleRelease _ | .releaseAfterEscape _ => true
  | .unknown => false

def leakFree (state : AbstractState) : Bool :=
  state.ledger.all fun entry => entry.2 != .live

def hasSafetyIssue (state : AbstractState) : Bool :=
  state.diagnostics.any isSafetyIssue

inductive Verdict where
  | safe
  | defect
  | indeterminate
  deriving DecidableEq

/-- Safety is reported only for a precise abstract state with no live resource.
    A safety diagnostic takes priority over an accompanying unknown. -/
def classify (state : AbstractState) : Verdict :=
  if hasSafetyIssue state then .defect
  else if state.precise && leakFree state then .safe
  else .indeterminate

def analyze (program : List Instruction) : Verdict :=
  classify (close (run program))

theorem safe_implies_no_exit_leak (state : AbstractState)
    (safe : classify state = .safe) : leakFree state = true := by
  unfold classify at safe
  split at safe
  · contradiction
  · split at safe
    · cases preciseEq : state.precise <;>
        cases leakEq : leakFree state <;> simp_all
    · contradiction

theorem safe_implies_no_safety_diagnostic (state : AbstractState)
    (safe : classify state = .safe) : hasSafetyIssue state = false := by
  unfold classify at safe
  split at safe <;> simp_all

theorem safe_implies_precise (state : AbstractState)
    (safe : classify state = .safe) : state.precise = true := by
  unfold classify at safe
  split at safe
  · contradiction
  · split at safe
    · cases preciseEq : state.precise <;>
        cases leakEq : leakFree state <;> simp_all
    · contradiction

/-- The formal no-false-negative contract for a `safe` report: analysis was
    precise, no resource may remain live, and no modeled safety defect exists. -/
theorem safe_report_is_sound (state : AbstractState)
    (safe : classify state = .safe) :
    state.precise = true ∧ leakFree state = true ∧ hasSafetyIssue state = false := by
  exact ⟨safe_implies_precise state safe, safe_implies_no_exit_leak state safe,
    safe_implies_no_safety_diagnostic state safe⟩

theorem safe_implies_no_double_release_diagnostic (state : AbstractState)
    (safe : classify state = .safe) :
    ¬ ∃ resource, .doubleRelease resource ∈ state.diagnostics := by
  intro doubleRelease
  have clear := safe_implies_no_safety_diagnostic state safe
  rcases doubleRelease with ⟨resource, member⟩
  have present : state.diagnostics.any isSafetyIssue = true :=
    List.any_eq_true.mpr ⟨.doubleRelease resource, member, by simp [isSafetyIssue]⟩
  simp [hasSafetyIssue, present] at clear

theorem safe_implies_no_release_after_escape (state : AbstractState)
    (safe : classify state = .safe) :
    ¬ ∃ resource, .releaseAfterEscape resource ∈ state.diagnostics := by
  intro releaseAfterEscape
  have clear := safe_implies_no_safety_diagnostic state safe
  rcases releaseAfterEscape with ⟨resource, member⟩
  have present : state.diagnostics.any isSafetyIssue = true :=
    List.any_eq_true.mpr ⟨.releaseAfterEscape resource, member, by simp [isSafetyIssue]⟩
  simp [hasSafetyIssue, present] at clear

theorem imprecise_is_never_safe (state : AbstractState)
    (imprecise : state.precise = false) : classify state ≠ .safe := by
  unfold classify
  split
  · simp
  · simp [imprecise]

theorem unsupported_becomes_unknown :
    classify (transfer initial .unsupported) = .indeterminate := by
  decide

theorem unsupported_is_never_safe (state : AbstractState) :
    classify (transfer state .unsupported) ≠ .safe := by
  apply imprecise_is_never_safe
  rfl

theorem seeded_normal_leak_is_unsafe (resource : ResourceId) :
    analyze [.acquire resource] = .defect := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

theorem seeded_double_release_is_unsafe (resource : ResourceId) :
    analyze [.acquire resource, .release resource, .release resource] = .defect := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

theorem acquired_then_released_is_safe (resource : ResourceId) :
    analyze [.acquire resource, .release resource] = .safe := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, leakFree]

/-- The importer maps unsupported reads to the existing unsupported effect.
    Releasing an allocation first cannot turn that missing access model into
    a neutral instruction. This theorem assumes that import decision; it is
    not a theorem about arbitrary C reads or lexical parsing. -/
theorem released_then_unmodeled_access_is_indeterminate (resource : ResourceId) :
    analyze [.acquire resource, .release resource, .unsupported] = .indeterminate := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

/-- A neutral instruction changes nothing: the ledger, the diagnostics and the
    precision flag all survive it unchanged. This is what makes classifying a
    statement as `noop` a *precision* decision rather than a custody one. -/
theorem noop_preserves_state (state : AbstractState) :
    transfer state .noop = state := by
  rfl

theorem noop_preserves_precision (state : AbstractState) :
    (transfer state .noop).precise = state.precise := by
  rfl

theorem noop_preserves_ledger (state : AbstractState) :
    (transfer state .noop).ledger = state.ledger := by
  rfl

theorem noop_preserves_diagnostics (state : AbstractState) :
    (transfer state .noop).diagnostics = state.diagnostics := by
  rfl

/-- A neutral instruction does not hide the leak the poisoned analysis could
    not see. Removing an unsupported effect can *surface* a defect. -/
theorem noop_does_not_hide_a_leak (resource : ResourceId) :
    analyze [.acquire resource, .noop] = .defect := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

theorem noop_does_not_hide_a_double_release (resource : ResourceId) :
    analyze [.acquire resource, .release resource, .noop, .release resource]
      = .defect := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

theorem noop_does_not_hide_a_release_after_escape (resource : ResourceId) :
    analyze [.acquire resource, .escape resource, .noop, .release resource]
      = .defect := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

/-- A discharged lifetime stays `safe` across a neutral instruction. This is
    the positive direction: the analysis completes instead of being poisoned. -/
theorem noop_between_acquire_and_release_is_safe (resource : ResourceId) :
    analyze [.acquire resource, .noop, .release resource] = .safe := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, leakFree]

/-- Classifying a statement neutral is the *only* difference from leaving it
    unsupported: the unsupported form of the same discharged program is
    indeterminate, so the change is a precision gain and not a weakened gate. -/
theorem unsupported_form_of_the_same_program_is_indeterminate
    (resource : ResourceId) :
    analyze [.acquire resource, .unsupported, .release resource]
      = .indeterminate := by
  simp [analyze, run, transfer, initial, lookup, update, close, classify,
    hasSafetyIssue, isSafetyIssue]

/-- The may-state used at CFG joins.  Each bit records a possible concrete
    state; join preserves every possibility from either predecessor. -/
structure MayStatus where
  live : Bool
  released : Bool
  escaped : Bool
  deriving DecidableEq

def MayStatus.join (left right : MayStatus) : MayStatus := {
  live := left.live || right.live
  released := left.released || right.released
  escaped := left.escaped || right.escaped }

theorem join_preserves_left_live (left right : MayStatus)
    (live : left.live = true) : (left.join right).live = true := by
  simp [MayStatus.join, live]

theorem join_preserves_right_live (left right : MayStatus)
    (live : right.live = true) : (left.join right).live = true := by
  simp [MayStatus.join, live]

structure AliasValue where
  targets : List ResourceId
  unknown : Bool
  deriving DecidableEq

def AliasValue.join (left right : AliasValue) : AliasValue := {
  targets := left.targets ++ right.targets
  unknown := left.unknown || right.unknown }

theorem alias_join_preserves_unknown_left (left right : AliasValue)
    (unknown : left.unknown = true) : (left.join right).unknown = true := by
  simp [AliasValue.join, unknown]

theorem alias_join_preserves_unknown_right (left right : AliasValue)
    (unknown : right.unknown = true) : (left.join right).unknown = true := by
  simp [AliasValue.join, unknown]

end QCore.LegacyCAnalyzer
