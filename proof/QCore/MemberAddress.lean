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
import QCore.BoundFieldRead

/-!
# Handing a callee the address of a member (card O04-B09B10, O00 blockers B09/B10)

`QCore.BorrowedActual` (card C02-P) and `QCore.BoundFieldRead` (card
O04-READ-BOUND) model member *reads*. Neither admits `& BASE - > FIELD`: the
lexical gate in `crates/qcore-c-gateway/src/legacy_c.rs` refuses any statement
carrying `->` outside an admitted span, so
`msgpack_packer_init(&ic->mp_pck, ic, …)`, `mk_list_add(&ic->_head, &in->chunks)`
and `pthread_mutex_lock(&in->metrics_chunk_lock)` all refused — O00's blockers
**B09** and **B10**, and seven of the eight refusals card O04-READ-BOUND left on
the constructor's success exit.

This module models the decision
`crates/qcore-c-gateway/src/aggregate_store.rs::member_address_actuals` makes,
together with the owner-liveness half of `legacy_c::transfer`.

## The premise that makes it safe

A member declared with **no** `*` is a sub-object of the record. Under the C
effective types premise this whole family inherits, its storage is disjoint from
every *other* member of the same `struct`, and this projection links only
single-`->` members of the base. So a callee holding `&BASE->FIELD` can reach no
field link at all. A **pointer** member's address is the opposite: it is a handle
with which the callee could overwrite the very link the projection maintains, and
it is refused.

The position must be `Borrowed`, exactly as in the borrowed read direction and
for the same reason: a borrowed actual discharges nothing, so admitting one can
only withdraw a coverage refusal.

Two directions are proved, and both matter.

* **Refusal.** Every premise refuses on its own; a refused address never reaches
  the callee; a bound base that does not resolve *here* — released, escaped,
  ambiguous, nullable, caller-held — refuses, which is card C00's lesson in this
  direction.
* **Non-vacuity.** An admitted address exists on both base routes, each premise
  flips it, and the admitted argument shape is proved to preserve premise **F**
  of `QCore.BoundFieldRead` while a pointer member's does not.

This is a model of the *decision*, not of C semantics. That the reader extracts
these declaration facts faithfully, that a declared `Borrowed` mode really
borrows, and C's object model itself, are premises supplied from outside.
Nothing here is a claim about arbitrary C memory safety.
-/

namespace QCore.MemberAddress

open QCore.AggregateStore
open QCore.BoundFieldRead

/-- Which of the two base routes the address rests on. -/
inductive BaseRoute where
  /-- A `store_bases` base this body bound: `legacy_c::transfer` must find it
      live *here*. -/
  | bound
  /-- A C02-P `untracked_bases` name this body can never bind to a tracked
      identity: there is no identity, so there is nothing to be live. -/
  | untracked
  /-- Neither route accepts the base. -/
  | refused
  deriving DecidableEq

/-- The member shapes this rule separates. `QCore.AggregateStore.MemberKind`
    classifies members that can hold a *value*; a member declared with no `*` is
    a sub-object, which that vocabulary calls `refusedShape` because it can
    never be read as one. This rule is about its **address**, so it needs its
    own classification. -/
inductive AddressedMember where
  /-- Declared with no `*`, no array modifier and no rejecting qualifier, over a
      specifier list the reader resolved to an arithmetic/enumeration type or to
      a `struct` tag. Its storage is disjoint from every linkable member. -/
  | nonPointer
  /-- Declared with at least one `*`. Its address is a handle to a link. -/
  | pointer
  /-- An array, a bit-field, a `union`-typed member, a qualified member or a
      typedef the reader will not resolve. -/
  | refusedShape
  deriving DecidableEq

/-- One candidate member-address actual, as the reader and the base scan
    describe it. -/
