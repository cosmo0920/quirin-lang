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

import QCore.LegacyCAnalyzer
import QCore.TypedFieldAccess

/-!
# Borrowed pointer-field actuals and per-exit custody residuals

Two rules card O04 adds to the resource projection
(`crates/qcore-c-gateway/src/borrowed_actual.rs` and the exit ledger in
`crates/qcore-c-gateway/src/legacy_c.rs`).

**The admission.** C02 admitted direct *scalar* field accesses as
custody-neutral no-ops and refused every pointer member. O04 admits a pointer
member too, but only as a *read*, only when it is the whole of an argument the
contract in force declares `Borrowed`, and only under every premise C02 already
required plus the position premise. Refusing an actual leaves it in the
statement's remainder, which the lexical gate still sees.

**The residual ledger.** The function-wide verdict is a join over every path:
one refused construct anywhere makes the whole function indeterminate and says
nothing about the paths that *are* modeled. The per-exit ledger records the
finer statement, and is fail-closed in both directions — an exit whose path
carried anything unmodeled claims neither a discharge nor a residual.

This is a model of the *decisions*, not of C semantics. The declared types,
record facts, parameter modes and path flags are premises supplied by the
reader and the transfer function; nothing here proves that either extracts them
faithfully, and nothing here is a claim about arbitrary C memory safety.
-/

namespace QCore.ExitResiduals

open QCore.LegacyCAnalyzer
open QCore.TypedFieldAccess

/-! ## The borrowed-actual admission -/

/-- The declared mode of the parameter an actual occupies. -/
inductive ParameterMode where
  | borrowed
  /-- Every mode that can change the argument's custody: owned, consumed,
      released, stored, removed, out. -/
  | owning
  deriving DecidableEq

/-- Where an actual sits with respect to the contract in force. -/
inductive Position where
  | declared (mode : ParameterMode)
  /-- Beyond the declared parameter vector: a variadic tail. -/
  | beyondVector
  /-- The callee has no contract at all. -/
  | uncontracted
  deriving DecidableEq

/-- One candidate actual: an access, where it sits, and whether it is the whole
    argument rather than a sub-expression of one. -/
structure Actual where
  access : Access
  position : Position
  wholeArgument : Bool
  deriving DecidableEq

/-- The whitelist. Every clause is a conjunct, so failing any one refuses.

    It differs from `QCore.TypedFieldAccess.admits` in exactly two places: a
    pointer member is admitted as well as a scalar one, and the actual must
    occupy a declared `Borrowed` position of the contract in force. Every other
    premise — declarations read, a `struct` tag with a unique complete
    definition, an unqualified member, a matching base the projection never
    binds, and an access that does not continue — is carried over unchanged. -/
def admitsActual (a : Actual) : Bool :=
  a.wholeArgument
    && a.position == Position.declared ParameterMode.borrowed
    && a.access.declarationsRead
    && a.access.record.kind == TagKind.recordTag
    && a.access.record.complete
    && a.access.record.unique
    && (a.access.member == MemberType.scalar || a.access.member == MemberType.pointer)
    && !a.access.qualified
    && a.access.baseMatches
    && !a.access.baseBound
    && !a.access.chained

/-- What the importer does with the whole call statement. -/
inductive Classification where
  /-- The lexical gate rejects it; the importer emits `Instruction.unsupported`. -/
  | refused
  /-- The gate passes and the contract in force governs the call. -/
  | modeledCall
  deriving DecidableEq

/-- The gate is run over the tokens the whitelist did *not* admit, so every
    refused actual is still in its view, and so is any other indirect access the
    statement carries. -/
def classifyStatement (actuals : List Actual) (otherIndirectAccess : Bool) :
    Classification :=
  if otherIndirectAccess || actuals.any (fun a => !admitsActual a) then
    .refused
  else
    .modeledCall

/-- The instruction a refused statement contributes. -/
def statementEffect (actuals : List Actual) (otherIndirectAccess : Bool) :
    Option Instruction :=
  match classifyStatement actuals otherIndirectAccess with
  | .refused => some .unsupported
  | .modeledCall => none

theorem a_refused_statement_is_unsupported (actuals : List Actual) (other : Bool)
    (refused : classifyStatement actuals other = .refused) :
    statementEffect actuals other = some .unsupported := by
  simp [statementEffect, refused]

