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

import QCore.Ct07LeaseLedger

-- Card `CT07-TWO-THREAD-PROOF-LEG` (wave 39): the fourth proof leg, and the
-- first bound through the scoped-task ledger rather than the F02 trace
-- relation — `reference_trace` refuses CT07's run at its first lease event by
-- constraint 15n's decided boundary, so the binding goes through the checked
-- `ScopedTasks` module's lease set. The named runner
-- `compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape`
-- transcribes the reference execution of
-- `compiler/tests/fixtures/ct/CT07/case.qr` into `Step`s, replays them
-- with the transcribed `replay`, recomputes the digest pin and checks the
-- scenario shape on the artifact's own ledger state.

-- The vocabulary the leg adds, printed so a later change is visible here.
#print axioms QCore.Ct07LeaseLedger.step
#print axioms QCore.Ct07LeaseLedger.replayFrom
#print axioms QCore.Ct07LeaseLedger.replay
#print axioms QCore.Ct07LeaseLedger.dischargesDisjointHalves
#print axioms QCore.Ct07LeaseLedger.mixLedger

-- **The equivalence the runner rests on**: a successful replay from the
-- empty ledger reaches only `Reachable` states, so the Rust transcription's
-- `Some` is a witness for the wave-31 theorem's hypothesis.
#print axioms QCore.Ct07LeaseLedger.step_reaches
#print axioms QCore.Ct07LeaseLedger.replayFrom_reaches
#print axioms QCore.Ct07LeaseLedger.replay_reaches

-- **CT07's own terms**: both opens admitted reaching `ct07Ledger`, the whole
-- run returning every lease, the state reachable, and the transcription pin.
#print axioms QCore.Ct07LeaseLedger.ct07_both_leases_are_admitted
#print axioms QCore.Ct07LeaseLedger.ct07_the_run_returns_every_lease
#print axioms QCore.Ct07LeaseLedger.ct07_ledger_is_reachable
#print axioms QCore.Ct07LeaseLedger.ct07_ledger_digest

-- **The bound theorems**: wave 31's `no_byte_has_two_write_authorities` and
-- wave 36's `no_byte_is_written_by_two_parties`, each instantiated on CT07's
-- ledger state, the artifact's two writes admitted against it.
#print axioms QCore.Ct07LeaseLedger.ct07_no_byte_has_two_write_authorities
#print axioms QCore.Ct07LeaseLedger.ct07_writes_are_admitted
#print axioms QCore.Ct07LeaseLedger.ct07_no_byte_is_written_by_two_parties

-- **The scenario shape**: two leases, two distinct holders, every payload
-- byte under exactly one write authority — and what a discharged state
-- means.
#print axioms QCore.Ct07LeaseLedger.ct07_discharges_disjoint_halves
#print axioms QCore.Ct07LeaseLedger.a_discharged_ledger_holds_every_byte_exactly_once
#print axioms QCore.Ct07LeaseLedger.a_discharged_ledger_has_two_distinct_holders

-- **The three mutations, decided for the theorem's reason**: overlapping
-- halves at the disjointness clause, an empty half at the nonemptiness
-- clause, a single writer reachable but not the scenario.
#print axioms QCore.Ct07LeaseLedger.overlapping_halves_are_refused
#print axioms QCore.Ct07LeaseLedger.the_overlap_is_the_disjointness_clause
#print axioms QCore.Ct07LeaseLedger.an_empty_half_is_refused
#print axioms QCore.Ct07LeaseLedger.the_empty_half_is_the_nonemptiness_clause
#print axioms QCore.Ct07LeaseLedger.a_single_writer_is_reachable_but_not_the_scenario
#print axioms QCore.Ct07LeaseLedger.a_single_writer_leaves_the_other_half_unheld
