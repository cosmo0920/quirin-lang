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
# Bounded aggregate-lifecycle composition

The model of the decision relation card O05 implements in
`crates/qcore-core/src/aggregate_composition.rs`: when may one aggregate parent,
one wrapper lifecycle and their admitted callbacks, scoped tasks and guards be
published as a single whole-composition claim?

Three properties the relation decides, one per O05 acceptance item:

1. **Identity separation across parent release.** No two members share a logical
   identity `(kind, generation)`, and two members that share a raw OS-visible
   address must have disjoint live intervals. A member whose disposition is
   `retained` never stops being live, so nothing may reuse its address.
2. **Quiescence before teardown.** Every callback with declared authority over a
   member payload must be quiescent, every task capturing a member must be
   joined, and every guard over a footprint touching a member must be released.
3. **Separated durability, and no partial composition.** A durability premise is
   never release evidence, and any declared unsupported class refuses the whole
   claim.

**There is no violation verdict.** `Verdict` has two constructors. Every input is
a record another pass supplied — a per-exit residual ledger, an OS endpoint
post-state, a declared destructor contract, a callback inventory, a task ledger,
a guard ledger — so a contradictory record is a reason to say nothing, never a
reason to accuse. That is the same fail-closed direction
`QCore.ExitResiduals.an_unmodeled_path_alleges_no_residual` pins for one exit.

This is a model of the *composition decisions*, not of C semantics, of any
kernel, or of any codebase. Declared types, dispositions, callback authorities,
guard footprints, task states and the structural refusal list are all premises
supplied by the reader; nothing here proves that any reader extracts them
faithfully, and nothing here is a claim about arbitrary C memory safety or about
durability, which the relation carries and never discharges.

Two deliberate abstractions from the Rust relation, recorded so the
correspondence is not overstated:

* the structural (well-formedness) refusals are an **input field** here rather
  than a computed function, because every semantic theorem below is stated under
  the hypothesis that they are empty — exactly the early return the Rust relation
  takes; and
* premise sets are carried on the Rust certificate and are not modeled here,
  because the relation decides nothing about them.
-/

namespace QCore.AggregateComposition

/-! ## Vocabulary -/

inductive EndpointKind where
  | descriptor
  | mapping
  | directoryEntry
  deriving DecidableEq

inductive MemberKind where
  /-- The aggregate parent itself. -/
  | aggregate
  /-- A heap allocation. -/
  | allocation
  /-- An OS endpoint. -/
  | endpoint (kind : EndpointKind)
  deriving DecidableEq

/-- Which lifecycle a member belongs to. -/
inductive Owner where
  | aggregateOwner
  | wrapperOwner
  deriving DecidableEq

/-- A fact that looks like progress and is not custody evidence. -/
inductive StatusFact where
  | ringMembership
  | zeroUpCounter
  | successInteger
  | nullFieldAfterRelease
  deriving DecidableEq

/-- A synchronization or durability property. Never discharged here. -/
inductive DurabilityPremise where
  | syncSucceeded
  | dataReachedStableStorage
  | metadataReachedStableStorage
  | mappingFlushed
  | renameIsAtomic
  deriving DecidableEq

inductive UnresolvedReason where
  | suppressedSyscall
  | uncontractedCallee
  | unmodeledBranch
  | unknownBackend
  | unmodeledAggregateStore
  deriving DecidableEq

inductive Disposition where
  | released
  /-- Still live afterwards, in the hands of a custodian outside the aggregate.
      `none` is an unattributed residual. -/
  | retained (custodian : Option Nat)
  | unresolved (reason : UnresolvedReason)
  deriving DecidableEq

inductive Evidence where
  | exitResidualDischarged
  | observedEndpointPostState
  | declaredDestructorContract
  | statusFact (fact : StatusFact)
  | durability (premise : DurabilityPremise)
  deriving DecidableEq

inductive UnsupportedClass where
  | capacity
  | metadata
  | schedule
  | callback
  | backend
  | osCustody
  deriving DecidableEq

inductive CallbackState where
  | registered
  | running
  | quiescent
  deriving DecidableEq

inductive TaskState where
  | outstanding
  | completed
  | joined
  deriving DecidableEq

inductive GuardState where
  | held
  | releasedGuard
  deriving DecidableEq

inductive Releaser where
  | parent
  | wrapper
  deriving DecidableEq

inductive HoldMode where
  | owned
  | borrowed
  deriving DecidableEq

/-! ## Programs -/

structure Member where
  id : Nat
  kind : MemberKind
  generation : Nat
  owner : Owner
  parent : Option Nat
  address : Option Nat
  payloads : List Nat
  deriving DecidableEq

structure ReleaseRecord where
  member : Nat
  releaser : Releaser
  disposition : Disposition
  evidence : Evidence
  deriving DecidableEq

inductive Step where
  | acquire (member : Nat)
  | publish (publication : Nat)
  | reclaim (publication : Nat)
  | dropBorrow
  | release (record : ReleaseRecord)
  deriving DecidableEq

structure Callback where
  id : Nat
  state : CallbackState
  authority : List Nat
  admitted : Bool
  spawned : List Nat
  deriving DecidableEq

structure Task where
  id : Nat
  state : TaskState
  captures : List Nat
  deriving DecidableEq

structure Guard where
  id : Nat
  state : GuardState
  footprint : Nat
  deriving DecidableEq

structure Footprint where
  id : Nat
  members : List Nat
  deriving DecidableEq

