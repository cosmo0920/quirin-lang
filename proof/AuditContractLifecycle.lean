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

import QCore.ContractLifecycle
#print axioms QCore.ContractLifecycle.erasure_preserves_semantics
#print axioms QCore.ContractLifecycle.creation_derives_no_fact
#print axioms QCore.ContractLifecycle.retirement_derives_no_fact
#print axioms QCore.ContractLifecycle.creation_does_not_support_open
#print axioms QCore.ContractLifecycle.retirement_is_not_refutation
#print axioms QCore.ContractLifecycle.missing_hypothesis_refuses
#print axioms QCore.ContractLifecycle.resource_target_is_symbolic
#print axioms QCore.ContractLifecycle.duplicate_entry_refuses
#print axioms QCore.ContractLifecycle.oversized_declaration_refuses
