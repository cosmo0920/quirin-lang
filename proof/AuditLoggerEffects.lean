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

import QCore.LoggerEffects
#print axioms QCore.LoggerEffects.step_preserves_paired_custody
#print axioms QCore.LoggerEffects.run_preserves_paired_custody
#print axioms QCore.LoggerEffects.annihilation_requires_text_cleanup
#print axioms QCore.LoggerEffects.creation_requires_fresh_resources
#print axioms QCore.LoggerEffects.transfer_preserves_both_resources
#print axioms QCore.LoggerEffects.queued_resource_cannot_be_freed_locally
#print axioms QCore.LoggerEffects.dead_operation_cannot_be_recreated
#print axioms QCore.LoggerEffects.immediate_callback_has_no_owned_allocation
#print axioms QCore.LoggerEffects.forwarding_then_disabled_can_destroy
#print axioms QCore.LoggerEffects.forwarded_failure_still_returns_one
#print axioms QCore.LoggerEffects.cyclic_closure_retains_all_reachable_effects
#print axioms QCore.LoggerEffects.mutually_recursive_calls_reach_both
