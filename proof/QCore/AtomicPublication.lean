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

/-!
# Bounded release/acquire publication

The rule is conditional on one publisher invocation and one successful consumer
observation. It proves ordering, not termination or eventual observation.
Faithful Clang extraction and Lean-to-Rust correspondence remain stated review
premises rather than extraction theorems.
-/

namespace QCore.AtomicPublication

inductive FlagKind where | plain | volatile | atomic
deriving DecidableEq, Repr

inductive StoreOrder where | relaxed | release | seqCst
deriving DecidableEq, Repr

inductive LoadOrder where | relaxed | acquire | seqCst
deriving DecidableEq, Repr

structure Bounds where
  publisherInvocations : Nat
  successfulConsumers : Nat
deriving DecidableEq, Repr

structure Program where
  flagKind : FlagKind
  initial : Int
  published : Int
  observed : Int
  storeOrder : StoreOrder
  loadOrder : LoadOrder
  bounds : Bounds
deriving DecidableEq, Repr

def releases : StoreOrder → Prop
  | .release | .seqCst => True
  | .relaxed => False

def acquires : LoadOrder → Prop
  | .acquire | .seqCst => True
  | .relaxed => False

def identifiesRelease (p : Program) : Prop :=
  p.initial ≠ p.published ∧ p.observed = p.published

def oneShot (p : Program) : Prop :=
  p.bounds.publisherInvocations = 1 ∧ p.bounds.successfulConsumers = 1

def synchronizesWith (p : Program) : Prop :=
  p.flagKind = .atomic ∧ identifiesRelease p ∧
    releases p.storeOrder ∧ acquires p.loadOrder ∧ oneShot p

inductive Event where
  | payloadWrite | releaseStore | acquireLoad | payloadRead
deriving DecidableEq, Repr

inductive PrimitiveEdge (p : Program) : Event → Event → Prop
  | producer : PrimitiveEdge p .payloadWrite .releaseStore
  | synchronization (h : synchronizesWith p) : PrimitiveEdge p .releaseStore .acquireLoad
  | consumer : PrimitiveEdge p .acquireLoad .payloadRead

inductive HappensBefore (p : Program) : Event → Event → Prop
  | step (h : PrimitiveEdge p a b) : HappensBefore p a b
  | trans (left : HappensBefore p a b) (right : HappensBefore p b c) : HappensBefore p a c

theorem synchronized_observation_orders_payload (p : Program)
    (h : synchronizesWith p) : HappensBefore p .payloadWrite .payloadRead := by
  exact .trans (.step .producer) (.trans (.step (.synchronization h)) (.step .consumer))

theorem volatile_has_no_synchronizes_with_edge (p : Program)
    (h : p.flagKind = .volatile) : ¬ synchronizesWith p := by
  intro sync
  exact FlagKind.noConfusion (h.symm.trans sync.1)

theorem plain_has_no_synchronizes_with_edge (p : Program)
    (h : p.flagKind = .plain) : ¬ synchronizesWith p := by
  intro sync
  exact FlagKind.noConfusion (h.symm.trans sync.1)

theorem relaxed_store_has_no_synchronizes_with_edge (p : Program)
    (h : p.storeOrder = .relaxed) : ¬ synchronizesWith p := by
  simp [synchronizesWith, releases, h]

theorem relaxed_load_has_no_synchronizes_with_edge (p : Program)
    (h : p.loadOrder = .relaxed) : ¬ synchronizesWith p := by
  simp [synchronizesWith, acquires, h]

theorem repeated_publisher_has_no_synchronizes_with_edge (p : Program)
    (h : p.bounds.publisherInvocations ≠ 1) : ¬ synchronizesWith p := by
  intro sync
  exact h sync.2.2.2.2.1

def releaseAcquireFixture : Program where
  flagKind := .atomic
  initial := 0
  published := 1
  observed := 1
  storeOrder := .release
  loadOrder := .acquire
  bounds := ⟨1, 1⟩

theorem fixture_has_a_real_three_edge_path :
    synchronizesWith releaseAcquireFixture ∧
      HappensBefore releaseAcquireFixture .payloadWrite .payloadRead := by
  have sync : synchronizesWith releaseAcquireFixture := by
    simp [synchronizesWith, identifiesRelease, releases, acquires, oneShot,
      releaseAcquireFixture]
  constructor
  · exact sync
  · exact synchronized_observation_orders_payload _ sync

theorem equal_initial_and_ready_do_not_identify_the_release :
    ¬ identifiesRelease { releaseAcquireFixture with initial := 1 } := by
  simp [identifiesRelease, releaseAcquireFixture]

private def side : Event → Bool
  | .payloadWrite | .releaseStore => false
  | .acquireLoad | .payloadRead => true

private theorem no_sync_preserves_side (p : Program) (h : ¬ synchronizesWith p)
    (path : HappensBefore p a b) : side a = side b := by
  induction path with
  | step edge =>
    cases edge with
    | producer => rfl
    | consumer => rfl
    | synchronization sync => exact False.elim (h sync)
  | trans _ _ left right => exact left.trans right

theorem payload_path_requires_synchronization (p : Program)
    (path : HappensBefore p .payloadWrite .payloadRead) : synchronizesWith p := by
  apply Classical.byContradiction
  intro h
  have impossible := no_sync_preserves_side p h path
  simp [side] at impossible

theorem non_atomic_completion_has_no_payload_path (p : Program)
    (h : p.flagKind = .plain ∨ p.flagKind = .volatile) :
    ¬ HappensBefore p .payloadWrite .payloadRead := by
  intro path
  have sync := payload_path_requires_synchronization p path
  cases h with
  | inl plain => exact plain_has_no_synchronizes_with_edge p plain sync
  | inr volatile => exact volatile_has_no_synchronizes_with_edge p volatile sync

def clangStoreValue : StoreOrder → Nat
  | .relaxed => 0
  | .release => 3
  | .seqCst => 5

def clangLoadValue : LoadOrder → Nat
  | .relaxed => 0
  | .acquire => 2
  | .seqCst => 5

theorem a_zero_store_value_cannot_release (order : StoreOrder)
    (h : clangStoreValue order = 0) : ¬ releases order := by
  cases order <;> simp_all [clangStoreValue, releases]

theorem a_zero_load_value_cannot_acquire (order : LoadOrder)
    (h : clangLoadValue order = 0) : ¬ acquires order := by
  cases order <;> simp_all [clangLoadValue, acquires]

end QCore.AtomicPublication