structure Wrapper where
  held : Nat
  hold : HoldMode
  publications : List Nat
  deriving DecidableEq

/-- Every typed reason the composition claim can be refused. None alleges a
    violation. -/
inductive Refusal where
  | emptyAggregate
  | structuralDefect
  | unsupportedClass (cls : UnsupportedClass)
  | duplicateIdentity
  | memberNeverAcquired
  | memberAcquiredTwice
  | memberNotReleased
  | memberReleasedTwice
  | releaseBeforeAcquire
  | statusFactIsNotCustodyEvidence (fact : StatusFact)
  | durabilityAssumedAsDischarged (premise : DurabilityPremise)
  | unresolvedDisposition (reason : UnresolvedReason)
  | retainedEndpointWithoutCustodian
  | retainedEndpointCustodianNotExternal
  | retainedAllocation
  | addressReusedWhileLive
  | parentNotReleased
  | callbackNotQuiescent (state : CallbackState)
  | callbackSpawnedTaskUnaccounted
  | taskNotJoined (state : TaskState)
  | guardOutstanding
  | wrapperOverlapsAggregate
  | wrapperReleasesForeignMember
  | ownedHoldAlsoReleasedByParent
  | borrowedHoldReleasedByWrapper
  | borrowedHoldOutlivesParentRelease
  | wrapperPublicationUnreclaimed
  | wrapperPublicationSurvivesTeardown
  deriving DecidableEq

structure Program where
  root : Nat
  members : List Member
  schedule : List Step
  callbacks : List Callback
  tasks : List Task
  guards : List Guard
  footprints : List Footprint
  wrapper : Option Wrapper
  externalCustodians : List Nat
  unsupported : List UnsupportedClass
  /-- The structural refusals the reader reported. In the Rust relation these
      are computed and short-circuit every semantic check; here they are an
      input, and every semantic theorem is stated with this list empty. -/
  structural : List Refusal
  deriving DecidableEq

/-! ## Decidable projections

    The decisions below compare states by pattern match rather than by `==`, so
    every hypothesis in this file is a Boolean fact about the reader's record
    and no lawfulness of a derived equality is assumed. -/

def isEndpoint : MemberKind → Bool
  | .endpoint _ => true
  | _ => false

def isQuiescent : CallbackState → Bool
  | .quiescent => true
  | _ => false

def isJoined : TaskState → Bool
  | .joined => true
  | _ => false

def isReleasedGuard : GuardState → Bool
  | .releasedGuard => true
  | _ => false

def isAggregateOwned : Owner → Bool
  | .aggregateOwner => true
  | _ => false

def sameOwner : Owner → Owner → Bool
  | .aggregateOwner, .aggregateOwner => true
  | .wrapperOwner, .wrapperOwner => true
  | _, _ => false

def isParentReleaser : Releaser → Bool
  | .parent => true
  | _ => false

def isBorrowed : HoldMode → Bool
  | .borrowed => true
  | _ => false

def isReleasedDisposition : Disposition → Bool
  | .released => true
  | _ => false

/-! ## Schedule projections -/

def releaseRecords (p : Program) : List ReleaseRecord :=
  p.schedule.filterMap fun step =>
    match step with
    | .release record => some record
    | _ => none

def releasesOf (p : Program) (member : Nat) : List ReleaseRecord :=
  (releaseRecords p).filter fun record => record.member == member

def acquisitionsOf (p : Program) (member : Nat) : Nat :=
  (p.schedule.filter fun step => step == Step.acquire member).length

def acquireIndex (p : Program) (member : Nat) : Option Nat :=
  p.schedule.findIdx? fun step => step == Step.acquire member

def releaseIndex (p : Program) (member : Nat) : Option Nat :=
  p.schedule.findIdx? fun step =>
    match step with
    | .release record => record.member == member
    | _ => false

def dropBorrowIndex (p : Program) : Option Nat :=
  p.schedule.findIdx? fun step => step == Step.dropBorrow

def reclaimIndex (p : Program) (publication : Nat) : Option Nat :=
  p.schedule.findIdx? fun step => step == Step.reclaim publication

/-- The teardown point: the first release the parent performs. -/
def teardownPoint (p : Program) : Option Nat :=
  p.schedule.findIdx? fun step =>
    match step with
    | .release record => isParentReleaser record.releaser
    | _ => false

/-- A member's live interval end. `none` means it never stops being live: a
    retained or unresolved member outlives the schedule, so nothing may take its
    raw address. -/
def liveEnd (p : Program) (member : Nat) : Option Nat :=
  match (releasesOf p member).head? with
  | some record => if isReleasedDisposition record.disposition then releaseIndex p member else none
  | none => none

/-! ## Acceptance 3 — unsupported classes -/

def unsupportedChecks (p : Program) : List Refusal :=
  p.unsupported.map Refusal.unsupportedClass ++
    (if p.callbacks.any (fun callback => !callback.admitted) then
      [Refusal.unsupportedClass UnsupportedClass.callback]
    else [])

/-! ## Acceptance 1 — logical identity separation -/

def sameIdentity (a b : Member) : Bool :=
  a.kind == b.kind && a.generation == b.generation

def hasDuplicateIdentity : List Member → Bool
  | [] => false
  | member :: rest => rest.any (sameIdentity member) || hasDuplicateIdentity rest

def identityChecks (p : Program) : List Refusal :=
  if hasDuplicateIdentity p.members then [Refusal.duplicateIdentity] else []

/-! ## Acceptance 1 — the release ledger -/

