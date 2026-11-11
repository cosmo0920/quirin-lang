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

import QCore.CustodySummary

/-! Nullable direct Removed custody. Nullness, resource dispositions and possible
containing owners are separate alternatives. Every non-null disposition must
be Contained, with exactly the requested owner; unions cannot silently collapse.
A null refinement removes only the refuted child's incoming containment edges.

The relations retain stable formal identities and a caller frame. Rust bit-set
and pointer/source correspondence, complete CFG exit coverage and provenance
remain explicit implementation boundaries; no general alias inference follows.
-/
namespace QCore.NullableRemoval
open QCore.CustodySummary (Disposition)
inductive Nullness where
  | null | nonnull | maybeNull | unknown
  deriving DecidableEq, Repr
inductive Presence where
  | absent | present
  deriving DecidableEq, Repr
structure State where
  owner : Nat
  child : Nat
  nullness : Nullness
  ownerState : Disposition
  childStates : List Disposition
  containers : List Nat
  callerFrame : List Nat
  /-- Concrete alias alternatives of the selected child pointer. -/
  targets : List Nat
  deriving DecidableEq, Repr

def identified (state : State) : Prop :=
  state.owner ≠ state.child ∧ state.owner ∉ state.callerFrame ∧
    state.child ∉ state.callerFrame ∧ state.callerFrame.Nodup

instance (state : State) : Decidable (identified state) := by
  unfold identified
  infer_instance

def everyContained (states : List Disposition) : Bool :=
  !states.isEmpty && states.all (fun disposition => disposition == .contained)

def detach (nullable : Bool) (before : State) : Option State :=
  if identified before ∧ before.ownerState = .live then
    match before.nullness with
    | .unknown => none
    | .null =>
      if nullable && before.childStates == [.absent] && before.containers == [] && before.targets == [] then
        some before else none
    | .nonnull | .maybeNull =>
      if (before.nullness == .nonnull || nullable) && everyContained before.childStates &&
          before.containers == [before.owner] && before.targets == [before.child] then
        some { before with childStates := [.live], containers := [] }
      else none
  else none

/-- Null refinement filters alternatives, not a live concrete resource. -/
def refineNull (before : State) (assumeNull : Bool) : Option State :=
  match before.nullness, assumeNull with
  | .unknown, _ => none
  | .null, true | .nonnull, false => some before
  | .null, false | .nonnull, true => none
  | .maybeNull, true => some { before with
      nullness := .null
      childStates := [.absent]
      containers := []
      targets := [] }
  | .maybeNull, false => some { before with nullness := .nonnull }

def permits : Nullness → Presence → Bool
  | .null, .absent | .nonnull, .present | .maybeNull, _ => true
  | _, _ => false

def childCustody (state : State) : Presence → List Disposition × List Nat
  | .absent => ([], [])
  | .present => (state.childStates, state.containers)

def nonnullUse (state : State) : Bool :=
  state.nullness == .nonnull && state.childStates == [.live] && state.containers == []

def liveIdentities (state : State) (presence : Presence) : List Nat :=
  state.callerFrame ++ [state.owner] ++
    if presence == .present && state.childStates.any (fun d => d == .live || d == .contained)
    then [state.child] else []

theorem checked_detach_retains_identity_and_owner
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after) :
    after.owner = before.owner ∧ after.child = before.child ∧
    after.callerFrame = before.callerFrame ∧ after.ownerState = .live ∧
    after.nullness = before.nullness := by
  unfold detach at checked
  split at checked
  next valid =>
    split at checked <;> try contradiction
    all_goals
      split at checked <;> try contradiction
      cases checked
      exact ⟨rfl, rfl, rfl, valid.2, rfl⟩
  next => contradiction

