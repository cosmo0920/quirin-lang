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

import QCore.ResourceKinds

/-! Card **CG25-RESOURCE-KINDS** (gateway rule 11v): the `dynamic_library` kind
and the in-place `regex` kind, absorbing `W44-DLOPEN-DECLARATION-REMEASURE` and
`W43-LIBC-IN-PLACE-LIFECYCLE`.

Six directions are audited. **The kind table**: one spelling and one carrier
per kind, bound to the Rust table and to the libc provider declarations by
`kindVectors` and `protocolVectors`. **The library handle** is a pointer kind:
a release of another kind discharges nothing and is a candidate in both
directions (`free(h)`, `dlclose(p)`), its null edge removes it, and a lookup
after `dlclose` is a candidate. **The in-place status is the sentinel**: the
zero edge certifies the obligation, the nonzero edge removes it, a written
status decides nothing, an unheld status and an undecided finaliser are
refused. **Distinctness and candidates**: a finaliser of another kind
discharges nothing, and double finalisation, use after finalisation and a data
route over tracked storage are candidates or refusals. **No `Safe`** from any
of them. **Witness programs**, including both paths of `rd_regex_match`.

What this is not: a proof about the Rust transfer (trust boundary 28's
correspondence premise) or about libc (the libc crate's trusted premises). A
candidate is never a validated upstream defect. -/

-- The kind table.
#print axioms QCore.ResourceKinds.kind_vectors_are_the_table
#print axioms QCore.ResourceKinds.every_kind_is_listed
#print axioms QCore.ResourceKinds.spellings_are_distinct
#print axioms QCore.ResourceKinds.protocol_vectors_count

-- The library handle.
#print axioms QCore.ResourceKinds.the_library_handle_is_a_pointer_kind
#print axioms QCore.ResourceKinds.freeing_a_library_handle_is_a_candidate
#print axioms QCore.ResourceKinds.closing_a_heap_allocation_as_a_library_is_a_candidate
#print axioms QCore.ResourceKinds.a_library_null_edge_removes_the_handle
#print axioms QCore.ResourceKinds.a_symbol_lookup_after_close_is_a_candidate

-- The in-place status is the sentinel.
#print axioms QCore.ResourceKinds.the_status_table_is_sound
#print axioms QCore.ResourceKinds.the_zero_status_edge_certifies
#print axioms QCore.ResourceKinds.the_nonzero_status_edge_owes_nothing
#print axioms QCore.ResourceKinds.a_written_status_decides_nothing
#print axioms QCore.ResourceKinds.an_untested_finalise_is_refused
#print axioms QCore.ResourceKinds.an_unheld_status_is_refused

-- Distinctness and the lifecycle candidates.
#print axioms QCore.ResourceKinds.a_finaliser_of_another_kind_is_a_candidate
#print axioms QCore.ResourceKinds.a_certified_object_is_finalised
#print axioms QCore.ResourceKinds.a_double_finalise_is_a_candidate
#print axioms QCore.ResourceKinds.a_use_after_finalise_is_a_candidate
#print axioms QCore.ResourceKinds.a_data_route_over_tracked_storage_is_refused

-- No `Safe`.
#print axioms QCore.ResourceKinds.no_candidate_is_safe

-- Witness programs.
#print axioms QCore.ResourceKinds.the_regex_match_failure_path_owes_nothing
#print axioms QCore.ResourceKinds.the_regex_match_success_path_discharges
#print axioms QCore.ResourceKinds.a_compiled_object_never_freed_leaks
#print axioms QCore.ResourceKinds.an_unheld_compile_is_never_safe
#print axioms QCore.ResourceKinds.an_overwritten_status_refuses_the_finaliser
#print axioms QCore.ResourceKinds.a_double_regfree_is_a_candidate
#print axioms QCore.ResourceKinds.a_library_release_of_a_regex_leaves_the_obligation
