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

import QCore.OpensslContractPlugin

/-! Card **W48-OPENSSL-CONTRACT-PLUGIN** (standing constraint 52): the
`quirin-audit openssl` plugin checks OpenSSL's contracts against the pinned
release and answers per binding through the host's plugin firewall.

Three directions are audited.

**The proposal.** `Safe` exactly when every obligation (release pin,
declaration, rule for the class, definition, class rule) is discharged;
never `Unsafe`; a pin mismatch, an unruled class or any undischarged
obligation is `Unknown`.

**The published row.** Never `Safe`; `Pending` exactly when the run completed
and every obligation is discharged; `Unknown` otherwise; a host verdict is
never lowered.

**The class rules' premises.** An admitted node has no effect, so a pure
reader's body has none; the member-store escapes are exactly the stored
pointer parameters, so an agreeing contract omits none; a no-pointer
interface reaches no caller object.

The Rust boundary — Clang's AST read into these nodes, the obligations as
`check_binding` lists them — is pinned by the crate's tests. -/

-- The proposal.
#print axioms QCore.OpensslContractPlugin.obligations_discharged_iff
#print axioms QCore.OpensslContractPlugin.a_binding_is_proposed_safe_iff_every_obligation_is_discharged
#print axioms QCore.OpensslContractPlugin.the_plugin_never_proposes_unsafe
#print axioms QCore.OpensslContractPlugin.a_pin_mismatch_is_unknown
#print axioms QCore.OpensslContractPlugin.an_unruled_class_is_unknown
#print axioms QCore.OpensslContractPlugin.an_undischarged_obligation_is_unknown

-- The published row.
#print axioms QCore.OpensslContractPlugin.propose_uncheckable
#print axioms QCore.OpensslContractPlugin.a_binding_row_is_never_safe
#print axioms QCore.OpensslContractPlugin.a_binding_row_is_pending_iff_discharged
#print axioms QCore.OpensslContractPlugin.a_binding_row_is_unknown_otherwise
#print axioms QCore.OpensslContractPlugin.a_binding_row_never_lowers_the_host

-- The class rules' premises.
#print axioms QCore.OpensslContractPlugin.an_admitted_node_has_no_effect
#print axioms QCore.OpensslContractPlugin.a_pure_reader_body_has_no_effect
#print axioms QCore.OpensslContractPlugin.an_effectful_node_is_refused
#print axioms QCore.OpensslContractPlugin.escapes_are_the_stored_pointers
#print axioms QCore.OpensslContractPlugin.a_stored_pointer_is_a_stated_escape
#print axioms QCore.OpensslContractPlugin.a_no_pointer_interface_reaches_nothing
