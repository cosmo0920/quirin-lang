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

import QCore.Ct50LifetimeIdentity

-- Card `CT-PROOF-LEG-3` (wave 41): the fifth proof leg, and the fourth bound
-- through the F02 trace relation. CT50's reference run is single-actor, so
-- the closure over admitted renderings is the degenerate case — proved here
-- as `an_admitted_rendering_is_the_reference_run` rather than assumed. The
-- named runner
-- `compiler/tests/ct50_proof_leg.rs::ct50_artifact_is_the_lean_proved_lifetime_identity_shape`
-- compiles `compiler/tests/fixtures/ct/CT50/case.qr`, executes the checked
-- task module, derives the admitted causal order, recomputes the digest pin
-- and runs the transcription of the procedure below on the artifact.
--
-- Scope: the *capability* half of CT50's scenario. The numeric address reuse
-- the scenario names is invisible to the F02 alphabet and stays with the
-- existing native observation, unproved.

-- The vocabulary the leg adds, printed so a later change is visible here.
#print axioms QCore.Ct50LifetimeIdentity.isStaleUse
#print axioms QCore.Ct50LifetimeIdentity.readsBack
#print axioms QCore.Ct50LifetimeIdentity.dischargesLifetimeIdentity

-- **CT50's own terms**: the reference run is an admitted causal order, every
-- fact is the root task's, and the transcription pin the runner recomputes.
#print axioms QCore.Ct50LifetimeIdentity.ct50_reference_run_is_an_admitted_causal_order
#print axioms QCore.Ct50LifetimeIdentity.root_projection_of_the_reference
#print axioms QCore.Ct50LifetimeIdentity.ct50_trace_digest
#print axioms QCore.Ct50LifetimeIdentity.ct50_reference_run_discharges_the_lifetime_identity

-- **Meaning**: what an accepted trace has been shown to satisfy, each stated
-- for an arbitrary trace.
#print axioms QCore.Ct50LifetimeIdentity.discharged_each_generation_is_allocated_and_released_once
#print axioms QCore.Ct50LifetimeIdentity.discharged_the_old_lifetime_closes_before_the_new_one_opens
#print axioms QCore.Ct50LifetimeIdentity.discharged_no_fact_uses_the_old_capability
#print axioms QCore.Ct50LifetimeIdentity.discharged_the_old_generation_is_never_read
#print axioms QCore.Ct50LifetimeIdentity.discharged_the_one_read_names_the_new_generation
#print axioms QCore.Ct50LifetimeIdentity.discharged_the_read_sits_inside_the_new_lifetime
#print axioms QCore.Ct50LifetimeIdentity.discharged_every_fact_is_the_root_tasks

-- **A clause that is not independent**, stated rather than left to be found:
-- the card's harness measured that weakening the payload clause reddens
-- nothing, because the read-window chain and the read count imply it.
#print axioms QCore.Ct50LifetimeIdentity.the_payload_clause_is_implied_by_the_read_window_and_the_read_count

-- **The closure**: a single-actor reference admits nothing but itself, so
-- every admitted rendering discharges the scenario.
#print axioms QCore.Ct50LifetimeIdentity.an_admitted_rendering_is_the_reference_run
#print axioms QCore.Ct50LifetimeIdentity.every_admitted_rendering_discharges_the_lifetime_identity

-- **The refusals**, and for four of them the measurement that the clause
-- catching each mutation is the named one: every other clause still holds.
#print axioms QCore.Ct50LifetimeIdentity.overlapping_lifetimes_are_refused
#print axioms QCore.Ct50LifetimeIdentity.overlapping_lifetimes_fail_only_the_old_lifetime_clause
#print axioms QCore.Ct50LifetimeIdentity.an_escaping_stale_capability_is_refused
#print axioms QCore.Ct50LifetimeIdentity.an_escaping_stale_capability_fails_only_the_stale_use_clause
#print axioms QCore.Ct50LifetimeIdentity.a_read_after_the_fresh_release_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_read_after_the_fresh_release_fails_only_the_read_window_clause
#print axioms QCore.Ct50LifetimeIdentity.a_result_claimed_by_a_second_actor_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_result_claimed_by_a_second_actor_fails_only_the_single_actor_clause
#print axioms QCore.Ct50LifetimeIdentity.a_read_by_a_second_actor_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_read_by_a_second_actor_fails_two_clauses
#print axioms QCore.Ct50LifetimeIdentity.a_read_of_the_stale_generation_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_read_carrying_the_old_payload_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_leaked_stale_generation_is_refused
#print axioms QCore.Ct50LifetimeIdentity.a_double_release_of_the_fresh_generation_is_refused