def releaseChecks (p : Program) (member : Member) (record : ReleaseRecord) : List Refusal :=
  (match record.evidence with
    | .statusFact fact => [Refusal.statusFactIsNotCustodyEvidence fact]
    | .durability premise => [Refusal.durabilityAssumedAsDischarged premise]
    | _ => []) ++
  (match record.disposition with
    | .released => []
    | .unresolved reason => [Refusal.unresolvedDisposition reason]
    | .retained none =>
        (if isEndpoint member.kind then [] else [Refusal.retainedAllocation]) ++
          [Refusal.retainedEndpointWithoutCustodian]
    | .retained (some custodian) =>
        (if isEndpoint member.kind then [] else [Refusal.retainedAllocation]) ++
          (if p.externalCustodians.contains custodian then []
            else [Refusal.retainedEndpointCustodianNotExternal]))

def memberChecks (p : Program) (member : Member) : List Refusal :=
  (if acquisitionsOf p member.id == 0 then [Refusal.memberNeverAcquired] else []) ++
  (if 1 < acquisitionsOf p member.id then [Refusal.memberAcquiredTwice] else []) ++
  (if (releasesOf p member.id).length == 0 then [Refusal.memberNotReleased] else []) ++
  (if 1 < (releasesOf p member.id).length then [Refusal.memberReleasedTwice] else []) ++
  (match acquireIndex p member.id, releaseIndex p member.id with
    | some acquired, some released =>
        if released < acquired then [Refusal.releaseBeforeAcquire] else []
    | _, _ => []) ++
  (match (releasesOf p member.id).head? with
    | none => []
    | some record => releaseChecks p member record)

def scheduleChecks (p : Program) : List Refusal :=
  p.members.flatMap (memberChecks p) ++
    (if (releaseRecords p).any
        (fun record => record.member == p.root && isParentReleaser record.releaser)
      then [] else [Refusal.parentNotReleased])

/-! ## Acceptance 1 — raw address separation -/

def sharesAddress (a b : Member) : Bool :=
  match a.address, b.address with
  | some x, some y => x == y
  | _, _ => false

def disjointLive (p : Program) (a b : Member) : Bool :=
  match acquireIndex p a.id, acquireIndex p b.id with
  | some acquiredA, some acquiredB =>
      (match liveEnd p a.id with
        | some endA => decide (endA < acquiredB)
        | none => false) ||
      (match liveEnd p b.id with
        | some endB => decide (endB < acquiredA)
        | none => false)
  | _, _ => true

def memberPairs : List Member → List (Member × Member)
  | [] => []
  | member :: rest => rest.map (fun other => (member, other)) ++ memberPairs rest

def addressChecks (p : Program) : List Refusal :=
  (memberPairs p.members).filterMap fun pair =>
    if sharesAddress pair.1 pair.2 && !disjointLive p pair.1 pair.2 then
      some Refusal.addressReusedWhileLive
    else none

/-! ## Acceptance 2 — quiescence prerequisites -/

def memberPayloads (p : Program) : List Nat :=
  p.members.flatMap (fun member => member.payloads)

def knownMember (p : Program) (id : Nat) : Bool :=
  p.members.any fun member => member.id == id

def callbackTouchesMember (p : Program) (callback : Callback) : Bool :=
  callback.authority.any fun payload => (memberPayloads p).contains payload

def callbackChecks (p : Program) (callback : Callback) : List Refusal :=
  (if callbackTouchesMember p callback && !isQuiescent callback.state then
    [Refusal.callbackNotQuiescent callback.state]
  else []) ++
  callback.spawned.flatMap fun spawned =>
    match p.tasks.find? (fun task => task.id == spawned) with
    | none => [Refusal.callbackSpawnedTaskUnaccounted]
    | some task => if isJoined task.state then [] else [Refusal.callbackSpawnedTaskUnaccounted]

def taskChecks (p : Program) (task : Task) : List Refusal :=
  if task.captures.any (knownMember p) && !isJoined task.state then
    [Refusal.taskNotJoined task.state]
  else []

def guardChecks (p : Program) (guard : Guard) : List Refusal :=
  match p.footprints.find? (fun footprint => footprint.id == guard.footprint) with
  | none => []
  | some footprint =>
      if footprint.members.any (knownMember p) && !isReleasedGuard guard.state then
        [Refusal.guardOutstanding]
      else []

def quiescenceChecks (p : Program) : List Refusal :=
  p.callbacks.flatMap (callbackChecks p) ++
    p.tasks.flatMap (taskChecks p) ++
    p.guards.flatMap (guardChecks p)

/-! ## The wrapper lifecycle -/

def containmentChecks (p : Program) : List Refusal :=
  p.members.flatMap fun member =>
    match member.parent.bind (fun id => p.members.find? (fun other => other.id == id)) with
    | none => []
    | some parent =>
        if sameOwner member.owner parent.owner then [] else [Refusal.wrapperOverlapsAggregate]

def heldMemberChecks (p : Program) (w : Wrapper) : List Refusal :=
  match p.members.find? (fun member => member.id == w.held) with
  | none => []
  | some held => if isAggregateOwned held.owner then [] else [Refusal.wrapperOverlapsAggregate]

def wrapperReleaseChecks (p : Program) (w : Wrapper) : List Refusal :=
  ((releaseRecords p).filter fun record => !isParentReleaser record.releaser).flatMap
    fun record =>
      if p.members.any (fun member => member.id == record.member
          && !isAggregateOwned member.owner) then []
      else if record.member == w.held then
        (if isBorrowed w.hold then [Refusal.borrowedHoldReleasedByWrapper] else [])
      else [Refusal.wrapperReleasesForeignMember]

