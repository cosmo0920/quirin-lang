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

import QCore.CProgram

-- Card `C-EMIT-MAIN-UNIFY`: the fragment assembler and its one injected entry
-- point.
--
-- The refactor itself — ten whole-program C backends turned into fragment
-- contributors with byte-identical output — is established by measurement, not
-- by this model: every published example's emitted C was compared before and
-- after, and the two F02 emission digests are unmoved. What is proved here is
-- that the assembly writes one entry point, that deduplication is idempotent
-- and keeps the first of each key, and that a fragment two routes share appears
-- once in the composed program.

-- Deduplication: no repeat survives, and the operation is idempotent, so
-- assembling an assembled contribution is the same assembly.
#print axioms QCore.CProgram.dedupFrom_skips_seen
#print axioms QCore.CProgram.dedupFrom_key_mem
#print axioms QCore.CProgram.dedupFrom_avoids_seen
#print axioms QCore.CProgram.dedupFrom_keys_nodup
#print axioms QCore.CProgram.deduplicate_keys_nodup
#print axioms QCore.CProgram.dedupFrom_of_nodup_disjoint
#print axioms QCore.CProgram.deduplicate_idempotent
#print axioms QCore.CProgram.deduplicate_is_a_sublist_in_membership

-- One entry point per program, and the honest converse: a fragment carrying its
-- own is counted too, which is why the Rust side keeps a scan beside the type.
#print axioms QCore.CProgram.countOccurrences_append_le
#print axioms QCore.CProgram.assemble_contains_the_entry
#print axioms QCore.CProgram.assemble_writes_exactly_one_entry
#print axioms QCore.CProgram.an_empty_contribution_still_has_one_entry
#print axioms QCore.CProgram.a_smuggled_entry_is_counted_too

-- Composition: nothing is lost, a shared fragment appears once, the declared
-- sections order the routes, and merging is idempotent.
#print axioms QCore.CProgram.insertBySection_length
#print axioms QCore.CProgram.insertBySection_mem
#print axioms QCore.CProgram.sortBySection_mem
#print axioms QCore.CProgram.merge_preserves_the_union_of_keys
#print axioms QCore.CProgram.a_shared_key_survives_a_merge_once
#print axioms QCore.CProgram.a_merged_program_still_has_one_entry
#print axioms QCore.CProgram.merging_puts_the_compatibility_header_first
#print axioms QCore.CProgram.merging_is_idempotent

-- The two-sided domain tie with `qcore_core::c_program`'s own test.
#print axioms QCore.CProgram.domain_has_six_contributions
#print axioms QCore.CProgram.the_domain_outcomes_are_pinned
