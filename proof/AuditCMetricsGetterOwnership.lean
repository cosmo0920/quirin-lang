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

import QCore.CMetricsGetterOwnership

#print axioms QCore.CMetricsGetterOwnership.witness_bytes_are_nul_free
#print axioms QCore.CMetricsGetterOwnership.pinned_secret_bitflip
#print axioms QCore.CMetricsGetterOwnership.exact_short_hash_is_zero
#print axioms QCore.CMetricsGetterOwnership.zero_hash_selects_embedded
#print axioms QCore.CMetricsGetterOwnership.primary_secondary_state_is_preserved_by_embedded_prepare
#print axioms QCore.CMetricsGetterOwnership.destroy_invalidates_borrow
#print axioms QCore.CMetricsGetterOwnership.labeled_zero_path_reaches_destruction_leak
#print axioms QCore.CMetricsGetterOwnership.parsed_contract_has_no_call_permission
