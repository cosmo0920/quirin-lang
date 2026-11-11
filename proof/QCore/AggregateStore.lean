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

import QCore.ExitResiduals

/-!
# Aggregate field stores and the containment edge (O00 blocker B08, card O04-B08)

`QCore.TypedFieldAccess` (card C02) and `QCore.ChainedAccess` (card O04-ARMS)
model *reads* through a typed member access. Both deliberately left the
**destination** side of the lexical gate in `crates/qcore-c-gateway/src/legacy_c.rs`
untouched, and `docs/reports/remaining-o04-arms/README.md` §3.2 records that as
one of the two reasons the caller's routes-mask arm C5 stays `Unknown`.

This module models the replacement decided by
`crates/qcore-c-gateway/src/aggregate_store.rs` together with the field-link
half of `legacy_c::transfer`:

* **`admitsStore`** — the conjunctive whitelist for a destination
  `BASE -> FIELD`, over declaration facts the reader supplies;
* **`ownerResolves`** — the flow-sensitive premise the transfer function adds:
  the aggregate must be exactly one *live* identity this body acquired, never a
  caller-held one. This is card C02's fail-closed premise in its **store**
  direction, and it is the one that refuses a store through a released
  aggregate;
* **`childAfter`** — what an admitted store's **containment edge** does when the
  aggregate is later escaped or released.

## What the containment edge is, and is not

An admitted store records a *field link* from the aggregate's identity to the
stored one. It is deliberately **not** the `contained` relation a declared
`Stored` contract parameter creates:

* escaping the aggregate carries its live linked children away, so publishing an
  aggregate does not report what it reaches as a leak;
* **releasing** the aggregate does *not* cascade. A declared `resource_released`
  release names exactly one identity, and a plain C field store carries no
  declared destructor obligation, so a child still live there stays live and is
  reported as the residual it is. This is premise **P-B08-SHALLOW**; it is an
  explicit assumption about what a declared release means, and it is **not
  proved here**;
* overwriting a member replaces its link and leaves the previously stored
  identity exactly as it was, so a second store never discharges the first.

Two directions are proved, and both matter.

* **Refusal.** Every premise refuses on its own, and a refused store never links
  anything, never settles an exit and never alleges a defect.
* **Non-vacuity.** An admitted store exists, each premise flips it, and the
  escape half really does move a live child.

This is a model of the *decision*, not of C semantics. That the reader extracts
these declaration facts faithfully, and that a declared release names exactly
one identity, are premises supplied from outside and assumed here, exactly as in
`QCore.TypedFieldAccess`. Nothing here relates two distinct declared names that
may denote the same object; the recorded C02-P may-alias boundary is inherited
unchanged and is not widened. Nothing here is a claim about arbitrary C memory
safety.
-/

namespace QCore.AggregateStore

open QCore.LegacyCAnalyzer
open QCore.ExitResiduals

/-! ## The destination whitelist -/

/-- How the stored-into member is declared, as far as the reader commits. -/
inductive MemberKind where
  /-- A non-pointer arithmetic or enumeration member. It can never hold a
      tracked identity. -/
  | scalar
  /-- Exactly one `*` over a specifier list the reader resolved to a `struct`
      tag or to an arithmetic type. It holds exactly one pointer value. -/
  | singlePointer
  /-- Card **ALIAS-AGGREGATE**: exactly `void *` — the literal one-token `void`
      specifier list with exactly one star, unqualified. It holds exactly one
      pointer value, so a *store* into it replaces exactly one identity, but it
      names no pointee, so no *read* of it can be matched against a callee's
      declared parameter type. Admitted as a store destination and refused as a
      bound read (`QCore.BoundFieldRead.admitsRead` requires `singlePointer`). -/
  | opaquePointer
  /-- `const void *`, `void **`, two or more stars, an array, a function
      pointer, a bit-field, an aggregate sub-object, or a typedef the reader
      will not resolve. Never admitted. -/
  | refusedShape
  deriving DecidableEq

/-- What the store writes into the member. -/
inductive Value where
  /-- A value that carries no tracked identity: an admitted scalar expression,
      or a declared pointer this function never binds. -/
  | untracked
  /-- The null pointer constant. -/
  | null
  /-- A place this function may bind to a tracked identity. -/
  | tracked
  deriving DecidableEq

