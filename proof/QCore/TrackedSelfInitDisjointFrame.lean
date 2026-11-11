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

import QCore.TrackedSelfInit
import QCore.DisjointFootprint

/-! The bounded rule-36b child-slot frame. Intervals belong to one owner's
declared layout; distinct resource identities never establish separation.
Clang/source extraction, the embedding footprint, finite-map projection and
Rust correspondence are reviewed premises, not C refinement theorems. -/
namespace QCore.TrackedSelfInitDisjointFrame

open DisjointFootprint

structure Slot where
  field : String
  bytes : Interval
  deriving DecidableEq, Repr

structure Source where
  original : TrackedSelfInit.Source := TrackedSelfInit.completeSource
  assignmentsClosed : Bool := true
  ordinaryCallsClosed : Bool := true
  pointerSlotAddressesAbsent : Bool := true
  uniqueCompleteLayout : Bool := true
  slots : List Slot := []
  writes : List Interval := []

def sourceAdmits (p : Source) : Bool :=
  TrackedSelfInit.sourceAdmits p.original && p.assignmentsClosed &&
  p.ordinaryCallsClosed && p.pointerSlotAddressesAbsent && p.uniqueCompleteLayout &&
  !p.writes.isEmpty

def slotProtected (slot : Interval) (writes : List Interval) : Bool :=
  !writes.isEmpty && decide (slot.start < slot.stop) && writes.all (slot.disjoint ·)

def fieldProtected (p : Source) (field : String) : Bool :=
  p.slots.any (fun slot => slot.field == field && slotProtected slot.bytes p.writes)

theorem protected_slot_shares_no_write_byte (slot : Interval) (writes : List Interval)
    (checked : slotProtected slot writes = true) (write : Interval) (mem : write ∈ writes)
    (byte : Nat) : ¬(slot.mem byte ∧ write.mem byte) := by
  simp only [slotProtected, Bool.and_eq_true, List.all_eq_true] at checked
  exact disjoint_intervals_share_no_byte (checked.2 write mem) byte

theorem an_enclosing_member_protects_against_each_subwrite
    (slot enclosing inner : Interval) (separate : slot.disjoint enclosing = true)
    (insideStart : enclosing.start ≤ inner.start) (insideStop : inner.stop ≤ enclosing.stop)
    (byte : Nat) : ¬(slot.mem byte ∧ inner.mem byte) := by
  intro overlap
  have enclosingMem : enclosing.mem byte := by
    rcases overlap.2 with ⟨lower, upper⟩
    exact ⟨Nat.le_trans insideStart lower, Nat.lt_of_lt_of_le upper insideStop⟩
  exact disjoint_intervals_share_no_byte separate byte ⟨overlap.1, enclosingMem⟩

def writeInterval (memory : Nat → α) (area : Interval) (value : α) (byte : Nat) : α :=
  if area.start ≤ byte ∧ byte < area.stop then value else memory byte

theorem a_separate_write_keeps_each_slot_byte (memory : Nat → α)
    (slot area : Interval) (value : α) (separate : slot.disjoint area = true)
    (byte : Nat) (inside : slot.mem byte) :
    writeInterval memory area value byte = memory byte := by
  have outside : ¬area.mem byte := fun hit => disjoint_intervals_share_no_byte separate byte ⟨inside, hit⟩
  unfold writeInterval
  exact ite_eq_right outside

def runWrites (memory : Nat → α) (writes : List (Interval × α)) : Nat → α :=
  writes.foldl (fun current write => writeInterval current write.1 write.2) memory

theorem finite_separate_writes_keep_each_slot_byte (memory : Nat → α)
    (slot : Interval) (writes : List (Interval × α))
    (separate : ∀ write ∈ writes, slot.disjoint write.1 = true)
    (byte : Nat) (inside : slot.mem byte) :
    runWrites memory writes byte = memory byte := by
  induction writes generalizing memory with
  | nil => rfl
  | cons write rest ih =>
    have tail : ∀ item ∈ rest, slot.disjoint item.1 = true :=
      fun item mem => separate item (List.mem_cons_of_mem write mem)
    exact (ih (writeInterval memory write.1 write.2) tail).trans
      (a_separate_write_keeps_each_slot_byte memory slot write.1 write.2
        (separate write (by simp)) byte inside)

structure Link where
  owner : Nat
  field : String
  child : Nat
  deriving DecidableEq, Repr

/-- `ownerRoot` covers every alias that may target the owner. The stronger
`compatibleIdentity` is the source/flow proof required to retain a direct slot.
Plain includes derived bindings themselves, never their member facts. -/
structure Fact where
  plain : Bool := false
  ownerRoot : Bool := false
  compatibleIdentity : Bool := false
  directSlot : Bool := false
  field : String := ""
  targets : List Nat := []
  unknown : Bool := false
  deriving DecidableEq, Repr

