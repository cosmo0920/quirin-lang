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

import QCore.TypedSourceControl
#print axioms QCore.TypedSourceControl.legacy_admits_nothing
#print axioms QCore.TypedSourceControl.unlisted_form_refused
#print axioms QCore.TypedSourceControl.invalid_admission_refuses
#print axioms QCore.TypedSourceControl.back_edge_refused
#print axioms QCore.TypedSourceControl.cycles_are_back_edges
#print axioms QCore.TypedSourceControl.acyclic_forms
#print axioms QCore.TypedSourceControl.valid_row_acyclic
#print axioms QCore.TypedSourceControl.unlisted_statement_refused
#print axioms QCore.TypedSourceControl.while_is_a_loop
#print axioms QCore.TypedSourceControl.unscanned_for_is_a_loop
#print axioms QCore.TypedSourceControl.scan_for_is_legacy
#print axioms QCore.TypedSourceControl.nonzero_do_is_a_loop
#print axioms QCore.TypedSourceControl.switch_is_read
#print axioms QCore.TypedSourceControl.indirect_goto_refused
#print axioms QCore.TypedSourceControl.case_outside_switch_refused
#print axioms QCore.TypedSourceControl.loops_refused
#print axioms QCore.TypedSourceControl.jump_out_ends_locals
#print axioms QCore.TypedSourceControl.inner_scope_ends_first
#print axioms QCore.TypedSourceControl.lifetimesOk_append
#print axioms QCore.TypedSourceControl.endedAfter_keeps
#print axioms QCore.TypedSourceControl.endedAfter_ends
#print axioms QCore.TypedSourceControl.ended_use_refused
#print axioms QCore.TypedSourceControl.jump_then_use_refused
#print axioms QCore.TypedSourceControl.fallthrough_composes
#print axioms QCore.TypedSourceControl.break_stops
#print axioms QCore.TypedSourceControl.case_entry_lowered
#print axioms QCore.TypedSourceControl.no_default_path
#print axioms QCore.TypedSourceControl.switch_vectors_hold
#print axioms QCore.TypedSourceControl.exhausted_unknown
#print axioms QCore.TypedSourceControl.exhausted_never_safe
#print axioms QCore.TypedSourceControl.one_path_too_many
