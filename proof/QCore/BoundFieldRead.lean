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

import QCore.AggregateStore

/-!
# Reading a member back through a base this body bound (card O04-READ-BOUND)

`QCore.BorrowedActual` (card C02-P) models the closure that decides which bases
a body **never** binds to a resource identity, and admits a member read only
through one of those. `QCore.AggregateStore` (card O04-B08) models the
complementary *store* into a member of a base this body **did** bind, and
records the read through such a base as its last named blocker,
`O04-B08-READ-BOUND-BASE`.

This module models the decision
`crates/qcore-c-gateway/src/aggregate_store.rs::bound_read_actuals` makes,
together with the `bound_read_resolves` half of `legacy_c::transfer`:

* **`admitsRead`** — the syntactic whitelist. It reuses the *identical* base
  premises `QCore.AggregateStore.admitsStore` checks, and adds two of its own:
  the member must be a single-star pointer (a scalar member can never hold an
  identity), and the actual must occupy a **`Borrowed`** parameter position;
* **`readResolves`** — the flow-sensitive premise. Four conjuncts: the owner
  resolves exactly as a store's owner must, the member place is bound to a
  definite non-empty target set, that set is exactly the field link an admitted
  store recorded, and every linked identity is itself live.

## Why `Borrowed` only

A borrowed actual discharges nothing, so admitting one can only withdraw a
coverage refusal — it can never manufacture a release. Every custody-bearing
position (`Owned`, `Consumed`, `Released`, `Stored`, `Removed`) is refused by
construction, because a contracted callee that merely *borrows* the aggregate
could rewrite the member invisibly and this projection maintains no frame
condition for that. That restriction is the named boundary
`O04-READ-BOUND-CUSTODY`; it is modeled here and proved to be load-bearing, and
it is **not** claimed to be unnecessary.

Two directions are proved, and both matter.

* **Refusal.** Every premise refuses on its own; a refused read never denotes an
  identity, and neither gate alone suffices. In particular a read whose linked
  child this body already released is refused — card C00's lesson in the read
  direction.
* **Non-vacuity.** An admitted read exists and denotes, each premise flips it,
  and the two read routes are disjoint by construction.

This is a model of the *decision*, not of C semantics. That the reader extracts
these declaration facts faithfully, that a declared `Borrowed` mode really
borrows, and premise **P-B08-SHALLOW**, are all premises supplied from outside
and assumed here. Nothing here relates two distinct declared names that may
denote the same object; the recorded C02-P may-alias boundary is inherited
unchanged and is not widened. Nothing here is a claim about arbitrary C memory
safety.
-/

namespace QCore.BoundFieldRead

open QCore.AggregateStore

/-! ## The argument position -/

/-- The declared mode of the parameter the actual occupies. Only `borrowed`
    discharges nothing, and only `borrowed` is admitted. -/
inductive Position where
  | borrowed
  | owned
  | consumed
  | released
  | stored
  | removed
  | out
  deriving DecidableEq

/-- Exactly one inhabitant is admitted, by construction. -/
def positionOk (p : Position) : Bool := p == Position.borrowed

/-! ## The syntactic whitelist -/

/-- One candidate bound read, as the reader and the base scan describe it.

    The five base fields are the *same* premises
    `QCore.AggregateStore.admitsStore` checks, restated here so the two
    decisions can be related; `readBaseOk` below states that relation. -/
structure Read where
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- The base is declared in the body, not a formal parameter. -/
  baseIsLocal : Bool
  /-- The base's declared type is exactly `struct TAG *`. -/
  baseIsRecordPointer : Bool
  /-- `TAG` has one complete, non-`union` definition whose every member
      declaration was read. -/
  recordComplete : Bool
  /-- The base is the destination of exactly one top-level assignment. -/
  baseBoundOnce : Bool
  /-- That single binding *calls* something. -/
  baseAcquired : Bool
  /-- The base is address-taken whole, incremented, compound-assigned,
      dereferenced, subscripted or `.`-selected somewhere in the body. -/
  baseEscapesPointer : Bool
  /-- The base could acquire a second declared name, or be changed invisibly
      through a rebinding argument position. -/
  baseAliased : Bool
  /-- The actual is exactly `BASE - > FIELD` and is the **whole** argument:
      no cast, no parentheses, no arithmetic, no continuation. -/
  wholeSingleSelection : Bool
  member : MemberKind
  /-- The member carries `volatile` or `_Atomic`. -/
  qualified : Bool
  /-- The callee has a contract in force. -/
  calleeContracted : Bool
  position : Position
  deriving DecidableEq

