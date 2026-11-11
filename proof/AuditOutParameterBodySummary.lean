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

import QCore.OutParameterBodySummary

/-! Audited companion to OUT-PARAMETER-BODY-SUMMARY (gateway rules 10k(5), 11r(6)). -/

#print axioms QCore.OutParameterBodySummary.class_mode_sound
#print axioms QCore.OutParameterBodySummary.unwritten_class_keeps
#print axioms QCore.OutParameterBodySummary.written_exit_refuses_unwritten
#print axioms QCore.OutParameterBodySummary.other_exit_refuses
#print axioms QCore.OutParameterBodySummary.exit_in_its_class
#print axioms QCore.OutParameterBodySummary.summary_effect_equals_body_on_every_path
#print axioms QCore.OutParameterBodySummary.relation_requires_a_writer
#print axioms QCore.OutParameterBodySummary.scalar_store_preserves_custody
#print axioms QCore.OutParameterBodySummary.released_base_refuses