theorem a_refused_statement_is_never_safe (actuals : List Actual) (other : Bool)
    (state : AbstractState)
    (_refused : classifyStatement actuals other = .refused) :
    classify (transfer state .unsupported) ≠ .safe :=
  unsupported_is_never_safe state

/-- Any remaining indirect access refuses the statement whatever the admitted
    actuals are: the admission removes only the spans it justified. -/
theorem a_remaining_indirect_access_still_refuses (actuals : List Actual) :
    classifyStatement actuals true = .refused := by
  simp [classifyStatement]

theorem an_unadmitted_actual_refuses_the_statement (actuals : List Actual)
    (other : Bool) (a : Actual) (member : a ∈ actuals)
    (refused : admitsActual a = false) :
    classifyStatement actuals other = .refused := by
  have present : actuals.any (fun a => !admitsActual a) = true :=
    List.any_eq_true.mpr ⟨a, member, by simp [refused]⟩
  simp [classifyStatement, present]

-- Each premise of the admission, refused on its own.

theorem a_partial_argument_never_admits (a : Actual)
    (partial_ : a.wholeArgument = false) : admitsActual a = false := by
  simp [admitsActual, partial_]

theorem a_non_borrowed_position_never_admits (a : Actual)
    (owning : a.position = Position.declared ParameterMode.owning) :
    admitsActual a = false := by
  simp [admitsActual, owning]

theorem an_actual_beyond_the_declared_vector_never_admits (a : Actual)
    (tail : a.position = Position.beyondVector) : admitsActual a = false := by
  simp [admitsActual, tail]

theorem an_uncontracted_callee_never_admits (a : Actual)
    (none_ : a.position = Position.uncontracted) : admitsActual a = false := by
  simp [admitsActual, none_]

theorem unread_declarations_never_admit_an_actual (a : Actual)
    (unread : a.access.declarationsRead = false) : admitsActual a = false := by
  simp [admitsActual, unread]

theorem a_union_never_admits_an_actual (a : Actual)
    (union : a.access.record.kind = TagKind.unionTag) : admitsActual a = false := by
  simp [admitsActual, union]

theorem an_incomplete_record_never_admits_an_actual (a : Actual)
    (incomplete : a.access.record.complete = false) : admitsActual a = false := by
  simp [admitsActual, incomplete]

theorem an_ambiguous_tag_never_admits_an_actual (a : Actual)
    (ambiguous : a.access.record.unique = false) : admitsActual a = false := by
  simp [admitsActual, ambiguous]

theorem an_aggregate_member_never_admits_an_actual (a : Actual)
    (aggregate : a.access.member = MemberType.aggregate) :
    admitsActual a = false := by
  simp [admitsActual, aggregate]

theorem a_qualified_member_never_admits_an_actual (a : Actual)
    (qualified : a.access.qualified = true) : admitsActual a = false := by
  simp [admitsActual, qualified]

theorem a_mismatched_base_never_admits_an_actual (a : Actual)
    (mismatch : a.access.baseMatches = false) : admitsActual a = false := by
  simp [admitsActual, mismatch]

/-- The premise C02 §2 had to add, carried over: a base the projection may bind
    can hold a released, escaped or reallocated identity, and a read through it
    is not something this rule may neutralize. -/
theorem a_bound_base_never_admits_an_actual (a : Actual)
    (bound : a.access.baseBound = true) : admitsActual a = false := by
  simp [admitsActual, bound]

theorem a_chained_access_never_admits_an_actual (a : Actual)
    (chained : a.access.chained = true) : admitsActual a = false := by
  simp [admitsActual, chained]

/-- Without this every refusal theorem above would hold vacuously. -/
theorem the_actual_whitelist_is_not_empty :
    ∃ a : Actual, admitsActual a = true := by
  refine ⟨{
    access := {
      record := { kind := TagKind.recordTag, complete := true, unique := true }
      member := MemberType.pointer
      qualified := false
      baseMatches := true
      baseBound := false
      baseResource := none
      chained := false
      escapes := false
      hasCall := true
      operandsScalar := false
      declarationsRead := true }
    position := Position.declared ParameterMode.borrowed
    wholeArgument := true }, ?_⟩
  decide

/-- And a *scalar* member is admitted at a borrowed position too, so the rule
    is a strict extension of the shapes C02 already handled. -/