structure Address where
  /-- Every declaration of the enclosing function was read. -/
  declarationsRead : Bool
  /-- The argument is exactly `& BASE - > FIELD` and is the **whole** argument:
      no cast, no parentheses, no arithmetic, no chain, no subscript. -/
  wholeAddressOfSelection : Bool
  /-- The base's declared type is exactly `struct TAG *`. -/
  baseIsRecordPointer : Bool
  /-- `TAG` has one complete, non-`union` definition whose every member
      declaration was read. -/
  recordComplete : Bool
  member : AddressedMember
  /-- The callee has a contract in force. -/
  calleeContracted : Bool
  position : Position
  route : BaseRoute
  deriving DecidableEq

/-- The member premise: only a member with no `*`. -/
def addressedMemberOk (a : Address) : Bool := a.member == AddressedMember.nonPointer

/-- The base premise: one of the two routes accepts it. -/
def routeOk (a : Address) : Bool := a.route != BaseRoute.refused

/-- The whitelist. Every clause is a conjunct, so failing any one refuses. -/
def admitsAddress (a : Address) : Bool :=
  a.declarationsRead
    && a.wholeAddressOfSelection
    && a.baseIsRecordPointer
    && a.recordComplete
    && addressedMemberOk a
    && a.calleeContracted
    && positionOk a.position
    && routeOk a

/-! ## Every premise refuses on its own -/

theorem unread_declarations_never_admit_an_address (a : Address)
    (unread : a.declarationsRead = false) : admitsAddress a = false := by
  simp [admitsAddress, unread]

theorem a_partial_address_never_admits (a : Address)
    (fragment : a.wholeAddressOfSelection = false) : admitsAddress a = false := by
  simp [admitsAddress, fragment]

theorem a_non_record_pointer_base_never_admits_an_address (a : Address)
    (mismatch : a.baseIsRecordPointer = false) : admitsAddress a = false := by
  simp [admitsAddress, mismatch]

theorem an_incomplete_record_never_admits_an_address (a : Address)
    (incomplete : a.recordComplete = false) : admitsAddress a = false := by
  simp [admitsAddress, incomplete]

/-- The premise the whole rule rests on: a pointer member's address is a handle
    with which the callee could overwrite a field link. -/
theorem a_pointer_member_address_is_never_admitted (a : Address)
    (pointer : a.member = AddressedMember.pointer) : admitsAddress a = false := by
  simp [admitsAddress, addressedMemberOk, pointer]

theorem a_refused_member_shape_never_admits_an_address (a : Address)
    (refused : a.member = AddressedMember.refusedShape) : admitsAddress a = false := by
  simp [admitsAddress, addressedMemberOk, refused]

theorem an_uncontracted_callee_never_admits_an_address (a : Address)
    (uncontracted : a.calleeContracted = false) : admitsAddress a = false := by
  simp [admitsAddress, uncontracted]

/-- A base neither route accepts — one bound more than once, one denoted by a
    second declared name, one this body may bind to an identity without
    `store_bases` accepting it — refuses. -/
theorem a_refused_base_never_admits_an_address (a : Address)
    (refused : a.route = BaseRoute.refused) : admitsAddress a = false := by
  simp [admitsAddress, routeOk, refused]

/-- Only a `Borrowed` position, quantified over the whole vocabulary so a later
    widening cannot slip a mode past it. Releasing a sub-object of a live
    aggregate is not a release this projection could account for. -/
theorem a_custody_bearing_position_never_admits_an_address (a : Address)
    (custody : a.position ≠ Position.borrowed) : admitsAddress a = false := by
  have refused : positionOk a.position = false := by
    cases hp : a.position
    case borrowed => exact absurd hp custody
    all_goals rfl
  simp [admitsAddress, refused]

theorem an_admitted_address_is_borrowed (a : Address) (admitted : admitsAddress a = true) :
    a.position = Position.borrowed := by
  cases hp : a.position
  case borrowed => rfl
  all_goals simp [admitsAddress, positionOk, hp] at admitted

theorem no_owned_consumed_released_stored_removed_or_out_position_admits_an_address
    (a : Address)
    (custody : a.position = Position.owned ∨ a.position = Position.consumed
      ∨ a.position = Position.released ∨ a.position = Position.stored
      ∨ a.position = Position.removed ∨ a.position = Position.out) :
    admitsAddress a = false := by
  apply a_custody_bearing_position_never_admits_an_address
  rcases custody with h | h | h | h | h | h <;> simp [h]

