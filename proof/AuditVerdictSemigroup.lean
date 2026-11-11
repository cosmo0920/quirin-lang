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

import QCore.VerdictSemigroup

/-! Card **W45-USER-CONTRACT-PLUGINS**: the four-value verdict
`Safe < Pending < Unknown < Unsafe`, composed by the join along every chain,
and the checker-plugin firewall (language spec §L21.2, contract spec §C11.1).

Six directions are audited.

**The algebra.** The join is associative, commutative and idempotent with
`Safe` as its identity; the fold of a chain is the join of its parts and does
not depend on their order; the reasoned verdict is associative with the empty
`Safe` as identity, commutative up to the order of its reasons, and keeps every
reason.

**Monotonicity.** Adding a link never lowers a chain's verdict, every link is
below the fold, and a chain is `Safe` exactly when every link is.

**Pending is fail-closed.** A `Pending` link, or any non-`Safe` link, keeps the
chain from `Safe`; a chain is `Pending` only when a link was held for a
human-resolvable reason, and `Unsafe` only when a link is a validated violation.

**Agreement with the historical roll-up.** Projected to three values (`Pending`
read as `Unknown`) the fold is the gateway's pre-card three-way decision, and on
inputs without `Pending` it is that decision exactly. A deferred-review verdict
resting only on declarations awaiting review is `Pending`.

**The plugin firewall.** A plugin's `Safe` is `Safe` only with an accepted
certificate and its `Unsafe` is `Unsafe` only with a validated witness; an
uncertified claim is `Pending` and only an unchecked claim is; a failure, a
refuted certificate or witness and an `Unknown` answer are `Unknown`; an
uncertified link keeps its chain from `Safe`.

The Rust boundary — that `qcore_schemas::verdict`, the gateway's
`legacy_verdict` and the host's `plugin_firewall` compute these functions — is
pinned by `crates/qcore-schemas/src/verdict.rs`'s tests,
`crates/qcore-c-gateway/tests/verdict_fold.rs` and
`crates/qcore-verifier-host/tests/plugin_firewall.rs`. -/

-- The algebra.
#print axioms QCore.VerdictSemigroup.join_rank
#print axioms QCore.VerdictSemigroup.join_assoc
#print axioms QCore.VerdictSemigroup.join_comm
#print axioms QCore.VerdictSemigroup.join_idem
#print axioms QCore.VerdictSemigroup.safe_join
#print axioms QCore.VerdictSemigroup.join_safe
#print axioms QCore.VerdictSemigroup.fold_append
#print axioms QCore.VerdictSemigroup.fold_append_comm
#print axioms QCore.VerdictSemigroup.Reasoned.join_assoc
#print axioms QCore.VerdictSemigroup.Reasoned.identity_join
#print axioms QCore.VerdictSemigroup.Reasoned.join_identity
#print axioms QCore.VerdictSemigroup.Reasoned.join_comm_value
#print axioms QCore.VerdictSemigroup.Reasoned.join_comm_reasons
#print axioms QCore.VerdictSemigroup.Reasoned.join_keeps_reasons

-- Monotonicity.
#print axioms QCore.VerdictSemigroup.le_join_left
#print axioms QCore.VerdictSemigroup.le_join_right
#print axioms QCore.VerdictSemigroup.adding_a_link_never_lowers
#print axioms QCore.VerdictSemigroup.every_link_is_below_the_fold
#print axioms QCore.VerdictSemigroup.fold_eq_safe_iff

-- Pending is fail-closed; Unsafe only from a validated violation.
#print axioms QCore.VerdictSemigroup.pending_never_folds_to_safe
#print axioms QCore.VerdictSemigroup.a_non_safe_link_keeps_the_chain_non_safe
#print axioms QCore.VerdictSemigroup.pending_only_from_a_human_resolvable_reason
#print axioms QCore.VerdictSemigroup.unsafe_only_from_a_violation

-- Agreement with the historical roll-up and the deferred-review model.
#print axioms QCore.VerdictSemigroup.fold_refines_the_historical_rollup
#print axioms QCore.VerdictSemigroup.fold_is_the_historical_rollup_without_pending
#print axioms QCore.VerdictSemigroup.a_verdict_resting_on_a_deferred_clause_is_pending

-- The plugin firewall.
#print axioms QCore.VerdictSemigroup.Outcome.all_is_complete
#print axioms QCore.VerdictSemigroup.a_plugin_is_safe_only_with_an_accepted_certificate
#print axioms QCore.VerdictSemigroup.a_plugin_is_unsafe_only_with_a_validated_witness
#print axioms QCore.VerdictSemigroup.an_uncertified_safe_claim_is_pending
#print axioms QCore.VerdictSemigroup.an_uncertified_unsafe_claim_is_pending
#print axioms QCore.VerdictSemigroup.a_plugin_failure_is_unknown
#print axioms QCore.VerdictSemigroup.a_plugin_is_pending_only_when_nothing_was_checked
#print axioms QCore.VerdictSemigroup.an_uncertified_plugin_link_keeps_the_chain_non_safe
