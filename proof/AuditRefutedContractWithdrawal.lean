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

import QCore.RefutedContractWithdrawal

/-! Card **W45-REFUTED-CONTRACT-WITHDRAWAL**: a declared contract its own
analysed body refutes serves no caller (contract spec §C06.4,
`crates/qcore-c-gateway/src/legacy_refuted_contract.rs`,
`crates/qcore-audit/src/refuted_contracts.rs`).

Four directions are audited.

**The held value.** A held call is `Unknown`, never `Pending`; with a pin
mismatch (`Pending`) the join is `Unknown`.

**Never resting on a refuted specification.** A caller with a call to a
refuted callee is neither `Safe` nor `Pending`, and its verdict does not depend
on what the refuted declaration claims; an unrefuted or external declaration
is applied as declared.

**Monotone withdrawal.** Withdrawing more never moves a caller into `Safe` or
`Pending`.

**Two phases compose.** The union of group tables is the whole table, a
grouped second phase equals the whole run, and the table is independent of
unit order under the membership premise.

That the gateway records exactly the refutations it finds and holds exactly
the table's call sites outside their own bodies are Rust boundaries pinned by
`crates/qcore-c-gateway/tests/refuted_contract_withdrawal.rs`. -/

-- The held value.
#print axioms QCore.RefutedContractWithdrawal.a_held_call_is_unknown
#print axioms QCore.RefutedContractWithdrawal.a_held_call_is_never_pending
#print axioms QCore.RefutedContractWithdrawal.pinned_pending_refuted_unknown_join_unknown

-- Never resting on a refuted specification.
#print axioms QCore.RefutedContractWithdrawal.never_rests_on_a_refuted_specification
#print axioms QCore.RefutedContractWithdrawal.the_verdict_ignores_what_a_refuted_contract_claims
#print axioms QCore.RefutedContractWithdrawal.an_unrefuted_contract_still_serves
#print axioms QCore.RefutedContractWithdrawal.an_external_contract_is_never_in_the_table

-- Monotone withdrawal.
#print axioms QCore.RefutedContractWithdrawal.item_value_under_more
#print axioms QCore.RefutedContractWithdrawal.withdrawal_never_certifies

-- Two phases compose.
#print axioms QCore.RefutedContractWithdrawal.the_union_of_group_tables_is_the_whole_table
#print axioms QCore.RefutedContractWithdrawal.two_phases_over_groups_equal_the_whole_run
#print axioms QCore.RefutedContractWithdrawal.mem_table_iff
#print axioms QCore.RefutedContractWithdrawal.the_table_is_independent_of_unit_order

-- Non-vacuity.
#print axioms QCore.RefutedContractWithdrawal.a_refuted_caller_moves_from_safe_to_unknown
#print axioms QCore.RefutedContractWithdrawal.an_independent_violation_stays
