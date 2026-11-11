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

import QCore.ComposedStdIo

-- Card `STD-IO-COMPOSE`: `source.std-io-extern.v1`, the checked composition of
-- `source.extern-call.v1` and `source.std-io.v1`. The join is what is proved
-- here; neither projection is reconstructed.

-- Custody is untouched by rendering through the composition, and the composed
-- run produces exactly the extern projection's ledger.
#print axioms QCore.ComposedStdIo.a_rendering_leaves_the_ledger_unchanged
#print axioms QCore.ComposedStdIo.the_shared_return_leaves_the_ledger_unchanged
#print axioms QCore.ComposedStdIo.the_order_runs_only_its_extern_steps
#print axioms QCore.ComposedStdIo.the_composed_ledger_is_the_extern_projections_ledger

-- A rendering reads a value, never an identity: the operand type has no
-- constructor denoting one, and no step moves a name between the two
-- namespaces.
#print axioms QCore.ComposedStdIo.an_operand_is_a_literal_a_value_name_or_a_borrow
#print axioms QCore.ComposedStdIo.no_step_moves_a_name_between_the_namespaces

-- Nothing is lost or duplicated by the interleaving.
#print axioms QCore.ComposedStdIo.the_projections_are_recovered_from_the_order

-- Determinacy is required to render, and the determined values are exactly the
-- ones some extern event computes a number for.
#print axioms QCore.ComposedStdIo.the_determined_values_are_exactly_these
#print axioms QCore.ComposedStdIo.a_call_without_a_declared_outcome_determines_nothing
#print axioms QCore.ComposedStdIo.a_literal_binding_determines_its_literal
#print axioms QCore.ComposedStdIo.a_determined_rendering_publishes_exactly_one_line
#print axioms QCore.ComposedStdIo.an_undetermined_operand_has_no_admitted_step
#print axioms QCore.ComposedStdIo.an_unresolved_name_has_no_admitted_step
#print axioms QCore.ComposedStdIo.a_refused_step_refuses_the_program

-- Ordering: a rendering reads only a value an earlier composed step bound, and
-- the reverse shape has no admitted step.
#print axioms QCore.ComposedStdIo.a_rendering_reads_only_an_earlier_binding
#print axioms QCore.ComposedStdIo.a_rendering_before_its_binding_has_no_admitted_step

-- The two streams stay separate; the extern trace joins standard output only.
-- Nothing here relates the two streams' orders.
#print axioms QCore.ComposedStdIo.print_never_writes_stderr
#print axioms QCore.ComposedStdIo.log_never_writes_stdout
#print axioms QCore.ComposedStdIo.the_extern_trace_shares_standard_output
#print axioms QCore.ComposedStdIo.a_barrier_publishes_no_line

-- Card `EXTERN-SCALAR-OUTCOME` (trace half): what an extern call's own trace
-- line names. A declared outcome is named; an undeclared one names nothing; and
-- the number a trace line names is the number that call determines, so a trace
-- line and a rendering of the same value cannot disagree.
#print axioms QCore.ComposedStdIo.a_declared_outcome_is_named_on_the_calls_trace_line
#print axioms QCore.ComposedStdIo.an_undeclared_outcome_names_no_number
#print axioms QCore.ComposedStdIo.the_undeclared_shapes_name_no_number
#print axioms QCore.ComposedStdIo.the_trace_line_names_the_number_the_call_determines

-- No assumption is dropped at the join: the composed row is the union, and
-- admitting the union admits each projection.
#print axioms QCore.ComposedStdIo.mem_row
#print axioms QCore.ComposedStdIo.the_composed_effect_row_is_the_union
#print axioms QCore.ComposedStdIo.no_projection_effect_is_dropped
#print axioms QCore.ComposedStdIo.admitting_the_union_admits_each_projection

-- Composing adds no premise.
#print axioms QCore.ComposedStdIo.a_rendering_carries_no_premise
#print axioms QCore.ComposedStdIo.composing_adds_no_premise

-- Non-vacuity: the card's payoff fixture, and the three shapes it refuses.
#print axioms QCore.ComposedStdIo.the_payoff_fixture_is_admitted
#print axioms QCore.ComposedStdIo.the_payoff_fixture_carries_only_its_extern_premises
#print axioms QCore.ComposedStdIo.a_rendering_before_its_binding_is_refused
#print axioms QCore.ComposedStdIo.an_undetermined_outcome_is_refused
#print axioms QCore.ComposedStdIo.the_undeclared_trace_line_is_reachable
#print axioms QCore.ComposedStdIo.an_acquiring_composition_needs_its_alloc_declared

-- Card `STD-FMT-BORROW`: the borrowed render of a live resource. Liveness is a
-- precondition, obligations are unchanged, and the two namespaces stay two.
#print axioms QCore.ComposedStdIo.a_borrowed_render_reads_only_a_live_identity
#print axioms QCore.ComposedStdIo.a_borrowed_render_of_a_dead_identity_has_no_admitted_step
#print axioms QCore.ComposedStdIo.an_unresolved_borrow_has_no_admitted_step
#print axioms QCore.ComposedStdIo.a_borrowed_render_publishes_exactly_one_line
#print axioms QCore.ComposedStdIo.a_borrowed_render_leaves_the_ledger_unchanged
#print axioms QCore.ComposedStdIo.a_program_of_renderings_leaves_the_ledger_unchanged
#print axioms QCore.ComposedStdIo.a_borrowed_render_reads_no_value
#print axioms QCore.ComposedStdIo.a_scalar_render_reads_no_identity
#print axioms QCore.ComposedStdIo.a_borrowed_render_binds_no_value
