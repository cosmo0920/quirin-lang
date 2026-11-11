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

import QCore.WholeProgram

-- Print the foundational dependencies of every Cycle 11 integration theorem.
#print axioms QCore.WholeProgram.checkPasses_implies_interfaces
#print axioms QCore.WholeProgram.checkPasses_preserves_input
#print axioms QCore.WholeProgram.accepted_passes_have_checked_cfg
#print axioms QCore.WholeProgram.accepted_passes_preserve_cfg_identities
#print axioms QCore.WholeProgram.checkProgram_implies_accepts
#print axioms QCore.WholeProgram.checkProgram_preserves_source
#print axioms QCore.WholeProgram.accepted_function_interfaces
#print axioms QCore.WholeProgram.recursive_function_has_declared_summary
#print axioms QCore.WholeProgram.accepted_program_function_has_checked_cfg
#print axioms QCore.WholeProgram.accepted_program_preserves_function_cfg_identities
#print axioms QCore.WholeProgram.consume_does_not_mint
#print axioms QCore.WholeProgram.consume_preserves_unique_identities
#print axioms QCore.WholeProgram.recursive_summary_round_uses_contract

#print axioms QCore.WholeProgram.accepted_function_has_no_unchecked_source_contract
#print axioms QCore.WholeProgram.unchecked_source_contract_rejects
#print axioms QCore.Frontend.BodyScope.binder_resolves_in_body
#print axioms QCore.Frontend.BodyScope.leaving_body_restores_outer
#print axioms QCore.Frontend.BodyScope.binder_cannot_escape
