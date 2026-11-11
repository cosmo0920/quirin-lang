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
import QCore.SizeofOperand

/-! Rule 37. The old aggregate base scan sees the `*` of sizeof(*owner).
Only the existing typed non-VLA operand reader can exempt that occurrence.
C extraction, checked-span fidelity and Rust/model correspondence remain premises.
This module does not equate a nullable owner with a live non-null owner. -/
namespace QCore.ResultConstructors

inductive Occurrence where
  | evaluatedDeref
  | checkedSizeofDeref
  | otherEscape
  | neutral
  deriving DecidableEq, Repr

def escapesBefore : Occurrence → Bool
  | .neutral => false
  | _ => true

def escapesAfter : Occurrence → Bool
  | .checkedSizeofDeref | .neutral => false
  | _ => true

def baseEscapes (uses : List Occurrence) : Bool := uses.any escapesAfter

theorem an_evaluated_dereference_still_refuses (uses : List Occurrence)
    (h : .evaluatedDeref ∈ uses) : baseEscapes uses = true := by
  exact List.any_eq_true.mpr ⟨_, h, rfl⟩

theorem another_escape_still_refuses (uses : List Occurrence)
    (h : .otherEscape ∈ uses) : baseEscapes uses = true := by
  exact List.any_eq_true.mpr ⟨_, h, rfl⟩

theorem only_a_checked_operand_is_newly_exempt (occurrence : Occurrence)
    (before : escapesBefore occurrence = true) (after : escapesAfter occurrence = false) :
    occurrence = .checkedSizeofDeref := by
  cases occurrence <;> simp_all [escapesBefore, escapesAfter]

theorem no_checked_operand_keeps_the_old_scan (uses : List Occurrence)
    (h : .checkedSizeofDeref ∉ uses) :
    baseEscapes uses = uses.any escapesBefore := by
  induction uses with
  | nil => rfl
  | cons occurrence rest ih =>
    simp only [List.mem_cons, not_or] at h
    have ne := h.1
    have tail := h.2
    cases occurrence <;> simp_all [baseEscapes, escapesBefore, escapesAfter]

/-- The existing store whitelist, with only the escape occurrence recomputed. -/
def storeShape (s : QCore.AggregateStore.Store) (uses : List Occurrence) :=
  {s with baseEscapesPointer := baseEscapes uses}

def constructorStep (s : QCore.AggregateStore.Store) (uses : List Occurrence)
    (o : QCore.AggregateStore.Owner) :=
  QCore.AggregateStore.storeStep (storeShape s uses) o

theorem a_nullable_constructor_owner_never_links
    (s : QCore.AggregateStore.Store) (uses : List Occurrence)
    (o : QCore.AggregateStore.Owner) (nullable : o.nonNull = false) :
    constructorStep s uses o = .refused := by
  apply QCore.AggregateStore.an_unresolved_owner_never_links
  exact QCore.AggregateStore.a_possibly_null_aggregate_never_resolves o nullable

theorem a_checked_sizeof_constructor_can_link :
    constructorStep QCore.AggregateStore.admittedExample [.checkedSizeofDeref]
      QCore.AggregateStore.liveOwner = .linked := by rfl

theorem an_evaluated_dereference_refuses_the_same_constructor :
    constructorStep QCore.AggregateStore.admittedExample [.evaluatedDeref]
      QCore.AggregateStore.liveOwner = .refused := by rfl

theorem a_checked_sizeof_does_not_excuse_an_alias :
    constructorStep { QCore.AggregateStore.admittedExample with baseAliased := true }
      [.checkedSizeofDeref] QCore.AggregateStore.liveOwner = .refused := by rfl

/-- Abstract normal-return semantics of a checked allocation/assert wrapper.
The abort arm has no normal return; the declared noreturn call is a premise. -/
def wrapperNormalResult (asserts : Bool) (allocation : Option Nat) : Option (Option Nat) :=
  if asserts && allocation.isNone then none else some allocation

theorem an_asserting_wrapper_never_returns_null (allocation : Option Nat)
    (result : Option Nat) (h : wrapperNormalResult true allocation = some result) :
    result ≠ none := by
  cases allocation with
  | none => simp [wrapperNormalResult] at h
  | some identity =>
    have same : some identity = result := by
      simpa [wrapperNormalResult] using h
    rw [← same]
    simp

theorem an_assert_disabled_wrapper_can_return_null :
    wrapperNormalResult false none = some none := by decide

theorem an_asserting_allocation_preserves_its_identity (identity : Nat) :
    wrapperNormalResult true (some identity) = some (some identity) := by rfl

theorem a_constructor_return_carries_the_linked_child :
    QCore.AggregateStore.childAfter .escape .live = .escaped := by decide

theorem a_shallow_release_does_not_discharge_the_child :
    QCore.AggregateStore.childAfter .release .live = .live := by decide

/-- Null-result readers take an independent source proof before a declaration.
A checked custody specification cannot supply an unchecked declaration fact. -/
def nullResultFact (sourceProof checked declared : Bool) : Bool :=
  sourceProof || (!checked && declared)

theorem a_checked_result_requires_the_independent_source_proof
    (sourceProof declared : Bool) :
    nullResultFact sourceProof true declared = sourceProof := by
  simp [nullResultFact]

theorem an_ordinary_declaration_keeps_its_result_fact
    (sourceProof declared : Bool) :
    nullResultFact sourceProof false declared = (sourceProof || declared) := by
  simp [nullResultFact]

theorem an_independent_source_proof_is_still_usable (checked declared : Bool) :
    nullResultFact true checked declared = true := by
  simp [nullResultFact]

/-- The span exemption reuses the checked operand model's no-effect theorem;
it does not assume that every syntactic sizeof is unevaluated. -/
theorem the_checked_operand_contributes_no_escape_event
    (facts : List QCore.SizeofOperand.DeclFact) (op : QCore.SizeofOperand.Operand)
    (baseVM : QCore.SizeofOperand.DeclFact → Bool)
    (hf : ∀ d ∈ facts, QCore.SizeofOperand.Faithful d (baseVM d))
    (h : QCore.SizeofOperand.admitted (some facts) op = true)
    (d : QCore.SizeofOperand.DeclFact) (hd : d ∈ facts)
    (effects : List QCore.SizeofOperand.Effect) :
    ∃ t, QCore.SizeofOperand.operandType op (d.type (baseVM d)) = some t ∧
      QCore.SizeofOperand.sizeofEffects t effects = [] := by
  exact QCore.SizeofOperand.an_admitted_operand_has_no_effect facts op baseVM hf h d hd effects

end QCore.ResultConstructors
