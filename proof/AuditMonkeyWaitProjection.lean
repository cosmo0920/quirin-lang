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

import QCore.MonkeyWaitProjection

#print axioms QCore.MonkeyWaitProjection.accepted_projection_has_exact_source
#print axioms QCore.MonkeyWaitProjection.accepted_projection_has_exact_observation_obligations
#print axioms QCore.MonkeyWaitProjection.accepted_callback_is_the_sole_return_path
#print axioms QCore.MonkeyWaitProjection.exact_source_is_accepted
#print axioms QCore.MonkeyWaitProjection.accepted_projection_has_exact_contracts
#print axioms QCore.MonkeyWaitProjection.missing_lifecycle_source_is_rejected
#print axioms QCore.MonkeyWaitProjection.callback_effect_substitution_is_rejected
#print axioms QCore.MonkeyWaitProjection.missing_event_free_is_rejected
#print axioms QCore.MonkeyWaitProjection.missing_callback_contract_is_rejected
#print axioms QCore.MonkeyWaitProjection.substituted_source_identity_is_rejected
