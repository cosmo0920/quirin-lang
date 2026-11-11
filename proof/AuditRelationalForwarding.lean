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

import QCore.RelationalForwarding

/-! Audited companion to CALLEE-RETENTION-CLASS1 stage (a): relational forwarding and the per-class row (gateway rule 12s). -/

#print axioms QCore.RelationalForwarding.every_path_is_kept_by_one_class
#print axioms QCore.RelationalForwarding.no_path_is_kept_by_two_classes
#print axioms QCore.RelationalForwarding.representative_sat
#print axioms QCore.RelationalForwarding.meets_self
#print axioms QCore.RelationalForwarding.a_forwarded_call_applies_the_callee_class
#print axioms QCore.RelationalForwarding.disjoint_others_miss
#print axioms QCore.RelationalForwarding.the_published_class_holds_at_every_exit
#print axioms QCore.RelationalForwarding.a_caller_edge_selects_the_exit_disposition
#print axioms QCore.RelationalForwarding.the_row_covers_every_path
#print axioms QCore.RelationalForwarding.a_conversion_can_move_a_value_between_classes
