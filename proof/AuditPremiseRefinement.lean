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

import QCore.PremiseRefinement
#check QCore.CjsonRepresentationPremises.chain
#check QCore.CjsonRepresentationPremises.distinct
#check QCore.CjsonRepresentationPremises.empty
#check QCore.CjsonRepresentationPremises.firstPrev
#check QCore.CjsonRepresentationPremises.lastNext
#check QCore.CjsonRepresentationPremises.linkTargets
#check QCore.CjsonRepresentationPremises.noWriter
#check QCore.CjsonRepresentationPremises.parentOutside
#check QCore.CjsonRepresentationPremises.predecessor
#print axioms QCore.CjsonRepresentationPremises.links_stay_live_owned_and_exclude_parent
#print axioms QCore.CjsonRepresentationPremises.distinct_indices_identify_distinct_children
#print axioms QCore.CjsonRepresentationPremises.a_writing_step_refutes_no_writer
#print axioms QCore.PremiseRefinement.exact_erasure
#print axioms QCore.PremiseRefinement.changing_refinement_preserves_legacy
#print axioms QCore.PremiseRefinement.refinement_support_requires_all
#print axioms QCore.PremiseRefinement.foreign_context_refuses
#print axioms QCore.PremiseRefinement.supported_refinement_does_not_support_legacy
#print axioms QCore.PremiseRefinement.refuted_refinement_does_not_refute_legacy
#print axioms QCore.PremiseRefinement.impact_is_exact_context_and_dependency
#print axioms QCore.PremiseRefinement.unrelated_record_unchanged
#print axioms QCore.PremiseRefinement.reference_identity_does_not_establish_no_writer
