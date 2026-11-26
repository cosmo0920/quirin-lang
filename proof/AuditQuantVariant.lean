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


import QCore.QuantVariant

#print axioms QCore.QuantVariant.sealed_alias_release_refuses
#print axioms QCore.QuantVariant.handoff_invalidates_source
#print axioms QCore.QuantVariant.wrong_generation_refuses
#print axioms QCore.QuantVariant.wrong_generation_handoff_refuses
#print axioms QCore.QuantVariant.open_consumes_once
#print axioms QCore.QuantVariant.open_preserves_current_frame
#print axioms QCore.QuantVariant.selected_arm_survives_handoff
#print axioms QCore.QuantVariant.delayed_match
