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

import QCore.PremiseMetadataBridge
#print axioms QCore.PremiseMetadataBridge.exact_original_bytes
#print axioms QCore.PremiseMetadataBridge.wrong_introduction_entry
#print axioms QCore.PremiseMetadataBridge.missing_atom_refuses
#print axioms QCore.PremiseMetadataBridge.changed_child_order_refuses
#print axioms QCore.PremiseMetadataBridge.checked_roundtrip
#print axioms QCore.PremiseMetadataBridge.stale_source_refuses
#print axioms QCore.PremiseMetadataBridge.graph_unchanged
#print axioms QCore.PremiseMetadataBridge.interpreted_bindings_unchanged
#print axioms QCore.PremiseMetadataBridge.no_disposition_transport