/-- The base premises, exactly as the store direction states them. -/
def readBaseOk (r : Read) : Bool :=
  r.declarationsRead
    && r.baseIsLocal
    && r.baseIsRecordPointer
    && r.recordComplete
    && r.baseBoundOnce
    && r.baseAcquired
    && !r.baseEscapesPointer
    && !r.baseAliased

/-- The member premise, *stricter* than the store direction's: only a
    single-star pointer member can denote an identity to read back. -/
def readMemberOk (r : Read) : Bool :=
  (r.member == MemberKind.singlePointer) && !r.qualified

/-- The whitelist. Every clause is a conjunct, so failing any one refuses. -/
def admitsRead (r : Read) : Bool :=
  readBaseOk r
    && r.wholeSingleSelection
    && readMemberOk r
    && r.calleeContracted
    && positionOk r.position

/-! ## Every premise refuses on its own -/

theorem unread_declarations_never_admit_a_read (r : Read)
    (unread : r.declarationsRead = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, unread]

theorem a_formal_base_never_admits_a_read (r : Read)
    (formal : r.baseIsLocal = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, formal]

theorem a_non_record_pointer_base_never_admits_a_read (r : Read)
    (mismatch : r.baseIsRecordPointer = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, mismatch]

theorem an_incomplete_record_never_admits_a_read (r : Read)
    (incomplete : r.recordComplete = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, incomplete]

theorem a_rebound_base_never_admits_a_read (r : Read)
    (rebound : r.baseBoundOnce = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, rebound]

/-- An aggregate read out of another object's memory is not one this body
    acquired, so no read through it is admitted either. -/
theorem an_unacquired_base_never_admits_a_read (r : Read)
    (read : r.baseAcquired = false) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, read]

theorem an_escaping_base_pointer_never_admits_a_read (r : Read)
    (escapes : r.baseEscapesPointer = true) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, escapes]

theorem an_aliased_base_never_admits_a_read (r : Read)
    (aliased : r.baseAliased = true) : admitsRead r = false := by
  simp [admitsRead, readBaseOk, aliased]

/-- A cast, a parenthesized actual, an arithmetic operand, a chained access or
    any other partial argument keeps the lexical gate's refusal. -/
theorem a_partial_actual_never_admits_a_read (r : Read)
    (fragment : r.wholeSingleSelection = false) : admitsRead r = false := by
  simp [admitsRead, fragment]

/-- `O04-READ-BOUND-SCALAR`: a scalar member can never hold an identity, so
    reading one back denotes nothing this projection tracks. -/
theorem a_scalar_member_never_admits_a_read (r : Read)
    (scalar : r.member = MemberKind.scalar) : admitsRead r = false := by
  simp [admitsRead, readMemberOk, scalar]

theorem a_refused_member_shape_never_admits_a_read (r : Read)
    (refused : r.member = MemberKind.refusedShape) : admitsRead r = false := by
  simp [admitsRead, readMemberOk, refused]

/-- Card **ALIAS-AGGREGATE**: the store/read asymmetry of the opaque member.
    `QCore.AggregateStore.admitsStore` admits a `void *` destination, and this
    rule refuses reading it back — in *every* position, borrowed or
    custody-taking — because `void *` names no pointee and this rule has
    nothing to match against the callee's declared parameter type. The store
    side needs no such match: a link is by identity, and identity carries no
    type. -/
theorem an_opaque_member_never_admits_a_read (r : Read)
    (isOpaque : r.member = MemberKind.opaquePointer) : admitsRead r = false := by
  simp [admitsRead, readMemberOk, isOpaque]

theorem a_qualified_member_never_admits_a_read (r : Read)
    (qualified : r.qualified = true) : admitsRead r = false := by
  simp [admitsRead, readMemberOk, qualified]

