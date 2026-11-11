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

import QCore.ComposeEventLines

-- Card `COMPOSE-EVENT-LINES`: `source.event.v1` composed with the source line
-- vocabulary. What is proved here is the composition's own question — whether
-- either half of card `STD-EVENT`'s answer moves — and the two things the
-- composition adds: a payload queue and a stream comparison.

-- A line decides nothing. Erasing every line leaves the reactor projection byte
-- for byte and the determinism ledger's run unchanged, so `QCore.StdEvent`'s
-- theorems apply verbatim to a composed program and this card proves no custody
-- or determinism claim of its own.
#print axioms QCore.ComposeEventLines.erase
#print axioms QCore.ComposeEventLines.project
#print axioms QCore.ComposeEventLines.a_line_erases_to_nothing
#print axioms QCore.ComposeEventLines.eraseProgram_cons
#print axioms QCore.ComposeEventLines.erasing_the_lines_leaves_the_reactor_projection
#print axioms QCore.ComposeEventLines.a_line_is_always_admitted
#print axioms QCore.ComposeEventLines.a_line_moves_no_ledger_state
#print axioms QCore.ComposeEventLines.erasing_the_lines_leaves_the_run_unchanged

-- The determinism boundary, inherited rather than restated: the payload
-- refinement decides nothing the coarse ledger did not, two ready registrations
-- still admit no declaration, and a binding buys no turn.
#print axioms QCore.ComposeEventLines.readySet
#print axioms QCore.ComposeEventLines.ready_set_is_the_coarse_one
#print axioms QCore.ComposeEventLines.two_ready_registrations_admit_no_turn
#print axioms QCore.ComposeEventLines.two_ready_registrations_admit_no_idle_turn
#print axioms QCore.ComposeEventLines.a_binding_admits_exactly_what_the_bare_turn_admits

-- The payload is the byte its own wake delivered: a queue rather than a count,
-- which is what the named premise EVENT-V1-STREAM-BYTE-ORDER is about and what
-- the emitted program checks at run time rather than trusting.
#print axioms QCore.ComposeEventLines.step
#print axioms QCore.ComposeEventLines.a_wake_queues_its_byte
#print axioms QCore.ComposeEventLines.the_wake_payload_example_binds_seventy_two_then_one_hundred_and_five
#print axioms QCore.ComposeEventLines.exchanging_the_two_wakes_exchanges_the_two_bound_bytes
#print axioms QCore.ComposeEventLines.an_unbound_turn_drains_the_same_byte
#print axioms QCore.ComposeEventLines.a_binding_without_a_wake_is_refused

-- The observation. Without `log_level` the retained trace is not stream bytes;
-- a source line is program output on its own stream and on no other.
#print axioms QCore.ComposeEventLines.records
#print axioms QCore.ComposeEventLines.a_quiet_program_writes_no_trace_record
#print axioms QCore.ComposeEventLines.a_line_writes_its_own_stream
#print axioms QCore.ComposeEventLines.a_line_writes_no_other_stream

-- And why the comparison is strict rather than reordering-aware: one emitter,
-- so every record is a barrier of `QCore.ObsReordering` and the declared
-- relation admits exactly the stream it declares. The strict comparison is that
-- relation's degenerate case, not a second rule beside it.
#print axioms QCore.ComposeEventLines.observation
#print axioms QCore.ComposeEventLines.every_window_is_a_barrier
#print axioms QCore.ComposeEventLines.every_record_is_the_entry_threads
#print axioms QCore.ComposeEventLines.the_event_relation_is_strict
#print axioms QCore.ComposeEventLines.a_native_event_stream_is_admitted_exactly_when_it_is_byte_identical
