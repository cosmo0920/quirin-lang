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

import QCore.IndirectChainAccess
import QCore.TypedPlace

/-!
# Pointer-valued stores into a caller's object (card CG11-POINTER-VALUED-STORES, rule 10v)

The abstract rule implemented by `crates/qcore-c-gateway/src/pointer_store.rs`
(`classify_value`, `decide`, `transfer`) and its hooks in
`typed_access_path.rs` (`Reader::statement`), `legacy_c.rs`
(`LegacyInstruction::PointerStore`) and `legacy_summary.rs`
(`observations_match_contract`).

A statement `P = V` whose place `P` has a pointer type was refused by every
route (W46's typed statement reader stores only scalars; W45's sub-object
store reads only a lexically resolved single member). This rule reads it from
the Clang AST and keeps four events apart:

* **the value store**: the write of `P`, with the per-hop obligations of the
  place (`IndirectChainAccess.resolves`, unchanged);
* **alias addition and overwrite**: the place's old bindings are forgotten; an
  admitted value adds no alias the projection tracks;
* **the old value's obligation**: a store is admitted only when the place's
  owner is *untracked* (no reaching path bound it, so it holds no field link
  and the body holds no obligation for its old value) and no binding of a
  place rooted at the destination carries a target. An owner that is one
  tracked identity may hold an owning field: the store refuses (the field-link
  store of O04-B08 is the only route that rewrites one);
* **escape into the owner**: only values that carry no identity are admitted
  (null, a string literal, a function, a name of function-pointer type that
  the flow proves carries none, and the address of the destination's own
  object). Nothing escapes, so no custody moves.

A formal's object pointer (Q36's example C, `s->p = q`) is refused: no contract
or tracked state gives the owner of `*s` or bounds `*q`'s lifetime by it, and
no summary mode states that transfer (the missing premise). A local's pointer
may carry an identity and is refused too; so are reads, calls, arithmetic and
any other address.

Premise P-CG11-CALLER-SLOT, stated, not proved: the old value of a slot in an
untracked object is not an obligation of this body. A body with an admitted
store publishes no derived summary, so no caller reads `Borrowed` across it.

## What this is not

A model of the decision. Faithful Clang extraction (types, cast kinds, DeclIDs)
and the Rust–Lean correspondence are review premises (trust boundary 28);
`decisionVectors` binds the Rust `decide` table mechanically
(`tests/unit/pointer_store.rs`).
-/

namespace QCore.PointerStore

open QCore.IndirectChainAccess (State Obligation Owner rootOwner walk resolves)

/-! ## The stored value -/

inductive Value where
  | null
  | string
  | function
  | functionPointerName
  | selfAddress
  | interiorAddress
  | formalPointer
  | localPointer
  | pointerRead
  | call
  | arithmetic
  | otherAddress
  | other
  deriving DecidableEq, Repr

/-- The classes whose value denotes no object with custody: static storage, a
function, or the destination's own object. -/
def Value.neutral : Value → Bool
  | .null | .string | .function | .functionPointerName | .selfAddress | .interiorAddress => true
  | _ => false

/-- The identities an admitted value adds to the projection: none. A name of
function-pointer type is admitted only when the flow shows it carries none
(`carries` below). -/
def storedIdentities (_ : Value) : List Nat := []

/-! ## The owner of the destination -/

inductive OwnerClass where
  | untracked
  | tracked
  | unresolved
  deriving DecidableEq, Repr

/-- The final owner of a destination's chain: the object the stored member
lives in (`chain_owner`). -/
def finalOwner (s : State) (c : Obligation) : Option Owner :=
  match rootOwner s c with
  | some owner => walk s c 0 c.links owner
  | none => none

theorem the_final_owner_decides_resolution (s : State) (c : Obligation) :
    resolves s c = (finalOwner s c).isSome := by
  unfold resolves finalOwner
  cases rootOwner s c <;> rfl

/-- Untracked only when the root and the member's object both are: a tracked
root's untracked child is not the caller's object the premise speaks of. -/
def ownerClass (s : State) (c : Obligation) : OwnerClass :=
  match rootOwner s c, finalOwner s c with
  | some .untracked, some .untracked => .untracked
  | some _, some _ => .tracked
  | _, _ => .unresolved

/-! ## The decision -/

inductive Decision where
  | admit
  | formalValue
  | namedValue
  | valueShape
  | ownedField
  | unresolvedOwner
  deriving DecidableEq, Repr

/-- `decide` in `pointer_store.rs` (`decideStore` here): the value's class, whether the flow found
an identity in a named value, and the owner. -/
def decideStore (v : Value) (carries : Bool) (o : OwnerClass) : Decision :=
  match v with
  | .formalPointer => .formalValue
  | .localPointer => .namedValue
  | v =>
    if !v.neutral then .valueShape
    else if carries then .namedValue
    else
      match o with
      | .untracked => .admit
      | .tracked => .ownedField
      | .unresolved => .unresolvedOwner

