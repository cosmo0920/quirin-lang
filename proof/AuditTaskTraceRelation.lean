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

import QCore.TaskTraceRelation

/-! F02: one source-to-native trace relation.

The Rust side is production code — `qcore_core::task_trace_relation` — measured
by `crates/qcore-core/tests/task_trace_relation.rs` and, on real native runs, by
`compiler/tests/f02_trace_relation.rs`. This audit covers the Lean
transcription and the finite-domain declarations that tie the two together.

Nothing here proves the Rust implementation correct, and nothing here says a
native execution took the order it rendered. Each domain theorem is a kernel
evaluation over the declared domain, pinning a number the Rust harness
recomputes independently. -/

-- The relation refuses what it cannot attribute or distinguish, for arbitrary
-- traces rather than only on a domain
#print axioms QCore.TaskTraceRelation.unattributable_is_refused
#print axioms QCore.TaskTraceRelation.duplicate_occurrences_are_refused

-- ... and an admitted trace really did pass every earlier conjunct
#print axioms QCore.TaskTraceRelation.admitted_traces_are_attributable
#print axioms QCore.TaskTraceRelation.admitted_traces_have_distinct_occurrences
#print axioms QCore.TaskTraceRelation.admitted_traces_satisfy_the_order_constraints
#print axioms QCore.TaskTraceRelation.admitted_traces_report_the_same_facts

-- Both selected reference runs are admitted causal orders
#print axioms QCore.TaskTraceRelation.the_round_trip_is_an_admitted_causal_order
#print axioms QCore.TaskTraceRelation.the_concurrent_run_is_an_admitted_causal_order

-- The enumerated domains are the ones the Rust harness enumerates
#print axioms QCore.TaskTraceRelation.domain_one_size
#print axioms QCore.TaskTraceRelation.domain_two_size
#print axioms QCore.TaskTraceRelation.domain_three_size
#print axioms QCore.TaskTraceRelation.domain_one_trace_digest
#print axioms QCore.TaskTraceRelation.domain_two_trace_digest
#print axioms QCore.TaskTraceRelation.domain_three_trace_digest

-- The transcription classifies every trace of every domain exactly as the Rust
-- relation does: these digests fold the class codes themselves
#print axioms QCore.TaskTraceRelation.domain_one_verdict_digest
#print axioms QCore.TaskTraceRelation.domain_two_verdict_digest
#print axioms QCore.TaskTraceRelation.domain_three_verdict_digest

-- The card's own statement: comparison is by admitted causal constraints, not
-- by an identical total schedule
#print axioms QCore.TaskTraceRelation.domain_one_admits_only_the_reference_order
#print axioms QCore.TaskTraceRelation.domain_two_admits_six_schedules
#print axioms QCore.TaskTraceRelation.domain_two_admits_the_reference_rendering
#print axioms QCore.TaskTraceRelation.domain_two_admits_a_schedule_other_than_the_reference
#print axioms QCore.TaskTraceRelation.domain_three_admits_eighteen_perturbations

-- ... and its non-vacuity in both directions
#print axioms QCore.TaskTraceRelation.domain_one_admits_one_schedule
#print axioms QCore.TaskTraceRelation.domain_one_refuses_part_of_its_domain
#print axioms QCore.TaskTraceRelation.domain_two_refuses_part_of_its_domain
#print axioms QCore.TaskTraceRelation.domain_three_refuses_most_of_its_domain
