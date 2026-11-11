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

import QCore.PluginReadingEnvironment
#print axioms QCore.PluginReadingEnvironment.valid_binds_exact_canonical_bytes
#print axioms QCore.PluginReadingEnvironment.stale_digest_refuses
#print axioms QCore.PluginReadingEnvironment.noncanonical_bytes_refuse
#print axioms QCore.PluginReadingEnvironment.publication_requires_retained_reference
#print axioms QCore.PluginReadingEnvironment.no_snapshot_survives_empty_rollback
#print axioms QCore.PluginReadingEnvironment.original_payload_is_unchanged
#print axioms QCore.PluginReadingEnvironment.no_existing_semantic_consumer_changes
#print axioms QCore.PluginReadingEnvironment.exact_reconstruction_has_held_digest
