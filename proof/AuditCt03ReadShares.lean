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

import QCore.Ct03ReadShares

-- Card `CT03-PROOF-ALPHABET`: the third proof leg, and the F02 alphabet grown
-- by the T05 Buffer write. Two modules are audited here: the conservative
-- alphabet extension (`QCore.TaskTraceWrites`, the transcription of the
-- extended production relation) and the shared-reads discharge proved on
-- CT03's own trace terms (`QCore.Ct03ReadShares`). The named runner
-- `compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape`
-- checks the reference run event for event against the real pipeline's
-- execution of `compiler/tests/fixtures/ct/CT03/case.qr`, recomputes the
-- digest pin and runs the same total decision procedure on the artifact.

-- The alphabet extension is conservative: on write-free traces the extended
-- relation, declaration and digest are the pinned ones — for arbitrary
-- traces, conjunct for conjunct — and one pinned F02 verdict digest is
-- carried onto the extended relation from the old theorem alone.
#print axioms QCore.TaskTraceWrites.relate_agrees_on_the_embedded_alphabet
#print axioms QCore.TaskTraceWrites.declare_agrees_on_the_embedded_alphabet
#print axioms QCore.TaskTraceWrites.mixTrace_agrees_on_the_embedded_alphabet
#print axioms QCore.TaskTraceWrites.write_free_traces_are_embedded
#print axioms QCore.TaskTraceWrites.write_free_traces_relate_exactly_as_before
#print axioms QCore.TaskTraceWrites.the_pinned_domain_one_verdicts_are_carried

-- The one rule the write extends (the payload conjunct) is not vacuous in
-- either direction.
#print axioms QCore.TaskTraceWrites.a_rewritten_payload_is_admitted
#print axioms QCore.TaskTraceWrites.a_stale_read_is_refused

-- CT03's reference run is a declared admitted causal order of the extended
-- relation; its projections and the transcription pin.
#print axioms QCore.Ct03ReadShares.ct03_reference_run_is_an_admitted_causal_order
#print axioms QCore.Ct03ReadShares.parent_projection_of_the_reference
#print axioms QCore.Ct03ReadShares.childOne_projection_of_the_reference
#print axioms QCore.Ct03ReadShares.childTwo_projection_of_the_reference
#print axioms QCore.Ct03ReadShares.ct03_trace_digest

-- Meaning: what the decision procedure's acceptance has been shown to imply,
-- for arbitrary traces — the parent's and each borrower's exact program
-- orders, the unique parent-side release after the restored read-back, the
-- writes bracketing the lending window, each borrower's read strictly inside
-- its own window, the written values observable through the shares, and the
-- one shared generation.
#print axioms QCore.Ct03ReadShares.discharged_parent_facts_are_exactly_the_scenario
#print axioms QCore.Ct03ReadShares.discharged_borrower_facts_are_exactly_the_scenario
#print axioms QCore.Ct03ReadShares.discharged_release_is_unique
#print axioms QCore.Ct03ReadShares.discharged_writes_sit_on_either_side_of_the_window
#print axioms QCore.Ct03ReadShares.discharged_release_follows_the_restored_read
#print axioms QCore.Ct03ReadShares.discharged_reads_report_the_written_values
#print axioms QCore.Ct03ReadShares.discharged_shared_generation_is_the_allocations
#print axioms QCore.Ct03ReadShares.discharged_each_borrower_reads_inside_its_window

-- CT03's reference run is accepted, and so is every rendering the extended
-- F02 relation admits against it — arbitrary traces, not an enumerated
-- domain, with the two window clauses extracted from the relation's own loan
-- and activity-after-join conjuncts.
#print axioms QCore.Ct03ReadShares.ct03_reference_run_discharges_the_shared_reads
#print axioms QCore.Ct03ReadShares.every_admitted_rendering_discharges_the_shared_reads

-- Named refusals: the procedure is not vacuous.
#print axioms QCore.Ct03ReadShares.a_read_before_the_loan_is_refused
#print axioms QCore.Ct03ReadShares.a_read_after_the_return_is_refused
#print axioms QCore.Ct03ReadShares.a_write_inside_the_window_is_refused
#print axioms QCore.Ct03ReadShares.a_stale_after_read_is_refused
#print axioms QCore.Ct03ReadShares.a_missing_release_is_refused
#print axioms QCore.Ct03ReadShares.a_double_release_is_refused
#print axioms QCore.Ct03ReadShares.a_borrower_that_writes_is_refused
