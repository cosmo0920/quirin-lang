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

import QCore.RefusedTestAccusation

/-! Card **SUP07-REFUSED-TEST-ACCUSATION**: a leak claim does not rest on a
refused test (gateway rule 12e).

Five directions are audited.

**The executions.** Under joined provenance's tie, no execution on a test's
null edge owes the tied site; without the tie one may, so the refused test
alone establishes nothing. `mk_http_error_page`'s executions are tied and owe
nothing there.

**The decision.** A site is settled exactly on a refused test's null edge,
targeted and tied; a decided test, the other edge, an untied and an
untargeted site are kept. The Rust table is `edgeVectors`
(`edge_vectors_hold`).

**Claims follow the executions.** No claim on a settled edge, matching the
executions; the pre-card claim was made while nothing was owed.

**Established accusations stay.** A claim established on the other edge
survives the join; an unresolved-effect marker would have withdrawn it.

**Never `Safe`.** The refused test's `Unknown` leaf keeps every function
carrying it off `Safe`; the control is `Unknown`, a leak beside it `Unsafe`.

Faithful provenance, targets and edges in the Rust, and the correspondence
with it, are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/legacy_refused_test.rs` and
`crates/qcore-c-gateway/tests/sup07_refused_test_accusation.rs`. -/

-- The executions.
#print axioms QCore.RefusedTestAccusation.tied_null_edge_owes_nothing
#print axioms QCore.RefusedTestAccusation.without_the_tie_a_null_edge_may_owe
#print axioms QCore.RefusedTestAccusation.the_error_page_is_tied
#print axioms QCore.RefusedTestAccusation.the_error_page_null_edge_owes_nothing

-- The decision.
#print axioms QCore.RefusedTestAccusation.settle_iff
#print axioms QCore.RefusedTestAccusation.a_decided_test_is_untouched
#print axioms QCore.RefusedTestAccusation.the_non_null_edge_is_untouched
#print axioms QCore.RefusedTestAccusation.an_untied_site_is_untouched
#print axioms QCore.RefusedTestAccusation.an_untargeted_site_is_untouched
#print axioms QCore.RefusedTestAccusation.edge_vectors_hold

-- Claims follow the executions.
#print axioms QCore.RefusedTestAccusation.the_rule_matches_the_executions
#print axioms QCore.RefusedTestAccusation.the_old_claim_was_false

-- Established accusations stay.
#print axioms QCore.RefusedTestAccusation.an_established_claim_survives_the_join
#print axioms QCore.RefusedTestAccusation.a_marker_would_withdraw_it

-- Never Safe.
#print axioms QCore.RefusedTestAccusation.a_refused_test_is_never_safe
#print axioms QCore.RefusedTestAccusation.the_shape_is_unknown
#print axioms QCore.RefusedTestAccusation.the_old_shape_was_unsafe
#print axioms QCore.RefusedTestAccusation.an_established_leak_beside_it_stays_unsafe
