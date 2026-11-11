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

/-!
Summary validation for the unrestricted-C gateway's interprocedural custody
inference (W4 of `docs/GENERALIZATION_PLAN.md`).

`crates/qcore-c-gateway/src/legacy_summary.rs` derives a candidate custody summary for an
internal C function and then *validates* it by re-running the intraprocedural
analyzer under the entry state the candidate prescribes. Only a validated
summary is trusted at call sites; anything else leaves the call uncontracted
and the unit `Unknown`.

This module models that validation relation and proves the properties the Rust
implementation relies on. The claims are all about the *checked* relation, not
about a flag: each theorem states what a validated summary necessarily implies
about every reachable exit of the body it summarizes.

The vocabulary covers the modes the pass infers: `borrowed`/`owned`/
`consumed`/`released`/`out`/`stored`/`removed` formals and neutral/owned/reallocated
results. Bounded non-null removal starts with the original child contained in
its distinct owned formal and requires that same child live and detached on
every exit. This relation does not extract general C heap links or aliases.
-/

namespace QCore.CustodySummary

/-- What became of one caller-owned resource at a particular exit. -/
inductive Disposition where
  /-- Still live: the caller keeps custody. -/
  | live
  /-- Destroyed by the callee. -/
  | released
  /-- Custody moved on through a trusted call. -/
  | escaped
  /-- Attached to an aggregate. -/
  | contained
  /-- Absent from the exit state: the argument was null on this path. -/
  | absent
  /-- Several dispositions joined; no single custody claim describes it. -/
  | mixed
  deriving DecidableEq, Repr

inductive ParameterMode where
  /-- No custody claim at all; nothing is bound at entry. -/
  | borrowed
  /-- The caller must hold a live resource across the call and still holds it. -/
  | owned
  /-- Custody moves to the callee. -/
  | consumed
  /-- The callee destroys the resource. -/
  | released
  /-- The callee initializes the slot this formal points at, and the caller
      receives whatever it finds there. -/
  | out (nullable : Bool)
  /-- The callee attaches this formal to the aggregate passed at `owner`;
      destroying the owner cascades to it. -/
  | stored (owner : Nat)
  /-- Entry containment in the given owner becomes direct custody at exit. -/
  | removed (owner : Nat)
  deriving DecidableEq, Repr

inductive ResultMode where
  /-- The result carries no custody. -/
  | neutral
  /-- The result is a fresh resource the caller must discharge. -/
  | owned (nullable : Bool)
  /-- On success the result replaces the formal at `parameter`; on failure the
      caller still holds the old one. Which of the two happened is decided by
      the caller's null test, so such a result is always nullable. -/
  | reallocated (parameter : Nat) (nullable : Bool)
  deriving DecidableEq, Repr

/-- The value one exit returns, as the exit state describes it. -/
inductive Returned where
  /-- No modeled resource place, including a null constant. -/
  | nothing
  /-- One modeled site. `fromParameter` marks a formal handed straight back. -/
  | site (fromParameter : Bool) (disposition : Disposition) (nullable : Bool)
  /-- More than one site, or an unknown one. -/
  | ambiguous
  deriving DecidableEq, Repr

/-- What an out slot holds at one exit. `unwritten` is deliberately distinct
    from `null`: a slot the callee never stored to leaves the caller's variable
    as it was, which no `out` mode describes. -/
inductive OutSlot where
  | unwritten
  | null
  | site (fromParameter : Bool) (disposition : Disposition) (nullable : Bool)
  | ambiguous
  deriving DecidableEq, Repr

structure ExitObservation where
  /-- Disposition of each formal; `none` where the contract bound nothing. -/
  parameters : List (Option Disposition)
  /-- What each formal's out slot holds; `none` where the contract declared
      no slot for that formal. -/
  outSlots : List (Option OutSlot)
  /-- For each formal, the formal whose aggregate holds it at this exit. -/
  containedIn : List (Option Nat)
  /-- For each formal, whether this exit left a replacement of it by the
      returned resource unresolved. -/
  pendingReplacement : List Bool
  returned : Returned
  /-- A resource that is none of the formals ended up inside an aggregate the
      caller holds. Such a transfer is invisible at the call site. -/
  foreignAttachment : Bool
  /-- The resources this exit hands back — through the result and through the
      out slots — are pairwise distinct sites. -/
  distinctHandback : Bool
  deriving DecidableEq, Repr

