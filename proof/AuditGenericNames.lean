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

import QCore.DescriptorCustody
import QCore.MonkeyEvent
import QCore.ExternalIntegration
import QCore.ChunkSource

-- The legacy namespace resolves the same types, constructors and checked functions.
example (pair : QCore.MonkeyEvent.SocketPair) :
    QCore.MonkeyEvent.checkSocketPair pair =
      QCore.DescriptorCustody.checkSocketPair pair := rfl

example (pair : QCore.MonkeyEvent.SocketPair) :
    pair.Valid = QCore.DescriptorCustody.SocketPair.Valid pair := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    composition.Valid = QCore.ExternalIntegration.EventComposition.Valid composition := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    composition.buildProfileAccepted =
      QCore.ExternalIntegration.EventComposition.buildProfileAccepted composition := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    QCore.ExternalIntegration.checkMonkeyComposition composition =
      QCore.ExternalIntegration.checkEventComposition composition := rfl

example (certificate : QCore.ExternalIntegration.MonkeyCompositionCertificate) :
    certificate.checked =
      QCore.ExternalIntegration.EventCompositionCertificate.checked certificate := rfl

#print axioms QCore.DescriptorCustody.accepted_descriptor_teardown_is_complete_and_unique
#print axioms QCore.MonkeyEvent.accepted_monkey_descriptor_teardown_is_complete_and_unique
#print axioms QCore.ChunkSource.busy_effect_preserves_lock
#print axioms QCore.ChunkSource.busy_effect_preserves_chunkio_lock
#print axioms QCore.ExternalIntegration.accepted_monkey_composition_has_event_teardown
