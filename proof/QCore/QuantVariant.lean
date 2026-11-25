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

end QCore.QuantVariant