theorem an_uncontracted_callee_never_admits_a_read (r : Read)
    (uncontracted : r.calleeContracted = false) : admitsRead r = false := by
  simp [admitsRead, uncontracted]

/-- `O04-READ-BOUND-CUSTODY`, stated as the theorem it is: every position that
    can discharge refuses. One theorem, quantified over the whole vocabulary,
    so a later widening cannot slip a mode past it. -/
theorem a_custody_bearing_position_never_admits_a_read (r : Read)
    (custody : r.position ≠ Position.borrowed) : admitsRead r = false := by
  have refused : positionOk r.position = false := by
    cases hp : r.position
    case borrowed => exact absurd hp custody
    all_goals rfl
  simp [admitsRead, refused]

/-- The contrapositive, used below: an admitted read's position is `borrowed`. -/
theorem an_admitted_read_is_borrowed (r : Read) (admitted : admitsRead r = true) :
    r.position = Position.borrowed := by
  cases hp : r.position
  case borrowed => rfl
  all_goals simp [admitsRead, positionOk, hp] at admitted

/-- Its corollary, stated per mode so each is separately audited. -/
theorem no_owned_consumed_released_stored_removed_or_out_position_admits_a_read (r : Read)
    (custody : r.position = Position.owned ∨ r.position = Position.consumed
      ∨ r.position = Position.released ∨ r.position = Position.stored
      ∨ r.position = Position.removed ∨ r.position = Position.out) :
    admitsRead r = false := by
  apply a_custody_bearing_position_never_admits_a_read
  rcases custody with h | h | h | h | h | h <;> simp [h]

/-! ## The flow-sensitive read premise -/

/-- What the projection knows about the *member place* at the read. -/
structure Link where
  /-- The projection binds the place `BASE->FIELD` here. -/
  bound : Bool
  /-- The place's abstract pointer is not `unknown`. -/
  known : Bool
  /-- The place denotes at least one abstract site. A member this body never
      stored into carries no identity to denote. -/
  nonEmpty : Bool
  /-- The place's target set is *exactly* the field link an admitted store
      recorded under the resolved owner. A binding that has drifted from its
      link is refused rather than believed. -/
  agreesWithFieldLink : Bool
  /-- Every linked identity's disposition. `live` is the only admitted one. -/
  childStatus : Disposition
  deriving DecidableEq

def readResolves (o : Owner) (l : Link) : Bool :=
  ownerResolves o && l.bound && l.known && l.nonEmpty && l.agreesWithFieldLink
    && l.childStatus == Disposition.live

theorem an_unresolved_owner_never_resolves_a_read (o : Owner) (l : Link)
    (refused : ownerResolves o = false) : readResolves o l = false := by
  simp [readResolves, refused]

/-- Card C00's lesson, one level up: a read through an aggregate this body
    already released is refused. -/
theorem a_read_through_a_released_aggregate_never_resolves (o : Owner) (l : Link)
    (released : o.status = Disposition.released) : readResolves o l = false := by
  simp [readResolves, ownerResolves, released]

theorem a_read_through_a_caller_held_aggregate_never_resolves (o : Owner) (l : Link)
    (caller : o.callerHeld = true) : readResolves o l = false := by
  simp [readResolves, ownerResolves, caller]

theorem an_unbound_member_never_resolves_a_read (o : Owner) (l : Link)
    (unbound : l.bound = false) : readResolves o l = false := by
  simp [readResolves, unbound]

theorem an_unknown_member_never_resolves_a_read (o : Owner) (l : Link)
    (unknown : l.known = false) : readResolves o l = false := by
  simp [readResolves, unknown]

/-- The member the body never stored into — the "read of a different member"
    row — carries no identity, so the read refuses. -/
theorem an_unlinked_member_never_resolves_a_read (o : Owner) (l : Link)
    (empty : l.nonEmpty = false) : readResolves o l = false := by
  simp [readResolves, empty]

/-- A binding that no longer agrees with the field link the store recorded is
    refused rather than believed. -/
theorem a_drifted_binding_never_resolves_a_read (o : Owner) (l : Link)
    (drift : l.agreesWithFieldLink = false) : readResolves o l = false := by
  simp [readResolves, drift]

/-- Card C00's lesson in the read direction proper: a member left pointing at an
    identity this body already released is a dangling read and is refused. -/
