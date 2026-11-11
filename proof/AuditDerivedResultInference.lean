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

import QCore.DerivedResultInference
import QCore.ExactFormalReturn

/-! Card **W43-RESULT-ALIAS-RETURNED**: inferred derived-result summaries
(`crates/qcore-c-gateway/src/legacy_derived_summary.rs`) and the gate's derived
arm (`result_matches`).

Six directions are audited. **The gate is sound against the contract meaning**:
every concrete result an accepted exit denotes is admitted by the spec, for
every base value. **The proposal validates and is tight** (exact exactly when no
exit is interior). **Every refused shape is refused**: a mixed exit, a pointer
derived from another site, a touched base, an unclean or exitless run, and the
proposal declines a mixed and an all-null body. **The name-only base stays
name-only** under borrowing, and a release reaching it is never silent. **An
offset base is interior**, not an exact alias. **Callers apply the summary as a
declaration**: no status moves, a wrapper's result is invalidated by its base's
release, and releasing it never releases the base. Three witnesses complete it.

What this is not: a proof about the Rust transfer (trust boundary 28's
correspondence premise). A candidate is never a validated upstream defect. -/

-- The gate is sound.
#print axioms QCore.DerivedResultInference.an_accepted_exit_is_admitted
#print axioms QCore.DerivedResultInference.validation_is_sound

-- Every refused shape is refused.
#print axioms QCore.DerivedResultInference.a_mixed_exit_refuses
#print axioms QCore.DerivedResultInference.another_base_refuses
#print axioms QCore.DerivedResultInference.a_touched_base_refuses
#print axioms QCore.DerivedResultInference.an_unclean_or_exitless_run_refuses

-- The proposal.
#print axioms QCore.DerivedResultInference.step_none
#print axioms QCore.DerivedResultInference.fold_some
#print axioms QCore.DerivedResultInference.the_proposal_accepts_every_exit
#print axioms QCore.DerivedResultInference.the_proposal_validates
#print axioms QCore.DerivedResultInference.the_proposal_is_exact_iff_every_exit_is
#print axioms QCore.DerivedResultInference.the_proposal_declines_a_mixed_exit
#print axioms QCore.DerivedResultInference.an_all_null_body_proposes_nothing

-- The name-only base.
#print axioms QCore.DerivedResultInference.borrowing_keeps_the_base_name_only
#print axioms QCore.DerivedResultInference.a_release_reaching_the_base_is_seen

-- An offset base.
#print axioms QCore.DerivedResultInference.an_offset_result_is_not_an_exact_alias
#print axioms QCore.DerivedResultInference.an_offset_result_is_interior

-- Callers apply the summary as a declaration.
#print axioms QCore.DerivedResultInference.applying_the_summary_changes_no_status
#print axioms QCore.DerivedResultInference.a_wrapper_result_is_invalidated
#print axioms QCore.DerivedResultInference.releasing_a_wrapper_result_never_releases_its_base

-- Witnesses.
#print axioms QCore.DerivedResultInference.the_exact_wrapper_witness
#print axioms QCore.DerivedResultInference.the_nullable_interior_wrapper_witness
#print axioms QCore.DerivedResultInference.the_mixed_exit_witness

-- Rule 36: exact original borrowed-formal source/flow admission.
#print axioms QCore.ExactFormalReturn.missing_source_fact_refuses
#print axioms QCore.ExactFormalReturn.admitted_transfer_preserves_every_field
#print axioms QCore.ExactFormalReturn.every_transfer_preserves_owed_resources
#print axioms QCore.ExactFormalReturn.every_transfer_preserves_pointer_and_frame
#print axioms QCore.ExactFormalReturn.refusal_keeps_prior_obligations_and_adds_unknown
#print axioms QCore.ExactFormalReturn.exact_observation_denotes_original_value
#print axioms QCore.ExactFormalReturn.null_base_is_never_promoted_to_nonnull
#print axioms QCore.ExactFormalReturn.interior_base_offset_is_preserved
#print axioms QCore.ExactFormalReturn.refused_exit_never_validates
#print axioms QCore.ExactFormalReturn.prior_unknown_cannot_be_cleared
#print axioms QCore.ExactFormalReturn.shared_flow_vectors
#print axioms QCore.ExactFormalReturn.exact_witness_preserves_null
#print axioms QCore.ExactFormalReturn.unknown_derived_base_cannot_prove_infeasible
#print axioms QCore.ExactFormalReturn.nonneutral_result_retains_precedence
#print axioms QCore.ExactFormalReturn.failed_validation_retains_old_summary
#print axioms QCore.ExactFormalReturn.refinement_preserves_parameter_custody
