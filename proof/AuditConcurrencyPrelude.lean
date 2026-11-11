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

import QCore.ConcurrencyPrelude

#print axioms QCore.ModuleImports.empty_imports_grant_no_concurrency
#print axioms QCore.ModuleImports.unknown_module_rejects
#print axioms QCore.ModuleImports.admitted_concurrency_requires_import
#print axioms QCore.ModuleImports.duplicate_import_is_idempotent
#print axioms QCore.ModuleImports.missing_required_capability_rejects
#print axioms QCore.ModuleImports.sequential_code_needs_no_concurrency

-- Importing the feature preserves the original theorem statements and premises.
#print axioms QCore.ConcurrencyPrelude.rollback_restores_original_custody
#print axioms QCore.ConcurrencyPrelude.transfer_preserves_live_identity_domain
#print axioms QCore.ConcurrencyPrelude.unjoined_capsule_is_unavailable_to_every_task
#print axioms QCore.ConcurrencyPrelude.checked_join_returns_capsule_resources_to_creator
#print axioms QCore.ConcurrencyPrelude.joined_task_cannot_execute_resource_use
#print axioms QCore.ConcurrencyPrelude.failed_create_changes_no_handles
#print axioms QCore.ConcurrencyPrelude.checked_join_consumes_handle_once
#print axioms QCore.ConcurrencyPrelude.checked_handler_exit_is_quiescent_and_preserves_outcome
#print axioms QCore.ConcurrencyPrelude.the_outstanding_total_never_exceeds_full
#print axioms QCore.ConcurrencyPrelude.reconstruction_waits_for_the_last_share
#print axioms QCore.ConcurrencyPrelude.parked_authority_is_conserved
#print axioms QCore.ConcurrencyPrelude.guard_authorizes_only_declared_footprint
#print axioms QCore.ConcurrencyPrelude.unlock_requires_closed_invariant
#print axioms QCore.ConcurrencyPrelude.forged_witness_is_refused
