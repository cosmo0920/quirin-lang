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

import QCore.LookupRestrictedReuse

/-! Card **MEASURE-LOOKUP-RECORD-REUSE**: pinned group checkpoints keyed on
the names their units read.

**One unit.** A consumer of the stage environment that read no whole table,
run under an environment agreeing on every name it read, makes the same reads
and returns the same result.

**A group.** A sealed checkpoint whose run identity, fingerprints and scope
(the environment restricted to the names its units read, or the whole
environment's digest for a whole reader) recomputed now equal the stored
ones yields exactly the rows recomputed now; a changed read name, or any
change for a whole reader, recomputes.

**The staged run.** Every group of every stage reused or recomputed this way
gives the whole run's final rows and publication
(`QCore.StagedCheckpoints.whole`, `QCore.Checkpoints.compose_eq_whole`).

The Rust boundary — that the recorder sees every read of the environment and
its clones on any thread, that the restricted digest covers each read name's
entries in every table and the scalars, and that every other input is in the
run identity or a fingerprint — is pinned by the tests of
`crates/qcore-schemas/src/contract_lookup_record.rs`,
`crates/qcore-c-gateway/src/lookup_record.rs` and
`crates/qcore-audit/src/pinned_route/checkpoints/lookup_tests.rs`, and argued
in `docs/reports/measure-lookup-record-reuse/README.md`. -/

-- One unit.
#print axioms QCore.LookupRestrictedReuse.run_eq_of_agree
#print axioms QCore.LookupRestrictedReuse.restrict_names
#print axioms QCore.LookupRestrictedReuse.agree_of_restrict_eq

-- A group.
#print axioms QCore.LookupRestrictedReuse.Consumer.determined
#print axioms QCore.LookupRestrictedReuse.recompute_eq
#print axioms QCore.LookupRestrictedReuse.progs_eq
#print axioms QCore.LookupRestrictedReuse.reused_eq_recomputed
#print axioms QCore.LookupRestrictedReuse.changed_key_recomputes
#print axioms QCore.LookupRestrictedReuse.whole_reader_recomputes

-- The staged run.
#print axioms QCore.LookupRestrictedReuse.stageResumed_eq_stage
#print axioms QCore.LookupRestrictedReuse.resumed_eq_whole
#print axioms QCore.LookupRestrictedReuse.resumed_publication_eq_whole