theorem checked_nonnul_detach_conditions
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (notNull : before.nullness ≠ .null) :
    everyContained before.childStates = true ∧ before.containers = [before.owner] ∧
      before.targets = [before.child] := by
  unfold detach at checked
  split at checked <;> try contradiction
  split at checked <;> try contradiction
  all_goals
    split at checked
    next valid =>
      simp only [Bool.and_eq_true, beq_iff_eq] at valid
      exact ⟨valid.1.1.2, valid.1.2, valid.2⟩
    next => contradiction

theorem checked_detach_requires_every_nonnul_alternative_contained
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (notNull : before.nullness ≠ .null) :
    before.childStates ≠ [] ∧ ∀ disposition ∈ before.childStates, disposition = .contained := by
  have contained := (checked_nonnul_detach_conditions checked notNull).1
  have nonempty : before.childStates ≠ [] := by
    intro empty
    simp [everyContained, empty] at contained
  refine ⟨nonempty, ?_⟩
  simp only [everyContained, Bool.and_eq_true] at contained
  intro disposition member
  simpa only [beq_iff_eq] using List.all_eq_true.mp contained.2 disposition member

theorem checked_detach_requires_unique_exact_owner
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (notNull : before.nullness ≠ .null) : before.containers = [before.owner] :=
  (checked_nonnul_detach_conditions checked notNull).2.1

theorem checked_detach_requires_one_concrete_child
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (notNull : before.nullness ≠ .null) : before.targets = [before.child] :=
  (checked_nonnul_detach_conditions checked notNull).2.2

theorem any_noncontained_alternative_prevents_detach
    (nullable : Bool) (before : State) (disposition : Disposition)
    (member : disposition ∈ before.childStates) (different : disposition ≠ .contained)
    (notNull : before.nullness ≠ .null) : detach nullable before = none := by
  cases result : detach nullable before with
  | none => rfl
  | some after =>
    exact False.elim (different
      ((checked_detach_requires_every_nonnul_alternative_contained result notNull).2 disposition member))

theorem alternate_owner_prevents_detach
    (nullable : Bool) (before : State) (other : Nat)
    (member : other ∈ before.containers) (different : other ≠ before.owner)
    (notNull : before.nullness ≠ .null) : detach nullable before = none := by
  cases result : detach nullable before with
  | none => rfl
  | some after =>
    have owners := checked_detach_requires_unique_exact_owner result notNull
    rw [owners] at member
    simp only [List.mem_singleton] at member
    exact False.elim (different member)

theorem checked_nonnul_detach_retains_live_child_outside_owner
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (notNull : before.nullness ≠ .null) :
    after.childStates = [.live] ∧ after.containers = [] := by
  unfold detach at checked
  split at checked <;> try contradiction
  split at checked <;> try contradiction
  all_goals
    split at checked <;> try contradiction
    cases checked
    exact ⟨rfl, rfl⟩

theorem checked_null_detach_has_no_child_custody
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (isNull : before.nullness = .null) :
    after = before ∧ after.childStates = [.absent] ∧ after.containers = [] := by
  unfold detach at checked
  split at checked <;> try contradiction
  simp only [isNull] at checked
  split at checked
  next valid =>
    cases checked
    simp only [Bool.and_eq_true, beq_iff_eq] at valid
    exact ⟨rfl, valid.1.1.2, valid.1.2⟩
  next => contradiction

theorem absent_alternative_never_carries_child_custody (state : State) :
    childCustody state .absent = ([], []) := rfl

theorem may_null_detach_does_not_authorize_unchecked_nonnull_use
    {before after : State} {nullable : Bool}
    (checked : detach nullable before = some after)
    (unresolved : before.nullness = .maybeNull) : nonnullUse after = false := by
  have same := (checked_detach_retains_identity_and_owner checked).2.2.2.2
  simp [nonnullUse, same, unresolved]

theorem null_refinement_removes_only_refuted_child_custody
    (before : State) (unresolved : before.nullness = .maybeNull) :
    refineNull before true = some { before with
      nullness := .null
      childStates := [.absent]
      containers := []
      targets := [] } := by
  simp [refineNull, unresolved]