theorem an_admitted_address_has_a_non_pointer_member (a : Address)
    (admitted : admitsAddress a = true) : a.member = AddressedMember.nonPointer := by
  cases hm : a.member
  case nonPointer => rfl
  all_goals simp [admitsAddress, addressedMemberOk, hm] at admitted

/-! ## The flow-sensitive premise

A bound base must resolve *here* to exactly one live identity this body
acquired. Handing a callee the address of a member of a released, escaped,
ambiguous, nullable or caller-held aggregate is refused — card C00's lesson in
this direction. An untracked base carries no identity at all, so there is
nothing to resolve. -/

def addressResolves (a : Address) (o : Owner) : Bool :=
  match a.route with
  | BaseRoute.bound => ownerResolves o
  | BaseRoute.untracked => true
  | BaseRoute.refused => false

theorem an_address_through_a_released_aggregate_never_resolves (a : Address) (o : Owner)
    (bound : a.route = BaseRoute.bound) (released : o.status = Disposition.released) :
    addressResolves a o = false := by
  simp [addressResolves, bound, ownerResolves, released]

theorem an_address_through_a_caller_held_aggregate_never_resolves (a : Address) (o : Owner)
    (bound : a.route = BaseRoute.bound) (caller : o.callerHeld = true) :
    addressResolves a o = false := by
  simp [addressResolves, bound, ownerResolves, caller]

theorem an_address_through_an_ambiguous_aggregate_never_resolves (a : Address) (o : Owner)
    (bound : a.route = BaseRoute.bound) (many : o.unique = false) :
    addressResolves a o = false := by
  simp [addressResolves, bound, ownerResolves, many]

theorem an_address_through_a_nullable_aggregate_never_resolves (a : Address) (o : Owner)
    (bound : a.route = BaseRoute.bound) (nullable : o.nonNull = false) :
    addressResolves a o = false := by
  simp [addressResolves, bound, ownerResolves, nullable]

/-- An untracked base needs no liveness check, because it carries no identity
    this body could have released. -/
theorem an_untracked_base_always_resolves (a : Address) (o : Owner)
    (untracked : a.route = BaseRoute.untracked) : addressResolves a o = true := by
  simp [addressResolves, untracked]

/-! ## The transfer -/

inductive AddressOutcome where
  /-- The argument is handed to the callee, and the declared `Borrowed` mode
      applies to it — which discharges nothing and changes no field link. -/
  | handed
  /-- The address is refused: the path is poisoned and can support no custody
      claim. -/
  | refused
  deriving DecidableEq

def addressStep (a : Address) (o : Owner) : AddressOutcome :=
  if admitsAddress a && addressResolves a o then AddressOutcome.handed
  else AddressOutcome.refused

theorem a_refused_address_is_never_handed (a : Address) (o : Owner)
    (refused : admitsAddress a = false) : addressStep a o = AddressOutcome.refused := by
  simp [addressStep, refused]

theorem an_unresolved_address_is_never_handed (a : Address) (o : Owner)
    (refused : addressResolves a o = false) : addressStep a o = AddressOutcome.refused := by
  simp [addressStep, refused]

/-- Both gates are required, and when an address is handed over its position is
    `borrowed` and its member is a non-pointer. -/
theorem only_a_doubly_admitted_address_is_handed (a : Address) (o : Owner)
    (handed : addressStep a o = AddressOutcome.handed) :
    admitsAddress a = true ∧ addressResolves a o = true
      ∧ a.position = Position.borrowed ∧ a.member = AddressedMember.nonPointer := by
  unfold addressStep at handed
  split at handed
  · rename_i gates
    simp only [Bool.and_eq_true] at gates
    obtain ⟨admitted, resolved⟩ := gates
    exact ⟨admitted, resolved, an_admitted_address_is_borrowed a admitted,
      an_admitted_address_has_a_non_pointer_member a admitted⟩
  · exact absurd handed (by simp)

/-! ## What an admitted address does, and does not, do