structure Summary where
  parameters : List ParameterMode
  result : ResultMode
  deriving DecidableEq, Repr

/-- One formal's declared mode against its observed exit disposition.

    `borrowed` and `out` bind nothing, so the observation carries no
    disposition for them; the accompanying requirement that the whole analysis
    be defect-free is what rules out a body that used an unbound formal as a
    resource. A nullable mode is not modeled here: the Rust check additionally
    accepts `absent` for it, which only ever *rejects* fewer bodies than the
    relation below. -/
def parameterMatches : ParameterMode → Option Disposition → Bool
  | .borrowed, Option.none => true
  | .out _, Option.none => true
  | .owned, Option.some .live => true
  | .removed _, Option.some .live => true
  | .consumed, Option.some .escaped => true
  | .released, Option.some .released => true
  | .stored _, Option.some .contained => true
  | _, _ => false

/-- `none` marks a formal no per-formal mode governs: the one a `reallocated`
    result names, whose disposition depends on which outcome the exit took.
    The result rule checks that formal instead. -/
def governedMatches : Option ParameterMode → Option Disposition → Bool
  | Option.none, _ => true
  | Option.some mode, disposition => parameterMatches mode disposition

def governedModes (summary : Summary) : List (Option ParameterMode) :=
  match summary.result with
  | .reallocated parameter _ =>
      summary.parameters.mapIdx fun index mode =>
        if index == parameter then Option.none else Option.some mode
  | _ => summary.parameters.map Option.some

/-- One formal's out slot against its declared mode.

    An `out` formal promises a fresh resource in the slot on return, so a slot
    left unwritten, holding an ambiguous value, holding a formal the caller
    already owns, or holding a value the callee never resolved to non-null all
    refute the summary. A null store is consistent only with a nullable mode.
    A formal the summary does not declare `out` has no slot at all. -/
def outSlotMatches : ParameterMode → Option OutSlot → Bool
  | .out _, Option.some (.site fromParameter disposition nullable) =>
      !fromParameter && disposition == Disposition.live && !nullable
  | .out nullable, Option.some OutSlot.null => nullable
  | .out _, _ => false
  | _, Option.none => true
  | _, Option.some _ => false

/-- One formal's containment against its declared mode. A `stored` formal has
    to sit in the aggregate the summary names, and every other formal has to
    sit in no caller-held aggregate at all. -/
def containmentMatches : ParameterMode → Option Nat → Bool
  | .stored owner, Option.some actual => owner == actual
  | .stored _, Option.none => false
  | _, Option.none => true
  | _, Option.some _ => false

/-- The shapes a reallocation's conditional custody can take at one exit. -/
inductive Reallocation where
  /-- Unresolved: the result may be null and the old resource is still live,
      pending the caller's null test. -/
  | pending
  /-- The callee resolved the success case: the old resource is gone and a
      definitely non-null fresh one is handed back. -/
  | replaced
  /-- The failure case: nothing is handed back and the caller keeps the old
      resource. -/
  | kept
  /-- No reallocation shape describes this exit. -/
  | none
  deriving DecidableEq, Repr

/-- Which shape one exit exhibits for the formal a `reallocated` result names,
    read off the same data the Rust classifier reads. -/
def reallocationExit (disposition : Option Disposition) (returned : Returned)
    (pending : Bool) : Reallocation :=
  match returned, pending with
  | .site false Disposition.escaped true, true =>
      if disposition == Option.some Disposition.live then .pending else .none
  | .site false Disposition.escaped false, false =>
      if disposition == Option.some Disposition.released then .replaced else .none
  | .nothing, false =>
      if disposition == Option.some Disposition.live then .kept else .none
  | _, _ => .none

def reallocationAt (observation : ExitObservation) (parameter : Nat) : Reallocation :=
  reallocationExit (observation.parameters[parameter]?).join observation.returned
    ((observation.pendingReplacement[parameter]?).getD false)

