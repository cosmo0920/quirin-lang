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

import QCore.ContractInherit

#print axioms QCore.ContractInherit.findCrate_sound
#print axioms QCore.ContractInherit.findCrate_duplicate_refused
#print axioms QCore.ContractInherit.resolveInclude_sound
#print axioms QCore.ContractInherit.no_dependency_includes_nothing_foreign
#print axioms QCore.ContractInherit.unpublished_is_refused
#print axioms QCore.ContractInherit.deriveNamespace_requires_prefix
#print axioms QCore.ContractInherit.dashToUnderscore_ne_dash
#print axioms QCore.ContractInherit.deriveNamespace_has_no_dash
#print axioms QCore.ContractInherit.stripPrefixes_sound
#print axioms QCore.ContractInherit.isPrefixOf_append_self
#print axioms QCore.ContractInherit.prefixes_exclusive
#print axioms QCore.ContractInherit.deriveNamespace_data
#print axioms QCore.ContractInherit.deriveNamespace_executable
#print axioms QCore.ContractInherit.same_remainder_same_namespace
#print axioms QCore.ContractInherit.deriveNamespace_other_prefix_none
#print axioms QCore.ContractInherit.cratePrefix_spelling
#print axioms QCore.ContractInherit.executablePrefix_spelling
#print axioms QCore.ContractInherit.real_namespaces
#print axioms QCore.ContractInherit.nodup_map_injective
#print axioms QCore.ContractInherit.namespace_names_one_crate
#print axioms QCore.ContractInherit.namespacesValid_records_derived
#print axioms QCore.ContractInherit.shared_namespace_refused
#print axioms QCore.ContractInherit.allOk_each
#print axioms QCore.ContractInherit.walk_visited
#print axioms QCore.ContractInherit.walk_refuses_reach_into_visiting
#print axioms QCore.ContractInherit.walk_refuses_cycle
#print axioms QCore.ContractInherit.walk_ok_is_acyclic
#print axioms QCore.ContractInherit.self_inheritance_refused
#print axioms QCore.ContractInherit.indexOf?_some
#print axioms QCore.ContractInherit.indexOf?_none
#print axioms QCore.ContractInherit.indexOf?_nodup
#print axioms QCore.ContractInherit.rename_word_meaning
#print axioms QCore.ContractInherit.rename_preserves_meaning
#print axioms QCore.ContractInherit.captures_each
#print axioms QCore.ContractInherit.rename_clauses_preserve_meaning
#print axioms QCore.ContractInherit.unique_sound
#print axioms QCore.ContractInherit.unique_ambiguous
#print axioms QCore.ContractInherit.lookup_own_sound
#print axioms QCore.ContractInherit.unqualified_included_only_refused
#print axioms QCore.ContractInherit.lookup_qualified_sound
#print axioms QCore.ContractInherit.lookup_qualified_ambiguous
#print axioms QCore.ContractInherit.lookup_unknown_namespace
#print axioms QCore.ContractInherit.extension_refused
#print axioms QCore.ContractInherit.same_name_refused
#print axioms QCore.ContractInherit.elaboration_exact
#print axioms QCore.ContractInherit.inherited_is_not_weaker
#print axioms QCore.ContractInherit.transitive_inheritance_preserves_meaning
#print axioms QCore.ContractInherit.positional_reading_can_diverge
#print axioms QCore.ContractInherit.consumerGate_sound
#print axioms QCore.ContractInherit.contractDeps_sound
#print axioms QCore.ContractInherit.include_follows_a_cargo_edge
#print axioms QCore.ContractInherit.no_cargo_edge_includes_nothing_foreign
#print axioms QCore.ContractInherit.admitPinned_sound
#print axioms QCore.ContractInherit.changed_parent_refused
#print axioms QCore.ContractInherit.pathsOf_map_path
#print axioms QCore.ContractInherit.refsOf_map_path
#print axioms QCore.ContractInherit.schema_two_paths_are_schema_one
