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

import QCore.WeakMemory

/-! Audited companion to CG28-WEAK-MEMORY-EVENTS (gateway rule 11t): the C11
(N1570) weak-memory checker's atomic fragment. -/
#print axioms QCore.WeakMemory.release_sequence_definition
#print axioms QCore.WeakMemory.rmw_continues_release_sequence
#print axioms QCore.WeakMemory.foreign_store_ends_release_sequence
#print axioms QCore.WeakMemory.release_acquire_synchronises
#print axioms QCore.WeakMemory.fence_fence_synchronises
#print axioms QCore.WeakMemory.fence_operation_synchronises
#print axioms QCore.WeakMemory.operation_fence_synchronises
#print axioms QCore.WeakMemory.signal_fence_is_not_a_thread_fence
#print axioms QCore.WeakMemory.signal_fence_neither_releases_nor_acquires
#print axioms QCore.WeakMemory.failures_nil_iff
#print axioms QCore.WeakMemory.check_accepts_iff
#print axioms QCore.WeakMemory.accepted_rmw_is_atomic
#print axioms QCore.WeakMemory.accepted_sw_is_expected
#print axioms QCore.WeakMemory.accepted_expected_is_supplied
#print axioms QCore.WeakMemory.accepted_is_race_free
#print axioms QCore.WeakMemory.cas_failure_no_stronger
#print axioms QCore.WeakMemory.strong_failure_observes_another_value
#print axioms QCore.WeakMemory.weak_failure_may_be_spurious
#print axioms QCore.WeakMemory.success_observes_expected
#print axioms QCore.WeakMemory.failed_cas_writes_nothing
#print axioms QCore.WeakMemory.failed_cas_reads_with_failure_order
#print axioms QCore.WeakMemory.mp_release_acquire_accepted
#print axioms QCore.WeakMemory.mp_relaxed_races
#print axioms QCore.WeakMemory.mp_fences_accepted
#print axioms QCore.WeakMemory.sb_seq_cst_weak_outcome_refused
#print axioms QCore.WeakMemory.sb_sc_fences_weak_outcome_refused
#print axioms QCore.WeakMemory.rmw_lost_update_refused
#print axioms QCore.WeakMemory.release_sequence_continues_through_rmw
#print axioms QCore.WeakMemory.release_sequence_ends_at_foreign_store
#print axioms QCore.WeakMemory.cas_weak_loop_accepted
#print axioms QCore.WeakMemory.litmus_vectors_hold