structure State (α : Type) where
  root : TrackedSelfInit.Pointer := {}
  ledger : Nat → Option Nat
  nullableSites : List Nat := []
  preciseHeap : Nat → Bool := fun _ => true
  aliasesCompatible : Bool := true
  recognizedPlaces : Bool := true
  callerAndOutAbsent : Bool := true
  otherCustodyEmpty : Bool := true
  links : List Link := []
  partialLinks : List Link := []
  facts : List Fact := []
  unknown : Bool := false
  frame : α

def kept (p : Source) (fact : Fact) : Bool :=
  fact.plain || (fact.ownerRoot && fact.compatibleIdentity && fact.directSlot &&
    fieldProtected p fact.field)

def oldValueClear (p : Source) (site : Nat) (fact : Fact) : Bool :=
  !fact.ownerRoot || kept p fact || (!fact.unknown && fact.targets.all (· == site))

def linkAccepted (p : Source) (s : State α) (site : Nat) (link : Link) : Bool :=
  link.owner == site && link.child != site && s.preciseHeap link.child &&
  s.ledger link.child == some 1 && !s.nullableSites.contains link.child &&
  fieldProtected p link.field

def rootAccepted (s : State α) (site : Nat) : Bool :=
  !s.unknown && !s.root.unknown && !s.root.nullable && s.root.identity &&
  s.root.targets == [site] && s.ledger site == some 1 &&
  !s.nullableSites.contains site && s.preciseHeap site

def frameAccepted (p : Source) (s : State α) (site : Nat) : Bool :=
  s.links.all (linkAccepted p s site) &&
    (!s.links.isEmpty && (s.partialLinks.all s.links.contains && s.facts.all (oldValueClear p site)))

def admits (p : Source) (s : State α) (site : Nat) : Bool :=
  sourceAdmits p && rootAccepted s site && s.otherCustodyEmpty &&
  s.aliasesCompatible && s.recognizedPlaces && s.callerAndOutAbsent && frameAccepted p s site

def transfer (p : Source) (s : State α) (site : Nat)
    (fallbackFacts : List Fact) (fallbackFrame : α) : State α :=
  if admits p s site then { s with facts := s.facts.filter (kept p) }
  else { s with facts := fallbackFacts, frame := fallbackFrame, unknown := true }

theorem the_complete_ledger_is_preserved (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) : (transfer p s site facts frame).ledger = s.ledger := by
  unfold transfer; split <;> rfl

theorem all_field_links_are_preserved (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) : (transfer p s site facts frame).links = s.links := by
  unfold transfer; split <;> rfl

theorem all_partial_links_are_preserved (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) : (transfer p s site facts frame).partialLinks = s.partialLinks := by
  unfold transfer; split <;> rfl

theorem all_nullable_facts_are_preserved (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) : (transfer p s site facts frame).nullableSites = s.nullableSites := by
  unfold transfer; split <;> rfl

