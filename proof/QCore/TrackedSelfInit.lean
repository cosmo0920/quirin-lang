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

import QCore.CheckedSelfInitCall

/-! Rule 36b. Explicit self-writes into exact live owned storage. Faithful
Clang extraction, embedding layout and reviewed Rust correspondence are
premises; this is an abstract effect model, not C extraction. -/
namespace QCore.TrackedSelfInit

structure Source where
  leafChecked : Bool
  directOriginalDeclaration : Bool
  exactOccurrence : Bool
  compatibleType : Bool
  uniqueRoot : Bool
  completeOrdinaryEmbedding : Bool
  boundedRootOnlyPath : Bool
  interfaceAbsent : Bool
  deriving DecidableEq, Repr

def sourceAdmits (s : Source) : Bool :=
  s.leafChecked && s.directOriginalDeclaration && s.exactOccurrence &&
  s.compatibleType && s.uniqueRoot && s.completeOrdinaryEmbedding &&
  s.boundedRootOnlyPath && s.interfaceAbsent

theorem missing_source_fact_refuses (s : Source)
    (h : s.leafChecked = false ∨ s.directOriginalDeclaration = false ∨
      s.exactOccurrence = false ∨ s.compatibleType = false ∨ s.uniqueRoot = false ∨
      s.completeOrdinaryEmbedding = false ∨ s.boundedRootOnlyPath = false ∨
      s.interfaceAbsent = false) : sourceAdmits s = false := by
  rcases h with h | h | h | h | h | h | h | h <;> simp [sourceAdmits, h]

/-- The fields remain explicit after substituting the caller's embedded path. -/
def instantiateWrites (path : List String) (fields : List String) : List (List String) :=
  fields.map (fun field => path ++ [field])

theorem every_leaf_write_is_retained (path : List String) (fields : List String)
    (field : String) (h : field ∈ fields) :
    path ++ [field] ∈ instantiateWrites path fields := by
  exact List.mem_map.mpr ⟨field, h, rfl⟩

structure Pointer where
  targets : List Nat := [7]
  unknown : Bool := false
  nullable : Bool := false
  identity : Bool := true
  deriving DecidableEq, Repr

structure Fact where
  rootTargets : List Nat
  member : Bool
  oldTargets : List Nat := []
  unknown : Bool := false
  deriving DecidableEq, Repr

structure State (α : Type) where
  pointer : Pointer := {}
  ledger : Nat → Option Nat
  nullableSites : List Nat := []
  otherCustodyEmpty : Bool := true
  facts : List Fact := []
  frame : α
  unknown : Bool := false

def affected (site : Nat) (fact : Fact) : Bool :=
  fact.member && fact.rootTargets.contains site

def oldValueClear (site : Nat) (fact : Fact) : Bool :=
  !affected site fact || (!fact.unknown && fact.oldTargets.all (· == site))

def admits (s : State α) (site : Nat) : Bool :=
  !s.unknown && !s.pointer.unknown && !s.pointer.nullable && s.pointer.identity &&
  s.pointer.targets == [site] && s.ledger site == some 1 &&
  !s.nullableSites.contains site && s.otherCustodyEmpty &&
  s.facts.all (oldValueClear site)

/-- The fallback's binding/frame result is the existing Unknown instruction.
Neither the admitted effect nor refusal discharges resource status. -/
def transfer (s : State α) (site : Nat) (fallbackFacts : List Fact)
    (fallbackFrame : α) : State α :=
  if admits s site then
    { s with facts := s.facts.filter (fun f => !affected site f) }
  else { s with facts := fallbackFacts, frame := fallbackFrame, unknown := true }

theorem resource_ledger_is_preserved (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) :
    (transfer s site facts frame).ledger = s.ledger := by
  unfold transfer; split <;> rfl

theorem nullable_facts_are_preserved (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) :
    (transfer s site facts frame).nullableSites = s.nullableSites := by
  unfold transfer; split <;> rfl

