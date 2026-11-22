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

import QCore.NullableGuardedRelease

/-! Card **NULLABLE-GUARDED-RELEASE**: a refused release is no leak, and a
guarded release of a spared binding is decided (gateway rule 74c).

Five directions are audited.

**A refused release claims nothing.** It marks its site; the union join keeps
the marker at every exit the refused path reaches.

**The claim follows the settled state.** The iterate claim was unsupported
by the settled exit; the Rust decision is `withdrawVectors`
(`withdraw_vectors_hold`), and it withdraws nothing the settled exit does not
mark.

**Only `Unsafe -> Unknown`.** A withdrawal sits beside an `Unknown` leaf.

**The frame.** A name whose only `&` uses are member addresses keeps its value
across a refused statement; the frame only widens; a slot address or a write
still poisons.

**The guard.** On a surviving binding the non-null edge denotes the site and
the null edge owes nothing; a reassignment breaks the premise.

Faithful states, frame reading and schedule in the Rust, and the
correspondence with it, are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_settled_leak.rs` and
`crates/qcore-c-gateway/tests/nullable_guarded_release.rs`. -/

-- A refused release claims nothing.
#print axioms QCore.NullableGuardedRelease.a_refused_release_claims_nothing
#print axioms QCore.NullableGuardedRelease.an_accepted_release_claims_nothing
#print axioms QCore.NullableGuardedRelease.the_marker_survives_the_join

-- The claim follows the settled state.
#print axioms QCore.NullableGuardedRelease.the_iterate_claim_was_unsupported
#print axioms QCore.NullableGuardedRelease.withdraw_vectors_hold
#print axioms QCore.NullableGuardedRelease.a_withdrawn_report_is_not_claimed
#print axioms QCore.NullableGuardedRelease.an_unmarked_report_is_published

-- Only Unsafe -> Unknown.
#print axioms QCore.NullableGuardedRelease.a_withdrawal_is_never_below_unknown
#print axioms QCore.NullableGuardedRelease.the_metadata_shape
#print axioms QCore.NullableGuardedRelease.a_settled_leak_stays_unsafe

-- The frame.
#print axioms QCore.NullableGuardedRelease.a_spared_name_keeps_its_value
#print axioms QCore.NullableGuardedRelease.the_frame_only_widens
#print axioms QCore.NullableGuardedRelease.a_slot_address_still_poisons
#print axioms QCore.NullableGuardedRelease.a_write_by_name_still_poisons
#print axioms QCore.NullableGuardedRelease.the_metadata_name_is_spared

-- The guard.
#print axioms QCore.NullableGuardedRelease.the_non_null_edge_denotes_the_site
#print axioms QCore.NullableGuardedRelease.the_null_edge_owes_nothing
#print axioms QCore.NullableGuardedRelease.a_reassignment_breaks_it
