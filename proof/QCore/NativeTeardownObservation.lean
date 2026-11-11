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

namespace QCore.NativeTeardownObservation

-- A trusted native observer and structural adapter derive an ordered release
-- plan from live, distinct constructor generations. This models the final
-- exact-plan gate only; neither C instrumentation nor Python is verified here.
def accepted (plan observed : List Nat) : Prop :=
  plan.Nodup ∧ observed = plan

theorem accepted_preserves_exact_generation_order {plan observed : List Nat}
    (h : accepted plan observed) : observed = plan := h.2

theorem accepted_has_no_duplicate_release {plan observed : List Nat}
    (h : accepted plan observed) : observed.Nodup := h.2 ▸ h.1

theorem accepted_releases_every_planned_generation {plan observed : List Nat}
    (h : accepted plan observed) (id : Nat) : id ∈ observed ↔ id ∈ plan := by
  rw [h.2]

theorem accepted_conserves_release_count {plan observed : List Nat}
    (h : accepted plan observed) : observed.length = plan.length := by rw [h.2]

theorem changed_release_order_rejected {plan observed : List Nat}
    (h : observed ≠ plan) : ¬accepted plan observed := fun a => h a.2

-- Rollback and surviving-tree teardown jointly consume the derived plan.
def acceptedPartition (plan rollback teardown : List Nat) : Prop :=
  accepted plan (rollback ++ teardown)

theorem rollback_and_teardown_conserve_count {plan rollback teardown : List Nat}
    (h : acceptedPartition plan rollback teardown) :
    rollback.length + teardown.length = plan.length := by
  have count := accepted_conserves_release_count h
  simpa using count

theorem rollback_and_teardown_are_disjoint {plan rollback teardown : List Nat}
    (h : acceptedPartition plan rollback teardown) :
    ∀ id ∈ rollback, id ∉ teardown := by
  have distinct := accepted_has_no_duplicate_release h
  intro id inRollback inTeardown
  exact (List.nodup_append.mp distinct).2.2 id inRollback id inTeardown rfl

end QCore.NativeTeardownObservation

namespace QCore.NativeTeardownObservation

-- This is a gate on a recorded result, not a theorem about arbitrary C calls.
def acceptedRejectedConstructor (requested observed : Int) (chunk : Option Nat)
    (errorBefore errorAfter : Int) (plan rollback teardown : List Nat) : Prop :=
  requested = observed ∧ chunk = none ∧ errorAfter = errorBefore ∧
    acceptedPartition plan rollback teardown

theorem rejected_constructor_returns_no_chunk {requested observed : Int}
    {chunk : Option Nat} {errorBefore errorAfter : Int} {plan rollback teardown : List Nat}
    (h : acceptedRejectedConstructor requested observed chunk errorBefore errorAfter plan rollback teardown) :
    chunk = none := h.2.1

theorem rejected_constructor_preserves_native_outputs {requested observed : Int}
    {chunk : Option Nat} {errorBefore errorAfter : Int} {plan rollback teardown : List Nat}
    (h : acceptedRejectedConstructor requested observed chunk errorBefore errorAfter plan rollback teardown) :
    observed = requested ∧ errorAfter = errorBefore := ⟨h.1.symm, h.2.2.1⟩

theorem rejected_constructor_rollback_does_not_escape {requested observed : Int}
    {chunk : Option Nat} {errorBefore errorAfter : Int} {plan rollback teardown : List Nat}
    (h : acceptedRejectedConstructor requested observed chunk errorBefore errorAfter plan rollback teardown) :
    ∀ id ∈ rollback, id ∉ teardown := rollback_and_teardown_are_disjoint h.2.2.2

end QCore.NativeTeardownObservation