def holdChecks (p : Program) (w : Wrapper) : List Refusal :=
  match w.hold with
  | .owned =>
      (match (releasesOf p w.held).head? with
        | some record =>
            if isParentReleaser record.releaser then [Refusal.ownedHoldAlsoReleasedByParent] else []
        | none => [])
  | .borrowed =>
      (match dropBorrowIndex p, releaseIndex p w.held with
        | some dropped, some released =>
            if dropped < released then [] else [Refusal.borrowedHoldOutlivesParentRelease]
        | _, _ => [Refusal.borrowedHoldOutlivesParentRelease])

def publicationChecks (p : Program) (w : Wrapper) : List Refusal :=
  w.publications.flatMap fun publication =>
    match reclaimIndex p publication, teardownPoint p with
    | none, _ => [Refusal.wrapperPublicationUnreclaimed]
    | some reclaimed, some teardown =>
        if reclaimed < teardown then [] else [Refusal.wrapperPublicationSurvivesTeardown]
    | some _, none => []

def wrapperChecks (p : Program) : List Refusal :=
  match p.wrapper with
  | none => []
  | some w =>
      containmentChecks p ++ heldMemberChecks p w ++ wrapperReleaseChecks p w ++
        holdChecks p w ++ publicationChecks p w

/-! ## The relation -/

def components (p : Program) : List (List Refusal) :=
  [unsupportedChecks p, identityChecks p, scheduleChecks p, addressChecks p,
    quiescenceChecks p, wrapperChecks p]

def semanticChecks (p : Program) : List Refusal := (components p).flatten

/-- Fail-closed: an empty aggregate is refused before anything else, a
    structurally malformed program reports only its structural refusals, and
    otherwise every semantic check runs. -/
def refusals (p : Program) : List Refusal :=
  if p.members.isEmpty then [Refusal.emptyAggregate]
  else if p.structural.isEmpty then semanticChecks p
  else p.structural

/-- Two verdicts and no third: composition never publishes a violation. -/
inductive Verdict where
  | composed
  | unknown
  deriving DecidableEq

def verdict (p : Program) : Verdict :=
  if (refusals p).isEmpty then Verdict.composed else Verdict.unknown

/-! ## Plumbing -/

theorem verdict_unknown_of_mem {p : Program} {r : Refusal} (mem : r ∈ refusals p) :
    verdict p = Verdict.unknown := by
  have nonempty : (refusals p).isEmpty = false := by
    cases h : refusals p with
    | nil => rw [h] at mem; cases mem
    | cons _ _ => rfl
  simp [verdict, nonempty]

theorem refusals_eq_semantic {p : Program}
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = []) :
    refusals p = semanticChecks p := by
  simp [refusals, nonempty, wellformed]

theorem mem_semantic {p : Program} {r : Refusal} {c : List Refusal}
    (component : c ∈ components p) (mem : r ∈ c) : r ∈ semanticChecks p := by
  simp only [semanticChecks, List.mem_flatten]
  exact ⟨c, component, mem⟩

/-- The workhorse: any refusal in any component of a nonempty, structurally
    well-formed program makes the whole composition `unknown`. -/
theorem unknown_of_component {p : Program} {r : Refusal} {c : List Refusal}
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (component : c ∈ components p) (mem : r ∈ c) : verdict p = Verdict.unknown := by
  refine verdict_unknown_of_mem (r := r) ?_
  rw [refusals_eq_semantic nonempty wellformed]
  exact mem_semantic component mem

/-- The contrapositive: a composed program carries no refusal in any component. -/
theorem no_component_refusal {p : Program} {r : Refusal} {c : List Refusal}
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (component : c ∈ components p) : r ∉ c := by
  intro mem
  have unknown := unknown_of_component nonempty wellformed component mem
  rw [unknown] at composed
  exact Verdict.noConfusion composed

theorem unsupported_component (p : Program) : unsupportedChecks p ∈ components p := by
  simp [components]

theorem identity_component (p : Program) : identityChecks p ∈ components p := by
  simp [components]

theorem schedule_component (p : Program) : scheduleChecks p ∈ components p := by
  simp [components]

theorem address_component (p : Program) : addressChecks p ∈ components p := by
  simp [components]

theorem quiescence_component (p : Program) : quiescenceChecks p ∈ components p := by
  simp [components]

theorem wrapper_component (p : Program) : wrapperChecks p ∈ components p := by
  simp [components]

/-! ## Structural discipline -/

/-- A program with no members is not a vacuous success. -/
theorem an_empty_aggregate_is_not_a_vacuous_success (p : Program)
    (empty : p.members.isEmpty = true) : verdict p = Verdict.unknown := by
  have refuse : refusals p = [Refusal.emptyAggregate] := by simp [refusals, empty]
  refine verdict_unknown_of_mem (r := Refusal.emptyAggregate) ?_
  rw [refuse]
  simp

/-- A structurally malformed program is refused. -/
theorem a_structurally_malformed_program_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) {r : Refusal} (mem : r ∈ p.structural) :
    verdict p = Verdict.unknown := by
  have notEmpty : p.structural.isEmpty = false := by
    cases h : p.structural with
    | nil => rw [h] at mem; cases mem
    | cons _ _ => rfl
  refine verdict_unknown_of_mem (r := r) ?_
  simp only [refusals, nonempty, notEmpty, ite_false, Bool.false_eq_true]
  exact mem

