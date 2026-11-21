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

import QCore.LogicalAndOperand

/-! Audited companion to CG10-READER-GAPS (gateway rule 71). -/

#print axioms QCore.LogicalAndOperand.decision_vectors_hold
#print axioms QCore.LogicalAndOperand.a_unary_address_is_read
#print axioms QCore.LogicalAndOperand.a_logical_and_is_one
#print axioms QCore.LogicalAndOperand.only_a_logical_and_is_dropped