/-- The declared result mode against what one exit hands back. -/
def resultMatches (summary : Summary) (observation : ExitObservation) : Bool :=
  match summary.result with
  | .neutral => observation.returned == Returned.nothing
  | .owned nullable =>
      match observation.returned with
      -- Returning nothing is consistent only with a result the caller must
      -- null-check before treating as a resource.
      | .nothing => nullable
      | .ambiguous => false
      | .site fromParameter disposition siteNullable =>
          !fromParameter && disposition == Disposition.escaped && (nullable || !siteNullable)
  | .reallocated parameter nullable =>
      nullable && reallocationAt observation parameter != Reallocation.none

/-- Every condition one reachable exit has to satisfy. -/
def exitMatches (summary : Summary) (observation : ExitObservation) : Bool :=
  !observation.foreignAttachment
    && observation.distinctHandback
    && observation.parameters.length == summary.parameters.length
    && observation.outSlots.length == summary.parameters.length
    && observation.containedIn.length == summary.parameters.length
    && ((governedModes summary).zip observation.parameters).all
        (fun pair => governedMatches pair.1 pair.2)
    && (summary.parameters.zip observation.outSlots).all
        (fun pair => outSlotMatches pair.1 pair.2)
    && (summary.parameters.zip observation.containedIn).all
        (fun pair => containmentMatches pair.1 pair.2)
    && resultMatches summary observation

/-- The validation relation itself: a defect-free analysis, at least one
    reachable exit, and every exit agreeing with the summary. -/
def validate (summary : Summary) (analysisSafe : Bool) (exits : List ExitObservation) : Bool :=
  analysisSafe && !exits.isEmpty && exits.all (exitMatches summary)

/-- The pre-state required by each formal. Formal positions stand for the
    distinct stable sites allocated by Rust EntryCustody, not mutable names. -/
def entryDisposition : ParameterMode → Option Disposition
  | .borrowed | .out _ => none
  | .removed _ => some .contained
  | _ => some .live

def entryOwner : ParameterMode → Option Nat
  | .removed owner => some owner
  | _ => none

def validRemovedOwners (summary : Summary) : Bool :=
  (summary.parameters.mapIdx fun index mode => match mode with
    | .removed owner => index != owner && summary.parameters[owner]? == some .owned
    | _ => true).all id

def entryMatches (summary : Summary) (states : List (Option Disposition))
    (owners : List (Option Nat)) : Bool :=
  validRemovedOwners summary && states == summary.parameters.map entryDisposition &&
    owners == summary.parameters.map entryOwner

/-- Full entry/exit relation for removal; the earlier validate relation states
    the exit conditions alone. The entry is constructed independently before
    re-analysis, and cannot be substituted with a live uncontained child. -/
def validateFromEntry (summary : Summary) (states : List (Option Disposition))
    (owners : List (Option Nat)) (safe : Bool) (exits : List ExitObservation) : Bool :=
  entryMatches summary states owners && validate summary safe exits

theorem validated_from_entry_has_original_custody (summary : Summary)
    (states : List (Option Disposition)) (owners : List (Option Nat))
    (safe : Bool) (exits : List ExitObservation)
    (valid : validateFromEntry summary states owners safe exits = true) :
    states = summary.parameters.map entryDisposition ∧
    owners = summary.parameters.map entryOwner := by
  simp only [validateFromEntry, entryMatches, Bool.and_eq_true, beq_iff_eq] at valid
  exact ⟨valid.1.1.2, valid.1.2⟩

/-- A body the analyzer could not clear never yields a summary. This is the
    "validated, not just inferred" discipline: an unsupported construct, an
    unresolved alias, or a leak all reach here as `analysisSafe = false`. -/
theorem unvalidated_analysis_never_validates (summary : Summary)
    (exits : List ExitObservation) : validate summary false exits = false := by
  simp [validate]

/-- A function with no reachable exit describes no call behavior. -/
theorem no_reachable_exit_never_validates (summary : Summary) :
    validate summary true [] = false := by
  simp [validate]

/-- Extraction lemma: a validated summary agrees with *every* reachable exit,
    not with some representative one. -/
theorem validated_summary_matches_every_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    ∀ observation ∈ exits, exitMatches summary observation = true := by
  intro observation member
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  exact List.all_eq_true.mp valid.2 observation member