theorem a_released_linked_child_never_resolves_a_read (o : Owner) (l : Link)
    (released : l.childStatus = Disposition.released) : readResolves o l = false := by
  simp [readResolves, released]

theorem an_escaped_linked_child_never_resolves_a_read (o : Owner) (l : Link)
    (escaped : l.childStatus = Disposition.escaped) : readResolves o l = false := by
  simp [readResolves, escaped]

theorem an_unresolved_linked_child_never_resolves_a_read (o : Owner) (l : Link)
    (unresolved : l.childStatus = Disposition.unresolved) : readResolves o l = false := by
  simp [readResolves, unresolved]

/-- The contrapositive, used below: a resolved read's linked child is live. -/
theorem a_resolved_read_has_a_live_child (o : Owner) (l : Link)
    (resolved : readResolves o l = true) : l.childStatus = Disposition.live := by
  cases hc : l.childStatus
  case live => rfl
  all_goals simp [readResolves, hc] at resolved

/-! ## The transfer -/

inductive ReadOutcome where
  /-- The actual denotes exactly the identities the member links, and the
      declared `Borrowed` mode applies to them — which discharges nothing. -/
  | denotes
  /-- The read is refused: the path is poisoned and can support no custody
      claim. -/
  | refused
  deriving DecidableEq

def readStep (r : Read) (o : Owner) (l : Link) : ReadOutcome :=
  if admitsRead r && readResolves o l then ReadOutcome.denotes else ReadOutcome.refused

/-- Fail-closed direction one: a read the whitelist refuses never denotes. -/
theorem a_refused_read_never_denotes (r : Read) (o : Owner) (l : Link)
    (refused : admitsRead r = false) : readStep r o l = ReadOutcome.refused := by
  simp [readStep, refused]

/-- Fail-closed direction two: a read whose owner or link does not resolve here
    and now never denotes, whatever the declarations say. -/
theorem an_unresolved_read_never_denotes (r : Read) (o : Owner) (l : Link)
    (refused : readResolves o l = false) : readStep r o l = ReadOutcome.refused := by
  simp [readStep, refused]

/-- The two directions together: only a read that passes *both* gates denotes,
    and when it does, its position is `borrowed` and its child is live. -/
theorem only_a_doubly_admitted_read_denotes (r : Read) (o : Owner) (l : Link)
    (denotes : readStep r o l = ReadOutcome.denotes) :
    admitsRead r = true ∧ readResolves o l = true
      ∧ r.position = Position.borrowed ∧ l.childStatus = Disposition.live := by
  unfold readStep at denotes
  split at denotes
  · rename_i gates
    simp only [Bool.and_eq_true] at gates
    obtain ⟨admitted, resolved⟩ := gates
    exact ⟨admitted, resolved, an_admitted_read_is_borrowed r admitted,
      a_resolved_read_has_a_live_child o l resolved⟩
  · exact absurd denotes (by simp)

/-! ## What an admitted read does, and does not, do

An admitted read occupies a `Borrowed` position, and a borrowed parameter's
declared effect is the identity on every disposition. That is the whole reason
this admission is safe to make: it can withdraw a coverage refusal, and it can
do nothing else. -/

/-- The declared effect of the only admitted position. -/
def borrowedEffect (child : Disposition) : Disposition := child

theorem an_admitted_read_changes_no_disposition (child : Disposition) :
    borrowedEffect child = child := by
  rfl

/-- The consequence that makes the admission fail-closed by construction: no
    admitted read can ever discharge an identity. -/
theorem an_admitted_read_never_discharges (child : Disposition)
    (live : child = Disposition.live) : borrowedEffect child = Disposition.live := by
  simp [borrowedEffect, live]

theorem an_admitted_read_never_releases (child : Disposition)
    (unreleased : child ≠ Disposition.released) :
    borrowedEffect child ≠ Disposition.released := by
  simpa [borrowedEffect] using unreleased

/-! ## The two read routes are disjoint

Card C02-P admits a member read only through a base its `untracked_bases`
closure did **not** refuse; this card admits one only through a base
`store_bases` accepted, which requires the base to be bound exactly once by a
call. A base cannot be both, so no statement is admitted twice and neither route
shadows the other. -/