theorem nonnull_refinement_preserves_child_custody
    (before : State) (unresolved : before.nullness = .maybeNull) :
    refineNull before false = some { before with nullness := .nonnull } := by
  simp [refineNull, unresolved]

theorem null_test_does_not_release_an_existing_nonnull_child
    (before : State) (nonnull : before.nullness = .nonnull) :
    refineNull before true = none := by simp [refineNull, nonnull]

theorem unknown_pointer_has_no_detachment_claim
    (before : State) (unknown : before.nullness = .unknown) (nullable : Bool) :
    detach nullable before = none := by simp [detach, unknown]

theorem checked_detachment_preserves_caller_live_identity_inventory
    {nullable : Bool} {before after : State} (checked : detach nullable before = some after)
    (presence : Presence) : liveIdentities after presence = liveIdentities before presence := by
  by_cases isNull : before.nullness = .null
  · rw [(checked_null_detach_has_no_child_custody checked isNull).1]
  · have source := checked_detach_requires_every_nonnul_alternative_contained checked isNull
    have post := checked_nonnul_detach_retains_live_child_outside_owner checked isNull
    have ids := checked_detach_retains_identity_and_owner checked
    have beforeLive : before.childStates.any (fun d => d == .live || d == .contained) = true := by
      cases shape : before.childStates with
      | nil => exact False.elim (source.1 shape)
      | cons head tail =>
        have contained := source.2 head (by simp [shape])
        simp [contained]
    simp [liveIdentities, ids.1, ids.2.1, ids.2.2.1, post.1, beforeLive]

theorem null_refinement_preserves_concrete_absent_inventory
    {before after : State} (unresolved : before.nullness = .maybeNull)
    (refined : refineNull before true = some after) :
    liveIdentities after .absent = liveIdentities before .absent ∧
    after.callerFrame = before.callerFrame := by
  rw [null_refinement_removes_only_refuted_child_custody before unresolved] at refined
  cases refined
  exact ⟨by simp [liveIdentities], rfl⟩

theorem nonnull_refinement_preserves_concrete_present_inventory
    {before after : State} (unresolved : before.nullness = .maybeNull)
    (refined : refineNull before false = some after) :
    liveIdentities after .present = liveIdentities before .present ∧
    childCustody after .present = childCustody before .present := by
  rw [nonnull_refinement_preserves_child_custody before unresolved] at refined
  cases refined
  exact ⟨rfl, rfl⟩

theorem ambiguous_child_targets_cannot_detach
    (before : State) (nullable : Bool) (notNull : before.nullness ≠ .null)
    (ambiguous : before.targets ≠ [before.child]) : detach nullable before = none := by
  cases result : detach nullable before with
  | none => rfl
  | some after => exact False.elim (ambiguous (checked_detach_requires_one_concrete_child result notNull))

/-- A selection guard shared by release, escape, storage and replacement.
This guards which identity may be strongly updated; each operation still has
its own custody/disposition requirements. It never updates every possible alias. -/
structure Selection where
  targets : List Nat
  mayNull : Bool
  unknown : Bool
  deriving DecidableEq, Repr

def selectIdentity (nullable : Bool) (pointer : Selection) : Option (Option Nat) :=
  if !pointer.unknown && (nullable || !pointer.mayNull) then
    match pointer.targets with
    | [] => if nullable then some none else none
    | [site] => some (some site)
    | _ => none
  else none

def strongUpdate (nullable : Bool) (pointer : Selection)
    (before : Nat → Disposition) (result : Disposition) : Option (Nat → Disposition) :=
  match selectIdentity nullable pointer with
  | none => none
  | some none => some before
  | some (some site) => some (fun other => if other = site then result else before other)