/-- The Rust `DECISION_VECTORS`: a statement spelling, the value class, whether
the named value carries an identity, the owner and the decision. -/
def decisionVectors : List (String × Value × Bool × OwnerClass × Decision) := [
  ("s->p = 0", .null, false, .untracked, .admit),
  ("s->in.p = 0", .null, false, .untracked, .admit),
  ("s->name = \"x\"", .string, false, .untracked, .admit),
  ("s->cb = f", .function, false, .untracked, .admit),
  ("s->cb = &f", .function, false, .untracked, .admit),
  ("s->cb = g", .functionPointerName, false, .untracked, .admit),
  ("s->self = s", .selfAddress, false, .untracked, .admit),
  ("s->pp = &s->p", .interiorAddress, false, .untracked, .admit),
  ("s->p = q", .formalPointer, false, .untracked, .formalValue),
  ("s->p = l", .localPointer, false, .untracked, .namedValue),
  ("s->p = t->p", .pointerRead, false, .untracked, .valueShape),
  ("s->p = cg_get()", .call, false, .untracked, .valueShape),
  ("s->name = s->name + 1", .arithmetic, false, .untracked, .valueShape),
  ("s->pp = &t->p", .otherAddress, false, .untracked, .valueShape),
  ("o->p = 0", .null, false, .tracked, .ownedField),
  ("o->self = o", .selfAddress, false, .tracked, .ownedField),
  ("d->p = 0", .null, false, .unresolved, .unresolvedOwner)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (_, v, carries, o, d) => decideStore v carries o == d) = true := by
  decide

/-- Only a neutral value, carrying no identity, into an untracked owner is
admitted. -/
theorem admitted_exactly_when (v : Value) (carries : Bool) (o : OwnerClass) :
    decideStore v carries o = .admit ↔ (v.neutral = true ∧ carries = false ∧ o = .untracked) := by
  cases v <;> cases carries <;> cases o <;> simp [decideStore, Value.neutral]

/-- Example C (Q36): a formal's object pointer is refused whatever the owner. -/
theorem example_c_is_refused (o : OwnerClass) : decideStore .formalPointer false o = .formalValue := by
  rfl

/-- A store into a tracked owner (an owning field may be overwritten) refuses. -/
theorem an_owned_field_is_never_overwritten (v : Value) (carries : Bool) :
    decideStore v carries .tracked ≠ .admit := by
  cases v <;> cases carries <;> simp [decideStore, Value.neutral]

/-- An owner the chain obligation does not resolve refuses. -/
theorem an_unresolved_owner_refuses (v : Value) (carries : Bool) :
    decideStore v carries .unresolved ≠ .admit := by
  cases v <;> cases carries <;> simp [decideStore, Value.neutral]

/-! ## Same object: the address values -/

/-- An address value names the destination's own object only when both places
share the root and neither reads a pointer past the root (a root-only pointer
place, or a record root with no hop). -/
def sameObject (destRoot valueRoot : Nat) (destRootOnly valueRootOnly : Bool) : Bool :=
  destRoot == valueRoot && destRootOnly && valueRootOnly

def classifyAddress (destRoot valueRoot : Nat) (destRootOnly valueRootOnly : Bool) : Value :=
  if sameObject destRoot valueRoot destRootOnly valueRootOnly then .interiorAddress
  else .otherAddress

theorem another_root_is_another_object (d v : Nat) (a b : Bool) (h : d ≠ v) :
    classifyAddress d v a b = .otherAddress := by
  simp [classifyAddress, sameObject, h]

theorem a_deeper_value_place_is_another_object (d : Nat) (a : Bool) :
    classifyAddress d d a false = .otherAddress := by
  simp [classifyAddress, sameObject]

/-- The CG10 place decision fixes `rootOnly` for a pointer root: a place that
reads a pointer past the root (`R->F->G`, `R->in.p->x`) is another object. -/
theorem a_direct_member_place_is_not_the_root_object :
    (TypedPlace.decidePlace .pointer [.arrow, .arrow]) ≠ .rootOnly := by
  decide

/-! ## The events of one store -/

/-- The projection facts one store may touch, abstractly: the sites with an
outstanding obligation, the field links, and the targets of each binding of a
place rooted at the destination's root (the old value, as the projection
knows it). -/
structure Facts where
  live : List Nat
  links : List (Nat × Nat × List Nat)
  rooted : List (Nat × List Nat)
  deriving DecidableEq, Repr

/-- The old value carries nothing the projection tracks. -/
def oldValueFree (f : Facts) : Bool := f.rooted.all (fun (_, targets) => targets.isEmpty)

/-- Overwrite: every binding of a place rooted at the destination is forgotten
(its fact no longer describes the object); the admitted value binds nothing. -/
def overwrite (f : Facts) : Facts := { f with rooted := [] }

