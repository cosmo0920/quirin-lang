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

import QCore.EventCompositionContext

#print axioms QCore.EventCompositionContext.accepted_context_is_complete
#print axioms QCore.EventCompositionContext.accepted_context_equals_caller_expected
#print axioms QCore.EventCompositionContext.accepted_context_rederives_payload
#print axioms QCore.EventCompositionContext.unequal_context_rejects
#print axioms QCore.EventCompositionContext.scope_substitution_rejects
#print axioms QCore.EventCompositionContext.observation_payload_substitution_rejects
#print axioms QCore.EventCompositionContext.binding_payload_substitution_rejects
#print axioms QCore.EventCompositionContext.context_validation_remains_unknown
