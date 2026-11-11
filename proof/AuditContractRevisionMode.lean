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

import QCore.ContractRevisionMode

/-! Card **W45-CONTRACT-REVISION-MODE**: revision modes of declared contracts
(contract spec §C06.2). Pinned is the default; a pinned contract is in force
only where the audited source matches its lock, and every mismatch is withheld
under `contract-pin-mismatch`; a flexible contract binds by name and signature
and is checked against any body the run analyses.

Six directions are audited.

**The default is pinned**, and one pinned clause pins the whole contract; a
flexible contract-level marker with no pinned clause is flexible.

**A mismatched pin is not in force.** It applies nothing, discharges nothing
and accuses nothing, so a verdict that rests on it is neither `Safe` nor
`Unsafe`, and its reason never supports `Unsafe`.

**Several reviewed identities.** A pinned contract is in force exactly when its
text is the pinned one and some reviewed identity is the audited one; adding an
identity disturbs none; a contract edit voids every identity.

**Re-pinning restores verdicts exactly.** Re-pinned declarations give the
verdict of the in-force environment, which is the verdict matching pins gave.

**Flexible re-verification.** A flexible contract is in force at every source
identity; an analysed body is checked against exactly the declared clauses at
every identity, as a matching pinned contract checks it; a trusted flexible
contract is always a listed assumption.

**The unit reason.** A summary naming a pin-held call is `contractPinMismatch`,
never a source statement; with no held call it is unchanged.

The Rust boundary — the loader's parse, the pin evaluation and the gateway's
rewrite — is pinned by `crates/qcore-contract/tests/contract_revision_mode.rs`,
`crates/qcore-c-gateway/tests/contract_pin_hold.rs` and
`crates/qcore-audit/tests/contract_revision_audit.rs`. -/

-- The default is pinned.
#print axioms QCore.ContractRevisionMode.the_default_is_pinned
#print axioms QCore.ContractRevisionMode.one_pinned_clause_pins_the_contract
#print axioms QCore.ContractRevisionMode.a_flexible_contract_without_pinned_clauses_is_flexible

-- A mismatched pin is not in force, and its dependants are neither Safe nor Unsafe.
#print axioms QCore.ContractRevisionMode.a_mismatched_pin_is_not_in_force
#print axioms QCore.ContractRevisionMode.a_mismatched_pin_contributes_nothing
#print axioms QCore.ContractRevisionMode.applied_is_all_or_nothing
#print axioms QCore.ContractRevisionMode.Declaration.contributes_only_what_it_touches
#print axioms QCore.ContractRevisionMode.a_verdict_resting_on_a_mismatched_pin_is_unknown
#print axioms QCore.ContractRevisionMode.a_mismatched_dependency_is_neither_safe_nor_unsafe
#print axioms QCore.ContractRevisionMode.the_pin_reason_never_supports_unsafe

-- Several reviewed identities: in force iff the text and some identity match.
#print axioms QCore.ContractRevisionMode.a_pin_is_in_force_iff_its_text_and_some_identity_match
#print axioms QCore.ContractRevisionMode.a_contract_edit_voids_every_identity
#print axioms QCore.ContractRevisionMode.adding_an_identity_keeps_every_reviewed_one
#print axioms QCore.ContractRevisionMode.pinned_at_two_revisions_holds_at_both_and_nowhere_else

-- Re-pinning restores verdicts exactly.
#print axioms QCore.ContractRevisionMode.Declaration.repin_in_force
#print axioms QCore.ContractRevisionMode.repinning_restores_the_verdict_in_force
#print axioms QCore.ContractRevisionMode.matching_pins_give_the_verdict_in_force
#print axioms QCore.ContractRevisionMode.removing_a_mismatch_restores_verdicts_exactly

-- Flexible re-verification: a checked body is still checked.
#print axioms QCore.ContractRevisionMode.a_flexible_contract_is_in_force_at_every_identity
#print axioms QCore.ContractRevisionMode.a_checked_body_is_still_checked
#print axioms QCore.ContractRevisionMode.flexible_and_matching_pinned_check_alike
#print axioms QCore.ContractRevisionMode.a_trusted_flexible_contract_is_a_listed_assumption

-- The unit reason names the pin.
#print axioms QCore.ContractRevisionMode.a_pin_held_summary_is_never_a_source_statement
#print axioms QCore.ContractRevisionMode.a_held_summary_says_nothing_about_the_source
#print axioms QCore.ContractRevisionMode.a_summary_without_held_calls_is_unchanged
