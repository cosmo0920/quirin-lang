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

set_option maxRecDepth 10000

namespace QCore.MonkeyWaitSourcePolicy

/-! Executable parity model for `monkey/wait_source_policy.qv`.

The model separates facts derived from typed source summaries from the trusted
expected values. In particular, artifact acceptance compares one complete
digest vector with either the fixture vector or the production vector; it has
no per-summary fallback that could admit a mixed vector. -/

structure SummaryFacts where
  name : String
  signature : String
  pathCount : Nat
  importRole : String
  pathOperationCounts : String
  pathCallSequences : String
  selectedPathIndex : Nat
  selectedOperationCount : Nat
  selectedCalls : String
  selectedReturnKind : String
  deriving DecidableEq, Repr

structure SummaryPolicy where
  name : String
  signatures : List String
  pathCount : Nat
  importRole : String
  pathOperationCounts : String
  pathCallSequences : String
  selectedPathIndex : Nat
  selectedOperationCount : Nat
  selectedCalls : String
  selectedReturnKind : String
  deriving DecidableEq, Repr

def summaryAccepted (facts : SummaryFacts) (policy : SummaryPolicy) : Bool :=
  facts.name == policy.name &&
  policy.signatures.contains facts.signature &&
  facts.pathCount == policy.pathCount &&
  facts.importRole == policy.importRole &&
  facts.pathOperationCounts == policy.pathOperationCounts &&
  facts.pathCallSequences == policy.pathCallSequences &&
  facts.selectedPathIndex == policy.selectedPathIndex &&
  facts.selectedOperationCount == policy.selectedOperationCount &&
  facts.selectedCalls == policy.selectedCalls &&
  facts.selectedReturnKind == policy.selectedReturnKind

def requiredSummaries : List SummaryPolicy := [
  ⟨"cb_wait_2_timeout",
    ["void (evutil_socket_t, short, void *)", "void (int, short, void *)"],
    1, "Callback", "1", "", 0, 1, "", "return"⟩,
  ⟨"_mk_event_loop_create", ["void *(int)"], 4, "Wrapper", "4,7,14,14",
    "_mk_event_init;_mk_event_init,mk_mem_alloc_z;_mk_event_init,mk_mem_alloc_z,event_base_new,mk_mem_alloc_z,mk_mem_free;_mk_event_init,mk_mem_alloc_z,event_base_new,mk_mem_alloc_z",
    3, 14, "_mk_event_init,mk_mem_alloc_z,event_base_new,mk_mem_alloc_z", "return_value"⟩,
  ⟨"_mk_event_wait_with_flags", ["int (struct mk_event_loop *, int)"], 1,
    "Wrapper", "10", "event_base_loop", 0, 10, "event_base_loop", "return_value"⟩,
  ⟨"_mk_event_wait_2", ["int (struct mk_event_loop *, int)"], 3, "Wrapper",
    "13,14,20",
    "_mk_event_wait_with_flags;_mk_event_wait_with_flags;event_new,event_add,_mk_event_wait_with_flags,event_free",
    2, 20, "event_new,event_add,_mk_event_wait_with_flags,event_free", "return_value"⟩,
  ⟨"_mk_event_loop_destroy", ["void (struct mk_event_ctx *)"], 1, "Wrapper",
    "6", "event_base_free,mk_mem_free,mk_mem_free", 0, 6,
    "event_base_free,mk_mem_free,mk_mem_free", "return"⟩
]

def acceptedSummaries (facts : List SummaryFacts) : Bool :=
  facts.length == requiredSummaries.length &&
  (facts.zip requiredSummaries).all fun pair => summaryAccepted pair.1 pair.2

structure ContractDeclarationFacts where
  name : String
  declarationIsExtern : Bool
  declarationHasBody : Bool
  declarationIsUnsafe : Bool
  declarationHasEffects : Bool
  parameterNames : String
  parameterTypes : String
  parameterUsages : String
  resultType : String
  clauseSequence : String
  deriving DecidableEq, Repr

def requiredContracts : List ContractDeclarationFacts := [
  ⟨"cb_wait_2_timeout", true, false, false, false, "callback_argument",
    "borrow<'callback,const,MonkeyCallbackArgument>", "one", "unit",
    "requires:callback_argument_is_borrowed|normal:callback_has_no_resource_effect"⟩,
  ⟨"_mk_event_loop_create", true, false, false, false, "size", "i32", "none",
    "ptr<mut,EventContext>",
    "requires:selected_unix_libevent_source_profile|normal:initialization_or_context_allocation_failure_returns_null|normal:event_base_new_result_is_stored_without_a_null_test|normal:fired_allocation_failure_frees_context_without_base_free|normal:successful_return_sets_queue_size_and_returns_context"⟩,
  ⟨"_mk_event_wait_with_flags", true, false, false, false, "loop,flags",
    "borrow<'call,mut,EventLoop>,i32", "one,none", "i32",
    "requires:selected_unix_libevent_source_profile|normal:fired_count_is_cleared_before_dispatch|normal:dispatch_result_is_ignored|normal:final_fired_count_is_copied_to_loop_and_returned"⟩,
  ⟨"_mk_event_wait_2", true, false, false, false, "loop,timeout",
    "borrow<'call,mut,EventLoop>,i32", "one,none", "i32",
    "requires:selected_unix_libevent_source_profile|normal:infinite_and_zero_wait_paths_delegate_without_timer_creation|normal:timed_wait_uses_explicit_stack_timeval|normal:timeout_event_creation_and_registration_results_are_not_checked|normal:timed_wait_calls_event_free_after_dispatch_and_returns_dispatch_result"⟩,
  ⟨"_mk_event_loop_destroy", true, false, false, false, "ctx", "own<EventContext>",
    "one", "unit",
    "requires:selected_unix_libevent_source_profile|requires:context_and_children_have_valid_destruction_provenance|normal:calls_base_free_then_fired_free_then_context_free|normal:nested_event_and_map_custody_still_requires_composition"⟩
]

