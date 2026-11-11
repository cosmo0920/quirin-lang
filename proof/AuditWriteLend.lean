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

import QCore.WriteLend

-- Card `SLICE-PERMISSIONS-WRITE-LEND`: a loan carrying **write authority**
-- over a range of an owned buffer, so two parties mutate two disjoint ranges
-- independently — the half of CT07 the read card recorded as not expressible,
-- and the custody shape (`STD-BUFFER-SLICE`'s footprint fraction in a borrow
-- node) it named as what the write direction needs.

-- The vocabulary the card adds, printed so that a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.WriteLend.blocksLend
#print axioms QCore.WriteLend.admitsLend
#print axioms QCore.WriteLend.blocksOwner
#print axioms QCore.WriteLend.ownerBlocked
#print axioms QCore.WriteLend.rebase
#print axioms QCore.WriteLend.hasLoan
#print axioms QCore.WriteLend.openLend
#print axioms QCore.WriteLend.confinedTo
#print axioms QCore.WriteLend.addedOperations

-- **The exclusivity rule is the disjointness question** — the read card's
-- `overlaps`, imported rather than restated, deciding which second lend is
-- admitted. No byte is lent twice.
#print axioms QCore.WriteLend.a_second_lend_is_admitted_iff_disjoint
#print axioms QCore.WriteLend.two_admitted_lends_share_no_byte
#print axioms QCore.WriteLend.an_empty_lend_is_admitted_beside_any_loan
#print axioms QCore.WriteLend.a_whole_borrow_blocks_every_nonempty_lend

-- **The owner's authority is transferred range-wise**: refused exactly where a
-- live lend holds the byte, retained everywhere else, and an empty footprint
-- (an extent query) conflicts with nothing.
#print axioms QCore.WriteLend.the_owner_is_excluded_exactly_where_the_loan_holds
#print axioms QCore.WriteLend.every_byte_has_exactly_one_write_authority
#print axioms QCore.WriteLend.an_extent_query_is_admitted_beside_any_loan
#print axioms QCore.WriteLend.a_shared_loan_blocks_no_read

-- **Rebasing into the owner's coordinates is sound**: an in-bounds loan access
-- stays inside its lend, is a byte the buffer has, and two disjoint lends'
-- accesses can never collide — the partition CT07's "independently" means.
#print axioms QCore.WriteLend.a_rebased_access_stays_inside_its_lend
#print axioms QCore.WriteLend.a_rebased_access_is_a_byte_the_buffer_has
#print axioms QCore.WriteLend.disjoint_lends_rebase_to_distinct_bytes

-- The release arm is **not** range-wise — the read card's asymmetry inherited
-- by the write one: a lend blocks the owner's drop however narrow.
#print axioms QCore.WriteLend.a_lend_blocks_the_owners_drop

-- **The unlent bytes survive the window** — the mirror image of the read
-- card's soundness statement: a window of stores confined to the lend
-- preserves every byte outside it, and a rebased in-bounds store is confined
-- by construction.
#print axioms QCore.WriteLend.a_confined_step_preserves_every_unlent_byte
#print axioms QCore.WriteLend.a_confined_window_preserves_every_unlent_byte
#print axioms QCore.WriteLend.a_rebased_loan_store_is_confined

-- What was added, as one list, so a widening changes a theorem rather than
-- slipping past.
#print axioms QCore.WriteLend.the_added_operation_is_exactly_one
