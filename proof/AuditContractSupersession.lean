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

import QCore.ContractSupersession

/-! Card **CONTRACT-SUPERSESSION**: contract bundles compose by supersession
(contract spec §C06.6; `docs/plans/pluggable-cards/README.md` seam G, user
decisions 9 and 9a). Model of `crates/qcore-schemas/src/contract_supersession.rs`
(`decide`) and of the retention in `crates/qcore-contract/src/contract_supersession.rs`.

Five directions are audited.

**Determinism.** The decision reads only the set of candidates: order and
multiplicity change nothing, so it is associative and commutative over the
union of bundles, and a byte-identical duplicate changes nothing.

**Newest wins.** A candidate whose stamp no candidate exceeds, every equal
stamp agreeing on its text, is in force; in particular a strictly newest one.

**A tie is a conflict.** Two newest candidates with different texts, or any
candidate without a usable stamp (an unpinned declaration never supersedes),
withhold the function; no candidate is no text.

**Retention.** A text in force is a newest candidate's; every candidate is in
force or retained in the record, which holds only candidates.

**Never optimistic.** A conflict's held call is `Unknown`, so the row it joins
is never `Safe` or `Pending` and is never lowered. The shared vectors are
checked against the Rust `decide` by `crates/qcore-schemas/tests/contract_supersession.rs`. -/

-- The greatest stamp reads only the set.
#print axioms QCore.ContractSupersession.le_top
#print axioms QCore.ContractSupersession.top_attained
#print axioms QCore.ContractSupersession.top_congr
#print axioms QCore.ContractSupersession.mem_newest
#print axioms QCore.ContractSupersession.allStamped_iff
#print axioms QCore.ContractSupersession.agreed_iff
#print axioms QCore.ContractSupersession.newest_congr
#print axioms QCore.ContractSupersession.allStamped_congr
#print axioms QCore.ContractSupersession.agreed_congr
#print axioms QCore.ContractSupersession.newest_nonempty

-- Determinism.
#print axioms QCore.ContractSupersession.decide_congr
#print axioms QCore.ContractSupersession.decide_union_comm
#print axioms QCore.ContractSupersession.decide_union_assoc
#print axioms QCore.ContractSupersession.decide_duplicate

-- Newest wins; a tie is a conflict.
#print axioms QCore.ContractSupersession.newest_wins
#print axioms QCore.ContractSupersession.strictly_newest_wins
#print axioms QCore.ContractSupersession.a_tie_is_a_conflict
#print axioms QCore.ContractSupersession.an_unusable_stamp_is_a_conflict
#print axioms QCore.ContractSupersession.nothing_is_a_conflict

-- Retention.
#print axioms QCore.ContractSupersession.in_force_is_a_newest_candidate
#print axioms QCore.ContractSupersession.nothing_is_erased
#print axioms QCore.ContractSupersession.the_record_holds_only_candidates

-- Never optimistic.
#print axioms QCore.ContractSupersession.a_conflict_is_never_safe
#print axioms QCore.ContractSupersession.a_conflict_is_never_pending
#print axioms QCore.ContractSupersession.a_conflict_never_lowers
#print axioms QCore.ContractSupersession.the_held_verdict_of_a_conflict

-- Shared vectors.
#print axioms QCore.ContractSupersession.vectors_hold
