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


namespace QCore.QuantVariant

/-! Executable L09.1/C04.13.2 model for the two custody dispositions of
`take`. The scalar is the single observed ABI return. An identity includes
its generation. Slots denote distinct local bindings, including formals in
another call frame. The maps track available destruction rights and sealed
results separately; aliases resolve to the same identity before a step. -/

structure Identity where
  allocation : Nat
  generation : Nat
  deriving DecidableEq

abbrev Slot := Nat

inductive Fragment where
  | empty
  | custody
  deriving DecidableEq

/-- One checked C04.13.2 mapping after the admission of its partition.
Tag numbers are local to the variant. -/
structure Mapping where
  tag : Nat
  fragment : Fragment
  carriesCode : Bool
  deriving DecidableEq

structure Arm where
  tag : Nat
  fragment : Fragment
  payload : Option Int32
  deriving DecidableEq

def selectArm (mapping : Mapping) (code : Int32) : Arm :=
  ⟨mapping.tag, mapping.fragment, if mapping.carriesCode then some code else none⟩

def takeMapping (code : Int32) : Mapping :=
  if code = 0 then ⟨0, .empty, false⟩ else ⟨1, .custody, true⟩

structure Package where
  subject : Identity
  arm : Arm
  deriving DecidableEq

structure State where
  owned : Identity → Bool
  slots : Slot → Option Package

private def write [DecidableEq α] (f : α → β) (atKey : α) (v : β) (key : α) : β :=
  if key = atKey then v else f key

/-- The selected class and its disposition are inputs from the validated
contract (QCore.RelationalPost); packing does not infer either from an
unchecked integer. -/
def pack (s : State) (id : Identity) (dst : Slot) (mapping : Mapping)
    (code : Int32) : Option State :=
  if s.owned id && (s.slots dst).isNone then
    some ⟨write s.owned id false, write s.slots dst (some ⟨id, selectArm mapping code⟩)⟩
  else none

def handoff (s : State) (id : Identity) (src dst : Slot) : Option State := do
  let pkg ← s.slots src
  if pkg.subject = id ∧ (s.slots dst).isNone then
    return { s with slots := write (write s.slots src none) dst (some pkg) }
  else none

def unpack (s : State) (id : Identity) (src : Slot) : Option (Arm × State) := do
  let pkg ← s.slots src
  if pkg.subject = id ∧ s.owned id = false then
    let owed := decide (pkg.arm.fragment = .custody)
    return (pkg.arm, ⟨write s.owned id owed, write s.slots src none⟩)
  else none

def release (s : State) (id : Identity) : Option State :=
  if s.owned id then some { s with owned := write s.owned id false } else none

theorem sealed_alias_release_refuses (s : State) (id aliasId : Identity)
    (dst : Slot) (mapping : Mapping) (code : Int32) (same : aliasId = id) :
    (pack s id dst mapping code >>= fun next => release next aliasId) = none := by
  subst aliasId
  unfold pack
  split <;> simp [release, write]

theorem handoff_invalidates_source (s next : State) (id : Identity) (src dst : Slot)
    (moved : handoff s id src dst = some next) : unpack next id src = none := by
  cases h : s.slots src with
  | none => simp [handoff, h] at moved
  | some pkg =>
    simp [handoff, h] at moved
    obtain ⟨⟨_, empty⟩, rfl⟩ := moved
    have diff : src ≠ dst := by
      intro eq
      subst dst
      simp [h] at empty
    simp [unpack, write, diff]

theorem wrong_generation_refuses (s : State) (src : Slot) (pkg : Package)
    (other : Identity) (atSrc : s.slots src = some pkg)
    (wrong : pkg.subject.generation ≠ other.generation) :
    unpack s other src = none := by
  have different : pkg.subject ≠ other := fun eq => wrong (congrArg Identity.generation eq)
  simp [unpack, atSrc, different]

