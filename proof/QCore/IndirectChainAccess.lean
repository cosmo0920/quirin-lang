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

import QCore.IndirectScalarAccess
import QCore.ChainedAccess

/-!
# Record-pointer member chains in statements and conditions (card W44-INDIRECT-CHAIN-ACCESS)

The abstract rule implemented by `crates/qcore-c-gateway/src/indirect_chain_access.rs`
(`parse_chain`, `chain_statement`, `chain_condition`, `chain_resolves`) and the
`chains` field of `LegacyInstruction::IndirectScalarAccess`.

Card W43-INDIRECT-ACCESS-GENERAL (`QCore.IndirectScalarAccess`) admits one member
selection per access. This card lets the same typed statement and condition
routes read and write through a **chain** `R->L1->…->F`, `(*R).L1->…->F` or
`S.L1->…->F`, and gives every pointer the chain reads through its own obligation.

## Syntax

A chain is read by card O04-ARMS's reader (`QCore.ChainedAccess`): every selection
before the last is a fully typed `struct POINTEE *` over a complete, unambiguous,
non-union record, and the walk is bounded. This card narrows the final member to a
**scalar** (a pointer-valued final member is a member-link or escape question it
does not decide), requires two or more selections (one is W43's), and moves the
root's premise from the syntax to the flow: a root the body may bind is no longer
refused outright, it must resolve to one live identity.

## The obligation

Each pointer on the chain designates an object read as *untracked* or *tracked*
(one known live identity):

* the root: a never-bound name with no binding or a non-null target-free one is
  untracked; exactly one known, non-null, live identity (identity derivation) is
  tracked; everything else refuses. A record-object root is untracked exactly
  when it is never bound;
* a link of an untracked owner is untracked (C02-P premise 3: no reaching path
  bound a place in that object);
* a link of a tracked owner is the member place `R->L1`: it must be bound, not
  possibly null and identity-derived, and either one known live child equal to
  the field link the admitted store recorded, or target-free, not known and with
  no field link at all (untracked). Anything else refuses;
* a tracked owner past the first link refuses: no deeper place is ever bound.

Admitted or refused, a chain changes no binding, status, marker or field link;
it only adds diagnostics to W43's transfer.

## What this is not

A model of the **decision**. Faithful token/type extraction and the Rust–Lean
correspondence are review premises (trust boundary 28). The untracked reading of
a link is C02-P's premise 3, inherited, not proved. Bounds, alignment, constness
and the nullness or liveness of untracked objects are outside the property.
-/

namespace QCore.IndirectChainAccess

open QCore.TypedFieldAccess (MemberType TagKind)
open QCore.IndirectScalarAccess (Binding Flags Diagnostic Access)

/-! ## Syntax: the scalar chain the reader admits -/

/-- The last selection selects a scalar member. -/
def lastIsScalar : List ChainedAccess.Link → Bool
  | [] => false
  | [l] => l.member == MemberType.scalar
  | _ :: l :: rest => lastIsScalar (l :: rest)

/-- `parse_chain` with a scalar final member, over O04-ARMS's chain facts. The
root's `baseBound` is deliberately absent: it is decided by the flow. -/
def admitsScalarChain (c : ChainedAccess.Chain) : Bool :=
  c.declarationsRead && c.baseMatches && !c.continuesAfterLast &&
    decide (c.links.length ≤ ChainedAccess.limit) && decide (2 ≤ c.links.length) &&
    ChainedAccess.linksOk c.links && lastIsScalar c.links

/-- The syntax never reads whether the root may be bound. -/
theorem the_root_premise_is_left_to_the_flow (c : ChainedAccess.Chain) (b : Bool) :
    admitsScalarChain { c with baseBound := b } = admitsScalarChain c := rfl

/-- Over a never-bound root, every chain this card reads is one O04-ARMS's reader
already reads: the reader is the same, only the final member narrows. -/
theorem a_never_bound_scalar_chain_is_an_o04_chain (c : ChainedAccess.Chain)
    (h : admitsScalarChain c = true) (hb : c.baseBound = false) :
    ChainedAccess.admitsChain c = true := by
  simp only [admitsScalarChain, Bool.and_eq_true, decide_eq_true_eq] at h
  simp [ChainedAccess.admitsChain, hb, h]

theorem a_bad_link_refuses (c : ChainedAccess.Chain) (l : ChainedAccess.Link)
    (mem : l ∈ c.links) (bad : ChainedAccess.finalOk l = false) :
    admitsScalarChain c = false := by
  simp [admitsScalarChain, ChainedAccess.linksOk_false_of_bad_link c.links l mem bad]

theorem a_non_continuable_intermediate_refuses (c : ChainedAccess.Chain)
    (l : ChainedAccess.Link) (rest : List ChainedAccess.Link) (shape : c.links = l :: rest)
    (nonEmpty : rest ≠ []) (notTyped : l.continuable = false) :
    admitsScalarChain c = false := by
  have bad : ChainedAccess.intermediateOk l = false := by
    simp [ChainedAccess.intermediateOk, notTyped]
  simp [admitsScalarChain,
    ChainedAccess.linksOk_false_of_unContinuable c.links l rest shape nonEmpty bad]

theorem a_single_selection_is_not_this_rule (c : ChainedAccess.Chain) (l : ChainedAccess.Link)
    (single : c.links = [l]) : admitsScalarChain c = false := by
  simp [admitsScalarChain, single]

theorem a_chain_beyond_the_bound_refuses (c : ChainedAccess.Chain)
    (long : c.links.length > ChainedAccess.limit) : admitsScalarChain c = false := by
  have : ¬ c.links.length ≤ ChainedAccess.limit := Nat.not_le.mpr long
  simp [admitsScalarChain, this]

theorem lastIsScalar_of_pointer_last :
    ∀ (ls : List ChainedAccess.Link) (l : ChainedAccess.Link),
      ls.getLast? = some l → l.member = MemberType.pointer → lastIsScalar ls = false := by
  intro ls
  induction ls with
  | nil => intro l h; cases h
  | cons head rest ih =>
    intro l hl hp
    cases rest with
    | nil =>
      simp at hl
      subst hl
      simp [lastIsScalar, hp]
    | cons next tail =>
      have : (next :: tail).getLast? = some l := by
        simpa [List.getLast?_cons_cons] using hl
      simpa [lastIsScalar] using ih l this hp

/-- A pointer-valued final member is a member-link question: refused. -/
theorem a_pointer_final_member_refuses (c : ChainedAccess.Chain) (l : ChainedAccess.Link)
    (last : c.links.getLast? = some l) (pointer : l.member = MemberType.pointer) :
    admitsScalarChain c = false := by
  simp [admitsScalarChain, lastIsScalar_of_pointer_last c.links l last pointer]

theorem a_continued_access_refuses (c : ChainedAccess.Chain)
    (continues : c.continuesAfterLast = true) : admitsScalarChain c = false := by
  simp [admitsScalarChain, continues]

/-! ## The flow obligation -/

/-- W43's state plus the field links an admitted aggregate store records:
`fieldLink owner field` is the linked sites. -/
structure State where
  base : IndirectScalarAccess.State
  fieldLink : Nat → Nat → Option (List Nat)

/-- One chain's obligation (`AccessChain`): the root name, whether it is
dereferenced, the classifier's never-bound answer, the place `R->L1` and the
dereferenced link members. -/
structure Obligation where
  root : Nat
  dereferencedRoot : Bool
  rootNeverBound : Bool
  firstPlace : Nat
  links : List Nat

inductive Owner where
  | untracked
  | tracked (site : Nat)
  deriving DecidableEq, Repr

/-- `ChainView::exactly_live`. -/
def exactlyLive (s : IndirectScalarAccess.State) (site : Nat) : Bool :=
  !s.nullable site && !s.unresolved site &&
    (match s.status site with
     | some flags => flags.exactlyLive
     | none => false)

/-- `single_identity`: a known, non-null, identity binding with one target. -/
def singleIdentity (b : Binding) : Option Nat :=
  match b.targets with
  | [site] =>
      if !b.unknown && !b.mayBeNull && b.derivation == .identity then some site else none
  | _ => none

/-- The owner reading of the root. -/
def rootOwner (s : State) (c : Obligation) : Option Owner :=
  if c.dereferencedRoot then
    match s.base.binding c.root with
    | none => if c.rootNeverBound then some .untracked else none
    | some b =>
        if b.mayBeNull then none
        else
          match b.targets with
          | [] => if b.unknown && c.rootNeverBound then some .untracked else none
          | _ =>
              match singleIdentity b with
              | some site => if exactlyLive s.base site then some (.tracked site) else none
              | none => none
  else if c.rootNeverBound then some .untracked
  else none

/-- The owner reading of one link, selected at `depth` from `owner`. -/
def linkOwner (s : State) (c : Obligation) (depth field : Nat) : Owner → Option Owner
  | .untracked => some .untracked
  | .tracked site =>
      if depth ≠ 0 || !c.dereferencedRoot then none
      else
        match s.base.binding c.firstPlace with
        | none => none
        | some b =>
            if b.mayBeNull || b.derivation != .identity then none
            else
              match b.targets with
              | [] =>
                  if b.unknown && (s.fieldLink site field).isNone then some .untracked else none
              | _ =>
                  match singleIdentity b with
                  | some child =>
                      if s.fieldLink site field == some b.targets && exactlyLive s.base child then
                        some (.tracked child)
                      else none
                  | none => none

def walk (s : State) (c : Obligation) : Nat → List Nat → Owner → Option Owner
  | _, [], owner => some owner
  | depth, field :: rest, owner =>
      match linkOwner s c depth field owner with
      | some next => walk s c (depth + 1) rest next
      | none => none

/-- `chain_resolves`. -/
def resolves (s : State) (c : Obligation) : Bool :=
  match rootOwner s c with
  | some owner => (walk s c 0 c.links owner).isSome
  | none => false

/-- W43's transfer of the enclosing typed access, with every refused chain adding
an unknown-alias reason. -/
def transfer (s : State) (a : Access) (cs : List Obligation) : State × List Diagnostic :=
  ({ s with base := (IndirectScalarAccess.transfer s.base a).1 },
    (IndirectScalarAccess.transfer s.base a).2 ++
      (cs.filter (fun c => !resolves s c)).map (fun c => Diagnostic.unknownAlias c.root))

/-! ## Custody neutrality and conservativity -/

/-- A chain changes no binding: the state after is W43's. -/
theorem a_chain_changes_no_binding (s : State) (a : Access) (cs : List Obligation) :
    (transfer s a cs).1.base = (IndirectScalarAccess.transfer s.base a).1 := rfl

theorem field_links_are_unchanged (s : State) (a : Access) (cs : List Obligation) :
    (transfer s a cs).1.fieldLink = s.fieldLink := rfl

theorem the_statuses_and_markers_are_unchanged (s : State) (a : Access) (cs : List Obligation) :
    (transfer s a cs).1.base.status = s.base.status ∧
      (transfer s a cs).1.base.nullable = s.base.nullable ∧
      (transfer s a cs).1.base.unresolved = s.base.unresolved := ⟨rfl, rfl, rfl⟩

/-- With no chain, the instruction is exactly W43's typed access. -/
theorem with_no_chain_the_access_is_w43s (s : State) (a : Access) :
    (transfer s a []).2 = (IndirectScalarAccess.transfer s.base a).2 := by
  simp [transfer]

/-- Chains that all resolve add no diagnostic to W43's. -/
theorem admitted_chains_add_no_diagnostic (s : State) (a : Access) (cs : List Obligation)
    (h : ∀ c ∈ cs, resolves s c = true) :
    (transfer s a cs).2 = (IndirectScalarAccess.transfer s.base a).2 := by
  simp only [transfer]
  rw [List.filter_eq_nil_iff.mpr]
  · simp
  · intro c hmem
    simp [h c hmem]

/-- A refused chain always leaves a reason: the function cannot be `Safe`. -/
theorem a_refused_chain_is_never_safe (s : State) (a : Access) (cs : List Obligation)
    (c : Obligation) (hmem : c ∈ cs) (hr : resolves s c = false) :
    IndirectScalarAccess.safe (transfer s a cs).2 = false := by
  simp only [IndirectScalarAccess.safe, transfer, List.isEmpty_eq_false_iff]
  intro hnil
  have hin : c ∈ cs.filter (fun c => !resolves s c) := by simp [hmem, hr]
  rw [List.append_eq_nil_iff] at hnil
  rw [List.map_eq_nil_iff] at hnil
  simp [hnil.2] at hin

/-! ## The root -/

theorem an_unbound_never_bound_root_is_untracked (s : State) (c : Obligation)
    (hd : c.dereferencedRoot = true) (hn : c.rootNeverBound = true)
    (hb : s.base.binding c.root = none) : rootOwner s c = some .untracked := by
  simp [rootOwner, hd, hn, hb]

theorem an_unbound_root_the_body_may_bind_refuses (s : State) (c : Obligation)
    (hd : c.dereferencedRoot = true) (hn : c.rootNeverBound = false)
    (hb : s.base.binding c.root = none) : resolves s c = false := by
  simp [resolves, rootOwner, hd, hn, hb]

theorem a_nullable_root_refuses (s : State) (c : Obligation) (b : Binding)
    (hd : c.dereferencedRoot = true) (hb : s.base.binding c.root = some b)
    (hnull : b.mayBeNull = true) : resolves s c = false := by
  simp [resolves, rootOwner, hd, hb, hnull]

theorem a_target_free_root_the_body_may_bind_refuses (s : State) (c : Obligation) (b : Binding)
    (hd : c.dereferencedRoot = true) (hb : s.base.binding c.root = some b)
    (ht : b.targets = []) (hn : c.rootNeverBound = false) : resolves s c = false := by
  simp [resolves, rootOwner, hd, hb, ht, hn]

theorem a_record_object_root_the_body_may_bind_refuses (s : State) (c : Obligation)
    (hd : c.dereferencedRoot = false) (hn : c.rootNeverBound = false) :
    resolves s c = false := by
  simp [resolves, rootOwner, hd, hn]

/-- What a tracked root reading establishes about the root's binding. -/
theorem a_tracked_root_is_one_live_identity (s : State) (c : Obligation) (site : Nat)
    (h : rootOwner s c = some (.tracked site)) :
    ∃ b : Binding, s.base.binding c.root = some b ∧ b.targets = [site] ∧ b.unknown = false ∧
      exactlyLive s.base site = true := by
  unfold rootOwner at h
  cases hd : c.dereferencedRoot <;> simp only [hd] at h
  · cases c.rootNeverBound <;> simp at h
  · cases hb : s.base.binding c.root with
    | none =>
      simp only [hb] at h
      cases c.rootNeverBound <;> simp at h
    | some b =>
      simp only [hb] at h
      cases hn : b.mayBeNull <;> simp only [hn] at h
      · cases ht : b.targets with
        | nil =>
          simp only [ht] at h
          cases b.unknown <;> cases c.rootNeverBound <;> simp at h
        | cons x rest =>
          simp only [ht] at h
          cases hs : singleIdentity b with
          | none => simp [hs] at h
          | some site' =>
            simp only [hs] at h
            cases hl : exactlyLive s.base site' <;> simp only [hl] at h
            · simp at h
            · cases h
              simp only [singleIdentity, ht] at hs
              cases rest with
              | cons _ _ => simp at hs
              | nil =>
                cases hu : b.unknown <;> simp [hu] at hs
                exact ⟨b, rfl, by simp [ht, hs.2], hu, hl⟩
      · simp at h

/-- A tracked root is never accused: W43's accusation and this admission never
meet on one root. -/
theorem a_tracked_root_is_never_accused (s : State) (c : Obligation) (site : Nat)
    (h : rootOwner s c = some (.tracked site)) :
    IndirectScalarAccess.accuses s.base c.root = none := by
  obtain ⟨b, hb, ht, hu, hl⟩ := a_tracked_root_is_one_live_identity s c site h
  obtain ⟨targets, unknown, mayBeNull, derivation⟩ := b
  simp only at ht hu
  subst ht hu
  simp only [exactlyLive, Bool.and_eq_true, Bool.not_eq_true'] at hl
  obtain ⟨⟨_, hunres⟩, hstatus⟩ := hl
  unfold IndirectScalarAccess.accuses
  rw [hb]
  simp only [hunres]
  cases hs : s.base.status site with
  | none => rfl
  | some flags =>
    rw [hs] at hstatus
    simp only [Flags.exactlyLive, Bool.and_eq_true, Bool.not_eq_true'] at hstatus
    simp [hstatus.1.2]

/-! ## The links -/

theorem an_untracked_owner_walks_untracked (s : State) (c : Obligation) :
    ∀ (depth : Nat) (fields : List Nat), walk s c depth fields .untracked = some .untracked := by
  intro depth fields
  induction fields generalizing depth with
  | nil => rfl
  | cons field rest ih => simp [walk, linkOwner, ih]

/-- C02-P premise 3 in the positive direction: from an untracked root, every link
at every depth resolves. -/
theorem every_link_of_an_untracked_root_resolves (s : State) (c : Obligation)
    (h : rootOwner s c = some .untracked) : resolves s c = true := by
  simp [resolves, h, an_untracked_owner_walks_untracked]

theorem a_tracked_owner_past_the_first_link_refuses (s : State) (c : Obligation)
    (depth field site : Nat) : linkOwner s c (depth + 1) field (.tracked site) = none := by
  simp [linkOwner]

theorem a_tracked_owner_needs_a_bound_place (s : State) (c : Obligation) (field site : Nat)
    (hp : s.base.binding c.firstPlace = none) : linkOwner s c 0 field (.tracked site) = none := by
  simp [linkOwner, hp]

theorem a_nullable_place_refuses (s : State) (c : Obligation) (field site : Nat) (b : Binding)
    (hp : s.base.binding c.firstPlace = some b) (hnull : b.mayBeNull = true) :
    linkOwner s c 0 field (.tracked site) = none := by
  simp [linkOwner, hp, hnull]

/-- A tracked link is exactly the live child the admitted store linked. -/
theorem a_tracked_link_is_the_stored_live_child (s : State) (c : Obligation)
    (field site child : Nat) (h : linkOwner s c 0 field (.tracked site) = some (.tracked child)) :
    s.fieldLink site field = some [child] ∧ exactlyLive s.base child = true := by
  simp only [linkOwner] at h
  split at h
  · simp at h
  · split at h
    · simp at h
    · rename_i b hb
      split at h
      · simp at h
      · split at h
        · split at h <;> simp at h
        · rename_i targets hne
          split at h
          · rename_i child' hsi
            split at h
            · rename_i hcond
              cases h
              simp only [Bool.and_eq_true, beq_iff_eq] at hcond
              simp only [singleIdentity] at hsi
              split at hsi
              · rename_i one hone
                split at hsi
                · cases hsi
                  exact ⟨by rw [hcond.1, hone], hcond.2⟩
                · simp at hsi
              · simp at hsi
            · simp at h
          · simp at h

/-- A child the field link does not name (a drifted binding) refuses. -/
theorem a_drifted_link_refuses (s : State) (c : Obligation) (field site child : Nat)
    (hd : c.dereferencedRoot = true)
    (hp : s.base.binding c.firstPlace = some ⟨[child], false, false, .identity⟩)
    (hl : s.fieldLink site field ≠ some [child]) :
    linkOwner s c 0 field (.tracked site) = none := by
  simp [linkOwner, hd, hp, singleIdentity]
  intro heq
  exact absurd heq hl

/-- A released child refuses. -/
theorem a_released_child_refuses (s : State) (c : Obligation) (field site child : Nat)
    (hd : c.dereferencedRoot = true)
    (hp : s.base.binding c.firstPlace = some ⟨[child], false, false, .identity⟩)
    (hdead : exactlyLive s.base child = false) :
    linkOwner s c 0 field (.tracked site) = none := by
  simp [linkOwner, hd, hp, singleIdentity, hdead]

/-! ## Non-vacuity -/

/-- Root 0 is an unbound never-bound formal; root 1 is bound to live site 7 whose
member place 2 holds live site 8, linked; place 3 holds released site 9, linked
under site 7's field 5. -/
def example_state : State :=
  { base :=
      { binding := fun n =>
          if n = 1 then some ⟨[7], false, false, .identity⟩
          else if n = 2 then some ⟨[8], false, false, .identity⟩
          else if n = 3 then some ⟨[9], false, false, .identity⟩
          else none
        status := fun site =>
          if site = 7 ∨ site = 8 then some ⟨true, false, false⟩
          else if site = 9 then some ⟨false, true, false⟩
          else none
        nullable := fun _ => false
        unresolved := fun _ => false
        rest := ⟨0, 0, 0⟩ }
    fieldLink := fun owner field =>
      if owner = 7 ∧ field = 4 then some [8]
      else if owner = 7 ∧ field = 5 then some [9]
      else none }

theorem an_unbound_formal_chain_of_depth_four_is_admitted :
    resolves example_state ⟨0, true, true, 99, [10, 11, 12]⟩ = true := by decide

theorem a_linked_live_child_is_admitted :
    resolves example_state ⟨1, true, false, 2, [4]⟩ = true := by decide

theorem a_linked_released_child_is_refused :
    resolves example_state ⟨1, true, false, 3, [5]⟩ = false := by decide

theorem a_tracked_owner_one_link_deeper_is_refused :
    resolves example_state ⟨1, true, false, 2, [4, 6]⟩ = false := by decide

theorem an_unbound_member_place_of_a_tracked_root_is_refused :
    resolves example_state ⟨1, true, false, 42, [4]⟩ = false := by decide

theorem a_full_scalar_chain_is_admitted :
    admitsScalarChain
      { baseMatches := true
        baseBound := true
        declarationsRead := true
        continuesAfterLast := false
        links :=
          [ { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.pointer, qualified := false, continuable := true }
          , { record := ⟨TagKind.recordTag, true, true⟩
              member := MemberType.scalar, qualified := false, continuable := false } ] }
      = true := by decide

end QCore.IndirectChainAccess