/-- ... and it reports only its structural refusals: no semantic refusal can be
    read off a program the reader could not parse. -/
theorem a_malformed_program_reports_only_structural_refusals (p : Program)
    (nonempty : p.members.isEmpty = false) (malformed : p.structural.isEmpty = false) :
    refusals p = p.structural := by
  simp [refusals, nonempty, malformed]

/-! ## Acceptance 3 — unsupported classes prevent whole-composition claims -/

/-- Any declared unsupported class — capacity, metadata, schedule, callback,
    backend or OS custody — refuses the whole composition. -/
theorem any_unsupported_class_blocks_the_composition (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (cls : UnsupportedClass) (declared : cls ∈ p.unsupported) :
    verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (unsupported_component p)
    (r := Refusal.unsupportedClass cls) ?_
  simp only [unsupportedChecks, List.mem_append, List.mem_map]
  exact Or.inl ⟨cls, declared, rfl⟩

/-- A callback outside the admitted family is an unsupported callback class. -/
theorem an_unadmitted_callback_blocks_the_composition (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (callback : Callback) (present : callback ∈ p.callbacks)
    (unadmitted : callback.admitted = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (unsupported_component p)
    (r := Refusal.unsupportedClass UnsupportedClass.callback) ?_
  have any : p.callbacks.any (fun c => !c.admitted) = true := by
    simp only [List.any_eq_true]
    exact ⟨callback, present, by simp [unadmitted]⟩
  simp [unsupportedChecks, any]

/-- A composed program declares no unsupported class at all. -/
theorem a_composed_program_has_no_unsupported_class (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed) : p.unsupported = [] := by
  cases h : p.unsupported with
  | nil => rfl
  | cons cls rest =>
      have unknown := any_unsupported_class_blocks_the_composition p nonempty wellformed cls
        (by rw [h]; simp)
      rw [unknown] at composed
      exact Verdict.noConfusion composed

/-! ## Acceptance 1 — identity separation -/

theorem duplicate_identities_are_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (duplicate : hasDuplicateIdentity p.members = true) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (identity_component p)
    (r := Refusal.duplicateIdentity) ?_
  simp [identityChecks, duplicate]

theorem a_composed_program_separates_every_identity (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed) : hasDuplicateIdentity p.members = false := by
  cases h : hasDuplicateIdentity p.members with
  | false => rfl
  | true =>
      have unknown := duplicate_identities_are_unknown p nonempty wellformed h
      rw [unknown] at composed
      exact Verdict.noConfusion composed

/-- Two members sharing a raw OS-visible number while both are live refuses.
    A `retained` endpoint never stops being live, so its number is not
    reusable. -/
theorem a_reused_live_address_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (pair : Member × Member) (present : pair ∈ memberPairs p.members)
    (shares : sharesAddress pair.1 pair.2 = true)
    (overlapping : disjointLive p pair.1 pair.2 = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (address_component p)
    (r := Refusal.addressReusedWhileLive) ?_
  simp only [addressChecks, List.mem_filterMap]
  exact ⟨pair, present, by simp [shares, overlapping]⟩

/-! ## Acceptance 1 — the release ledger -/

theorem member_component {p : Program} {member : Member} {r : Refusal}
    (present : member ∈ p.members) (mem : r ∈ memberChecks p member) : r ∈ scheduleChecks p := by
  simp only [scheduleChecks, List.mem_append, List.mem_flatMap]
  exact Or.inl ⟨member, present, mem⟩

theorem release_component {p : Program} {member : Member} {record : ReleaseRecord} {r : Refusal}
    (present : member ∈ p.members) (head : (releasesOf p member.id).head? = some record)
    (mem : r ∈ releaseChecks p member record) : r ∈ scheduleChecks p := by
  refine member_component present ?_
  simp only [memberChecks, List.mem_append, head]
  exact Or.inr mem

theorem an_unreleased_member_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members)
    (never : (releasesOf p member.id).length = 0) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.memberNotReleased) (member_component present ?_)
  simp only [memberChecks, List.mem_append, never]
  exact Or.inl (Or.inl (Or.inl (Or.inr (by simp))))

theorem a_member_released_twice_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members)
    (twice : 1 < (releasesOf p member.id).length) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.memberReleasedTwice) (member_component present ?_)
  simp only [memberChecks, List.mem_append]
  exact Or.inl (Or.inl (Or.inr (by simp [twice])))

theorem a_never_acquired_member_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members)
    (never : acquisitionsOf p member.id = 0) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.memberNeverAcquired) (member_component present ?_)
  simp only [memberChecks, List.mem_append, never]
  exact Or.inl (Or.inl (Or.inl (Or.inl (Or.inl (by simp)))))

theorem a_release_before_its_acquisition_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (acquired released : Nat)
    (atAcquire : acquireIndex p member.id = some acquired)
    (atRelease : releaseIndex p member.id = some released)
    (early : released < acquired) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.releaseBeforeAcquire) (member_component present ?_)
  simp only [memberChecks, List.mem_append, atAcquire, atRelease]
  exact Or.inl (Or.inr (by simp [early]))

/-- No status fact discharges custody: ring membership, a zero up counter, a
    success-looking integer and a nulled field are each refused. -/