/-- The whole store: the decision, the old-value premise, then the overwrite.
`none` is the fallback (the gate's refusal, byte for byte). -/
def store (f : Facts) (v : Value) (carries : Bool) (o : OwnerClass) : Option Facts :=
  if (decideStore v carries o == .admit && oldValueFree f) = true then some (overwrite f) else none

/-- Frame preservation for a non-owning store: no obligation and no field link
changes, and no identity escapes. -/
theorem frame_preserved (f g : Facts) (v : Value) (carries : Bool) (o : OwnerClass)
    (h : store f v carries o = some g) :
    g.live = f.live ∧ g.links = f.links ∧ storedIdentities v = [] := by
  unfold store at h
  split at h
  · cases h; exact ⟨rfl, rfl, rfl⟩
  · cases h

/-- No obligation is lost on overwrite: an admitted store's old value carried
no tracked identity, its owner is untracked (so it holds no field link), and
every outstanding obligation is still outstanding afterwards. -/
theorem no_obligation_lost_on_overwrite (f g : Facts) (v : Value) (carries : Bool)
    (o : OwnerClass) (h : store f v carries o = some g) :
    oldValueFree f = true ∧ o = .untracked ∧ ∀ site, site ∈ f.live → site ∈ g.live := by
  unfold store at h
  split at h
  · rename_i hc
    cases h
    simp only [Bool.and_eq_true, beq_iff_eq] at hc
    have ho := ((admitted_exactly_when v carries o).mp hc.1).2.2
    -- `hc.2` is the old-value premise.
    exact ⟨hc.2, ho, fun _ hs => hs⟩
  · cases h

/-- A tracked owner's field (which may own its child) is never overwritten by
this rule, whatever the old value. -/
theorem an_owning_field_overwrite_refuses (f : Facts) (v : Value) (carries : Bool) :
    store f v carries .tracked = none := by
  unfold store
  have := an_owned_field_is_never_overwritten v carries
  simp [this]

/-- An old value the projection tracks keeps the refusal. -/
theorem a_tracked_old_value_refuses (f : Facts) (v : Value) (carries : Bool) (o : OwnerClass)
    (h : oldValueFree f = false) : store f v carries o = none := by
  simp [store, h]

/-! ## No double release through a stored alias -/

/-- What a release through the destination place would release afterwards: the
targets its binding names. -/
def releaseThrough (f : Facts) (place : Nat) : List Nat :=
  ((f.rooted.find? (fun (p, _) => p == place)).map Prod.snd).getD []

/-- After an admitted store, a release through the stored place names no tracked
identity: it can never release a site a second time. -/
theorem no_double_release_through_a_stored_alias (f g : Facts) (v : Value) (carries : Bool)
    (o : OwnerClass) (place : Nat) (h : store f v carries o = some g) :
    releaseThrough g place = [] := by
  unfold store at h
  split at h
  · cases h; simp [releaseThrough, overwrite]
  · cases h

/-- STABILITY-REEVAL-5's `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` needs a
tracked identity in a member (`n->buf = b`, then `free(n->buf)` and a
destructor releasing `n->buf` again). This rule never stores one: a local
pointer, a formal's pointer and a named value carrying an identity all refuse,
into any owner. -/
theorem the_member_release_shape_is_refused (o : OwnerClass) (v : Value) (carries : Bool) :
    (v = .localPointer ∨ v = .formalPointer ∨ carries = true) → decideStore v carries o ≠ .admit := by
  intro h
  rcases h with h | h | h
  · subst h; simp [decideStore]
  · subst h; simp [decideStore]
  · subst h; cases v <;> simp [decideStore, Value.neutral]

/-! ## Unlisted destinations -/

/-- A destination is a CG10 place: every step listed, then its owner decided. -/
def destinationAdmits (forms : List TypedPlace.Form) (v : Value) (carries : Bool)
    (o : OwnerClass) : Bool :=
  (TypedPlace.readPlace forms).isSome && decideStore v carries o == .admit

theorem an_unlisted_destination_refuses (forms : List TypedPlace.Form) (v : Value)
    (carries : Bool) (o : OwnerClass) (h : TypedPlace.Form.other ∈ forms) :
    destinationAdmits forms v carries o = false := by
  simp [destinationAdmits, TypedPlace.every_unlisted_form_refuses forms h]

/-- An untracked root walks untracked (C02-P premise 3): the owner of a
never-bound root's place is untracked at every depth. -/
theorem a_never_bound_root_owns_untracked (s : State) (c : Obligation)
    (h : rootOwner s c = some .untracked) : ownerClass s c = .untracked := by
  simp [ownerClass, finalOwner, h, IndirectChainAccess.an_untracked_owner_walks_untracked]

/-- A root bound to one live identity (a local acquisition, or a formal bound to
its caller's site while a summary is derived) is a tracked owner of a root-only
place: the store refuses, so no summary is derived through it. -/
theorem a_live_root_is_a_tracked_owner (s : State) (c : Obligation) (site : Nat)
    (hl : c.links = []) (h : rootOwner s c = some (.tracked site)) :
    ownerClass s c = .tracked := by
  simp [ownerClass, finalOwner, h, hl, walk]

end QCore.PointerStore