/-- Whether card C02-P's route admits the base: it refuses every name the body
    may bind to an identity, including one bound by a call. -/
def borrowedActualRouteAdmits (r : Read) : Bool := !r.baseBoundOnce || !r.baseAcquired

theorem the_two_read_routes_are_disjoint (r : Read) (admitted : admitsRead r = true) :
    borrowedActualRouteAdmits r = false := by
  have once : r.baseBoundOnce = true := by
    cases hb : r.baseBoundOnce
    · rw [a_rebound_base_never_admits_a_read r hb] at admitted
      exact Bool.noConfusion admitted
    · rfl
  have acquired : r.baseAcquired = true := by
    cases ha : r.baseAcquired
    · rw [an_unacquired_base_never_admits_a_read r ha] at admitted
      exact Bool.noConfusion admitted
    · rfl
  simp [borrowedActualRouteAdmits, once, acquired]

/-! ## Non-vacuity, in both directions -/

/-- The admitted read the routes-mask statement needs: a single-pointer member
    of a local aggregate this body acquired, whole, at a borrowed position. -/
def admittedExample : Read :=
  { declarationsRead := true
    baseIsLocal := true
    baseIsRecordPointer := true
    recordComplete := true
    baseBoundOnce := true
    baseAcquired := true
    baseEscapesPointer := false
    baseAliased := false
    wholeSingleSelection := true
    member := MemberKind.singlePointer
    qualified := false
    calleeContracted := true
    position := Position.borrowed }

def liveLink : Link :=
  { bound := true
    known := true
    nonEmpty := true
    agreesWithFieldLink := true
    childStatus := Disposition.live }

theorem the_read_whitelist_is_not_vacuous : admitsRead admittedExample = true := by
  rfl

theorem the_admitted_read_denotes :
    readStep admittedExample liveOwner liveLink = ReadOutcome.denotes := by
  rfl

/-- One flipped premise refuses the same read: the whitelist is doing work. -/
theorem one_scalar_member_flips_the_admitted_read :
    admitsRead { admittedExample with member := MemberKind.scalar } = false := by
  rfl

/-- Card **ALIAS-AGGREGATE**, non-vacuously: the admitted read refuses when the
    only thing changed is that the member is `void *`. -/
theorem one_opaque_member_flips_the_admitted_read :
    admitsRead { admittedExample with member := MemberKind.opaquePointer } = false := by
  rfl

theorem one_releasing_position_flips_the_admitted_read :
    admitsRead { admittedExample with position := Position.released } = false := by
  rfl

theorem one_unacquired_base_flips_the_admitted_read :
    admitsRead { admittedExample with baseAcquired := false } = false := by
  rfl

theorem one_aliased_base_flips_the_admitted_read :
    admitsRead { admittedExample with baseAliased := true } = false := by
  rfl

/-- And the flow-sensitive gate is doing work too: the same admitted read
    refuses against a released aggregate, against a released linked child, and
    against a member this body never linked. -/
theorem a_released_owner_flips_the_admitted_read :
    readStep admittedExample { liveOwner with status := Disposition.released } liveLink
      = ReadOutcome.refused := by
  rfl

theorem a_released_child_flips_the_admitted_read :
    readStep admittedExample liveOwner { liveLink with childStatus := Disposition.released }
      = ReadOutcome.refused := by
  rfl

theorem an_unlinked_member_flips_the_admitted_read :
    readStep admittedExample liveOwner { liveLink with nonEmpty := false }
      = ReadOutcome.refused := by
  rfl

/-- Non-vacuity of the disjointness theorem: C02-P's route really does admit a
    base this card refuses, so the two are complements rather than both empty. -/
theorem the_borrowed_actual_route_is_not_vacuous :
    borrowedActualRouteAdmits { admittedExample with baseAcquired := false } = true := by
  rfl

/-! ## The custody-taking direction (card O04-B09B10)

Card O04-B09B10 supersedes `O04-READ-BOUND-CUSTODY` at a **narrower, checked**
boundary. Everything above is unchanged and still models exactly what it
modeled: `admitsRead` is the *borrowed* half of
`aggregate_store.rs::bound_read_actuals`, and every theorem about it — including
`a_custody_bearing_position_never_admits_a_read` — still holds of that half.

