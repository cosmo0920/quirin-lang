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

import QCore.KindedCustody

/-! Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: kind-indexed custody
(`resource_kind`, `resource_return_kind`), integer-carried identities with
sentinel validity, and the gateway's descriptor guards.

Seven directions are audited. **A release discharges only its own kind**: a
mismatched release moves no status and is a candidate (`free(fopen(…))`,
`fclose(malloc(…))`, `close(fp)`), a matching one releases. **Validity is the
integer counterpart of nullness**: the comparison table is sound for every
value a descriptor call can produce, and the two edges it leaves unrefined are
unrefinable because descriptor `0` is valid. **A null test of a descriptor
refines nothing**, beside the pointer rule it must not inherit. **A tracked
descriptor on a custody-neutral scalar route is refused**, never neutral data,
while an untracked name stays neutral; arithmetic creates no identity. **Use
after close** is a candidate for a non-heap identity; the heap keeps its
refusal. **No `Safe`** from any candidate or refusal. **Conservativity**: an
all-heap program runs the one-kind transfer exactly. Five witness programs
complete it, including the corpus's `if (fd > 0)` leak shape.

What this is not: a proof about the Rust transfer (trust boundary 28's
correspondence premise) or about libc (the libc crate's trusted premises). A
candidate is never a validated upstream defect. -/

-- A release discharges only its own kind.
#print axioms QCore.KindedCustody.a_mismatched_release_discharges_nothing
#print axioms QCore.KindedCustody.a_mismatched_release_is_a_candidate
#print axioms QCore.KindedCustody.a_matching_release_releases
#print axioms QCore.KindedCustody.freeing_a_stream_is_a_candidate
#print axioms QCore.KindedCustody.closing_a_heap_allocation_as_a_stream_is_a_candidate
#print axioms QCore.KindedCustody.closing_a_stream_as_a_descriptor_is_a_candidate

-- Validity: the integer counterpart of nullness.
#print axioms QCore.KindedCustody.facts_are_sound
#print axioms QCore.KindedCustody.the_positive_tests_false_edge_includes_a_valid_descriptor
#print axioms QCore.KindedCustody.equality_with_zero_decides_no_validity
#print axioms QCore.KindedCustody.an_invalid_edge_removes_the_descriptor
#print axioms QCore.KindedCustody.a_valid_edge_keeps_a_valid_descriptor
#print axioms QCore.KindedCustody.an_unrefined_edge_changes_nothing
#print axioms QCore.KindedCustody.a_validity_test_of_a_pointer_kind_is_refused

-- A null test of a descriptor refines nothing.
#print axioms QCore.KindedCustody.a_descriptor_null_test_refines_nothing
#print axioms QCore.KindedCustody.a_pointer_null_edge_removes_its_site

-- A tracked descriptor is custody, not data.
#print axioms QCore.KindedCustody.a_tracked_descriptor_on_a_scalar_route_is_refused
#print axioms QCore.KindedCustody.a_tracked_descriptor_is_never_neutral_data
#print axioms QCore.KindedCustody.an_untracked_scalar_route_stays_neutral
#print axioms QCore.KindedCustody.arithmetic_creates_no_identity

-- Use after close.
#print axioms QCore.KindedCustody.a_use_after_close_is_a_candidate
#print axioms QCore.KindedCustody.a_heap_use_after_release_keeps_its_refusal

-- No `Safe` from any of it.
#print axioms QCore.KindedCustody.the_kind_candidates_are_safety_issues
#print axioms QCore.KindedCustody.a_kind_candidate_is_never_safe
#print axioms QCore.KindedCustody.a_refusal_is_never_safe

-- Conservativity.
#print axioms QCore.KindedCustody.an_all_heap_program_is_unaffected

-- Witness programs.
#print axioms QCore.KindedCustody.a_guarded_close_discharges
#print axioms QCore.KindedCustody.the_positive_guard_leaks_on_its_false_edge
#print axioms QCore.KindedCustody.a_double_close_is_a_candidate
#print axioms QCore.KindedCustody.a_mismatched_release_leaves_the_obligation
#print axioms QCore.KindedCustody.storing_a_descriptor_into_a_member_is_refused