theorem a_scalar_member_is_admitted_at_a_borrowed_position :
    ∃ a : Actual, a.access.member = MemberType.scalar ∧ admitsActual a = true := by
  refine ⟨{
    access := {
      record := { kind := TagKind.recordTag, complete := true, unique := true }
      member := MemberType.scalar
      qualified := false
      baseMatches := true
      baseBound := false
      baseResource := none
      chained := false
      escapes := false
      hasCall := true
      operandsScalar := false
      declarationsRead := true }
    position := Position.declared ParameterMode.borrowed
    wholeArgument := true }, rfl, ?_⟩
  decide

/-! ## The opaque field read

`dest = base -> field`, when the read is admitted, binds a value that carries
no identity this function tracks. In this ledger model that is exactly a
no-op: nothing is created, released or escaped, and no diagnostic is emitted.
What matters is the two directions below — it neither invents a discharge nor
hides one. -/

def opaqueValueEffect : Instruction := .noop

theorem an_admitted_field_read_changes_no_custody (state : AbstractState) :
    transfer state opaqueValueEffect = state :=
  noop_preserves_state state

/-- It cannot discharge a live identity: the leak is still reported. -/
theorem an_admitted_field_read_does_not_discharge_a_live_identity
    (resource : ResourceId) :
    analyze [.acquire resource, opaqueValueEffect] = .defect := by
  simp [analyze, run, transfer, opaqueValueEffect, initial, lookup, update, close,
    classify, hasSafetyIssue, isSafetyIssue]

/-- And it cannot manufacture one either: a discharged lifetime stays safe. -/
theorem an_admitted_field_read_preserves_a_discharged_lifetime
    (resource : ResourceId) :
    analyze [.acquire resource, opaqueValueEffect, .release resource] = .safe := by
  simp [analyze, run, transfer, opaqueValueEffect, initial, lookup, update, close,
    classify, hasSafetyIssue, leakFree]

/-! ## Per-exit custody residuals -/

/-- One path's settled state at one exit, as the residual ledger reads it.
    Identities the caller owns are already excluded by the reader. -/
structure ExitState where
  ledger : List (ResourceId × ResourceStatus)
  /-- Some instruction on a path reaching this exit was refused, reported an
      unresolved effect, or alleged a custody defect. -/
  unmodeled : Bool
  /-- Some identity's disposition joined to more than one value, or its alias
      set was widened. -/
  mixed : Bool
  deriving DecidableEq

inductive ExitCustody where
  | discharged
  | residual
  | unknown
  deriving DecidableEq

def anyLive (ledger : List (ResourceId × ResourceStatus)) : Bool :=
  ledger.any fun entry => entry.2 == ResourceStatus.live

theorem anyLive_append (a b : List (ResourceId × ResourceStatus)) :
    anyLive (a ++ b) = (anyLive a || anyLive b) := by
  simp [anyLive, List.any_append]

def exitCustody (e : ExitState) : ExitCustody :=
  if e.unmodeled || e.mixed then .unknown
  else if anyLive e.ledger then .residual
  else .discharged

theorem discharged_implies_every_identity_settled (e : ExitState)
    (settled : exitCustody e = .discharged) : anyLive e.ledger = false := by
  unfold exitCustody at settled
  split at settled
  · contradiction
  · split at settled
    · contradiction
    · simp_all

theorem discharged_implies_a_fully_modeled_path (e : ExitState)
    (settled : exitCustody e = .discharged) : e.unmodeled = false := by
  unfold exitCustody at settled
  split at settled
  · contradiction
  · cases flag : e.unmodeled
    · rfl
    · simp_all

theorem discharged_implies_no_joined_disposition (e : ExitState)
    (settled : exitCustody e = .discharged) : e.mixed = false := by
  unfold exitCustody at settled
  split at settled
  · contradiction
  · cases flag : e.mixed
    · rfl
    · simp_all

/-- The full contract of a `discharged` exit. -/
theorem discharged_exit_is_sound (e : ExitState)
    (settled : exitCustody e = .discharged) :
    e.unmodeled = false ∧ e.mixed = false ∧ anyLive e.ledger = false :=
  ⟨discharged_implies_a_fully_modeled_path e settled,
    discharged_implies_no_joined_disposition e settled,
    discharged_implies_every_identity_settled e settled⟩

theorem residual_implies_a_live_identity (e : ExitState)
    (retained : exitCustody e = .residual) : anyLive e.ledger = true := by
  unfold exitCustody at retained
  split at retained
  · contradiction
  · cases live : anyLive e.ledger
    · simp_all
    · rfl