The implementation's decision is now the **union** of that half and the one
below, which admits a read at a `Released` or `Consumed` position under one
extra premise:

* **F (insulation).** No *contracted* call of the body may leave the aggregate
  both live and rewritable. Only three argument shapes mentioning the base are
  permitted: an admitted member *read* (reading cannot write), an admitted
  member *address* on a member declared with no `*` (whose storage C's object
  model keeps disjoint from every linkable member), and the base whole at a
  position that is itself `Released` or `Consumed` (after which no later read
  can resolve at all, so the destructor call a real body must make does not cost
  the premise). An **un**contracted call needs no premise: such a statement is
  refused, and a refused statement poisons every pointer on its path.

Why the borrowed half needs no such premise, and this one does: borrowing is the
identity on every disposition, so a borrowed read of a stale link at worst
misnames a pointer the callee only reads. A read that *releases* would record
the release against the field link, discharging an obligation nothing discharged
and hiding a real one. That asymmetry is proved below.

The four remaining custody-bearing positions — `owned`, `stored`, `removed`,
`out` — still refuse in **both** halves. -/

/-- Whether the declared position takes custody of what it reads, in the two
    modes card O04-B09B10 admits. -/
def custodyPosition (p : Position) : Bool :=
  p == Position.released || p == Position.consumed

/-- The custody half of the whitelist: the identical base and member premises,
    a custody-taking position, and premise **F**. -/
def admitsCustodyRead (r : Read) (insulated : Bool) : Bool :=
  readBaseOk r
    && r.wholeSingleSelection
    && readMemberOk r
    && r.calleeContracted
    && custodyPosition r.position
    && insulated

/-- The implementation's decision: the union of the two halves. -/
def admitsAnyRead (r : Read) (insulated : Bool) : Bool :=
  admitsRead r || admitsCustodyRead r insulated

/-- Premise **F** is load-bearing: an exposed base keeps card O04-READ-BOUND's
    refusal in full, so nothing this card admits rests on an unproved frame
    condition. -/
theorem an_exposed_base_never_admits_a_custody_read (r : Read) :
    admitsCustodyRead r false = false := by
  simp [admitsCustodyRead]

theorem premise_f_is_required (r : Read) : admitsCustodyRead r false = false := by
  simp [admitsCustodyRead]

/-- And on an exposed base the union collapses to exactly the borrowed half. -/
theorem an_exposed_base_admits_only_the_borrowed_half (r : Read) :
    admitsAnyRead r false = admitsRead r := by
  simp [admitsAnyRead, premise_f_is_required]

/-- The four positions neither half admits, quantified so a later widening
    cannot slip one past. -/
theorem no_owned_stored_removed_or_out_position_takes_custody (r : Read) (insulated : Bool)
    (mode : r.position = Position.owned ∨ r.position = Position.stored
      ∨ r.position = Position.removed ∨ r.position = Position.out) :
    admitsCustodyRead r insulated = false := by
  rcases mode with h | h | h | h <;> simp [admitsCustodyRead, custodyPosition, h]

theorem no_refused_position_is_admitted_by_either_half (r : Read) (insulated : Bool)
    (mode : r.position = Position.owned ∨ r.position = Position.stored
      ∨ r.position = Position.removed ∨ r.position = Position.out) :
    admitsAnyRead r insulated = false := by
  have borrowed : admitsRead r = false := by
    apply a_custody_bearing_position_never_admits_a_read
    rcases mode with h | h | h | h <;> simp [h]
  simp [admitsAnyRead, borrowed,
    no_owned_stored_removed_or_out_position_takes_custody r insulated mode]

/-- The two halves are disjoint on position, so no read is admitted twice and
    the union is unambiguous about which effect applies. -/
theorem the_two_read_halves_are_disjoint (r : Read) (insulated : Bool)
    (borrowed : admitsRead r = true) : admitsCustodyRead r insulated = false := by
  have position : r.position = Position.borrowed := an_admitted_read_is_borrowed r borrowed
  simp [admitsCustodyRead, custodyPosition, position]