theorem admitted_root_pointer_is_preserved (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (h : admits s site = true) :
    (transfer s site facts frame).pointer = s.pointer := by
  simp [transfer, h]

theorem admitted_effect_removes_stale_member_facts (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (h : admits s site = true) :
    ∀ f ∈ (transfer s site facts frame).facts, affected site f = false := by
  simp only [transfer, h, ↓reduceIte]
  intro f hf
  have kept := (List.mem_filter.mp hf).2
  cases value : affected site f <;> simp_all

theorem unaffected_facts_survive (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (h : admits s site = true)
    (f : Fact) (mem : f ∈ s.facts) (unaffected : affected site f = false) :
    f ∈ (transfer s site facts frame).facts := by
  simp [transfer, h, mem, unaffected]

theorem no_child_or_membership_frame_is_erased (s : State α) (site : Nat)
    (h : s.otherCustodyEmpty = false) : admits s site = false := by
  simp [admits, h]

theorem a_borrowed_name_only_site_refuses (s : State α) (site : Nat)
    (h : s.ledger site = none) : admits s site = false := by simp [admits, h]

theorem refusal_is_unknown (s : State α) (site : Nat)
    (facts : List Fact) (frame : α) (h : admits s site = false) :
    (transfer s site facts frame).unknown = true := by simp [transfer, h]

def runEffects (s : State α) (sites : List Nat) (facts : List Fact) (frame : α) : State α :=
  sites.foldl (fun state site => transfer state site facts frame) s

theorem finite_composition_keeps_every_obligation (s : State α) (sites : List Nat)
    (facts : List Fact) (frame : α) :
    (runEffects s sites facts frame).ledger = s.ledger := by
  induction sites generalizing s with
  | nil => rfl
  | cons site rest ih =>
    exact (ih (transfer s site facts frame)).trans (resource_ledger_is_preserved s site facts frame)

/-- An embedded object's storage ends with its containing allocation. -/
structure Embedded where
  parent : Nat
  object : Nat

def selfEdges (target : Embedded) (fields : List String) : List (Nat × Nat) :=
  fields.map (fun _ => (target.object, target.object))

theorem concrete_self_writes_preserve_lifetimes (ends : Nat → Nat)
    (old : List (Nat × Nat)) (target : Embedded) (fields : List String)
    (valid : CheckedSelfInitCall.lifetimeValid ends old) :
    CheckedSelfInitCall.lifetimeValid ends (selfEdges target fields ++ old) := by
  intro edge mem
  rcases List.mem_append.mp mem with new | previous
  · obtain ⟨_, _, eq⟩ := List.mem_map.mp new
    subst edge
    exact Nat.le_refl _
  · exact valid edge previous

theorem embedded_self_edges_end_with_the_parent (ends : Nat → Nat)
    (target : Embedded) (sameStorage : ends target.object = ends target.parent)
    (fields : List String) :
    ∀ edge ∈ selfEdges target fields, ends edge.2 ≤ ends target.parent := by
  intro edge mem
  obtain ⟨_, _, eq⟩ := List.mem_map.mp mem
  subst edge
  simp [sameStorage]

inductive Consumer where
  | checkedLocalEffect | ordinarySummary | callback | exported
  deriving DecidableEq

def available (consumer : Consumer) (sourceChecked : Bool) : Bool :=
  consumer == .checkedLocalEffect && sourceChecked

theorem no_neutral_summary_transport (s : Bool) : available .ordinarySummary s = false := by rfl
theorem no_indirect_callback_transport (s : Bool) : available .callback s = false := by rfl
theorem no_cross_unit_effect_privilege (s : Bool) : available .exported s = false := by rfl

def initial : State Unit := { ledger := fun n => if n == 7 then some 1 else none, frame := () }
def flowVectors : List Bool :=
  [ admits initial 7,
    admits { initial with ledger := fun _ => none } 7,
    admits { initial with ledger := fun _ => some 2 } 7,
    admits { initial with ledger := fun _ => some 4 } 7,
    admits { initial with ledger := fun _ => some 3 } 7,
    admits { initial with pointer := { unknown := true } } 7,
    admits { initial with pointer := { nullable := true } } 7,
    admits { initial with nullableSites := [7] } 7,
    admits { initial with pointer := { identity := false } } 7,
    admits { initial with pointer := { targets := [7,8] } } 7,
    admits { initial with pointer := { targets := [] } } 7,
    admits { initial with otherCustodyEmpty := false } 7,
    admits { initial with facts := [⟨[7],true,[],false⟩] } 7 ]

theorem shared_flow_vectors : flowVectors =
    [true,false,false,false,false,false,false,false,false,false,false,false,true] := by decide

/-- One ordinary embedded target followed by loaded pointer, indexed target,
cast, volatile view and union embedding refusals, in Rust test order. -/
def completeSource : Source := ⟨true,true,true,true,true,true,true,true⟩
def sourceTargetVectors : List Bool :=
  [ sourceAdmits completeSource,
    sourceAdmits { completeSource with boundedRootOnlyPath := false },
    sourceAdmits { completeSource with boundedRootOnlyPath := false },
    sourceAdmits { completeSource with boundedRootOnlyPath := false },
    sourceAdmits { completeSource with completeOrdinaryEmbedding := false },
    sourceAdmits { completeSource with completeOrdinaryEmbedding := false } ]
theorem shared_source_target_vectors :
    sourceTargetVectors = [true,false,false,false,false,false] := by decide

end QCore.TrackedSelfInit