theorem selected_identity_has_exact_singleton_inventory
    {nullable : Bool} {pointer : Selection} {site : Nat}
    (selected : selectIdentity nullable pointer = some (some site)) :
    pointer.targets = [site] ∧ pointer.unknown = false ∧ (nullable = true ∨ pointer.mayNull = false) := by
  unfold selectIdentity at selected
  split at selected
  next known =>
    split at selected
    · split at selected <;> cases selected
    · cases selected
      simp only [Bool.and_eq_true, Bool.or_eq_true] at known
      refine ⟨by assumption, ?_, ?_⟩
      · cases shape : pointer.unknown <;> simp_all
      · cases nullable <;> cases shape : pointer.mayNull <;> simp_all
    · contradiction
  next => contradiction

theorem multiple_targets_never_receive_a_strong_update
    (nullable : Bool) (pointer : Selection) (first second : Nat) (rest : List Nat)
    (multiple : pointer.targets = first :: second :: rest)
    (before : Nat → Disposition) (result : Disposition) :
    strongUpdate nullable pointer before result = none := by
  simp [strongUpdate, selectIdentity, multiple]

theorem selected_strong_update_preserves_every_other_identity
    {nullable : Bool} {pointer : Selection} {site : Nat}
    {before after : Nat → Disposition} {result : Disposition}
    (selected : selectIdentity nullable pointer = some (some site))
    (updated : strongUpdate nullable pointer before result = some after)
    (other : Nat) (different : other ≠ site) : after other = before other := by
  simp only [strongUpdate, selected] at updated
  cases updated
  simp [different]

theorem nullable_null_selection_changes_no_custody
    {nullable : Bool} {pointer : Selection} {before after : Nat → Disposition}
    {result : Disposition}
    (selected : selectIdentity nullable pointer = some none)
    (updated : strongUpdate nullable pointer before result = some after) : after = before := by
  simp only [strongUpdate, selected, Option.some.injEq] at updated
  exact updated.symm

/-- A direct consuming/escaping/releasing effect requires definite live custody;
terminal or mixed alternatives cannot be overwritten with a new disposition. -/
def directEffectAllowed (states : List Disposition) : Bool :=
  !states.isEmpty && states.all (fun disposition => disposition == .live)

def cascadeAllowed (states : List Disposition) (containers : List Nat) (owner : Nat) : Bool :=
  everyContained states && containers == [owner]

theorem direct_effect_requires_every_alternative_live
    {states : List Disposition} (allowed : directEffectAllowed states = true)
    (disposition : Disposition) (member : disposition ∈ states) : disposition = .live := by
  simp only [directEffectAllowed, Bool.and_eq_true] at allowed
  simpa only [beq_iff_eq] using List.all_eq_true.mp allowed.2 disposition member

theorem terminal_alternative_prevents_direct_effect
    (states : List Disposition) (terminal : Disposition)
    (member : terminal ∈ states) (notLive : terminal ≠ .live) : directEffectAllowed states = false := by
  cases result : directEffectAllowed states with
  | false => rfl
  | true => exact False.elim (notLive (direct_effect_requires_every_alternative_live result terminal member))

theorem cascade_requires_every_alternative_contained_in_unique_owner
    {states : List Disposition} {containers : List Nat} {owner : Nat}
    (allowed : cascadeAllowed states containers owner = true) :
    containers = [owner] ∧ ∀ disposition ∈ states, disposition = .contained := by
  simp only [cascadeAllowed, everyContained, Bool.and_eq_true, beq_iff_eq] at allowed
  refine ⟨allowed.2, ?_⟩
  intro disposition member
  simpa only [beq_iff_eq] using List.all_eq_true.mp allowed.1.2 disposition member

theorem conditionally_contained_child_cannot_be_cascaded
    {states : List Disposition} (containers : List Nat) (owner : Nat)
    (live : Disposition.live ∈ states) : cascadeAllowed states containers owner = false := by
  cases result : cascadeAllowed states containers owner with
  | false => rfl
  | true =>
    have impossible := (cascade_requires_every_alternative_contained_in_unique_owner result).2 .live live
    cases impossible