/-- Every base premise still refuses on its own in the custody half. -/
theorem an_aliased_base_never_admits_a_custody_read (r : Read) (insulated : Bool)
    (aliased : r.baseAliased = true) : admitsCustodyRead r insulated = false := by
  simp [admitsCustodyRead, readBaseOk, aliased]

theorem an_unacquired_base_never_admits_a_custody_read (r : Read) (insulated : Bool)
    (read : r.baseAcquired = false) : admitsCustodyRead r insulated = false := by
  simp [admitsCustodyRead, readBaseOk, read]

theorem a_scalar_member_never_admits_a_custody_read (r : Read) (insulated : Bool)
    (scalar : r.member = MemberKind.scalar) : admitsCustodyRead r insulated = false := by
  simp [admitsCustodyRead, readMemberOk, scalar]

/-- Card **ALIAS-AGGREGATE**: an opaque member is refused in the custody half
    too, insulated base or not, so no `void *` member can ever hand an identity
    to a callee. -/
theorem an_opaque_member_never_admits_a_custody_read (r : Read) (insulated : Bool)
    (isOpaque : r.member = MemberKind.opaquePointer) : admitsCustodyRead r insulated = false := by
  simp [admitsCustodyRead, readMemberOk, isOpaque]

/-- The complete statement of the asymmetry over the implementation's own
    decision: an opaque member is admitted by neither read half. -/
theorem an_opaque_member_is_admitted_by_no_read_half (r : Read) (insulated : Bool)
    (isOpaque : r.member = MemberKind.opaquePointer) : admitsAnyRead r insulated = false := by
  simp [admitsAnyRead, an_opaque_member_never_admits_a_read r isOpaque,
    an_opaque_member_never_admits_a_custody_read r insulated isOpaque]

/-! ### What an admitted custody read does

The transfer applies the declared effect to the linked child and then **drops
the link**: the member no longer holds anything this projection may denote. A
second read of the same member therefore fails the flow-sensitive gate rather
than releasing the same identity twice, and an escape of the aggregate no longer
cascades to a child it no longer holds. -/

/-- The declared effect of the two admitted custody positions. -/
def custodyEffect (p : Position) (child : Disposition) : Disposition :=
  if p == Position.released then Disposition.released
  else if p == Position.consumed then Disposition.escaped
  else child

/-- What the member holds afterwards: nothing this projection may denote. -/
def linkAfterCustodyRead (l : Link) : Link :=
  { l with bound := false, nonEmpty := false, agreesWithFieldLink := false }

theorem a_custody_read_releases_at_a_released_position (child : Disposition) :
    custodyEffect Position.released child = Disposition.released := by
  rfl

theorem a_custody_read_escapes_at_a_consumed_position (child : Disposition) :
    custodyEffect Position.consumed child = Disposition.escaped := by
  rfl

/-- The double-release row: after a custody read the member resolves no more. -/
theorem a_second_read_of_a_transferred_member_never_resolves (o : Owner) (l : Link) :
    readResolves o (linkAfterCustodyRead l) = false := by
  simp [readResolves, linkAfterCustodyRead]

/-- The asymmetry this card rests on, stated directly: the borrowed effect is
    the identity on every disposition, and the custody effect is not. That is
    why the borrowed half needs no frame condition and this half does. -/
theorem only_the_custody_effect_can_change_a_disposition :
    borrowedEffect Disposition.live = Disposition.live
      ∧ custodyEffect Position.released Disposition.live ≠ Disposition.live := by
  exact ⟨rfl, by simp [custodyEffect]⟩

/-! ### Non-vacuity of the custody half -/

def custodyExample : Read := { admittedExample with position := Position.released }

theorem the_custody_whitelist_is_not_vacuous :
    admitsCustodyRead custodyExample true = true := by
  rfl

theorem the_uninsulated_custody_read_is_refused :
    admitsCustodyRead custodyExample false = false := by
  rfl

theorem the_borrowed_half_refuses_the_custody_example :
    admitsRead custodyExample = false := by
  rfl

theorem one_scalar_member_flips_the_custody_read :
    admitsCustodyRead { custodyExample with member := MemberKind.scalar } true = false := by
  rfl

theorem one_owned_position_flips_the_custody_read :
    admitsCustodyRead { custodyExample with position := Position.owned } true = false := by
  rfl

end QCore.BoundFieldRead