theorem residual_implies_a_fully_modeled_path (e : ExitState)
    (retained : exitCustody e = .residual) : e.unmodeled = false := by
  unfold exitCustody at retained
  split at retained
  · contradiction
  · cases flag : e.unmodeled
    · rfl
    · simp_all

/-- Fail-closed, first direction: an unmodeled path claims no discharge. -/
theorem an_unmodeled_path_is_never_discharged (e : ExitState)
    (poisoned : e.unmodeled = true) : exitCustody e ≠ .discharged := by
  simp [exitCustody, poisoned]

/-- Fail-closed, second direction, and the one that matters most: an unmodeled
    path alleges no residual either. A refused construct is a reason to say
    nothing, never a reason to accuse. -/
theorem an_unmodeled_path_alleges_no_residual (e : ExitState)
    (poisoned : e.unmodeled = true) : exitCustody e = .unknown := by
  simp [exitCustody, poisoned]

theorem a_joined_disposition_is_never_discharged (e : ExitState)
    (joined : e.mixed = true) : exitCustody e = .unknown := by
  simp [exitCustody, joined]

/-- Marking a path unmodeled is what the transfer function does when it emits a
    refusal or a custody diagnostic on that path. -/
def markUnmodeled (e : ExitState) : ExitState := { e with unmodeled := true }

theorem a_poisoned_path_is_unknown_at_every_exit (e : ExitState) :
    exitCustody (markUnmodeled e) = .unknown := by
  simp [exitCustody, markUnmodeled]

/-- Paths meet at an exit by disjunction on both flags and by concatenation of
    the ledgers, which is monotone: the work list still terminates. -/
def joinExit (a b : ExitState) : ExitState := {
  ledger := a.ledger ++ b.ledger
  unmodeled := a.unmodeled || b.unmodeled
  mixed := a.mixed || b.mixed }

theorem a_joined_exit_is_discharged_only_if_both_paths_are (a b : ExitState)
    (settled : exitCustody (joinExit a b) = .discharged) :
    exitCustody a = .discharged ∧ exitCustody b = .discharged := by
  obtain ⟨noUnmodeled, noMixed, noLive⟩ := discharged_exit_is_sound _ settled
  simp only [joinExit, Bool.or_eq_false_iff] at noUnmodeled noMixed
  rw [joinExit, anyLive_append, Bool.or_eq_false_iff] at noLive
  obtain ⟨aUnmodeled, bUnmodeled⟩ := noUnmodeled
  obtain ⟨aMixed, bMixed⟩ := noMixed
  obtain ⟨aLive, bLive⟩ := noLive
  refine ⟨?_, ?_⟩ <;>
    simp [exitCustody, aUnmodeled, bUnmodeled, aMixed, bMixed, aLive, bLive]

private theorem foldl_close_preserves_precision :
    ∀ (l : List (ResourceId × ResourceStatus)) (s : AbstractState),
      (l.foldl (fun current entry =>
        if entry.2 = ResourceStatus.live then
          { current with diagnostics := Diagnostic.leak entry.1 :: current.diagnostics }
        else current) s).precise = s.precise := by
  intro l
  induction l with
  | nil => intro s; rfl
  | cons entry rest ih =>
      intro s
      simp only [List.foldl_cons]
      rw [ih]
      split <;> rfl

theorem close_preserves_precision (state : AbstractState) :
    (close state).precise = state.precise :=
  foldl_close_preserves_precision state.ledger state

/-- The ledger is an additional observation, never an input to a verdict: an
    imprecise program stays indeterminate however many of its exits settle. -/
theorem a_discharged_exit_does_not_promote_an_imprecise_program
    (program : List Instruction) (e : ExitState)
    (imprecise : (run program).precise = false)
    (_settled : exitCustody e = .discharged) : analyze program ≠ .safe := by
  unfold analyze
  apply imprecise_is_never_safe
  rw [close_preserves_precision]
  exact imprecise

/-- Non-vacuity, for each of the three answers. -/
theorem every_exit_answer_is_reachable :
    (∃ e : ExitState, exitCustody e = .discharged)
      ∧ (∃ e : ExitState, exitCustody e = .residual)
      ∧ (∃ e : ExitState, exitCustody e = .unknown) := by
  refine ⟨⟨{ ledger := [(0, .released)], unmodeled := false, mixed := false }, by decide⟩,
    ⟨{ ledger := [(0, .live)], unmodeled := false, mixed := false }, by decide⟩,
    ⟨{ ledger := [], unmodeled := true, mixed := false }, by decide⟩⟩

end QCore.ExitResiduals