def acceptedContracts (facts : List ContractDeclarationFacts) : Bool :=
  facts == requiredContracts

structure ArtifactDigests where
  callback : String
  create : String
  waitWithFlags : String
  timedWait : String
  destroy : String
  deriving DecidableEq, Repr

def fixtureDigests : ArtifactDigests :=
  ⟨"8308bac83724f6f2417f6d22bbf3e65afbbf491b8ad91b4a9217b1aebd22efdc",
   "2e32d6c7bbc40031c0f9828031d1591478cc2d556c2504dcf4f2687b0943214d",
   "3637e3f8ddfb6ae389d571a05f13bb9e32e675e95bfb4d5134ee8ab73923ff1f",
   "88d9d6f2d7dad652503bd218ac9fad80c8c16afc53beb8ec0e174e2232628dff",
   "c0d3215c02ed7f421f725596fbcee27547d0cf2a5f8534aa532f794cbd9a5e0e"⟩

def productionDigests : ArtifactDigests :=
  ⟨"8308bac83724f6f2417f6d22bbf3e65afbbf491b8ad91b4a9217b1aebd22efdc",
   "d1c48805630eacec138b0ed1eae834195815b9e4ce61a731c40ac3fd46bc7756",
   "3637e3f8ddfb6ae389d571a05f13bb9e32e675e95bfb4d5134ee8ab73923ff1f",
   "69511d42a65bbed00f7c54e2890cc641578cf60029260893a8aa2db585bca4bd",
   "c0d3215c02ed7f421f725596fbcee27547d0cf2a5f8534aa532f794cbd9a5e0e"⟩

def artifactAccepted (digests : ArtifactDigests) : Bool :=
  digests == fixtureDigests || digests == productionDigests

structure PolicyInput where
  revision : String
  library : String
  sourceSet : String
  profileDigest : String
  callback : String
  eventFlags : Nat
  descriptorAbsent : Bool
  priorityRequired : Bool
  summaries : List SummaryFacts
  contracts : List ContractDeclarationFacts
  artifacts : ArtifactDigests
  deriving DecidableEq, Repr

def accepted (input : PolicyInput) : Bool :=
  input.revision == "66910c10a4d7eafa810b84229cf2dcf7b0f26f97" &&
  (input.library == "monkey" &&
  (input.sourceSet == "fluent-bit" &&
  (!input.profileDigest.isEmpty &&
  (input.callback == "cb_wait_2_timeout" &&
  (input.eventFlags == 1 &&
  (input.descriptorAbsent &&
  (input.priorityRequired &&
  (acceptedSummaries input.summaries &&
  (acceptedContracts input.contracts &&
  artifactAccepted input.artifacts)))))))))

def fixtureFacts : List SummaryFacts := requiredSummaries.map fun policy =>
  ⟨policy.name, policy.signatures.head!, policy.pathCount, policy.importRole,
   policy.pathOperationCounts, policy.pathCallSequences, policy.selectedPathIndex,
   policy.selectedOperationCount, policy.selectedCalls, policy.selectedReturnKind⟩

def fixtureInput : PolicyInput :=
  ⟨"66910c10a4d7eafa810b84229cf2dcf7b0f26f97", "monkey", "fluent-bit",
   "profile", "cb_wait_2_timeout", 1, true, true, fixtureFacts, requiredContracts,
   fixtureDigests⟩

theorem exact_fixture_is_accepted : accepted fixtureInput = true := by decide

theorem accepted_has_pinned_revision {input : PolicyInput}
    (h : accepted input = true) :
    input.revision = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97" := by
  simp [accepted] at h
  exact h.1

def mixedArtifacts : ArtifactDigests :=
  { fixtureDigests with create := productionDigests.create }

theorem mixed_artifact_vector_is_rejected :
    artifactAccepted mixedArtifacts = false := by decide

def wrongSelectedPath : List SummaryFacts := fixtureFacts.modify 3 fun facts =>
  { facts with selectedPathIndex := 1 }

theorem changed_selected_path_is_rejected :
    acceptedSummaries wrongSelectedPath = false := by decide

def missingTimedWait : List SummaryFacts := fixtureFacts.eraseIdx 3

theorem missing_summary_is_rejected :
    acceptedSummaries missingTimedWait = false := by decide

def changedCallSequence : List SummaryFacts := fixtureFacts.modify 3 fun facts =>
  { facts with selectedCalls := "event_new,event_add,_mk_event_wait_with_flags" }

theorem changed_call_sequence_is_rejected :
    acceptedSummaries changedCallSequence = false := by decide

def changedContractClause : List ContractDeclarationFacts :=
  requiredContracts.modify 3 fun facts => { facts with clauseSequence := "changed" }

theorem changed_contract_clause_is_rejected :
    acceptedContracts changedContractClause = false := by decide

end QCore.MonkeyWaitSourcePolicy
