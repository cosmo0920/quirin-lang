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

import QCore.ComposedTasks

-- Card `COMPOSE-CONCURRENCY`: `source.std-io-tasks.v1`, the checked composition
-- of `source.std-io.v1` with the checked task route. The join is what is proved
-- here; the task model's custody rules are not reconstructed.

-- A line moves no custody: the ledger is a function of the task steps alone, and
-- erasing every line leaves exactly the task route's own ledger.
#print axioms QCore.ComposedTasks.ledger_of_step
#print axioms QCore.ComposedTasks.a_line_leaves_the_ledger_unchanged
#print axioms QCore.ComposedTasks.the_ledger_is_a_function_of_the_task_steps_alone
#print axioms QCore.ComposedTasks.ledgerRun_ignores_lines
#print axioms QCore.ComposedTasks.the_composed_ledger_is_the_task_projections_ledger
#print axioms QCore.ComposedTasks.a_program_of_lines_leaves_the_ledger_unchanged

-- A line moves no synthesized trace byte. This is why every pinned trace byte —
-- the F02 emission digests, the CT fixtures, the published transcripts — is
-- untouched by this card.
#print axioms QCore.ComposedTasks.trace_of_step
#print axioms QCore.ComposedTasks.the_trace_is_a_function_of_the_task_steps_alone
#print axioms QCore.ComposedTasks.traceRun_ignores_lines
#print axioms QCore.ComposedTasks.erasing_the_source_lines_leaves_the_task_trace

-- The interleaving argument the v1 boundary rests on: a worker's buffered trace
-- enters the observation only at that worker's join, so the parent's own
-- emissions are the same list whatever the workers wrote.
#print axioms QCore.ComposedTasks.agree_step
#print axioms QCore.ComposedTasks.agree_exec
#print axioms QCore.ComposedTasks.the_parents_own_emissions_do_not_depend_on_the_workers
#print axioms QCore.ComposedTasks.the_logger_stream_does_not_depend_on_the_workers
#print axioms QCore.ComposedTasks.the_ledger_does_not_depend_on_the_workers
#print axioms QCore.ComposedTasks.only_a_join_replays_a_workers_trace

-- Placement: the dual of the above. Nothing of the kind holds for a line inside
-- a worker, and the placement predicate refuses one.
#print axioms QCore.ComposedTasks.a_line_in_a_worker_has_no_admitted_placement
#print axioms QCore.ComposedTasks.a_line_in_the_parent_is_admitted
#print axioms QCore.ComposedTasks.the_placements_are_exactly_these

-- The line vocabulary is `source.std-io.v1`'s own: the stream a statement
-- writes and the lines it publishes are read off that route's own function.
-- Nothing here relates the two streams' orders.
#print axioms QCore.ComposedTasks.the_streams_agree_with_the_std_io_route
#print axioms QCore.ComposedTasks.print_never_writes_stderr
#print axioms QCore.ComposedTasks.log_never_writes_stdout
#print axioms QCore.ComposedTasks.a_barrier_publishes_no_line
#print axioms QCore.ComposedTasks.a_resolved_print_publishes_exactly_one_line
#print axioms QCore.ComposedTasks.an_unresolved_operand_publishes_nothing

-- The operand surface, and what a rendering may read. Neither constructor can
-- denote a resource, a read loan or a task handle.
#print axioms QCore.ComposedTasks.an_operand_is_a_literal_or_a_scalar_name
#print axioms QCore.ComposedTasks.a_rendering_reads_only_a_bound_name

-- The effect row, derived by `source.std-io.v1`'s own function over the
-- projected events: any line at all carries `Io`, and never `Alloc` or `Mut`.
#print axioms QCore.ComposedTasks.a_program_of_no_lines_carries_nothing
#print axioms QCore.ComposedTasks.writing_requires_io
#print axioms QCore.ComposedTasks.a_barrier_still_carries_io
#print axioms QCore.ComposedTasks.no_line_carries_alloc_or_mut

-- One `private` helper lemma is not listed: `filter_replicate`, following the
-- convention `QCore.ComposedStdIo` established (report §7.5 of card
-- `STD-IO-COMPOSE`). It is covered twice over — by `AuditAll.lean`'s
-- whole-project axiom closure, and transitively by every public theorem above
-- that depends on it, which prints its axioms through it.
