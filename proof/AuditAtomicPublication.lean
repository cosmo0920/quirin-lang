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

import QCore.AtomicPublication

/-! Audited parity for the bounded CT41 synchronization prerequisite. -/

#print axioms QCore.AtomicPublication.synchronized_observation_orders_payload
#print axioms QCore.AtomicPublication.volatile_has_no_synchronizes_with_edge
#print axioms QCore.AtomicPublication.plain_has_no_synchronizes_with_edge
#print axioms QCore.AtomicPublication.relaxed_store_has_no_synchronizes_with_edge
#print axioms QCore.AtomicPublication.relaxed_load_has_no_synchronizes_with_edge
#print axioms QCore.AtomicPublication.repeated_publisher_has_no_synchronizes_with_edge
#print axioms QCore.AtomicPublication.fixture_has_a_real_three_edge_path
#print axioms QCore.AtomicPublication.equal_initial_and_ready_do_not_identify_the_release

#print axioms QCore.AtomicPublication.payload_path_requires_synchronization
#print axioms QCore.AtomicPublication.non_atomic_completion_has_no_payload_path

#print axioms QCore.AtomicPublication.a_zero_store_value_cannot_release
#print axioms QCore.AtomicPublication.a_zero_load_value_cannot_acquire