/-- Recording an unresolved effect keeps abstract custody rather than erasing
possible obligations. Such an affected site is not a definite leak witness;
a distinct unrelated live site still is. This is uncertainty, not safety. -/
structure EffectState where
  custody : Nat → Disposition
  unresolved : List Nat

def markUnresolved (before : EffectState) (affected : List Nat) : EffectState :=
  { before with unresolved := affected ++ before.unresolved }

def definiteLeak (state : EffectState) (site : Nat) : Bool :=
  state.custody site == .live && !state.unresolved.contains site

theorem unresolved_effect_preserves_abstract_custody
    (before : EffectState) (affected : List Nat) (site : Nat) :
    (markUnresolved before affected).custody site = before.custody site := rfl

theorem affected_unresolved_site_is_not_a_definite_leak
    (before : EffectState) (affected : List Nat) (site : Nat) (member : site ∈ affected) :
    definiteLeak (markUnresolved before affected) site = false := by
  simp [definiteLeak, markUnresolved, List.contains_eq_mem, member]

theorem unrelated_definite_leak_is_retained
    (before : EffectState) (affected : List Nat) (site : Nat) (outside : site ∉ affected) :
    definiteLeak (markUnresolved before affected) site = definiteLeak before site := by
  simp [definiteLeak, markUnresolved, List.contains_eq_mem, outside]

/-- Every validated exit retains the original formal identities and owner.
The child may only be absent on the null alternative, or live and detached. -/
def exitMatches (before after : State) : Bool :=
  before.owner == after.owner && before.child == after.child &&
  before.callerFrame == after.callerFrame && after.ownerState == .live &&
  after.containers == [] &&
  match after.nullness with
  | .unknown => false
  | .null => permits before.nullness .absent && after.childStates == [.absent] && after.targets == []
  | .nonnull => permits before.nullness .present && after.childStates == [.live] && after.targets == [after.child]
  | .maybeNull => before.nullness == .maybeNull && after.childStates == [.live] && after.targets == [after.child]

def validate (nullable : Bool) (before : State) (analysisSafe : Bool) (exits : List State) : Bool :=
  analysisSafe && (detach nullable before).isSome && !exits.isEmpty && exits.all (exitMatches before)

theorem nullable_summary_checks_every_exit
    {nullable : Bool} {before : State} {safe : Bool} {exits : List State}
    (validated : validate nullable before safe exits = true)
    (after : State) (member : after ∈ exits) : exitMatches before after = true := by
  simp only [validate, Bool.and_eq_true] at validated
  exact List.all_eq_true.mp validated.2 after member

theorem nullable_exit_preserves_original_owner_child_and_frame
    {before after : State} (matched : exitMatches before after = true) :
    before.owner = after.owner ∧ before.child = after.child ∧
    before.callerFrame = after.callerFrame ∧ after.ownerState = .live ∧ after.containers = [] := by
  simp only [exitMatches, Bool.and_eq_true, beq_iff_eq] at matched
  exact ⟨matched.1.1.1.1.1, matched.1.1.1.1.2, matched.1.1.1.2, matched.1.1.2, matched.1.2⟩

theorem nullable_exit_cannot_hide_release_or_escape
    {before after : State} (matched : exitMatches before after = true) :
    after.childStates = [.live] ∨ after.childStates = [.absent] := by
  simp only [exitMatches, Bool.and_eq_true] at matched
  have shape := matched.2
  split at shape <;> try contradiction
  all_goals
    simp only [Bool.and_eq_true, beq_iff_eq] at shape
    first | exact Or.inl shape.1.2 | exact Or.inr shape.1.2

theorem unsupported_body_cannot_publish_nullable_summary
    (nullable : Bool) (before : State) (exits : List State) :
    validate nullable before false exits = false := by simp [validate]

theorem empty_exit_inventory_cannot_publish_nullable_summary
    (nullable : Bool) (before : State) (safe : Bool) :
    validate nullable before safe [] = false := by simp [validate]

