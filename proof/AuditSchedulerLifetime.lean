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

import QCore.SchedulerLifetime
#print axioms QCore.SchedulerLifetime.release_protected_refuses
#print axioms QCore.SchedulerLifetime.write_unowned_refuses
#print axioms QCore.SchedulerLifetime.absent_lifetime_refuses
#print axioms QCore.SchedulerLifetime.released_resource_is_absent
#print axioms QCore.SchedulerLifetime.retirement_preserves_borrow
#print axioms QCore.SchedulerLifetime.retired_owner_cannot_remain
#print axioms QCore.SchedulerLifetime.invalid_membership_forbids_admission
#print axioms QCore.SchedulerLifetime.retiring_current_tls_forbids_admission
#print axioms QCore.SchedulerLifetime.tls_absent_destroy_has_no_params_free_and_clear
#print axioms QCore.SchedulerLifetime.callback_is_explicit_before_retirement

#print axioms QCore.SchedulerLifetime.wrong_worker_refuses
#print axioms QCore.SchedulerLifetime.empty_cleanup_has_no_effects

#print axioms QCore.SchedulerLifetime.non_parent_current_tls_forbids_admission