/-- One candidate aggregate store, as the reader and the base scan describe it. -/
structure Store where
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
  /-- That single binding *calls* something, so the aggregate is one this body
      obtained rather than one it read out of another object's memory. -/
  baseAcquired : Bool
  /-- The base is address-taken whole, incremented, compound-assigned,
      dereferenced, subscripted or `.`-selected somewhere in the body. -/
  baseEscapesPointer : Bool
  /-- The base is the entire right-hand side of an assignment, or a whole
      argument at an `Out`, `Stored` or `Removed` position of a contracted
      call — either of which could give a second name to the same aggregate or
      change it invisibly. -/
  baseAliased : Bool
  /-- The destination is exactly `BASE - > FIELD`: one selection, written `->`,
      with nothing before or after it. -/
  oneSelection : Bool
  member : MemberKind
  /-- The member carries `volatile` or `_Atomic`. -/
  qualified : Bool
  value : Value
  /-- Whether the stored variable's declared type is **exactly** the member's
      declared pointee type. A `singlePointer` member requires it; an
      `opaquePointer` member has no declared pointee, so there is nothing to
      match and the requirement is replaced by
      [`valueIsDeclaredRecordPointer`] (card **ALIAS-AGGREGATE**). -/
  valueTagMatches : Bool
  /-- Whether the stored value is a single declared name whose declared type is
      `struct TAG *` for *some* tag. This is what an `opaquePointer` member
      requires instead of an exact match: the value must be one pointer
      identity this projection can name. A cast, a call, an address-of, a
      scalar and every non-record pointer make it `false`. -/
  valueIsDeclaredRecordPointer : Bool
  deriving DecidableEq

/-- The member premise: an admitted member kind, unqualified. -/
def memberOk (s : Store) : Bool :=
  (s.member == MemberKind.scalar
      || s.member == MemberKind.singlePointer
      || s.member == MemberKind.opaquePointer)
    && !s.qualified

/-- The value premise. A scalar member can never hold an identity, so only an
    untracked value may reach it; a `singlePointer` member additionally admits
    the null constant and a declared pointer of *exactly* the member's declared
    type; an `opaquePointer` member admits the null constant and any declared
    record pointer, because it has no declared pointee to match. -/
def valueOk (s : Store) : Bool :=
  match s.value with
  | Value.untracked => true
  | Value.null =>
      s.member == MemberKind.singlePointer || s.member == MemberKind.opaquePointer
  | Value.tracked =>
      (s.member == MemberKind.singlePointer && s.valueTagMatches)
        || (s.member == MemberKind.opaquePointer && s.valueIsDeclaredRecordPointer)

/-- The whitelist. Every clause is a conjunct, so failing any one refuses. -/
def admitsStore (s : Store) : Bool :=
  s.declarationsRead
    && s.baseIsLocal
    && s.baseIsRecordPointer
    && s.recordComplete
    && s.baseBoundOnce
    && s.baseAcquired
    && !s.baseEscapesPointer
    && !s.baseAliased
    && s.oneSelection
    && memberOk s
    && valueOk s

/-! ## Every premise refuses on its own -/

theorem unread_declarations_never_admit_a_store (s : Store)
    (unread : s.declarationsRead = false) : admitsStore s = false := by
  simp [admitsStore, unread]

theorem a_formal_base_never_admits_a_store (s : Store)
    (formal : s.baseIsLocal = false) : admitsStore s = false := by
  simp [admitsStore, formal]

theorem a_non_record_pointer_base_never_admits_a_store (s : Store)
    (mismatch : s.baseIsRecordPointer = false) : admitsStore s = false := by
  simp [admitsStore, mismatch]

theorem an_incomplete_record_never_admits_a_store (s : Store)
    (incomplete : s.recordComplete = false) : admitsStore s = false := by
  simp [admitsStore, incomplete]

theorem a_rebound_base_never_admits_a_store (s : Store)
    (rebound : s.baseBoundOnce = false) : admitsStore s = false := by
  simp [admitsStore, rebound]

/-- The premise that keeps card C02-P's pin
    `a_store_through_the_field_between_the_read_and_the_call_is_never_admitted`
    refusing: an aggregate read out of another object's memory is not one this
    body acquired, so no store through it is admitted. -/