The declared `Borrowed` effect is the identity on every disposition, and the
member the callee receives is disjoint from every member this projection links.
So an admitted address changes neither a disposition nor a field link — which is
what makes the admission structurally incapable of manufacturing a discharge. -/

theorem an_admitted_address_changes_no_disposition (child : Disposition) :
    borrowedEffect child = child := by
  rfl

theorem an_admitted_address_never_discharges (child : Disposition)
    (live : child = Disposition.live) : borrowedEffect child = Disposition.live := by
  simp [borrowedEffect, live]

/-- The field link the projection maintains for a *pointer* member is untouched
    by an admitted address, because an admitted address is never a pointer
    member's. Stated as the disjointness it is. -/
theorem an_admitted_address_names_no_linkable_member (a : Address)
    (admitted : admitsAddress a = true) : a.member ≠ AddressedMember.pointer := by
  rw [an_admitted_address_has_a_non_pointer_member a admitted]
  simp

/-! ## Interaction with premise F of `QCore.BoundFieldRead`

Premise **F** asks whether any contracted call can leave the aggregate both live
and rewritable. An admitted member address does not: the callee receives storage
disjoint from every linkable member. A *pointer* member's address does, which is
why the same member classification decides both rules. -/

/-- Whether an argument mentioning the base preserves premise **F**. -/
def preservesInsulation (a : Address) : Bool := admitsAddress a

theorem an_admitted_address_preserves_insulation (a : Address)
    (admitted : admitsAddress a = true) : preservesInsulation a = true := by
  simpa [preservesInsulation] using admitted

theorem a_pointer_member_address_breaks_insulation (a : Address)
    (pointer : a.member = AddressedMember.pointer) : preservesInsulation a = false := by
  simpa [preservesInsulation] using a_pointer_member_address_is_never_admitted a pointer

/-! ## Non-vacuity, in both directions -/

/-- The admitted address `msgpack_packer_init(&ic->mp_pck, …)` needs: a
    non-pointer member of a bound local aggregate, whole, at a borrowed
    position. -/
def boundExample : Address :=
  { declarationsRead := true
    wholeAddressOfSelection := true
    baseIsRecordPointer := true
    recordComplete := true
    member := AddressedMember.nonPointer
    calleeContracted := true
    position := Position.borrowed
    route := BaseRoute.bound }

/-- The C02-P route: `pthread_mutex_lock(&in->metrics_chunk_lock)`. -/
def untrackedExample : Address := { boundExample with route := BaseRoute.untracked }

theorem the_address_whitelist_is_not_vacuous : admitsAddress boundExample = true := by
  rfl

theorem the_untracked_route_is_not_vacuous : admitsAddress untrackedExample = true := by
  rfl

theorem the_bound_address_is_handed : addressStep boundExample liveOwner
    = AddressOutcome.handed := by
  rfl

theorem the_untracked_address_needs_no_live_owner :
    addressStep untrackedExample { liveOwner with status := Disposition.released }
      = AddressOutcome.handed := by
  rfl

theorem one_pointer_member_flips_the_admitted_address :
    admitsAddress { boundExample with member := AddressedMember.pointer } = false := by
  rfl

theorem one_releasing_position_flips_the_admitted_address :
    admitsAddress { boundExample with position := Position.released } = false := by
  rfl

theorem one_refused_base_flips_the_admitted_address :
    admitsAddress { boundExample with route := BaseRoute.refused } = false := by
  rfl

theorem one_partial_argument_flips_the_admitted_address :
    admitsAddress { boundExample with wholeAddressOfSelection := false } = false := by
  rfl

/-- The flow-sensitive gate is doing work too: the same admitted address refuses
    against a released aggregate on the bound route. -/
theorem a_released_owner_flips_the_bound_address :
    addressStep boundExample { liveOwner with status := Disposition.released }
      = AddressOutcome.refused := by
  rfl

theorem an_ambiguous_owner_flips_the_bound_address :
    addressStep boundExample { liveOwner with unique := false }
      = AddressOutcome.refused := by
  rfl

end QCore.MemberAddress