theorem wrong_generation_handoff_refuses (s : State) (src dst : Slot) (pkg : Package)
    (other : Identity) (atSrc : s.slots src = some pkg)
    (wrong : pkg.subject.generation ≠ other.generation) :
    handoff s other src dst = none := by
  have different : pkg.subject ≠ other := fun eq => wrong (congrArg Identity.generation eq)
  simp [handoff, atSrc, different]

theorem open_consumes_once (s next : State) (id : Identity) (slot : Slot) (arm : Arm)
    (opened : unpack s id slot = some (arm, next)) : unpack next id slot = none := by
  cases h : s.slots slot with
  | none => simp [unpack, h] at opened
  | some pkg =>
    by_cases ok : pkg.subject = id ∧ s.owned id = false
    · simp [unpack, h, ok] at opened
      obtain ⟨_, rfl⟩ := opened
      simp [unpack, write]
    · simp [unpack, h, ok] at opened

theorem open_preserves_current_frame (s next : State) (id other : Identity)
    (slot : Slot) (arm : Arm) (disjoint : other ≠ id)
    (opened : unpack s id slot = some (arm, next)) : next.owned other = s.owned other := by
  cases h : s.slots slot with
  | none => simp [unpack, h] at opened
  | some pkg =>
    by_cases ok : pkg.subject = id ∧ s.owned id = false
    · simp [unpack, h, ok] at opened
      obtain ⟨_, rfl⟩ := opened
      simp [write, disjoint]
    · simp [unpack, h, ok] at opened

/-- Passing the package to another frame and matching returns the exact
selected tag, its declared scalar (if any), and its custody fragment. -/
def delayedMatch (s : State) (id : Identity) (src dst : Slot) (mapping : Mapping)
    (code : Int32) : Option (Arm × Bool) := do
  let packed ← pack s id src mapping code
  let moved ← handoff packed id src dst
  let (arm, afterOpen) ← unpack moved id dst
  pure (arm, afterOpen.owned id)

theorem selected_arm_survives_handoff (s : State) (id : Identity) (src dst : Slot)
    (mapping : Mapping) (code : Int32)
    (live : s.owned id = true) (srcFree : s.slots src = none)
    (dstFree : s.slots dst = none) (distinct : dst ≠ src) :
    delayedMatch s id src dst mapping code =
      some (selectArm mapping code, decide (mapping.fragment = .custody)) := by
  simp [delayedMatch, pack, handoff, unpack, selectArm, write, live, srcFree, dstFree, distinct]

theorem delayed_match (s : State) (id : Identity) (src dst : Slot) (code : Int32)
    (live : s.owned id = true) (srcFree : s.slots src = none)
    (dstFree : s.slots dst = none) (distinct : dst ≠ src) :
    delayedMatch s id src dst (takeMapping code) code =
      if code = 0 then some (⟨0, .empty, none⟩, false)
      else some (⟨1, .custody, some code⟩, true) := by
  rw [selected_arm_survives_handoff s id src dst _ code live srcFree dstFree distinct]
  by_cases zero : code = 0 <;> simp [takeMapping, selectArm, zero]

namespace Verification

/-- A generation has at most one sealed slot. While sealed it has no
    accessible destruction right. Values in other slots are unrestricted. -/
def Exclusive (s : State) : Prop :=
  ∀ k p, s.slots k = some p →
    s.owned p.subject = false ∧
    ∀ j q, s.slots j = some q → q.subject = p.subject → j = k

private theorem pack_shape (s t : State) (id : Identity) (k : Slot)
    (m : Mapping) (c : Int32) (h : pack s id k m c = some t) :
    s.owned id = true ∧ s.slots k = none ∧
    t = ⟨write s.owned id false, write s.slots k (some ⟨id, selectArm m c⟩)⟩ := by
  unfold pack at h
  split at h <;> simp_all <;> exact h.symm

