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

import QCore.OnigmoContractPlugin

/-! Card **ONIGMO-PROVIDER-PLUGIN**: the readings the Onigmo user crate's
declarations rest on, and its checker's composition (contract specification
C04.2, C04.5, C04.14, C11.1).

**`onig_new`'s slot.** Fresh exactly on the `0` result; NULL on every other;
so the unconditional nullable out states every exit.

**`onig_region_free`.** The declaration bounded by `free_self == 1` agrees
with the body there; every other actual is undeclared and never `Safe`; the
`0` arm is not a release.

**The `global` holder.** It admits only static storage or no object, and
whatever it admits every holder admits.

**The checker.** One failed check establishes nothing; an established binding
is `Pending`; nothing is published `Safe` or provisional `Unsafe`; a bound
caller is never `Safe`; `flb_regex_init` over `onig_init` is `Pending`. -/

-- onig_new's slot.
#print axioms QCore.OnigmoContractPlugin.fresh_iff_normal
#print axioms QCore.OnigmoContractPlugin.a_failure_leaves_null
#print axioms QCore.OnigmoContractPlugin.every_exit_is_a_nullable_out

-- onig_region_free's free_self split.
#print axioms QCore.OnigmoContractPlugin.the_declared_arm_agrees_with_the_body
#print axioms QCore.OnigmoContractPlugin.other_actuals_are_undeclared
#print axioms QCore.OnigmoContractPlugin.the_zero_arm_is_not_a_release
#print axioms QCore.OnigmoContractPlugin.an_undeclared_region_free_is_never_safe

-- The global holder.
#print axioms QCore.OnigmoContractPlugin.global_admits_only_static
#print axioms QCore.OnigmoContractPlugin.global_admission_holds_for_every_holder
#print axioms QCore.OnigmoContractPlugin.a_file_scope_address_is_refused_under_global

-- The checker and the firewall.
#print axioms QCore.OnigmoContractPlugin.one_failed_check_establishes_nothing
#print axioms QCore.OnigmoContractPlugin.never_published_safe
#print axioms QCore.OnigmoContractPlugin.never_published_unsafe
#print axioms QCore.OnigmoContractPlugin.published_is_pending_or_unknown
#print axioms QCore.OnigmoContractPlugin.an_established_binding_is_pending
#print axioms QCore.OnigmoContractPlugin.caller_never_below_host
#print axioms QCore.OnigmoContractPlugin.a_bound_caller_is_never_safe
#print axioms QCore.OnigmoContractPlugin.a_safe_host_over_one_established_binding_is_pending
#print axioms QCore.OnigmoContractPlugin.every_reason_is_kept
