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

import QCore.LifetimeCorrespondence

/-! F01: one checker/model correspondence boundary.

`QCore.Concurrency.Lifetime` is unchanged and stays registered in
`AuditCycle14.lean`. This audit covers only the finite-domain declarations that
tie it to the Rust checker `qcore_core::concurrency::check_concurrent`, whose
side is measured by `crates/qcore-core/tests/lifetime_correspondence.rs`.

Nothing here proves the Rust implementation correct. Each theorem is a kernel
evaluation over the declared domain, pinning a number the Rust harness
recomputes independently. -/

-- The enumerated domain is the one the Rust harness enumerates
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_size
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_trace_digest

-- The harness's transcription of `Lifetime.step`/`Lifetime.run` agrees with the
-- model on every trace of that domain, verdict and final state alike
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_outcome_digest
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_accepted_trace_count

-- The selected model lemma over the whole domain
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_refuses_every_terminal_participant

-- ... and its non-vacuity
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_terminal_reaching_trace_count
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_terminal_participant_probe_count
#print axioms QCore.Concurrency.LifetimeCorrespondence.domain_terminal_participant_probes_are_nonvacuous