private theorem handoff_shape (s t : State) (id : Identity) (a b : Slot)
    (h : handoff s id a b = some t) :
    ∃ p, s.slots a = some p ∧ p.subject = id ∧ s.slots b = none ∧
      t = ⟨s.owned, write (write s.slots a none) b (some p)⟩ := by
  cases hp : s.slots a with
  | none => simp [handoff, hp] at h
  | some p =>
    simp [handoff, hp] at h
    obtain ⟨⟨hid, hb⟩, rfl⟩ := h
    exact ⟨p, rfl, hid, hb, rfl⟩

private theorem unpack_shape (s t : State) (id : Identity) (k : Slot)
    (arm : Arm) (h : unpack s id k = some (arm, t)) :
    ∃ p, s.slots k = some p ∧ p.subject = id ∧ p.arm = arm ∧
      t = ⟨write s.owned id (decide (arm.fragment = .custody)), write s.slots k none⟩ := by
  cases hp : s.slots k with
  | none => simp [unpack, hp] at h
  | some p =>
    by_cases ok : p.subject = id ∧ s.owned id = false
    · simp [unpack, hp, ok] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨p, rfl, ok.1, rfl, rfl⟩
    · simp [unpack, hp, ok] at h

variable {id : Identity} {s t : State} {a b k : Slot} {m : Mapping} {c : Int32} {arm : Arm}

theorem pack_exclusive (hs : Exclusive s) (h : pack s id k m c = some t) :
    Exclusive t := by
  obtain ⟨_, _, rfl⟩ := pack_shape _ _ _ _ _ _ h
  unfold Exclusive at *
  simp only [write]
  grind

theorem handoff_exclusive (hs : Exclusive s) (h : handoff s id a b = some t) :
    Exclusive t := by
  obtain ⟨_, _, _, _, rfl⟩ := handoff_shape _ _ _ _ _ h
  unfold Exclusive at *
  simp only [write]
  grind

theorem unpack_exclusive (hs : Exclusive s) (h : unpack s id k = some (arm, t)) :
    Exclusive t := by
  obtain ⟨_, _, _, _, rfl⟩ := unpack_shape _ _ _ _ _ h
  unfold Exclusive at *
  simp only [write]
  grind

theorem release_exclusive (hs : Exclusive s) (h : release s id = some t) :
    Exclusive t := by
  have shape : release s id = (if s.owned id then some { s with owned := write s.owned id false } else none) := rfl
  rw [shape] at h
  split at h <;> simp_all
  subst t
  unfold Exclusive at *
  simp only [write]
  grind

/-- Available or sealed custody, observed without modifying the ledger. -/
def HasCustody (s : State) (i : Identity) : Prop :=
  s.owned i = true ∨ ∃ k p, s.slots k = some p ∧ p.subject = i ∧ p.arm.fragment = .custody

theorem handoff_conserves (h : handoff s id a b = some t) (i : Identity) :
    HasCustody t i ↔ HasCustody s i := by
  obtain ⟨p, _, _, _, rfl⟩ := handoff_shape _ _ _ _ _ h
  simp only [HasCustody, write]
  grind

theorem unpack_conserves (h : unpack s id k = some (arm, t)) (i : Identity) :
    HasCustody t i ↔ HasCustody s i := by
  obtain ⟨p, hp, hi, ha, rfl⟩ := unpack_shape _ _ _ _ _ h
  have sealed : s.owned id = false := by
    cases ht : s.owned id with
    | false => rfl
    | true =>
      simp [unpack, hp, ht] at h
  simp only [HasCustody, write]
  grind

inductive Trace (start : State) : State → Prop where
  | initial : Trace start start
  | packed {s t : State} {id : Identity} {k : Slot} {m : Mapping} {c : Int32} : Trace start s → pack s id k m c = some t → Trace start t
  | moved {s t : State} {id : Identity} {a b : Slot} : Trace start s → handoff s id a b = some t → Trace start t
  | opened {s t : State} {id : Identity} {k : Slot} {arm : Arm} : Trace start s → unpack s id k = some (arm, t) → Trace start t
  | released {s t : State} {id : Identity} : Trace start s → release s id = some t → Trace start t

