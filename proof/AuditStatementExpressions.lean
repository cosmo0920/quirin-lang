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

import QCore.StatementExpressions

/-! Audited companion to CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (gateway rule 11g). -/
#print axioms QCore.StatementExpressions.select_events
#print axioms QCore.StatementExpressions.control_no_event
#print axioms QCore.StatementExpressions.unselected_no_event
#print axioms QCore.StatementExpressions.value_is_final
#print axioms QCore.StatementExpressions.lifetimes_end_at_block
#print axioms QCore.StatementExpressions.events_exec
#print axioms QCore.StatementExpressions.lowering_exact
#print axioms QCore.StatementExpressions.discarded_exact
#print axioms QCore.StatementExpressions.returned_exact
#print axioms QCore.StatementExpressions.flat_names_exact
#print axioms QCore.StatementExpressions.shadow_breaks_flat
#print axioms QCore.StatementExpressions.admitted_no_escape
#print axioms QCore.StatementExpressions.escape_refused
#print axioms QCore.StatementExpressions.shadow_refused
#print axioms QCore.StatementExpressions.static_refused
#print axioms QCore.StatementExpressions.no_value_refused
#print axioms QCore.StatementExpressions.discarded_needs_no_value
#print axioms QCore.StatementExpressions.admission_vectors_hold
#print axioms QCore.StatementExpressions.unlisted_position_refused
#print axioms QCore.StatementExpressions.admitted_positions
#print axioms QCore.StatementExpressions.operand_positions_refused
#print axioms QCore.StatementExpressions.lifetime_touches_own
#print axioms QCore.StatementExpressions.lifetime_vectors_hold
