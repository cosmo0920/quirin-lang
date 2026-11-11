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

import QCore.ContractTipMode

/-! Card **CONTRACT-PIN-TIP-MODE**: tip identities of pinned contracts
(contract spec §C06.2). A pinned declaration's lock entry holds reviewed
identities or one tip identity (a repository and a ref); a tip entry is in
force exactly where the contract text is the pinned one and the sites read
from the ref's current commit are the audited ones.

Six directions are audited.

**A reviewed pin means what it meant**: no tip, the revision-mode decision and
no notice.

**In force exactly at the tip's sites**: an unresolved tip, or one whose sites
differ, is never in force and is a pin mismatch with a warning notice; at the
audited sites it is in force with an info notice.

**Held, never decided**: a verdict resting only on out-of-force entries is
`Unknown` under `contractPinMismatch`, neither `Safe` nor `Unsafe`.

**A tip decides as a reviewed pin at its sites**, so the tip path adds no
verdict of its own.

**The notice decides nothing**: info exactly in force, warning otherwise; the
release label and the notices themselves change no verdict.

**A named assumption and the decision table**: an in-force tip is a listed
assumption and a reviewed pin never is; `tipVectors` is `decision`, which is
the model.

The Rust boundary — tip resolution, blob reads and the sidecar — is pinned by
`crates/qcore-audit/tests/contract_tip_mode.rs`; `tipVectors` is bound to
`TIP_VECTORS` by `contract_tip::tests::tip_lean_vectors_match`. -/

-- A reviewed pin means what it meant.
#print axioms QCore.ContractTipMode.effective_mode
#print axioms QCore.ContractTipMode.a_reviewed_pin_means_what_it_meant

-- In force exactly at the tip's sites.
#print axioms QCore.ContractTipMode.a_tip_is_in_force_iff_its_sites_are_the_audited_ones
#print axioms QCore.ContractTipMode.an_unresolved_tip_is_never_in_force
#print axioms QCore.ContractTipMode.a_tip_whose_sites_differ_is_held
#print axioms QCore.ContractTipMode.a_tip_at_the_audited_sites_is_in_force

-- Held, never decided.
#print axioms QCore.ContractTipMode.a_verdict_resting_on_an_out_of_force_tip_is_unknown
#print axioms QCore.ContractTipMode.an_out_of_force_tip_dependency_is_neither_safe_nor_unsafe

-- A tip decides as a reviewed pin at its sites.
#print axioms QCore.ContractTipMode.a_tip_decides_as_a_reviewed_pin_at_its_sites

-- The notice decides nothing.
#print axioms QCore.ContractTipMode.the_notice_is_info_exactly_when_in_force
#print axioms QCore.ContractTipMode.the_release_label_decides_nothing
#print axioms QCore.ContractTipMode.the_notice_changes_no_verdict

-- A named assumption and the decision table.
#print axioms QCore.ContractTipMode.an_in_force_tip_is_a_listed_assumption
#print axioms QCore.ContractTipMode.a_reviewed_pin_is_never_a_tip_assumption
#print axioms QCore.ContractTipMode.tip_vectors_hold
#print axioms QCore.ContractTipMode.the_decision_is_the_model