theorem an_unacquired_base_never_admits_a_store (s : Store)
    (read : s.baseAcquired = false) : admitsStore s = false := by
  simp [admitsStore, read]

theorem an_escaping_base_pointer_never_admits_a_store (s : Store)
    (escapes : s.baseEscapesPointer = true) : admitsStore s = false := by
  simp [admitsStore, escapes]

/-- The card's "no aliasing of the destination" premise. -/
theorem an_aliased_base_never_admits_a_store (s : Store)
    (aliased : s.baseAliased = true) : admitsStore s = false := by
  simp [admitsStore, aliased]

theorem a_chained_destination_never_admits_a_store (s : Store)
    (chained : s.oneSelection = false) : admitsStore s = false := by
  simp [admitsStore, chained]

theorem a_refused_member_shape_never_admits_a_store (s : Store)
    (refused : s.member = MemberKind.refusedShape) : admitsStore s = false := by
  simp [admitsStore, memberOk, refused]

theorem a_qualified_member_never_admits_a_store (s : Store)
    (qualified : s.qualified = true) : admitsStore s = false := by
  simp [admitsStore, memberOk, qualified]

/-- A scalar member can never hold a tracked identity, so a tracked value into
    one is refused rather than modeled. -/
theorem a_tracked_value_never_reaches_a_scalar_member (s : Store)
    (scalar : s.member = MemberKind.scalar) (tracked : s.value = Value.tracked) :
    admitsStore s = false := by
  simp [admitsStore, valueOk, scalar, tracked]

theorem a_null_value_never_reaches_a_scalar_member (s : Store)
    (scalar : s.member = MemberKind.scalar) (isNull : s.value = Value.null) :
    admitsStore s = false := by
  simp [admitsStore, valueOk, scalar, isNull]

/-! ## The flow-sensitive owner premise -/

/-- The disposition of one abstract site, as the residual ledger reads it. -/
inductive Disposition where
  | live
  | released
  | escaped
  | contained
  /-- The site left the state on this path — a refuted nullable acquisition. -/
  | absent
  /-- An effect this projection could not resolve reached the site. -/
  | unresolved
  deriving DecidableEq

/-- What the projection knows about the aggregate's own place *at the store*. -/
structure Owner where
  /-- The projection binds the base here. -/
  bound : Bool
  /-- The base denotes exactly one abstract site. -/
  unique : Bool
  /-- The base's abstract pointer is not `unknown`. -/
  known : Bool
  /-- Neither the pointer nor its site is possibly null. -/
  nonNull : Bool
  /-- The site is entry custody the *caller* holds. A transfer into a
      caller-owned aggregate is invisible at the call site and is refused
      rather than modeled. -/
  callerHeld : Bool
  status : Disposition
  deriving DecidableEq

def ownerResolves (o : Owner) : Bool :=
  o.bound && o.unique && o.known && o.nonNull && !o.callerHeld
    && o.status == Disposition.live

theorem an_unbound_aggregate_never_resolves (o : Owner) (unbound : o.bound = false) :
    ownerResolves o = false := by
  simp [ownerResolves, unbound]

theorem an_ambiguous_aggregate_never_resolves (o : Owner) (many : o.unique = false) :
    ownerResolves o = false := by
  simp [ownerResolves, many]

theorem an_unknown_aggregate_never_resolves (o : Owner) (unknown : o.known = false) :
    ownerResolves o = false := by
  simp [ownerResolves, unknown]

theorem a_possibly_null_aggregate_never_resolves (o : Owner)
    (nullable : o.nonNull = false) : ownerResolves o = false := by
  simp [ownerResolves, nullable]

theorem a_caller_held_aggregate_never_resolves (o : Owner)
    (caller : o.callerHeld = true) : ownerResolves o = false := by
  simp [ownerResolves, caller]

/-- Card C02's fail-closed premise in its **store** direction: an aggregate this
    body already released is not live, so the store refuses. The read direction
    is `QCore.TypedFieldAccess`; this is its mirror. -/
theorem a_released_aggregate_never_resolves (o : Owner)
    (released : o.status = Disposition.released) : ownerResolves o = false := by
  simp [ownerResolves, released]

