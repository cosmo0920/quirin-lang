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

import QCore.MetricTraceContractGaps

/-! Audited custody-vector parity for W46-METRIC-TRACE-CONTRACT-GAPS.
The body/ABI readings at the two source pins are external premises. -/

#print axioms QCore.MetricTraceContractGaps.selected_count
#print axioms QCore.MetricTraceContractGaps.selected_contracts_have_no_custody_transfer
#print axioms QCore.MetricTraceContractGaps.borrowed_does_not_discharge_owned
#print axioms QCore.MetricTraceContractGaps.releasing_a_selected_formal_contradicts
#print axioms QCore.MetricTraceContractGaps.missing_body_evidence_keeps_premise
#print axioms QCore.MetricTraceContractGaps.aggregate_read_count_and_arity
#print axioms QCore.MetricTraceContractGaps.aggregate_reads_retain_custody
