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

import QCore.Concurrency
import QCore.ModuleImports
import QCore.ScopedTasks
import QCore.TaskExecution
import QCore.OwnedTaskExecution
import QCore.ReadLoan
import QCore.ReadShare
import QCore.ErrorPayload
import QCore.MutexInvariant
import QCore.TaskTraceRelation
import QCore.RegionTaskCredit
import QCore.LifetimeCorrespondence

/-! Proof entry point for `qcore_core::prelude::concurrency`.

This module imports the existing bounded models and exports selected proved
properties without modifying their definitions or premises. The underlying
namespaces remain available for constructing each model's context. In particular,
this is not a combined task/mutex transition system and it does not prove general
source lowering, arbitrary native execution, or a Rust-to-Lean refinement.
-/
namespace QCore.ConcurrencyPrelude

export QCore.ScopedTasks (
  rollback_restores_original_custody
  transfer_preserves_live_identity_domain
  unjoined_capsule_is_unavailable_to_every_task
  checked_join_returns_capsule_resources_to_creator
  joined_task_cannot_execute_resource_use)

export QCore.TaskExecution (
  failed_create_changes_no_handles
  checked_join_consumes_handle_once
  checked_handler_exit_is_quiescent_and_preserves_outcome)

export QCore.ReadShare (
  the_outstanding_total_never_exceeds_full
  reconstruction_waits_for_the_last_share)

export QCore.Concurrency.MutexInvariant (
  parked_authority_is_conserved
  guard_authorizes_only_declared_footprint
  unlock_requires_closed_invariant
  forged_witness_is_refused)

end QCore.ConcurrencyPrelude