theorem an_escaped_aggregate_never_resolves (o : Owner)
    (escaped : o.status = Disposition.escaped) : ownerResolves o = false := by
  simp [ownerResolves, escaped]

theorem an_unresolved_aggregate_never_resolves (o : Owner)
    (unresolved : o.status = Disposition.unresolved) : ownerResolves o = false := by
  simp [ownerResolves, unresolved]

/-! ## The transfer -/

inductive Outcome where
  /-- The member now links the stored identity. -/
  | linked
  /-- The member links nothing; whatever it previously linked is unlinked. -/
  | unlinked
  /-- The store is refused: the path is poisoned and can support no custody
      claim. -/
  | refused
  deriving DecidableEq

def storeStep (s : Store) (o : Owner) : Outcome :=
  if admitsStore s && ownerResolves o then
    match s.value with
    | Value.tracked => Outcome.linked
    | Value.null => Outcome.unlinked
    | Value.untracked => Outcome.unlinked
  else
    Outcome.refused

/-- Fail-closed direction one: a store the whitelist refuses never links. -/
theorem a_refused_store_never_links (s : Store) (o : Owner)
    (refused : admitsStore s = false) : storeStep s o = Outcome.refused := by
  simp [storeStep, refused]

/-- Fail-closed direction two: a store whose aggregate does not resolve here and
    now never links, whatever the declarations say. -/
theorem an_unresolved_owner_never_links (s : Store) (o : Owner)
    (refused : ownerResolves o = false) : storeStep s o = Outcome.refused := by
  simp [storeStep, refused]

/-- The two directions together: only a store that passes *both* gates links. -/
theorem only_a_doubly_admitted_store_links (s : Store) (o : Owner)
    (linked : storeStep s o = Outcome.linked) :
    admitsStore s = true ∧ ownerResolves o = true ∧ s.value = Value.tracked := by
  unfold storeStep at linked
  split at linked
  · rename_i gates
    cases hv : s.value <;> simp [hv] at linked
    simp_all
  · exact absurd linked (by simp)

/-- A store of a value that carries no tracked identity never links one; it can
    only *unlink* what the member held before. -/
theorem an_untracked_value_never_links (s : Store) (o : Owner)
    (untracked : s.value = Value.untracked) : storeStep s o ≠ Outcome.linked := by
  unfold storeStep
  split <;> simp [untracked]

theorem a_null_value_never_links (s : Store) (o : Owner)
    (isNull : s.value = Value.null) : storeStep s o ≠ Outcome.linked := by
  unfold storeStep
  split <;> simp [isNull]

/-! ## The containment edge: what happens to a linked child -/

/-- The two events that can reach the aggregate a member links from. -/
inductive OwnerEvent where
  /-- The aggregate is published: returned, or consumed by a contracted call. -/
  | escape
  /-- The aggregate is released by a declared `resource_released` destructor. -/
  | release
  deriving DecidableEq

/-- The linked child's disposition after the event, under premise
    **P-B08-SHALLOW**.

    Escaping publishes what the aggregate reaches, so a *live* child escapes
    with it and is not reported as a leak. A child in any other state is left
    exactly as it is and recorded unresolved: a member still pointing at a
    released identity when its aggregate escapes is a hazard this projection
    will not decide in either direction.

    Releasing does **not** cascade. A declared release names exactly one
    identity and a plain C field store carries no declared destructor
    obligation, so the child keeps its disposition — which is what makes a
    still-live child the reported residual it is. -/
def childAfter (e : OwnerEvent) (child : Disposition) : Disposition :=
  match e with
  | OwnerEvent.escape =>
    match child with
    | Disposition.live => Disposition.escaped
    | Disposition.escaped => Disposition.escaped
    | Disposition.absent => Disposition.absent
    | _ => Disposition.unresolved
  | OwnerEvent.release => child

/-- The containment edge's positive half: publishing the aggregate carries its
    live child away, so the success exit reports no leak for it. -/
theorem an_escaping_aggregate_carries_its_live_child :
    childAfter OwnerEvent.escape Disposition.live = Disposition.escaped := by
  rfl

/-- The escape cascade never manufactures a release: nothing it does can turn an
    unsettled identity into a discharged one. -/