theorem validated_summary_is_analyzed_safely (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    safe = true := by
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  exact valid.1.1

theorem validated_summary_has_a_reachable_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true) :
    exits.isEmpty = false := by
  unfold validate at valid
  simp only [Bool.and_eq_true] at valid
  have := valid.1.2
  cases isEmpty : exits.isEmpty <;> simp_all

/-- No validated summary hides a resource handed to a caller-owned aggregate
    that is none of the formals. Such a transfer is invisible at the call site,
    so it may never be summarized away. -/
theorem validated_summary_attaches_nothing_foreign (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    observation.foreignAttachment = false := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  cases flag : observation.foreignAttachment <;> simp_all

/-- Every resource a validated exit hands back is a distinct one: the same
    site returned twice, or returned and also stored through a slot, would
    mint a second custody for one resource. -/
theorem validated_summary_hands_back_distinct_resources (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    observation.distinctHandback = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  cases flag : observation.distinctHandback <;> simp_all

/-- Every reachable exit of a validated summary returns what the result mode
    claims. -/
theorem validated_summary_result_matches (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    resultMatches summary observation = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact matched.2

/-- Every formal's declared mode holds at every reachable exit. -/
theorem validated_summary_parameters_match (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (mode : Option ParameterMode) (disposition : Option Disposition)
    (paired : (mode, disposition) ∈ (governedModes summary).zip observation.parameters) :
    governedMatches mode disposition = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact List.all_eq_true.mp matched.1.1.1.2 (mode, disposition) paired

/-- Every formal's out slot agrees with its declared mode at every exit. -/
theorem validated_summary_out_slots_match (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (mode : ParameterMode) (slot : Option OutSlot)
    (paired : (mode, slot) ∈ summary.parameters.zip observation.outSlots) :
    outSlotMatches mode slot = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact List.all_eq_true.mp matched.1.1.2 (mode, slot) paired

/-- Every formal's containment agrees with its declared mode at every exit. -/
theorem validated_summary_containment_matches (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (mode : ParameterMode) (owner : Option Nat)
    (paired : (mode, owner) ∈ summary.parameters.zip observation.containedIn) :
    containmentMatches mode owner = true := by
  have matched := validated_summary_matches_every_exit summary safe exits valid observation member
  unfold exitMatches at matched
  simp only [Bool.and_eq_true] at matched
  exact List.all_eq_true.mp matched.1.2 (mode, owner) paired

/-- A neutral result claim forces every exit to return no resource. This is the
    exact hole LC01 reproduced: a wrapper that returns a fresh owned resource
    cannot be summarized as carrying no custody. -/
theorem neutral_result_returns_nothing (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (neutral : summary.result = ResultMode.neutral)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits) :
    observation.returned = Returned.nothing := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  unfold resultMatches at result
  rw [neutral] at result
  cases returned : observation.returned with
  | nothing => rfl
  | site fromParameter disposition nullable => rw [returned] at result; simp at result
  | ambiguous => rw [returned] at result; simp at result

/-- An owned result is never a formal handed straight back: that would mint a
    second custody for one resource. -/
theorem owned_result_never_returns_a_formal (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (nullable : Bool)
    (owned : summary.result = ResultMode.owned nullable)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Disposition) (siteNullable : Bool)
    (returned : observation.returned = Returned.site true disposition siteNullable) :
    False := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  unfold resultMatches at result
  rw [owned, returned] at result
  simp at result

/-- A non-null owned result forbids an exit that returns nothing, so a caller is
    never handed a resource that may not exist without being told. -/
theorem nonnull_owned_result_returns_a_resource (summary : Summary) (safe : Bool)
    (exits : List ExitObservation)
    (owned : summary.result = ResultMode.owned false)
    (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (returned : observation.returned = Returned.nothing) : False := by
  have result :=
    validated_summary_result_matches summary safe exits valid observation member
  unfold resultMatches at result
  rw [owned, returned] at result
  simp at result

/-- Declaring a formal released commits the callee to releasing it on *every*
    reachable exit; a path that leaves it live refutes the summary. -/
theorem released_formal_is_released_at_every_exit (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Option Disposition)
    (paired :
      (Option.some ParameterMode.released, disposition)
        ∈ (governedModes summary).zip observation.parameters) :
    disposition = Option.some Disposition.released := by
  have matched :=
    validated_summary_parameters_match summary safe exits valid observation member
      (Option.some ParameterMode.released) disposition paired
  cases disposition with
  | none => simp [governedMatches, parameterMatches] at matched
  | some observed => cases observed <;> simp [governedMatches, parameterMatches] at matched ⊢

/-- A borrowed formal binds nothing: the summary makes no custody claim about
    it, and the exit state carries no disposition for it. -/
theorem borrowed_formal_binds_nothing (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (disposition : Option Disposition)
    (paired :
      (Option.some ParameterMode.borrowed, disposition)
        ∈ (governedModes summary).zip observation.parameters) :
    disposition = Option.none := by
  have matched :=
    validated_summary_parameters_match summary safe exits valid observation member
      (Option.some ParameterMode.borrowed) disposition paired
  cases disposition with
  | none => rfl
  | some observed => cases observed <;> simp [governedMatches, parameterMatches] at matched

/-- A stored formal ends up inside the aggregate the summary names, at every
    exit. Nothing else about it is claimed, and nothing else is permitted. -/
theorem stored_formal_sits_in_its_declared_owner (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (owner : Nat) (actual : Option Nat)
    (paired :
      (ParameterMode.stored owner, actual) ∈ summary.parameters.zip observation.containedIn) :
    actual = Option.some owner := by
  have matched :=
    validated_summary_containment_matches summary safe exits valid observation member
      (ParameterMode.stored owner) actual paired
  cases actual with
  | none => simp [containmentMatches] at matched
  | some observed =>
      unfold containmentMatches at matched
      simp at matched
      simp [matched]

/-- A formal the summary does not declare `stored` is in no caller-held
    aggregate at any exit. A body that quietly attaches one yields no summary. -/
theorem unstored_formal_is_never_contained (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (mode : ParameterMode) (owner : Nat)
    (unstored : ∀ declared, mode ≠ ParameterMode.stored declared)
    (paired : (mode, Option.some owner) ∈ summary.parameters.zip observation.containedIn) :
    False := by
  have matched :=
    validated_summary_containment_matches summary safe exits valid observation member
      mode (Option.some owner) paired
  cases mode with
  | stored declared => exact unstored declared rfl
  | _ => simp [containmentMatches] at matched

/-- A non-null `out` formal never has a null slot: the caller is told it
    receives a resource, so every exit has to have produced one. -/
theorem nonnull_out_slot_is_never_null (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (paired :
      (ParameterMode.out false, Option.some OutSlot.null)
        ∈ summary.parameters.zip observation.outSlots) :
    False := by
  have matched :=
    validated_summary_out_slots_match summary safe exits valid observation member
      (ParameterMode.out false) (Option.some OutSlot.null) paired
  simp [outSlotMatches] at matched

/-- A slot the callee never stored to validates under no mode at all, nullable
    or not: the caller's variable keeps whatever it held, which no `out` mode
    describes. -/
theorem an_unwritten_slot_matches_no_mode (mode : ParameterMode) :
    outSlotMatches mode (Option.some OutSlot.unwritten) = false := by
  cases mode <;> simp [outSlotMatches]

/-- An out slot never holds a formal the caller already owns. -/
theorem out_slot_never_holds_a_formal (nullable : Bool) (disposition : Disposition)
    (siteNullable : Bool) :
    outSlotMatches (ParameterMode.out nullable)
      (Option.some (OutSlot.site true disposition siteNullable)) = false := by
  simp [outSlotMatches]

/-- A `reallocated` result is never claimed as non-null. Which of the two
    custody outcomes happened is decided by the caller's null test, so a
    summary that removed the test would remove the decision. -/
theorem nonnullable_reallocated_result_matches_no_exit (summary : Summary)
    (parameter : Nat) (observation : ExitObservation)
    (reallocated : summary.result = ResultMode.reallocated parameter false) :
    resultMatches summary observation = false := by
  unfold resultMatches
  rw [reallocated]
  simp

theorem nonnullable_reallocated_never_validates (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (parameter : Nat)
    (reallocated : summary.result = ResultMode.reallocated parameter false)
    (valid : validate summary safe exits = true) : False := by
  have nonempty := validated_summary_has_a_reachable_exit summary safe exits valid
  cases exits with
  | nil => simp at nonempty
  | cons observation rest =>
      have result :=
        validated_summary_result_matches summary safe (observation :: rest) valid observation
          (by simp)
      rw [nonnullable_reallocated_result_matches_no_exit summary parameter observation
        reallocated] at result
      simp at result

/-- The classic double-custody trap. A realloc wrapper that frees the old
    resource on its failure path leaves that formal *released* while returning
    nothing, and no reallocation shape describes such an exit. Claiming
    `reallocated` there would tell the caller it still owns a pointer the
    callee already freed. -/
theorem a_discarded_argument_is_no_reallocation :
    reallocationExit (Option.some Disposition.released) Returned.nothing false
      = Reallocation.none := by
  decide

/-- Nor is a formal handed straight back a reallocation. -/
theorem a_returned_formal_is_no_reallocation (disposition : Option Disposition)
    (returnedDisposition : Disposition) (nullable pending : Bool) :
    reallocationExit disposition (Returned.site true returnedDisposition nullable) pending
      = Reallocation.none := by
  cases pending <;> cases returnedDisposition <;> cases nullable <;>
    simp [reallocationExit]

/-- An exit with no formals and no aggregates: the shape LC01's wrapper has. -/
def plainExit (returned : Returned) : ExitObservation :=
  { parameters := [], outSlots := [], containedIn := [], pendingReplacement := [],
    returned, foreignAttachment := false, distinctHandback := true }

/-- An exit of a one-formal wrapper that declares no slot and no aggregate. -/
def oneFormalExit (disposition : Option Disposition) (returned : Returned)
    (pending : Bool) : ExitObservation :=
  { parameters := [disposition], outSlots := [Option.none], containedIn := [Option.none],
    pendingReplacement := [pending], returned, foreignAttachment := false,
    distinctHandback := true }

/-- LC01's counterexample, as observations: a body that returns a fresh owned
    resource cannot validate as a resource-neutral call. -/
theorem lc01_allocation_wrapper_is_not_neutral :
    validate { parameters := [], result := ResultMode.neutral }
      true
      [plainExit (Returned.site false Disposition.escaped false)] = false := by
  decide

/-- The same body does validate as an owned-return summary, which is what makes
    the caller's discarded result a reported leak. -/
theorem lc01_allocation_wrapper_is_an_owned_return :
    validate { parameters := [], result := ResultMode.owned false }
      true
      [plainExit (Returned.site false Disposition.escaped false)] = true := by
  decide

/-- A formal released on one path and left live on another matches no single
    parameter mode, so under a neutral result the wrapper yields no summary. -/
theorem conditionally_released_formal_validates_under_no_mode (mode : ParameterMode) :
    validate { parameters := [mode], result := ResultMode.neutral }
      true
      [oneFormalExit (Option.some Disposition.released) Returned.nothing false,
       oneFormalExit (Option.some Disposition.live) Returned.nothing false] = false := by
  cases mode <;> simp [validate, exitMatches, governedModes, governedMatches,
    parameterMatches, outSlotMatches, containmentMatches, resultMatches,
    oneFormalExit]

/-- A resource attached to a caller-supplied aggregate that is none of the
    formals refutes every summary, whatever the summary claims. -/
theorem foreign_attachment_validates_under_no_summary (summary : Summary) :
    validate summary true
      [{ plainExit Returned.nothing with foreignAttachment := true }] = false := by
  simp [validate, exitMatches, plainExit]

/-- The reallocation pass-through: one exit, the result may be null, the old
    resource is still live, and the replacement is unresolved. That is exactly
    what a `reallocated` result describes, and it validates. -/
theorem realloc_passthrough_validates_as_reallocated :
    validate { parameters := [ParameterMode.owned], result := ResultMode.reallocated 0 true }
      true
      [oneFormalExit (Option.some Disposition.live)
        (Returned.site false Disposition.escaped true) true] = true := by
  decide

/-- The trap, end to end: the wrapper that discards its argument on failure
    validates as no reallocation, nullable or not. Its caller is therefore
    never told it still owns the old pointer. -/
theorem discarding_realloc_wrapper_is_no_reallocation (nullable : Bool) :
    validate { parameters := [ParameterMode.owned], result := ResultMode.reallocated 0 nullable }
      true
      [oneFormalExit (Option.some Disposition.released)
        (Returned.site false Disposition.escaped false) false,
       oneFormalExit (Option.some Disposition.released) Returned.nothing false] = false := by
  cases nullable <;> decide

/-- Removal preserves the original child's identity: the governed formal is
    live at every reachable exit, not merely replaced by another allocation. -/
theorem removed_formal_is_live_after_validation (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (owner : Nat) (disposition : Option Disposition)
    (paired : (some (ParameterMode.removed owner), disposition) ∈
      (governedModes summary).zip observation.parameters) :
    disposition = some .live := by
  have matched := validated_summary_parameters_match summary safe exits valid observation member
    (some (ParameterMode.removed owner)) disposition paired
  cases disposition with
  | none => simp [governedMatches, parameterMatches] at matched
  | some current => cases current <;> simp_all [governedMatches, parameterMatches]

theorem removed_formal_is_detached_after_validation (summary : Summary) (safe : Bool)
    (exits : List ExitObservation) (valid : validate summary safe exits = true)
    (observation : ExitObservation) (member : observation ∈ exits)
    (owner : Nat) (actual : Option Nat)
    (paired : (ParameterMode.removed owner, actual) ∈ summary.parameters.zip observation.containedIn) :
    actual = none := by
  have matched := validated_summary_containment_matches summary safe exits valid observation member
    (ParameterMode.removed owner) actual paired
  cases actual with
  | none => rfl
  | some _ => simp [containmentMatches] at matched

def removalSummary : Summary := ⟨[.owned, .removed 0], .neutral⟩
def removalExit : ExitObservation :=
  ⟨[some .live, some .live], [none, none], [none, none], [false, false],
    .nothing, false, true⟩

theorem direct_removal_validates_from_contained_entry :
    validateFromEntry removalSummary [some .live, some .contained] [none, some 0]
      true [removalExit] = true := by decide

theorem removal_from_uncontained_entry_rejects :
    validateFromEntry removalSummary [some .live, some .live] [none, none]
      true [removalExit] = false := by decide

theorem removal_from_wrong_owner_rejects :
    validateFromEntry removalSummary [some .live, some .contained] [none, some 1]
      true [removalExit] = false := by decide

theorem removal_left_contained_at_an_exit_rejects :
    validate removalSummary true
      [removalExit, { removalExit with parameters := [some .live, some .contained], containedIn := [none, some 0] }] = false := by decide

/-- Reduced direct Out forwarding. Formal positions represent symbolic slots;
    sanctioned lists come from the enclosing candidate, and written pointers
    from the modeled instruction. The importer separately rejects shadowing,
    unmodeled mutation and slot exposure; this relation assumes those guards
    succeeded. An ordinary local alias is not sanctioned.
    The produced value is preserved exactly rather than replaced by a new claim. -/
def forwardOutSlot (original current : Nat) (sanctioned written : List Nat)
    (produced : OutSlot) : Option OutSlot :=
  if original = current ∧ original ∈ sanctioned ∧ original ∉ written then
    some produced
  else none

theorem forwarded_out_keeps_original_slot (original current : Nat)
    (sanctioned written : List Nat) (produced delivered : OutSlot)
    (forwarded : forwardOutSlot original current sanctioned written produced = some delivered) :
    original = current ∧ original ∈ sanctioned ∧ original ∉ written ∧ delivered = produced := by
  unfold forwardOutSlot at forwarded
  split at forwarded
  next identity =>
    simp only [Option.some.injEq] at forwarded
    exact ⟨identity.1, identity.2.1, identity.2.2, forwarded.symm⟩
  next => contradiction

theorem reassigned_out_pointer_cannot_forward (original current : Nat)
    (sanctioned written : List Nat) (produced : OutSlot) (changed : original ∈ written) :
    forwardOutSlot original current sanctioned written produced = none := by
  simp [forwardOutSlot, changed]

theorem unsanctioned_out_pointer_cannot_forward (original current : Nat)
    (sanctioned written : List Nat) (produced : OutSlot) (absent : original ∉ sanctioned) :
    forwardOutSlot original current sanctioned written produced = none := by
  simp [forwardOutSlot, absent]

theorem forwarded_nullable_site_does_not_discharge_nonnull_out :
    outSlotMatches (.out false)
      (forwardOutSlot 0 0 [0] [] (.site false .live true)) = false := by decide

theorem direct_fresh_forwarding_discharges_out :
    outSlotMatches (.out false)
      (forwardOutSlot 0 0 [0] [] (.site false .live false)) = true := by decide

end QCore.CustodySummary

namespace QCore.CustodySummary

/-- A reduced immutable output-pointer environment. Values identify original
caller slots, not resource allocations. Source type equality, dominating prefix
initialization, no shadowing and complete use classification are premises of the
Rust token normalizer; these theorems do not prove C parsing or alias extraction. -/
abbrev OutAliasBindings := Nat → Option Nat

def copyOutAlias (bindings : OutAliasBindings) (source alias : Nat) : Option OutAliasBindings :=
  if bindings alias = none then
    match bindings source with
    | none => none
    | some slot => some (fun name => if name = alias then some slot else bindings name)
  else none

theorem alias_copy_retains_exact_source_slot
    {before after : OutAliasBindings} {source alias slot : Nat}
    (sourceSlot : before source = some slot)
    (copied : copyOutAlias before source alias = some after) : after alias = some slot := by
  unfold copyOutAlias at copied
  split at copied
  next fresh =>
    simp only [sourceSlot] at copied
    cases copied
    simp
  next => contradiction

theorem alias_copy_preserves_every_other_binding
    {before after : OutAliasBindings} {source alias name : Nat}
    (copied : copyOutAlias before source alias = some after) (different : name ≠ alias) :
    after name = before name := by
  unfold copyOutAlias at copied
  split at copied
  next fresh =>
    cases value : before source with
    | none => simp [value] at copied
    | some slot =>
      simp only [value] at copied
      cases copied
      simp [different]
  next => contradiction

theorem alias_copy_cannot_overwrite_existing_binding
    (before : OutAliasBindings) (source alias slot : Nat) (bound : before alias = some slot) :
    copyOutAlias before source alias = none := by simp [copyOutAlias, bound]

theorem two_alias_copies_preserve_original_slot
    {before middle after : OutAliasBindings} {source first last slot : Nat}
    (sourceSlot : before source = some slot)
    (firstCopy : copyOutAlias before source first = some middle)
    (lastCopy : copyOutAlias middle first last = some after) : after last = some slot := by
  exact alias_copy_retains_exact_source_slot
    (alias_copy_retains_exact_source_slot sourceSlot firstCopy) lastCopy

def forwardAliasedOut (original : Nat) (bindings : OutAliasBindings) (alias : Nat)
    (sanctioned written : List Nat) (produced : OutSlot) : Option OutSlot := do
  let current ← bindings alias
  forwardOutSlot original current sanctioned written produced

theorem alias_forward_retains_original_sanction_and_unwritten_slot
    {original alias : Nat} {bindings : OutAliasBindings} {sanctioned written : List Nat}
    {produced delivered : OutSlot}
    (forwarded : forwardAliasedOut original bindings alias sanctioned written produced = some delivered) :
    bindings alias = some original ∧ original ∈ sanctioned ∧ original ∉ written ∧ delivered = produced := by
  unfold forwardAliasedOut at forwarded
  cases value : bindings alias with
  | none => simp [value] at forwarded
  | some current =>
    simp only [value] at forwarded
    have facts := forwarded_out_keeps_original_slot original current sanctioned written produced delivered forwarded
    exact ⟨by rw [facts.1], facts.2.1, facts.2.2.1, facts.2.2.2⟩

theorem unsanctioned_alias_cannot_create_output_authority
    (original alias : Nat) (bindings : OutAliasBindings) (sanctioned written : List Nat)
    (produced : OutSlot) (absent : original ∉ sanctioned) :
    forwardAliasedOut original bindings alias sanctioned written produced = none := by
  unfold forwardAliasedOut
  cases value : bindings alias <;> simp [forwardOutSlot, absent]

end QCore.CustodySummary