/-- The affected inventory includes conservative contained descendants (or all
tracked sites for an unknown pointer). Computing that inventory is a separate
Rust correspondence premise. Marked sites stay Mixed for summary checking. -/
def summaryDisposition (state : EffectState) (site : Nat) : Disposition :=
  if state.unresolved.contains site then .mixed else state.custody site

def reacquireAtSite (before : EffectState) (site : Nat) : EffectState :=
  { before with custody := fun other => if other = site then .live else before.custody other }

theorem unresolved_effect_prevents_single_disposition_summary
    (before : EffectState) (affected : List Nat) (site : Nat) (member : site ∈ affected) :
    summaryDisposition (markUnresolved before affected) site = .mixed := by
  simp [summaryDisposition, markUnresolved, List.contains_eq_mem, member]

theorem reused_abstract_site_does_not_clear_unresolved_effect
    (before : EffectState) (site : Nat) (pending : site ∈ before.unresolved) :
    summaryDisposition (reacquireAtSite before site) site = .mixed := by
  simp [summaryDisposition, reacquireAtSite, List.contains_eq_mem, pending]

def exampleEntry : State :=
  ⟨1, 2, .maybeNull, .live, [.contained], [1], [3], [2]⟩
def exampleDetached : State := { exampleEntry with childStates := [.live], containers := [] }
def exampleNull : State := { exampleEntry with
  nullness := .null
  childStates := [.absent]
  containers := []
  targets := [] }

theorem unresolved_nullable_removal_validates_without_inventing_nonnullness :
    detach true exampleEntry = some exampleDetached ∧
    exampleDetached.nullness = .maybeNull ∧ nonnullUse exampleDetached = false := by decide

theorem nullable_null_branch_retains_owner_without_child :
    refineNull exampleEntry true = some exampleNull ∧
    detach true exampleNull = some exampleNull ∧
    liveIdentities exampleNull .absent = [3, 1] := by decide

theorem nonnull_test_allows_nonnullable_removal_of_original_child :
    refineNull exampleEntry false = some { exampleEntry with nullness := .nonnull } ∧
    detach false { exampleEntry with nullness := .nonnull } =
      some { exampleDetached with nullness := .nonnull } := by decide

theorem nullable_null_and_detached_exits_validate_together :
    validate true exampleEntry true [exampleNull, exampleDetached] = true := by decide

theorem nullable_released_child_exit_does_not_validate :
    validate true exampleEntry true
      [{ exampleDetached with childStates := [.released] }] = false := by decide

theorem repeated_removal_cannot_collapse_contained_and_live_alternatives :
    detach true { exampleEntry with childStates := [.contained, .live] } = none := by decide

theorem selected_owner_cannot_erase_an_alternative_containing_owner :
    detach true { exampleEntry with containers := [1, 4] } = none := by decide

theorem selected_child_cannot_remove_both_pointer_alternatives :
    detach true { exampleEntry with targets := [2, 4] } = none := by decide

/-- Direct-effect eligibility consumes the observation after unresolved-effect
classification. Even a retained LIVE bit cannot authorize an operation while
that site's previous effect remains unresolved. -/
def checkedDirectEffect (state : EffectState) (site : Nat) : Bool :=
  directEffectAllowed [summaryDisposition state site]

theorem unresolved_site_cannot_authorize_direct_live_effect
    (before : EffectState) (affected : List Nat) (site : Nat) (member : site ∈ affected) :
    checkedDirectEffect (markUnresolved before affected) site = false := by
  rw [checkedDirectEffect, unresolved_effect_prevents_single_disposition_summary before affected site member]
  decide

/-- Nullable old-argument selection permits the null case of realloc; this
states the old-custody effect only. Fresh result allocation is checked by the
separate reallocation/result relation. -/
theorem nullable_old_argument_can_select_no_existing_identity :
    selectIdentity true ⟨[], true, false⟩ = some none := by decide

theorem nonnullable_old_argument_cannot_select_null :
    selectIdentity false ⟨[], true, false⟩ = none := by decide

end QCore.NullableRemoval
