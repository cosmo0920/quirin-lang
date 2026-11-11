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

import Std

namespace QCore.MonkeyWaitProjection

/-! Abstract parity model for the sealed Monkey timed-wait source projection.

The Rust boundary additionally authenticates constructor-private typed source
summaries and their canonical digests. This model states the semantic decision:
only the exact selected callback/wrapper source family produces the ordered
native obligations. It deliberately has no event-program or Safe constructor. -/

inductive SourceOperation where
  | call (name : String)
  | returned
  deriving DecidableEq, Repr

structure SelectedSource where
  name : String
  pathCount : Nat
  selectedPath : Nat
  operations : List SourceOperation
  deriving DecidableEq, Repr

inductive SourceArtifactIdentity where
  | fixture
  | production
  | other (digest : String)
  deriving DecidableEq, Repr

inductive RequiredObservation where
  | baseCreateSuccess
  | eventCreateTimeoutNoDescriptorWithPriority
  | eventAddSuccess
  | dispatchBegin
  | timeoutActivation
  | callbackEnter
  | callbackExit
  | dispatchEnd
  | eventFree
  | argumentScopeEnd
  | baseFree
  deriving DecidableEq, Repr

inductive ContractFact where
  | callbackArgumentBorrowed
  | callbackHasNoResourceEffect
  | loopCreateReturnsInitializedContext
  | waitClearsCountBeforeDispatch
  | waitCopiesFinalCount
  | timedWaitUsesStackTimeval
  | timedWaitLeavesCreateAndAddUnchecked
  | timedWaitFreesEventAfterDispatch
  | destroyRequiresValidProvenance
  | destroyFreesBaseThenStorage
  deriving DecidableEq, Repr

def requiredSource : List SelectedSource := [
  ⟨"cb_wait_2_timeout", 1, 0, [.returned]⟩,
  ⟨"_mk_event_loop_create", 4, 3,
    [.call "_mk_event_init", .call "mk_mem_alloc_z", .call "event_base_new",
      .call "mk_mem_alloc_z", .returned]⟩,
  ⟨"_mk_event_wait_with_flags", 1, 0, [.call "event_base_loop", .returned]⟩,
  ⟨"_mk_event_wait_2", 3, 2,
    [.call "event_new", .call "event_add", .call "_mk_event_wait_with_flags",
      .call "event_free", .returned]⟩,
  ⟨"_mk_event_loop_destroy", 1, 0,
    [.call "event_base_free", .call "mk_mem_free", .call "mk_mem_free", .returned]⟩
]

def requiredObservations : List RequiredObservation := [
  .baseCreateSuccess,
  .eventCreateTimeoutNoDescriptorWithPriority,
  .eventAddSuccess,
  .dispatchBegin,
  .timeoutActivation,
  .callbackEnter,
  .callbackExit,
  .dispatchEnd,
  .eventFree,
  .argumentScopeEnd,
  .baseFree
]

def requiredContracts : List ContractFact := [
  .callbackArgumentBorrowed,
  .callbackHasNoResourceEffect,
  .loopCreateReturnsInitializedContext,
  .waitClearsCountBeforeDispatch,
  .waitCopiesFinalCount,
  .timedWaitUsesStackTimeval,
  .timedWaitLeavesCreateAndAddUnchecked,
  .timedWaitFreesEventAfterDispatch,
  .destroyRequiresValidProvenance,
  .destroyFreesBaseThenStorage
]

structure SourceProjection where
  checkedSource : List SelectedSource
  checkedContracts : List ContractFact
  obligations : List RequiredObservation
  deriving DecidableEq, Repr

def approvedIdentity : SourceArtifactIdentity → Bool
  | .fixture | .production => true
  | .other _ => false

def project (source : List SelectedSource) (contracts : List ContractFact)
    (identity : SourceArtifactIdentity) : Option SourceProjection :=
  if source = requiredSource ∧ contracts = requiredContracts ∧ approvedIdentity identity then
    some ⟨source, contracts, requiredObservations⟩
  else
    none

theorem accepted_projection_has_exact_source
    {source : List SelectedSource} {projection : SourceProjection}
    {contracts : List ContractFact} {identity : SourceArtifactIdentity}
    (accepted : project source contracts identity = some projection) :
    source = requiredSource := by
  unfold project at accepted
  split at accepted <;> rename_i condition
  · exact condition.1
  · simp at accepted

theorem accepted_projection_has_exact_observation_obligations
    {source : List SelectedSource} {projection : SourceProjection}
    {contracts : List ContractFact} {identity : SourceArtifactIdentity}
    (accepted : project source contracts identity = some projection) :
    projection.obligations = requiredObservations := by
  unfold project at accepted
  split at accepted
  · have equal :
        (⟨source, contracts, requiredObservations⟩ : SourceProjection) =
          projection := Option.some.inj accepted
    rw [← equal]
  · simp at accepted

theorem accepted_callback_is_the_sole_return_path
    {source : List SelectedSource} {projection : SourceProjection}
    {contracts : List ContractFact} {identity : SourceArtifactIdentity}
    (accepted : project source contracts identity = some projection) :
    source.head? = some ⟨"cb_wait_2_timeout", 1, 0, [.returned]⟩ := by
  rw [accepted_projection_has_exact_source accepted]
  rfl

theorem exact_source_is_accepted :
    (project requiredSource requiredContracts .production).isSome = true := by
  rfl

theorem accepted_projection_has_exact_contracts
    {source : List SelectedSource} {contracts : List ContractFact}
    {identity : SourceArtifactIdentity} {projection : SourceProjection}
    (accepted : project source contracts identity = some projection) :
    contracts = requiredContracts := by
  unfold project at accepted
  split at accepted <;> rename_i condition
  · exact condition.2.1
  · simp at accepted

def missingDestroy : List SelectedSource := requiredSource.take 4

theorem missing_lifecycle_source_is_rejected :
    project missingDestroy requiredContracts .production = none := by
  decide

def callbackWithEffect : List SelectedSource :=
  ⟨"cb_wait_2_timeout", 1, 0, [.call "perror", .returned]⟩ :: requiredSource.tail

theorem callback_effect_substitution_is_rejected :
    project callbackWithEffect requiredContracts .production = none := by
  decide

def waitWithoutFree : List SelectedSource :=
  requiredSource.take 3 ++ [
    ⟨"_mk_event_wait_2", 3, 2,
      [.call "event_new", .call "event_add", .call "_mk_event_wait_with_flags", .returned]⟩,
    ⟨"_mk_event_loop_destroy", 1, 0,
      [.call "event_base_free", .call "mk_mem_free", .call "mk_mem_free", .returned]⟩
  ]

theorem missing_event_free_is_rejected :
    project waitWithoutFree requiredContracts .production = none := by
  decide

def missingNoEffectContract : List ContractFact :=
  requiredContracts.erase .callbackHasNoResourceEffect

theorem missing_callback_contract_is_rejected :
    project requiredSource missingNoEffectContract .production = none := by
  decide

theorem substituted_source_identity_is_rejected :
    project requiredSource requiredContracts (.other "changed") = none := by
  decide

end QCore.MonkeyWaitProjection
