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

import QCore.PluginPremiseDecomposition

#print axioms QCore.PluginPremiseDecomposition.resolved_loaded_symbol_has_expected_abi
#print axioms QCore.PluginPremiseDecomposition.loaded_callback_after_close_is_refuted
#print axioms QCore.PluginPremiseDecomposition.loaded_fluent_core_store_is_refuted
#print axioms QCore.PluginPremiseDecomposition.callback_without_registration_or_default_is_refuted
#print axioms QCore.PluginPremiseDecomposition.semanticClauses_append
#print axioms QCore.PluginPremiseDecomposition.attachment_preserves_original_clause_interpretation
#print axioms QCore.PluginPremiseDecomposition.attachment_preserves_any_existing_null_checker
#print axioms QCore.PluginPremiseDecomposition.impact_requires_exact_context_and_dependency
#print axioms QCore.PluginPremiseDecomposition.foreign_property_is_not_selected

-- Check every named proposition bound by the explicit catalogs.
#check QCore.PluginPremiseDecomposition.artifactSymbolAbi
#check QCore.PluginPremiseDecomposition.callbackCandidates
#check QCore.PluginPremiseDecomposition.cjsonHooks
#check QCore.PluginPremiseDecomposition.cjsonShape
#check QCore.PluginPremiseDecomposition.codeContextLifetime
#check QCore.PluginPremiseDecomposition.coreFrame
#check QCore.PluginPremiseDecomposition.handedAccess
#check QCore.PluginPremiseDecomposition.interceptorCandidates
#check QCore.PluginPremiseDecomposition.interceptorCustody
#check QCore.PluginPremiseDecomposition.internalFrameApi
#check QCore.PluginPremiseDecomposition.loadedArtifacts
#check QCore.PluginPremiseDecomposition.loadedCustody
#check QCore.PluginPremiseDecomposition.loadedLifetime
#check QCore.PluginPremiseDecomposition.loadedRegistration
#check QCore.PluginPremiseDecomposition.loadedUnload
#check QCore.PluginPremiseDecomposition.loaderEnvironment
#check QCore.PluginPremiseDecomposition.outerBridge
#check QCore.PluginPremiseDecomposition.preexistingAllocatorHooks
#check QCore.PluginPremiseDecomposition.publicCallbackRegistration
#check QCore.PluginPremiseDecomposition.registrationLifecycle
#check QCore.PluginPremiseDecomposition.unloadOrder