theorem the_escape_cascade_never_releases (child : Disposition) :
    childAfter OwnerEvent.escape child ≠ Disposition.released := by
  cases child <;> simp [childAfter]

/-- A member left pointing at a released identity when its aggregate escapes is
    decided in neither direction. -/
theorem an_escaping_aggregate_leaves_a_released_child_unresolved :
    childAfter OwnerEvent.escape Disposition.released = Disposition.unresolved := by
  rfl

/-- P-B08-SHALLOW, stated as the theorem it is: releasing the aggregate changes
    nothing about what its members link. -/
theorem a_released_aggregate_changes_no_child (child : Disposition) :
    childAfter OwnerEvent.release child = child := by
  rfl

/-- Its consequence, and the reason the routes-mask arm settles only because the
    body released the chunk itself: releasing the aggregate never discharges a
    live child. -/
theorem a_released_aggregate_never_discharges_a_live_child :
    childAfter OwnerEvent.release Disposition.live = Disposition.live := by
  rfl

/-- Overwriting a member replaces its link and leaves the previously stored
    identity exactly as it was, so a second store never discharges the first —
    it stays live, and is reported. -/
def afterOverwrite (previous : Disposition) : Disposition := previous

theorem a_double_store_never_discharges_the_first_identity :
    afterOverwrite Disposition.live = Disposition.live := by
  rfl

theorem a_double_store_changes_no_disposition (previous : Disposition) :
    afterOverwrite previous = previous := by
  rfl

/-! ## Bridging into the exit ledger -/

/-- A refused store poisons its path, so every exit it reaches is `unknown` —
    never `discharged`, and never a `residual` claim in the defect direction. -/
theorem a_refused_store_settles_no_exit (e : ExitState) (poisoned : e.unmodeled = true) :
    exitCustody e = ExitCustody.unknown := by
  simp [exitCustody, poisoned]

/-- A live linked child left behind on a fully modeled path is a `residual`, not
    a discharge: the ledger reports it rather than explaining it away. -/
theorem a_live_linked_child_is_a_residual (e : ExitState)
    (modeled : e.unmodeled = false) (settled : e.mixed = false)
    (live : anyLive e.ledger = true) : exitCustody e = ExitCustody.residual := by
  simp [exitCustody, modeled, settled, live]

/-! ## Non-vacuity, in both directions -/

/-- The admitted store the routes-mask arm needs: a tracked identity into a
    single-pointer member of a local aggregate this body acquired. -/
def admittedExample : Store :=
  { declarationsRead := true
    baseIsLocal := true
    baseIsRecordPointer := true
    recordComplete := true
    baseBoundOnce := true
    baseAcquired := true
    baseEscapesPointer := false
    baseAliased := false
    oneSelection := true
    member := MemberKind.singlePointer
    qualified := false
    value := Value.tracked
    valueTagMatches := true
    valueIsDeclaredRecordPointer := true }

/-- Card **ALIAS-AGGREGATE**: the same admitted store through a `void *`
    member, whose value is a declared record pointer of an unrelated tag. -/
def opaqueExample : Store :=
  { admittedExample with
    member := MemberKind.opaquePointer
    valueTagMatches := false }

def liveOwner : Owner :=
  { bound := true
    unique := true
    known := true
    nonNull := true
    callerHeld := false
    status := Disposition.live }

theorem the_store_whitelist_is_not_vacuous : admitsStore admittedExample = true := by
  rfl

theorem the_admitted_store_links : storeStep admittedExample liveOwner = Outcome.linked := by
  rfl

/-- One flipped premise refuses the same store: the whitelist is doing work. -/
theorem one_refused_member_shape_flips_the_admitted_store :
    admitsStore { admittedExample with member := MemberKind.refusedShape } = false := by
  rfl

theorem one_unacquired_base_flips_the_admitted_store :
    admitsStore { admittedExample with baseAcquired := false } = false := by
  rfl

theorem one_aliased_base_flips_the_admitted_store :
    admitsStore { admittedExample with baseAliased := true } = false := by
  rfl

/-- And the flow-sensitive gate is doing work too: the same admitted store
    refuses against an aggregate this body already released. -/
