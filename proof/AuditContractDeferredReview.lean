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

import QCore.ContractDeferredReview

/-! Card **W45-CONTRACT-DEFERRED-REVIEW**: a declared contract, or one of its
clauses, marked as awaiting human review (`resource_review_pending`, contract
spec §C06). The loader validates and withholds it; every call site that would
apply it stays `Unknown` under `awaiting-human-review`.

Five directions are audited.

**Withholding is total.** A clause-level marker withholds the declaration as a
contract-level one does, and the loader applies all of a declaration's clauses
or none, so no deferred clause is dropped while the rest decide.

**No discharge, no accusation.** A pending declaration contributes nothing, so a
verdict that rests on a deferred clause is neither `Safe` nor `Unsafe`, and its
reason is an analyser-subject category that never supports `Unsafe`.

**Monotonicity.** Resolving reviews only adds contributions, and cannot move a
`Safe` verdict on an obligation the resolved declarations do not touch.

**Inheritance.** An inherited declaration carries its parent's markers clause by
clause, through any chain, so a child of a pending parent is withheld too.

**The unit reason.** A projection summary naming a held call classifies as
`awaiting-human-review` and never as a source statement, and a summary with no
held call classifies exactly as before the card.

The Rust boundary — that the gateway's token rewrite reaches every call site and
that the loader's parse matches this model — is pinned by
`crates/qcore-contract/tests/contract_deferred_review.rs` and
`crates/qcore-c-gateway/tests/contract_deferred_review.rs`. -/

-- Withholding is total over the declaration.
#print axioms QCore.ContractDeferredReview.a_deferred_clause_withholds_the_declaration
#print axioms QCore.ContractDeferredReview.applied_is_all_or_nothing
#print axioms QCore.ContractDeferredReview.a_pending_declaration_contributes_nothing

-- A verdict that depends on a deferred clause is neither Safe nor Unsafe.
#print axioms QCore.ContractDeferredReview.a_verdict_resting_on_a_deferred_clause_is_unknown
#print axioms QCore.ContractDeferredReview.a_deferred_dependency_is_neither_safe_nor_unsafe
#print axioms QCore.ContractDeferredReview.the_review_reason_never_supports_unsafe

-- Monotonicity of review.
#print axioms QCore.ContractDeferredReview.Declaration.resolve_not_pending
#print axioms QCore.ContractDeferredReview.Declaration.resolve_touches
#print axioms QCore.ContractDeferredReview.Declaration.untouched_contributes_nothing
#print axioms QCore.ContractDeferredReview.resolving_only_adds_contributions
#print axioms QCore.ContractDeferredReview.resolving_reviews_keeps_unrelated_safe

-- Inheritance propagates a deferral.
#print axioms QCore.ContractDeferredReview.inheritance_propagates_pending
#print axioms QCore.ContractDeferredReview.inheritance_propagates_each_clause
#print axioms QCore.ContractDeferredReview.an_inherited_deferral_withholds_the_child
#print axioms QCore.ContractDeferredReview.inheritance_chains_propagate_pending

-- The unit reason names the review first.
#print axioms QCore.ContractDeferredReview.a_held_summary_is_never_a_source_statement
#print axioms QCore.ContractDeferredReview.a_summary_without_held_calls_is_unchanged
