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

import QCore.ProviderSelection

#print axioms QCore.ProviderSelection.expand_sound
#print axioms QCore.ProviderSelection.expandAll_sound
#print axioms QCore.ProviderSelection.closure_contains
#print axioms QCore.ProviderSelection.closure_closed
#print axioms QCore.ProviderSelection.closure_known
#print axioms QCore.ProviderSelection.closure_unknown_refused
#print axioms QCore.ProviderSelection.quirinOnly_sound
#print axioms QCore.ProviderSelection.resolve_sound
#print axioms QCore.ProviderSelection.resolve_empty_refused
#print axioms QCore.ProviderSelection.resolve_unknown_provider_refused
#print axioms QCore.ProviderSelection.resolve_missing_module_refused
#print axioms QCore.ProviderSelection.resolve_duplicate_refused
#print axioms QCore.ProviderSelection.resolve_guard_refuses
#print axioms QCore.ProviderSelection.findCrate_mem
#print axioms QCore.ProviderSelection.expand_whole_is_view
#print axioms QCore.ProviderSelection.expand_whole_without_view_refused
#print axioms QCore.ProviderSelection.resolve_empty_view_refused
#print axioms QCore.ProviderSelection.resolve_modules
#print axioms QCore.ProviderSelection.findCrate_self
#print axioms QCore.ProviderSelection.expandAll_whole_crates
#print axioms QCore.ProviderSelection.resolve_allLoaded_is_allViews
#print axioms QCore.ProviderSelection.allLoaded_is_every_crate_whole
#print axioms QCore.ProviderSelection.full_mode_unchanged
#print axioms QCore.ProviderSelection.contracts_only_default_is_all_loaded
#print axioms QCore.ProviderSelection.contracts_only_all_loaded_needs_load_path
#print axioms QCore.ProviderSelection.contracts_only_explicit_narrows
#print axioms QCore.ProviderSelection.allLoaded_only_when_contracts_only
#print axioms QCore.ProviderSelection.lower_keeps_selection
#print axioms QCore.ProviderSelection.lower_reference_iff
#print axioms QCore.ProviderSelection.agree_ok_iff
#print axioms QCore.ProviderSelection.agree_conflict
#print axioms QCore.ProviderSelection.no_default_provider_set
#print axioms QCore.ProviderSelection.providers_and_explicit_manifest_refused
#print axioms QCore.ProviderSelection.providers_supersede_profile_default
#print axioms QCore.ProviderSelection.chooseSource_providers_only_when_selected
#print axioms QCore.ProviderSelection.contractsOnly_admits_only_contracts
#print axioms QCore.ProviderSelection.contractsOnly_mixed_refused
#print axioms QCore.ProviderSelection.admitRun_never_narrows
#print axioms QCore.ProviderSelection.jobs_flag_first
#print axioms QCore.ProviderSelection.jobs_config_second
#print axioms QCore.ProviderSelection.jobs_environment_third
#print axioms QCore.ProviderSelection.jobs_default_capped
#print axioms QCore.ProviderSelection.jobs_positive
#print axioms QCore.ProviderSelection.jobs_zero_flag_refused
#print axioms QCore.ProviderSelection.jobs_zero_config_refused
#print axioms QCore.ProviderSelection.libraryClosure_contains
#print axioms QCore.ProviderSelection.libraryClosure_closed
#print axioms QCore.ProviderSelection.libraryClosure_known
#print axioms QCore.ProviderSelection.libraryClosure_unknown_refused
#print axioms QCore.ProviderSelection.resolveLibrarySlice_unknown_library_refused
#print axioms QCore.ProviderSelection.resolveLibrarySlice_contains_root
#print axioms QCore.ProviderSelection.resolveLibrarySlice_closed_under_dependents
#print axioms QCore.ProviderSelection.resolveSliceMembers_sound
#print axioms QCore.ProviderSelection.joinSliceVerdict_assoc
#print axioms QCore.ProviderSelection.joinSliceVerdict_comm
#print axioms QCore.ProviderSelection.joinSliceVerdict_unknown_never_safe
#print axioms QCore.ProviderSelection.foldSlices_append