theorem a_released_owner_flips_the_admitted_store :
    storeStep admittedExample { liveOwner with status := Disposition.released }
      = Outcome.refused := by
  rfl

theorem a_caller_held_owner_flips_the_admitted_store :
    storeStep admittedExample { liveOwner with callerHeld := true } = Outcome.refused := by
  rfl

/-! ## The opaque member (card ALIAS-AGGREGATE)

The third member class is admitted **only** as a store destination, and the
asymmetry is the whole of its soundness argument. What a link is used for —
escape cascade, overwrite replacement, and the deliberate non-cascade of a
release — reads the *identity*, never the member's declared type; the one use
that would need a declared type is the bound read, and
`QCore.BoundFieldRead.admitsRead` requires `singlePointer`, so an opaque member
can never occupy one. What the store still requires is that the value be a
pointer identity this projection can name.
-/

/-- Non-vacuity: a `void *` member with a record-pointer value of an unrelated
    tag is admitted, and links against a live aggregate. -/
theorem an_opaque_member_store_is_admitted : admitsStore opaqueExample = true := by
  rfl

theorem the_admitted_opaque_store_links :
    storeStep opaqueExample liveOwner = Outcome.linked := by
  rfl

/-- The exact-tag premise is **not** what the opaque member requires: the same
    store is admitted with `valueTagMatches := false`, which is exactly the
    production shape (`void *chunk` receiving a `struct cio_chunk *`). -/
theorem the_opaque_member_does_not_require_an_exact_tag :
    admitsStore { opaqueExample with valueTagMatches := false } = true := by
  rfl

/-- What replaces it. A value that is not a declared record pointer — a cast, a
    call, an address-of, a scalar, a non-record pointer — refuses. -/
theorem an_opaque_member_refuses_a_value_that_is_not_a_declared_record_pointer
    (s : Store) (isOpaque : s.member = MemberKind.opaquePointer)
    (tracked : s.value = Value.tracked)
    (notPointer : s.valueIsDeclaredRecordPointer = false) : admitsStore s = false := by
  simp [admitsStore, valueOk, isOpaque, tracked, notPointer]

/-- The typed member is unchanged in the other direction: it still requires the
    exact tag, so the opaque relaxation did not leak into it. -/
theorem a_single_pointer_member_still_requires_the_exact_tag (s : Store)
    (typed : s.member = MemberKind.singlePointer) (tracked : s.value = Value.tracked)
    (mismatch : s.valueTagMatches = false) : admitsStore s = false := by
  simp [admitsStore, valueOk, typed, tracked, mismatch]

/-- Every base and destination premise still refuses an opaque store: the third
    class widened the *member* premise and nothing else. -/
theorem an_opaque_member_store_keeps_every_base_premise (s : Store)
    (_isOpaque : s.member = MemberKind.opaquePointer) (aliased : s.baseAliased = true) :
    admitsStore s = false := by
  simp [admitsStore, aliased]

theorem an_opaque_member_store_keeps_the_qualifier_premise (s : Store)
    (_isOpaque : s.member = MemberKind.opaquePointer) (qualified : s.qualified = true) :
    admitsStore s = false := by
  simp [admitsStore, memberOk, qualified]

/-- The flow-sensitive owner premise is untouched: an opaque store through an
    aggregate this body already released refuses exactly as a typed one does. -/
theorem a_released_owner_flips_the_admitted_opaque_store :
    storeStep opaqueExample { liveOwner with status := Disposition.released }
      = Outcome.refused := by
  rfl

theorem a_caller_held_owner_flips_the_admitted_opaque_store :
    storeStep opaqueExample { liveOwner with callerHeld := true } = Outcome.refused := by
  rfl

/-- The member classes are mutually exclusive by construction, so no member is
    decided by two rules and admission cannot depend on the order in which the
    reader asks. This is the model half of the Rust `admitted_field` check that
    the three classifications sum to exactly one. -/
theorem the_member_classes_are_exclusive (k : MemberKind) :
    (k == MemberKind.scalar).toNat
        + (k == MemberKind.singlePointer).toNat
        + (k == MemberKind.opaquePointer).toNat
        + (k == MemberKind.refusedShape).toNat = 1 := by
  cases k <;> rfl

end QCore.AggregateStore
