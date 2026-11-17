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

import QCore.LibraryVersionRange

/-! Card **OPENSSL-PROVIDER-PLUGIN**: library version ranges (contract spec
§C06.2) and the OpenSSL plugin's premises.

**The in-force predicate.** A declaration stating a range is in force exactly
when the audited version was read, lies in `[lower, removed)` and equals the
lock's pinned version; below the lower bound, at or past the removal,
unreadable or different from the pin, it is not.

**Out of range discharges nothing.** A withheld declaration holds its calls at
`Pending` (a changed version) or `Unknown` (out of range, unreadable,
ambiguous), never `Safe`, and no chain containing such a call is `Safe`.

**The hold under a reduced environment** (card
LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING). The classifier reads a held
call's reason from the environment it analyses under; a reduction (the ABI
gate's) that carries the withheld table classifies as the unreduced one, one
that drops it publishes a range failure `Pending`; a checker binding's claim
is joined, never lowering a hold, so a `Pending` claim leaves a range failure
`Unknown`.

**The OpenSSL premises.** A counted release frees only at a zero count, so it
is not final while another reference is outstanding, and reading it as a final
free is unsound; a NULL-accepting release of NULL is a no-op and discharges
nothing.

The Rust boundary is pinned by the tests of `qcore_schemas::library_version`,
`qcore_audit::library_version` and `quirin-provider-openssl`'s checker. -/

-- The in-force predicate.
#print axioms QCore.LibraryVersionRange.Version.le_refl
#print axioms QCore.LibraryVersionRange.Version.lt_irrefl
#print axioms QCore.LibraryVersionRange.decide_inForce_iff
#print axioms QCore.LibraryVersionRange.below_lower_not_in_force
#print axioms QCore.LibraryVersionRange.past_removal_not_in_force
#print axioms QCore.LibraryVersionRange.unreadable_not_in_force
#print axioms QCore.LibraryVersionRange.pin_mismatch_not_in_force
#print axioms QCore.LibraryVersionRange.in_range_and_pinned_in_force

-- Out of range discharges nothing.
#print axioms QCore.LibraryVersionRange.not_in_force_never_safe
#print axioms QCore.LibraryVersionRange.changed_is_pending
#print axioms QCore.LibraryVersionRange.range_failure_is_unknown
#print axioms QCore.LibraryVersionRange.held_call_keeps_chain_non_safe

-- The hold under a reduced environment (card
-- LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING).
#print axioms QCore.LibraryVersionRange.heldBy_agrees_with_held
#print axioms QCore.LibraryVersionRange.dropped_table_lifts_a_range_failure
#print axioms QCore.LibraryVersionRange.carried_reduction_keeps_the_verdict
#print axioms QCore.LibraryVersionRange.carried_reduction_agrees_with_held
#print axioms QCore.LibraryVersionRange.binding_never_lowers_a_hold
#print axioms QCore.LibraryVersionRange.pending_binding_keeps_the_split

-- The OpenSSL premises.
#print axioms QCore.LibraryVersionRange.counted_release_is_not_final
#print axioms QCore.LibraryVersionRange.last_drop_frees
#print axioms QCore.LibraryVersionRange.final_free_reading_is_unsound
#print axioms QCore.LibraryVersionRange.null_release_is_noop
#print axioms QCore.LibraryVersionRange.null_drop_discharges_nothing
