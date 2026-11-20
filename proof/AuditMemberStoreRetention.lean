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

import QCore.MemberStoreRetention

/-! Audited companion to CG10-MEMBER-STORE-RETAINED-BY (gateway rule 70): a
formal's pointer stored into a member of another formal's object is read as
`retained_by`, stated in the summary and decided at each call by rule 12t's
table. The flow decision is bound to `member_store_retention.rs`'s
`DECISION_VECTORS`; the soundness argument is over a concrete memory whose
holders keep pointers. -/

#print axioms QCore.MemberStoreRetention.decision_vectors_hold
#print axioms QCore.MemberStoreRetention.admitted_iff_untracked_and_carrying_nothing
#print axioms QCore.MemberStoreRetention.rule_10v_refuses_every_formal_value
#print axioms QCore.MemberStoreRetention.a_call_takes_no_custody
#print axioms QCore.MemberStoreRetention.an_unnamed_overwrite_refuses
#print axioms QCore.MemberStoreRetention.a_callers_object_is_recorded
#print axioms QCore.MemberStoreRetention.a_summary_call_states_its_retention
#print axioms QCore.MemberStoreRetention.a_tracked_object_is_held
#print axioms QCore.MemberStoreRetention.an_undecided_actual_refuses
#print axioms QCore.MemberStoreRetention.a_function_pointer_adds_no_obligation
#print axioms QCore.MemberStoreRetention.step_sound
#print axioms QCore.MemberStoreRetention.checked_traces_do_not_fault
#print axioms QCore.MemberStoreRetention.an_admitted_caller_never_faults
#print axioms QCore.MemberStoreRetention.checkAll_append
#print axioms QCore.MemberStoreRetention.caller_discharge_composes
#print axioms QCore.MemberStoreRetention.summary_calls_compose
#print axioms QCore.MemberStoreRetention.a_static_store_adds_no_obligation
#print axioms QCore.MemberStoreRetention.releasing_the_retained_pointer_is_refused
#print axioms QCore.MemberStoreRetention.the_refused_trace_faults_when_the_holder_reads
#print axioms QCore.MemberStoreRetention.a_reset_before_the_release_discharges_it
#print axioms QCore.MemberStoreRetention.a_caller_that_cannot_discharge_is_refused