theorem no_status_fact_discharges_custody (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (fact : StatusFact)
    (offered : record.evidence = Evidence.statusFact fact) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.statusFactIsNotCustodyEvidence fact) (release_component present head ?_)
  simp only [releaseChecks, List.mem_append, offered]
  exact Or.inl (by simp)

/-- No durability premise discharges custody. Synchronization and durability are
    carried and never established by this relation. -/
theorem no_durability_premise_discharges_custody (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (premise : DurabilityPremise)
    (offered : record.evidence = Evidence.durability premise) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.durabilityAssumedAsDischarged premise) (release_component present head ?_)
  simp only [releaseChecks, List.mem_append, offered]
  exact Or.inl (by simp)

/-- An unresolved disposition refuses under its own reason. It names the reason
    and nothing else: it never alleges a leak or a double release. -/
theorem an_unresolved_disposition_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (reason : UnresolvedReason)
    (unresolved : record.disposition = Disposition.unresolved reason) :
    verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.unresolvedDisposition reason) (release_component present head ?_)
  simp only [releaseChecks, List.mem_append, unresolved]
  exact Or.inr (by simp)

theorem a_retained_endpoint_without_a_custodian_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record)
    (retained : record.disposition = Disposition.retained none) :
    verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.retainedEndpointWithoutCustodian) (release_component present head ?_)
  simp only [releaseChecks, List.mem_append, retained]
  refine Or.inr ?_
  cases isEndpoint member.kind <;> simp

/-- Only an OS endpoint may survive parent release. A retained heap allocation
    is a leak, not a residual. -/
theorem a_retained_allocation_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (custodian : Option Nat)
    (retained : record.disposition = Disposition.retained custodian)
    (heap : isEndpoint member.kind = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.retainedAllocation) (release_component present head ?_)
  simp only [releaseChecks, List.mem_append, retained]
  refine Or.inr ?_
  cases custodian <;> simp [heap]

/-- The aggregate root is released, or nothing composes. -/
theorem a_composition_that_never_releases_the_parent_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (never : (releaseRecords p).any
      (fun record => record.member == p.root && isParentReleaser record.releaser) = false) :
    verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (schedule_component p)
    (r := Refusal.parentNotReleased) ?_
  simp [scheduleChecks, never]

/-! ## Acceptance 2 — quiescence before teardown -/

theorem callback_component {p : Program} {callback : Callback} {r : Refusal}
    (present : callback ∈ p.callbacks) (mem : r ∈ callbackChecks p callback) :
    r ∈ quiescenceChecks p := by
  simp only [quiescenceChecks, List.mem_append, List.mem_flatMap]
  exact Or.inl (Or.inl ⟨callback, present, mem⟩)

/-- A callback with declared authority over a member payload must be quiescent
    at the teardown point. -/
