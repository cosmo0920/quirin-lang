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

import QCore.SlicePermissions

-- Card `CT07-SLICE-PERMISSIONS`: a borrow that names a **range** of an owned
-- buffer, and the range-wise exclusion rule it makes possible. The capability
-- card `STD-BUFFER` recorded as `STD-BUFFER-SLICE` and the wave-26 CT lane named
-- as what keeps CT07 and CT08 unmapped.

-- The vocabulary the card adds, printed so that a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.SlicePermissions.Range.contains
#print axioms QCore.SlicePermissions.overlaps
#print axioms QCore.SlicePermissions.writeRange
#print axioms QCore.SlicePermissions.liveAt
#print axioms QCore.SlicePermissions.excludedStore
#print axioms QCore.SlicePermissions.excludedRelease
#print axioms QCore.SlicePermissions.wholeRange
#print axioms QCore.SlicePermissions.Loan.toBorrow
#print axioms QCore.SlicePermissions.byteAt
#print axioms QCore.SlicePermissions.touchesRange
#print axioms QCore.SlicePermissions.admitsRange
#print axioms QCore.SlicePermissions.addedOperations

-- **The disjointness decision is the question, not an approximation of it.**
-- Every bound on this route is a check-time constant, so "provably disjoint" and
-- "disjoint" are one predicate.
#print axioms QCore.SlicePermissions.overlaps_iff_a_shared_byte
#print axioms QCore.SlicePermissions.overlaps_symm
#print axioms QCore.SlicePermissions.an_empty_range_overlaps_nothing
#print axioms QCore.SlicePermissions.adjacent_ranges_are_disjoint
#print axioms QCore.SlicePermissions.disjoint_ranges_share_no_byte

-- A write is a one-byte range, which is what makes "is this write inside that
-- loan" and "do these two loans overlap" one question decided by one function.
#print axioms QCore.SlicePermissions.a_write_conflicts_exactly_when_its_byte_is_lent

-- **The exclusion rule**, in both directions: refused inside the lent range,
-- admitted outside it — and the admission is the card.
#print axioms QCore.SlicePermissions.a_write_inside_a_live_loan_is_refused
#print axioms QCore.SlicePermissions.a_write_outside_the_loan_is_admitted
#print axioms QCore.SlicePermissions.a_write_after_the_window_is_admitted
#print axioms QCore.SlicePermissions.a_loan_constrains_only_its_own_buffer

-- The release arm is deliberately **not** range-wise: it ends the storage every
-- loan addresses, so it conflicts with any live loan however narrow.
#print axioms QCore.SlicePermissions.a_release_is_refused_for_any_live_loan

-- **Two disjoint loans**, which is the shape CT07 asks for.
#print axioms QCore.SlicePermissions.disjoint_loans_partition_their_writes
#print axioms QCore.SlicePermissions.a_write_outside_both_loans_is_admitted

-- **The shipped whole-buffer rule is this rule's degenerate case** — an equation
-- with `QCore.StringConcat.excluded`, the definition the route has been running
-- since card `STRING-CONCAT`. Nothing was replaced.
#print axioms QCore.SlicePermissions.a_whole_extent_loan_lends_every_byte
#print axioms QCore.SlicePermissions.the_whole_extent_decision_is_the_shipped_one
#print axioms QCore.SlicePermissions.the_release_decision_is_the_shipped_one

-- **The exclusion as soundness, byte by byte.** Over a window in which no
-- statement stores inside the lent range and none releases the buffer, every
-- byte the loan's view addresses is the byte that was there when its UTF-8
-- decision was made — and writes outside the range happen freely.
#print axioms QCore.SlicePermissions.an_untouching_step_preserves_the_lent_bytes
#print axioms QCore.SlicePermissions.an_excluded_range_preserves_its_bytes
#print axioms QCore.SlicePermissions.a_changed_lent_byte_had_a_touching_statement

-- The bound, decided at check time, and what an admitted range can see.
#print axioms QCore.SlicePermissions.an_admitted_range_lends_only_bytes_that_exist
#print axioms QCore.SlicePermissions.an_inverted_range_is_refused
#print axioms QCore.SlicePermissions.a_range_past_the_extent_is_refused
#print axioms QCore.SlicePermissions.the_whole_extent_is_an_admitted_range

-- What the card added, written as one list so a widening changes a theorem
-- rather than slipping past it. In particular there is no **write** lend.
#print axioms QCore.SlicePermissions.the_added_operation_is_exactly_one
