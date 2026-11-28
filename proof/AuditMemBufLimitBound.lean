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
import QCore.MemBufLimitBound
#print axioms QCore.MemBufLimitBound.overlimit_false_iff
#print axioms QCore.MemBufLimitBound.overlimit_true_iff
#print axioms QCore.MemBufLimitBound.inv_init
#print axioms QCore.MemBufLimitBound.release_preserves
#print axioms QCore.MemBufLimitBound.refresh_preserves
#print axioms QCore.MemBufLimitBound.setLimits_preserves
#print axioms QCore.MemBufLimitBound.stored_mem_preserves
#print axioms QCore.MemBufLimitBound.stored_memrb_bound
#print axioms QCore.MemBufLimitBound.append_mem_preserves
#print axioms QCore.MemBufLimitBound.append_memrb_preserves
#print axioms QCore.MemBufLimitBound.step_preserves
#print axioms QCore.MemBufLimitBound.run_preserves
#print axioms QCore.MemBufLimitBound.mem_buf_limit_plus_one_write
#print axioms QCore.MemBufLimitBound.running_mem_below_limit
#print axioms QCore.MemBufLimitBound.gateC_is_admitted
#print axioms QCore.MemBufLimitBound.gate_vectors
#print axioms QCore.MemBufLimitBound.protect_matches
#print axioms QCore.MemBufLimitBound.protect_vectors
#print axioms QCore.MemBufLimitBound.setLimits_resumes
#print axioms QCore.MemBufLimitBound.resume_vectors
#print axioms QCore.MemBufLimitBound.bound_reached
#print axioms QCore.MemBufLimitBound.paused_refuses
#print axioms QCore.MemBufLimitBound.rate_gate_breaks_bound
#print axioms QCore.MemBufLimitBound.shutdown_flush_breaks_bound
#print axioms QCore.MemBufLimitBound.shutdown_flush_append_grows
#print axioms QCore.MemBufLimitBound.memrb_expanding_filter_breaks_bound
#print axioms QCore.MemBufLimitBound.fs_not_paused_by_mem_buf_limit
#print axioms QCore.MemBufLimitBound.zero_limit_unbounded