theorem plain_and_derived_bindings_survive (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (admitted : admits p s site = true)
    (fact : Fact) (mem : fact ∈ s.facts) (plain : fact.plain = true) :
    fact ∈ (transfer p s site facts frame).facts := by
  simp [transfer, admitted, mem, kept, plain]

theorem surviving_member_facts_have_slot_protection (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (admitted : admits p s site = true)
    (fact : Fact) (mem : fact ∈ (transfer p s site facts frame).facts)
    (member : fact.plain = false) :
    fact.ownerRoot = true ∧ fact.compatibleIdentity = true ∧ fact.directSlot = true ∧
      fieldProtected p fact.field = true := by
  simp only [transfer, admitted, ↓reduceIte, List.mem_filter] at mem
  simpa [kept, member, Bool.and_eq_true, and_assoc] using mem.2

theorem accepted_links_have_the_actual_owner (p : Source) (s : State α) (site : Nat)
    (admitted : admits p s site = true) (link : Link) (mem : link ∈ s.links) :
    link.owner = site := by
  simp only [admits, Bool.and_eq_true] at admitted
  have checked := admitted.2
  simp only [frameAccepted, Bool.and_eq_true, List.all_eq_true] at checked
  have linked := checked.1 link mem
  simp only [linkAccepted, Bool.and_eq_true, beq_iff_eq] at linked
  exact linked.1.1.1.1.1

theorem accepted_partial_links_are_still_real_links (p : Source) (s : State α) (site : Nat)
    (admitted : admits p s site = true) (link : Link) (mem : link ∈ s.partialLinks) :
    link ∈ s.links := by
  simp only [admits, Bool.and_eq_true] at admitted
  have checked := admitted.2
  simp only [frameAccepted, Bool.and_eq_true, List.all_eq_true] at checked
  simpa using checked.2.2.1 link mem

theorem unsupported_custody_is_never_erased_to_admit (p : Source) (s : State α) (site : Nat)
    (nondefault : s.otherCustodyEmpty = false) : admits p s site = false := by
  simp [admits, nondefault]

theorem missing_source_calls_refuse (p : Source) (s : State α) (site : Nat)
    (missing : p.ordinaryCallsClosed = false) : admits p s site = false := by
  simp [admits, sourceAdmits, missing]

theorem pointer_rebinding_refuses (p : Source) (s : State α) (site : Nat)
    (rebound : p.assignmentsClosed = false ∨ p.pointerSlotAddressesAbsent = false) :
    admits p s site = false := by
  rcases rebound with h | h <;> simp [admits, sourceAdmits, h]

theorem missing_layout_refuses (p : Source) (s : State α) (site : Nat)
    (missing : p.uniqueCompleteLayout = false) : admits p s site = false := by
  simp [admits, sourceAdmits, missing]

theorem refusal_is_unknown (p : Source) (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (refused : admits p s site = false) :
    (transfer p s site facts frame).unknown = true := by simp [transfer, refused]

def runEffects (s : State α) (effects : List (Source × Nat))
    (facts : List Fact) (frame : α) : State α :=
  effects.foldl (fun current effect => transfer effect.1 current effect.2 facts frame) s

theorem finite_composition_preserves_every_obligation (s : State α)
    (effects : List (Source × Nat)) (facts : List Fact) (frame : α) :
    (runEffects s effects facts frame).ledger = s.ledger := by
  induction effects generalizing s with
  | nil => rfl
  | cons effect rest ih =>
    exact (ih (transfer effect.1 s effect.2 facts frame)).trans
      (the_complete_ledger_is_preserved effect.1 s effect.2 facts frame)

def available (consumer : TrackedSelfInit.Consumer) (sourceChecked : Bool) : Bool :=
  TrackedSelfInit.available consumer sourceChecked

theorem no_ordinary_summary (checked : Bool) : available .ordinarySummary checked = false := by rfl
theorem no_callback_permission (checked : Bool) : available .callback checked = false := by rfl
theorem no_cross_unit_export (checked : Bool) : available .exported checked = false := by rfl

def goodSource : Source :=
  { slots := [⟨"payload", ⟨16,24⟩⟩], writes := [⟨0,16⟩] }
def initial : State Unit :=
  { ledger := fun n => if n == 7 || n == 8 then some 1 else none,
    links := [⟨7,"payload",8⟩], frame := (),
    facts := [{ plain := true }, ⟨false,true,true,true,"payload",[8],false⟩] }

def flowVectors : List Bool :=
  [ admits goodSource initial 7,
    admits goodSource { initial with ledger := fun n => if n == 8 then some 2 else initial.ledger n } 7,
    admits goodSource { initial with nullableSites := [8] } 7,
    admits goodSource { initial with partialLinks := initial.links } 7,
    admits goodSource { initial with partialLinks := [⟨7,"payload",9⟩] } 7,
    admits { goodSource with slots := [] } initial 7,
    admits goodSource { initial with links := [⟨7,"head",8⟩] } 7,
    admits goodSource { initial with otherCustodyEmpty := false } 7,
    admits goodSource { initial with aliasesCompatible := false } 7,
    admits goodSource { initial with recognizedPlaces := false } 7,
    admits goodSource { initial with callerAndOutAbsent := false } 7,
    admits goodSource { initial with preciseHeap := fun n => n != 8 } 7,
    admits goodSource { initial with preciseHeap := fun n => n != 7 } 7,
    admits goodSource { initial with ledger := fun n => if n == 7 then some 2 else initial.ledger n } 7,
    admits goodSource { initial with root := { nullable := true } } 7,
    admits goodSource { initial with root := { identity := false } } 7,
    admits goodSource { initial with links := initial.links ++ [⟨9,"payload",8⟩] } 7,
    admits goodSource { initial with preciseHeap := fun n => n != 7 } 7,
    admits goodSource { initial with facts := [{ ownerRoot := true, targets := [8] }] } 7,
    admits goodSource { initial with facts := [{ unknown := true }] } 7 ]

theorem shared_flow_vectors : flowVectors =
    [true,false,false,true,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,true] := by decide

def layoutVectors : List Bool :=
  [ slotProtected ⟨0,8⟩ [⟨8,16⟩], slotProtected ⟨8,16⟩ [⟨0,8⟩],
    slotProtected ⟨0,8⟩ [⟨0,8⟩], slotProtected ⟨0,8⟩ [⟨0,8⟩] ]
theorem shared_layout_vectors : layoutVectors = [true,true,false,false] := by decide

end QCore.TrackedSelfInitDisjointFrame