theorem a_live_callback_over_a_member_payload_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (callback : Callback) (present : callback ∈ p.callbacks)
    (touches : callbackTouchesMember p callback = true)
    (live : isQuiescent callback.state = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (quiescence_component p)
    (r := Refusal.callbackNotQuiescent callback.state) (callback_component present ?_)
  simp only [callbackChecks, List.mem_append]
  exact Or.inl (by simp [touches, live])

/-- A quiescent callback that started a task nobody joined is still a refusal. -/
theorem an_unaccounted_spawned_task_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (callback : Callback) (present : callback ∈ p.callbacks) (spawned : Nat)
    (started : spawned ∈ callback.spawned)
    (unjoined : ∀ task, p.tasks.find? (fun t => t.id == spawned) = some task →
      isJoined task.state = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (quiescence_component p)
    (r := Refusal.callbackSpawnedTaskUnaccounted) (callback_component present ?_)
  simp only [callbackChecks, List.mem_append, List.mem_flatMap]
  refine Or.inr ⟨spawned, started, ?_⟩
  cases h : p.tasks.find? (fun t => t.id == spawned) with
  | none => simp
  | some task => simp [unjoined task h]

theorem task_component {p : Program} {task : Task} {r : Refusal}
    (present : task ∈ p.tasks) (mem : r ∈ taskChecks p task) : r ∈ quiescenceChecks p := by
  simp only [quiescenceChecks, List.mem_append, List.mem_flatMap]
  exact Or.inl (Or.inr ⟨task, present, mem⟩)

/-- A task that captured a member and was not joined blocks teardown: a
    completed but unjoined child still holds its captures. -/
theorem an_unjoined_task_capturing_a_member_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (task : Task) (present : task ∈ p.tasks)
    (captures : task.captures.any (knownMember p) = true)
    (unjoined : isJoined task.state = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (quiescence_component p)
    (r := Refusal.taskNotJoined task.state) (task_component present ?_)
  simp [taskChecks, captures, unjoined]

theorem guard_component {p : Program} {guard : Guard} {r : Refusal}
    (present : guard ∈ p.guards) (mem : r ∈ guardChecks p guard) : r ∈ quiescenceChecks p := by
  simp only [quiescenceChecks, List.mem_append, List.mem_flatMap]
  exact Or.inr ⟨guard, present, mem⟩

/-- A guard still held over a footprint touching a member blocks teardown. -/
theorem an_outstanding_guard_over_a_member_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (guard : Guard) (present : guard ∈ p.guards) (footprint : Footprint)
    (found : p.footprints.find? (fun f => f.id == guard.footprint) = some footprint)
    (touches : footprint.members.any (knownMember p) = true)
    (held : isReleasedGuard guard.state = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (quiescence_component p)
    (r := Refusal.guardOutstanding) (guard_component present ?_)
  simp [guardChecks, found, touches, held]

/-! ## The wrapper lifecycle -/

theorem wrapper_part {p : Program} {w : Wrapper} {r : Refusal} (declared : p.wrapper = some w)
    (mem : r ∈ containmentChecks p ++ heldMemberChecks p w ++ wrapperReleaseChecks p w ++
      holdChecks p w ++ publicationChecks p w) : r ∈ wrapperChecks p := by
  simp only [wrapperChecks, declared]
  exact mem

/-- The two containment trees stay separate: a wrapper allocation is never a
    child of the aggregate, and an aggregate child is never a child of the
    wrapper. -/
theorem an_overlapping_containment_tree_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (w : Wrapper) (declared : p.wrapper = some w)
    (member parent : Member) (present : member ∈ p.members)
    (link : member.parent.bind (fun id => p.members.find? (fun other => other.id == id))
      = some parent)
    (crosses : sameOwner member.owner parent.owner = false) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (wrapper_component p)
    (r := Refusal.wrapperOverlapsAggregate) (wrapper_part declared ?_)
  simp only [List.mem_append]
  refine Or.inl (Or.inl (Or.inl (Or.inl ?_)))
  simp only [containmentChecks, List.mem_flatMap]
  exact ⟨member, present, by simp [link, crosses]⟩

/-- An owned hold the parent also releases is a double custody claim. -/
theorem an_owned_hold_the_parent_also_releases_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (w : Wrapper) (declared : p.wrapper = some w) (owning : w.hold = HoldMode.owned)
    (record : ReleaseRecord) (head : (releasesOf p w.held).head? = some record)
    (byParent : isParentReleaser record.releaser = true) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (wrapper_component p)
    (r := Refusal.ownedHoldAlsoReleasedByParent) (wrapper_part declared ?_)
  simp only [List.mem_append]
  refine Or.inl (Or.inr ?_)
  simp [holdChecks, owning, head, byParent]

/-- A borrow the wrapper never drops outlives what it points at. -/
theorem a_borrow_that_is_never_dropped_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (w : Wrapper) (declared : p.wrapper = some w) (borrowing : w.hold = HoldMode.borrowed)
    (never : dropBorrowIndex p = none) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (wrapper_component p)
    (r := Refusal.borrowedHoldOutlivesParentRelease) (wrapper_part declared ?_)
  simp only [List.mem_append]
  refine Or.inl (Or.inr ?_)
  simp [holdChecks, borrowing, never]

/-- A publication the wrapper never withdraws keeps a reference the aggregate's
    teardown cannot see. -/
theorem an_unreclaimed_publication_is_unknown (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (w : Wrapper) (declared : p.wrapper = some w) (publication : Nat)
    (published : publication ∈ w.publications)
    (never : reclaimIndex p publication = none) : verdict p = Verdict.unknown := by
  refine unknown_of_component nonempty wellformed (wrapper_component p)
    (r := Refusal.wrapperPublicationUnreclaimed) (wrapper_part declared ?_)
  simp only [List.mem_append]
  refine Or.inr ?_
  simp only [publicationChecks, List.mem_flatMap]
  exact ⟨publication, published, by simp [never]⟩

/-! ## Standing properties -/

/-- Composition never publishes a violation. `Verdict` has two constructors, so
    a refused composition is a reason to say nothing and never an allegation:
    the aggregate analogue of
    `QCore.ExitResiduals.an_unmodeled_path_alleges_no_residual`. -/
theorem composition_is_never_a_violation (p : Program) :
    verdict p = Verdict.composed ∨ verdict p = Verdict.unknown := by
  cases h : verdict p with
  | composed => exact Or.inl rfl
  | unknown => exact Or.inr rfl

/-- A composed claim is exactly a refusal-free one. -/
theorem composed_iff_no_refusal (p : Program) :
    verdict p = Verdict.composed ↔ refusals p = [] := by
  constructor
  · intro composed
    cases h : refusals p with
    | nil => rfl
    | cons _ _ =>
        rw [verdict, h] at composed
        simp at composed
  · intro empty
    simp [verdict, empty]

/-- Fail-closed in the promotion direction: an unresolved member never composes,
    so an aggregate with unresolved OS custody, an uncontracted callee or an
    unmodeled branch is never published as a whole-composition claim. -/
theorem an_unresolved_member_never_composes (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (reason : UnresolvedReason)
    (unresolved : record.disposition = Disposition.unresolved reason) :
    verdict p ≠ Verdict.composed := by
  intro composed
  have unknown := an_unresolved_disposition_is_unknown p nonempty wellformed member present record
    head reason unresolved
  rw [unknown] at composed
  exact Verdict.noConfusion composed

/-- A composed claim rests on no status fact anywhere in its release ledger. -/
theorem a_composed_program_offers_no_status_fact (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (fact : StatusFact) :
    record.evidence ≠ Evidence.statusFact fact := by
  intro offered
  have unknown := no_status_fact_discharges_custody p nonempty wellformed member present record head
    fact offered
  rw [unknown] at composed
  exact Verdict.noConfusion composed

/-- A composed claim discharges no durability premise. -/
theorem a_composed_program_discharges_no_durability_premise (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (member : Member) (present : member ∈ p.members) (record : ReleaseRecord)
    (head : (releasesOf p member.id).head? = some record) (premise : DurabilityPremise) :
    record.evidence ≠ Evidence.durability premise := by
  intro offered
  have unknown := no_durability_premise_discharges_custody p nonempty wellformed member present
    record head premise offered
  rw [unknown] at composed
  exact Verdict.noConfusion composed

/-- Every member of a composed aggregate is released at least once. -/
theorem a_composed_program_releases_every_member (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (member : Member) (present : member ∈ p.members) :
    (releasesOf p member.id).length ≠ 0 := by
  intro never
  have unknown := an_unreleased_member_is_unknown p nonempty wellformed member present never
  rw [unknown] at composed
  exact Verdict.noConfusion composed

/-- Every member of a composed aggregate is acquired. -/
theorem a_composed_program_acquires_every_member (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (member : Member) (present : member ∈ p.members) : acquisitionsOf p member.id ≠ 0 := by
  intro never
  have unknown := a_never_acquired_member_is_unknown p nonempty wellformed member present never
  rw [unknown] at composed
  exact Verdict.noConfusion composed

/-- Every callback of a composed aggregate holding authority over a member
    payload is quiescent. -/
theorem a_composed_program_quiesces_every_member_callback (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (callback : Callback) (present : callback ∈ p.callbacks)
    (touches : callbackTouchesMember p callback = true) :
    isQuiescent callback.state = true := by
  cases h : isQuiescent callback.state with
  | true => rfl
  | false =>
      have unknown := a_live_callback_over_a_member_payload_is_unknown p nonempty wellformed
        callback present touches h
      rw [unknown] at composed
      exact Verdict.noConfusion composed

/-- Every task of a composed aggregate that captured a member is joined. -/
theorem a_composed_program_joins_every_capturing_task (p : Program)
    (nonempty : p.members.isEmpty = false) (wellformed : p.structural = [])
    (composed : verdict p = Verdict.composed)
    (task : Task) (present : task ∈ p.tasks)
    (captures : task.captures.any (knownMember p) = true) : isJoined task.state = true := by
  cases h : isJoined task.state with
  | true => rfl
  | false =>
      have unknown := an_unjoined_task_capturing_a_member_is_unknown p nonempty wellformed task
        present captures h
      rw [unknown] at composed
      exact Verdict.noConfusion composed

/-! ## Non-vacuity

    A relation that refuses everything proves every refusal theorem. These three
    witnesses are the guard against that: a complete aggregate with a borrowing
    wrapper composes, and two single mutations of it do not. -/

/-- Root, one child allocation, one descriptor endpoint retained by a named
    external custodian, and one wrapper allocation. The wrapper borrows the
    child and drops the borrow before the parent releases anything. -/
def referenceProgram : Program where
  root := 1
  members :=
    [ { id := 1, kind := .aggregate, generation := 1, owner := .aggregateOwner,
        parent := none, address := none, payloads := [] },
      { id := 2, kind := .allocation, generation := 1, owner := .aggregateOwner,
        parent := some 1, address := none, payloads := [10] },
      { id := 3, kind := .endpoint .descriptor, generation := 1, owner := .aggregateOwner,
        parent := some 2, address := some 7, payloads := [] },
      { id := 4, kind := .allocation, generation := 2, owner := .wrapperOwner,
        parent := none, address := none, payloads := [] } ]
  schedule :=
    [ .acquire 1, .acquire 2, .acquire 3, .acquire 4,
      .publish 1, .reclaim 1, .dropBorrow,
      .release { member := 4, releaser := .wrapper, disposition := .released,
                 evidence := .exitResidualDischarged },
      .release { member := 3, releaser := .parent, disposition := .retained (some 42),
                 evidence := .observedEndpointPostState },
      .release { member := 2, releaser := .parent, disposition := .released,
                 evidence := .declaredDestructorContract },
      .release { member := 1, releaser := .parent, disposition := .released,
                 evidence := .declaredDestructorContract } ]
  callbacks :=
    [ { id := 1, state := .quiescent, authority := [10], admitted := true, spawned := [1] } ]
  tasks := [ { id := 1, state := .joined, captures := [2] } ]
  guards := [ { id := 1, state := .releasedGuard, footprint := 1 } ]
  footprints := [ { id := 1, members := [2] } ]
  wrapper := some { held := 2, hold := .borrowed, publications := [1] }
  externalCustodians := [42]
  unsupported := []
  structural := []

theorem the_reference_aggregate_composes : verdict referenceProgram = Verdict.composed := by
  decide

/-- The same aggregate with its callback still running does not compose, so the
    quiescence clause is not vacuous. -/
def referenceWithLiveCallback : Program :=
  { referenceProgram with
    callbacks :=
      [ { id := 1, state := .running, authority := [10], admitted := true, spawned := [1] } ] }

theorem a_live_callback_makes_the_reference_unknown :
    verdict referenceWithLiveCallback = Verdict.unknown := by
  decide

/-- The same aggregate with its retained descriptor number taken by a second
    live epoch does not compose, so the address clause is not vacuous either. -/
def referenceWithAddressReuse : Program :=
  { referenceProgram with
    members := referenceProgram.members ++
      [ { id := 5, kind := .endpoint .descriptor, generation := 2, owner := .aggregateOwner,
          parent := some 2, address := some 7, payloads := [] } ],
    schedule := referenceProgram.schedule ++
      [ .acquire 5,
        .release { member := 5, releaser := .parent, disposition := .released,
                   evidence := .observedEndpointPostState } ] }

theorem a_reused_live_descriptor_number_makes_the_reference_unknown :
    verdict referenceWithAddressReuse = Verdict.unknown := by
  decide

end QCore.AggregateComposition
