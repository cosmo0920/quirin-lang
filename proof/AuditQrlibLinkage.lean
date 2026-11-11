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

import QCore.QrlibLinkage

-- Card `W48-QRLIB-LINKAGE`: `.qrlib` as the C linkage manifest of the FFI
-- route — the resolver, the generator and the stale check, and the FFI
-- publication's linkage premise.

-- An admitted binding names an exported defined symbol, uniquely.
#print axioms QCore.QrlibLinkage.an_admitted_binding_names_an_exported_defined_symbol
#print axioms QCore.QrlibLinkage.an_admitted_binding_is_the_only_linkable_one

-- Non-vacuity over the fixture's facts: weak and default-version admissions;
-- missing, undefined, local-only, duplicate, hidden-only, unindexed and
-- ordinal-only refusals.
#print axioms QCore.QrlibLinkage.a_weak_definition_is_admitted_as_weak
#print axioms QCore.QrlibLinkage.the_default_version_is_the_one_bound
#print axioms QCore.QrlibLinkage.missing_undefined_local_and_duplicate_refuse
#print axioms QCore.QrlibLinkage.hidden_unindexed_and_ordinal_only_refuse

-- The generated manifest is a function of the artifact and the declared
-- inputs; every entry names an exported defined symbol; the hash is pinned;
-- the stale check.
#print axioms QCore.QrlibLinkage.entries_resolve
#print axioms QCore.QrlibLinkage.every_generated_entry_names_an_exported_defined_symbol
#print axioms QCore.QrlibLinkage.the_generated_manifest_pins_the_hash
#print axioms QCore.QrlibLinkage.a_freshly_generated_manifest_is_not_stale
#print axioms QCore.QrlibLinkage.another_artifact_makes_the_stored_manifest_stale
#print axioms QCore.QrlibLinkage.one_refused_binding_refuses_the_manifest
#print axioms QCore.QrlibLinkage.entries_read_only_bound_names
#print axioms QCore.QrlibLinkage.generation_reads_only_the_hash_and_the_bound_names

-- The FFI verdict carries the artifact premise and is never plain Safe.
#print axioms QCore.QrlibLinkage.a_publication_names_every_called_functions_artifact
#print axioms QCore.QrlibLinkage.an_ffi_publication_is_never_plain_safe
#print axioms QCore.QrlibLinkage.an_unlinked_call_is_not_published
#print axioms QCore.QrlibLinkage.every_composed_verdict_keeps_the_linkage_premise
