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

import QCore.StdEvent

-- Card `STD-EVENT`: `source.event.v1`, the checked event surface over the
-- existing `event_reactor`/`os_endpoint` models. What is proved here is this
-- card's own two claims — the determinism boundary and the reactor projection.
-- The custody model's rules are `QCore.DescriptorCustody`'s and are not
-- reconstructed.

-- The quiet default and explicit six-level threshold used by event examples.
#print axioms QCore.StdEvent.diagnosticEnabled
#print axioms QCore.StdEvent.an_absent_log_level_is_quiet
#print axioms QCore.StdEvent.trace_log_level_admits_every_level

-- The determinism boundary. A ready set with two members admits **no**
-- declaration: not either registration's, and not the idle one. That is the
-- by-name refusal the card rests on, and the reason it does not need
-- `OBS-REORDERING` to stand.
#print axioms QCore.StdEvent.readySet
#print axioms QCore.StdEvent.admits
#print axioms QCore.StdEvent.two_ready_registrations_admit_no_turn
#print axioms QCore.StdEvent.two_ready_registrations_admit_no_idle_turn
#print axioms QCore.StdEvent.an_idle_turn_is_admitted_exactly_when_nothing_is_ready
#print axioms QCore.StdEvent.a_ready_turn_names_the_one_ready_registration
#print axioms QCore.StdEvent.an_empty_ready_set_admits_no_ready_turn

-- Readiness is induced by the program alone. These are what make the platform
-- premise EVENT-V1-LEVEL-TRIGGERED-READINESS the only thing about the outside
-- world a v1 verdict rests on: nothing but a `wake` can put a registration in
-- the ready set, and a ledger with no wake admits no ready turn.
#print axioms QCore.StdEvent.a_ready_registration_is_a_read_on_a_pending_endpoint
#print axioms QCore.StdEvent.a_program_with_no_wake_admits_no_ready_turn
#print axioms QCore.StdEvent.readiness_comes_only_from_a_wake
#print axioms QCore.StdEvent.a_ready_turn_drains_exactly_one_byte

-- The check-only arm, from the model's side: a write registration is never in
-- the ready set, because this route induces no send-buffer pressure and
-- therefore has nothing to say about one.
#print axioms QCore.StdEvent.a_write_registration_is_never_ready
#print axioms QCore.StdEvent.a_write_interest_is_not_executable

-- The projection onto the reactor model, i.e. the surface table as code. A
-- `wake` projects to nothing, and the strong form: erasing every wake leaves
-- the projection byte for byte, so `wake` is outside the custody model rather
-- than a hidden part of it.
#print axioms QCore.StdEvent.project
#print axioms QCore.StdEvent.a_wake_projects_to_nothing
#print axioms QCore.StdEvent.only_a_wake_projects_to_nothing
#print axioms QCore.StdEvent.projectProgram_cons
#print axioms QCore.StdEvent.erasing_the_wakes_leaves_the_projection_unchanged
#print axioms QCore.StdEvent.a_watch_projects_to_the_reactor_registration
#print axioms QCore.StdEvent.every_registration_has_priority_zero
#print axioms QCore.StdEvent.a_ready_turn_brackets_exactly_one_callback
#print axioms QCore.StdEvent.an_idle_turn_runs_no_callback

-- The derived effect row. `alloc` and `mut` are unreachable not because a
-- checker refuses them but because the grammar has no value vocabulary at all.
#print axioms QCore.StdEvent.derivedEffects
#print axioms QCore.StdEvent.no_program_carries_alloc_or_mut
#print axioms QCore.StdEvent.a_program_of_returns_carries_nothing
#print axioms QCore.StdEvent.opening_a_context_carries_io
#print axioms QCore.StdEvent.log_level_carries_io_and_projects_no_reactor_operation

-- The canonical program the Rust route compiles, runs and byte-compares, stated
-- here as an executable check so that the fixture and this module cannot drift
-- apart silently — together with the three concrete refusals that bound it.
#print axioms QCore.StdEvent.the_canonical_program_is_admitted
#print axioms QCore.StdEvent.the_canonical_program_without_its_wake_is_refused
#print axioms QCore.StdEvent.two_induced_readinesses_refuse_every_declaration
#print axioms QCore.StdEvent.two_induced_readinesses_refuse_the_other_declaration
#print axioms QCore.StdEvent.two_induced_readinesses_refuse_the_idle_declaration
#print axioms QCore.StdEvent.draining_one_readiness_admits_the_next_turn
#print axioms QCore.StdEvent.a_write_registration_is_checked_but_not_executable
