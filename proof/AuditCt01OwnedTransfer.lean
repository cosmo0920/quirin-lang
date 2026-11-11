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

import QCore.Ct01OwnedTransfer

-- Card `CT01-PROOF-LEG`: the owned-transfer discharge, proved on CT01's own
-- trace terms. `QCore.TaskTraceRelation.roundTrip` is CT01's reference run —
-- the workspace test
-- `compiler/tests/f02_trace_relation.rs::the_declared_trace_fixtures_are_the_reference_runs`
-- checks it event for event against the real pipeline's execution of
-- `compiler/tests/fixtures/ct/CT01/case.qr`, and the named runner
-- `compiler/tests/ct01_proof_leg.rs` recomputes the digest pin and runs the
-- same total decision procedure on the artifact.

-- The reference projections and the transcription pin.
#print axioms QCore.Ct01OwnedTransfer.parent_projection_of_the_reference
#print axioms QCore.Ct01OwnedTransfer.child_projection_of_the_reference
#print axioms QCore.Ct01OwnedTransfer.ct01_trace_digest

-- Meaning: what the decision procedure's acceptance has been shown to imply,
-- for arbitrary traces — the parent's exact program order, the unique
-- parent-side release after the join, the read-back between them, and the
-- generation-preserving child return.
#print axioms QCore.Ct01OwnedTransfer.discharged_parent_facts_are_exactly_the_scenario
#print axioms QCore.Ct01OwnedTransfer.discharged_release_is_unique
#print axioms QCore.Ct01OwnedTransfer.discharged_join_precedes_the_release
#print axioms QCore.Ct01OwnedTransfer.discharged_read_back_sits_between_join_and_release
#print axioms QCore.Ct01OwnedTransfer.discharged_child_return_preserves_the_generation

-- CT01's reference run is accepted, and so is every rendering the F02
-- relation admits against it — arbitrary traces, not an enumerated domain.
#print axioms QCore.Ct01OwnedTransfer.ct01_reference_run_discharges_the_owned_transfer
#print axioms QCore.Ct01OwnedTransfer.every_admitted_rendering_discharges_the_owned_transfer

-- Named refusals: the procedure is not vacuous.
#print axioms QCore.Ct01OwnedTransfer.a_release_before_the_join_is_refused
#print axioms QCore.Ct01OwnedTransfer.a_missing_release_is_refused
#print axioms QCore.Ct01OwnedTransfer.a_release_by_the_child_is_refused
#print axioms QCore.Ct01OwnedTransfer.a_double_release_is_refused
#print axioms QCore.Ct01OwnedTransfer.a_generation_changing_return_is_refused
