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

import QCore.MutexShareCrossEmission

/-! Card `MUTEX-SHARE-CROSS-EMISSION`: rendering a checked mutex crossing with
real C11 threads.

Card `MUTEX-SHARE-CROSS` (wave 32) gave `source.mutex.guard.v1` a declared
crossing and recorded, as the residual this card discharges, that the C backend
rendered nothing for any trace carrying task structure — the crossing was
checked and never executed. This module covers the decision that replaces that
refusal, in both directions.

* **Fail-closed.** A trace the classification refuses has no rendering at all,
  and the reference interpreter refuses exactly where the backend does. There is
  no partial or approximate lowering.
* **The fork/join premise.** Every event strictly between a segment's `spawn`
  and its `join` belongs to that child. This is what makes "create the thread,
  then block in `thrd_join`" a realisation of the *checked* order rather than
  one interleaving of it, and it is proved rather than assumed.
* **The boundary, proved rather than promised.**
  `the_creator_performs_no_event_while_its_child_runs` is the same fact read the
  other way: the creator is idle for the whole of the child's segment, so the
  two threads' trace events never interleave. This backend therefore witnesses
  **nothing** about contention or about any data race, and no such claim is made
  here or in the CT manifest row
  (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`). What it does witness is a real
  thread boundary: the payload is reached, and the generation protecting it
  acquired and released, by the child's own thread.
* **The two renderings agree.** The statement renderer and the observation
  renderer are written separately, as `emit_mutex_c11` and
  `interpret_mutex_program` are in Rust, and running the emitted program under
  the schedule the rendering realises prints exactly the reference observation.
  That a logging statement prints its own argument is this model's one premise
  about C, as in `QCore.ProtectionGrowthEmission`.
* **Every logged acquisition is a real one.** This exists because a mutation
  escaped without it: deleting a real `mtx_lock` while keeping its log line left
  the sanitized native run byte-identical, since that run compares the *logged*
  sequence. What rules it out is that one renderer writes the operation and its
  log adjacently — proved here, and asserted on the emitted text by
  `compiler/tests/ct_corpus.rs::run_mutex_crossing`.
* **Decidable scenarios** over CT46's own shape: it classifies as a fork/join
  trace, it renders, the child's section is in the child's function and not in
  the creator's, its output is the reference observation, and six ways out of
  the shape are each refused at their own event.

The model does not claim that two threads run concurrently; the schedule it
formalises is the serial one the checked total order describes. -/

-- Fail-closed
#print axioms QCore.MutexShareCrossEmission.outside_renders_nothing
#print axioms QCore.MutexShareCrossEmission.refusals_agree

-- The fork/join premise, and the boundary it implies
#print axioms QCore.MutexShareCrossEmission.segmentEnd_finds_the_join
#print axioms QCore.MutexShareCrossEmission.segmentEnd_is_contiguous
#print axioms QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs

-- The emitted program and the reference execution agree
#print axioms QCore.MutexShareCrossEmission.step_renderings_agree
#print axioms QCore.MutexShareCrossEmission.runStatements_of_a_thread_free_body
#print axioms QCore.MutexShareCrossEmission.a_thread_free_body_observes_the_reference
#print axioms QCore.MutexShareCrossEmission.the_emitted_program_observes_the_reference

-- Every logged mutex operation is a real one
#print axioms QCore.MutexShareCrossEmission.a_logged_acquisition_is_preceded_by_a_real_one
#print axioms QCore.MutexShareCrossEmission.a_logged_release_is_preceded_by_a_real_one
#print axioms QCore.MutexShareCrossEmission.an_error_exit_still_releases
#print axioms QCore.MutexShareCrossEmission.quiet_events_perform_no_mutex_operation
#print axioms QCore.MutexShareCrossEmission.task_structure_renders_no_mutex_operation

-- A child's section runs in the child's own function
#print axioms QCore.MutexShareCrossEmission.a_childs_section_is_rendered_into_the_childs_own_function
#print axioms QCore.MutexShareCrossEmission.the_creators_body_carries_the_thread_operations
#print axioms QCore.MutexShareCrossEmission.every_access_runs_inside_its_own_critical_section

-- CT46's own shape
#print axioms QCore.MutexShareCrossEmission.ct46_is_a_fork_join_trace
#print axioms QCore.MutexShareCrossEmission.ct46_renders
#print axioms QCore.MutexShareCrossEmission.ct46_child_function_holds_the_childs_section
#print axioms QCore.MutexShareCrossEmission.ct46_creator_body_has_no_child_statement
#print axioms QCore.MutexShareCrossEmission.ct46_run_is_the_reference_observation
#print axioms QCore.MutexShareCrossEmission.ct46_observation_is_pinned
#print axioms QCore.MutexShareCrossEmission.a_nested_spawn_is_refused
#print axioms QCore.MutexShareCrossEmission.an_interleaved_creator_event_is_refused
#print axioms QCore.MutexShareCrossEmission.a_child_error_exit_is_refused
#print axioms QCore.MutexShareCrossEmission.a_guard_local_join_is_refused
#print axioms QCore.MutexShareCrossEmission.a_stray_join_is_refused
#print axioms QCore.MutexShareCrossEmission.a_segment_running_off_the_end_is_refused
#print axioms QCore.MutexShareCrossEmission.a_refused_trace_renders_nothing