theorem trace_exclusive (init : Exclusive start) (path : Trace start t) : Exclusive t := by
  induction path with
  | initial => exact init
  | packed _ h ih => exact pack_exclusive ih h
  | moved _ h ih => exact handoff_exclusive ih h
  | opened _ h ih => exact unpack_exclusive ih h
  | released _ h ih => exact release_exclusive ih h

theorem empty_slots_exclusive (owned : Identity → Bool) :
    Exclusive ⟨owned, fun _ => none⟩ := by simp [Exclusive]

theorem reachable_sealed_alias_refuses (init : Exclusive start) (path : Trace start t)
    (hp : t.slots k = some p) (aliasId : Identity) (same : aliasId = p.subject) :
    release t aliasId = none := by
  simp [same, release, (trace_exclusive init path k p hp).1]

/-- Negative control: manufacture two sealed copies in the initial ledger.
    The step guards then allow two releases for the same generation. -/
private def duplicateSeed : State :=
  ⟨fun _ => false, fun slot =>
    if slot < 2 then some ⟨⟨7, 1⟩, ⟨1, .custody, some 42⟩⟩ else none⟩

def twiceReleased : Bool := (do
  let (_, first) ← unpack duplicateSeed ⟨7, 1⟩ 0
  let released ← release first ⟨7, 1⟩
  let (_, second) ← unpack released ⟨7, 1⟩ 1
  release second ⟨7, 1⟩).isSome

theorem malformed_seed_allows_two_releases : twiceReleased = true := by decide

theorem malformed_seed_unreachable (init : Exclusive start) : ¬Trace start duplicateSeed := by
  intro path
  have h := trace_exclusive init path 0 ⟨⟨7,1⟩, ⟨1,.custody,some 42⟩⟩ (by rfl)
  have impossible := h.2 1 ⟨⟨7,1⟩, ⟨1,.custody,some 42⟩⟩ (by rfl) rfl
  contradiction

/-- On the packed generation, the contract selects exactly whether custody
    survives. Every other generation retains its current responsibility. -/
theorem contract_pack_custody (init : Exclusive s) (h : pack s id k m c = some t)
    (i : Identity) :
    HasCustody t i ↔ (if i = id then m.fragment = .custody else HasCustody s i) := by
  obtain ⟨_, _, rfl⟩ := pack_shape _ _ _ _ _ _ h
  unfold Exclusive at init
  simp only [HasCustody, write, selectArm]
  grind

theorem release_removes_custody (init : Exclusive s) (h : release s id = some t)
    (i : Identity) : HasCustody t i ↔ i ≠ id ∧ HasCustody s i := by
  have shape : release s id = (if s.owned id then some { s with owned := write s.owned id false } else none) := rfl
  rw [shape] at h
  split at h <;> simp_all
  subst t
  unfold Exclusive at init
  simp only [HasCustody, write]
  grind

/-- Any finite sequence of private helper calls can reuse a vacated slot. -/
inductive Transfers (id : Identity) (start : State) (origin : Slot) : State → Slot → Prop where
  | initial : Transfers id start origin start origin
  | moved {s t : State} {a b : Slot} : Transfers id start origin s a →
      handoff s id a b = some t → Transfers id start origin t b

theorem transfers_preserve_package (path : Transfers id s a t b)
    (hp : s.slots a = some p) : t.slots b = some p := by
  induction path with
  | initial => exact hp
  | moved _ h ih =>
    obtain ⟨pkg, hs, _, _, rfl⟩ := handoff_shape _ _ _ _ _ h
    simp_all [write]

end Verification
end QCore.QuantVariant
