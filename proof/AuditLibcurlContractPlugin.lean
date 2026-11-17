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

import QCore.LibcurlContractPlugin

/-! Card **W48-LIBCURL-CONTRACT-PLUGIN**: the plugin-specific premises of the
libcurl contract plugin (contract specification §C11.1, §C11.2).

**The checker's decision.** A binding is established exactly when the pin
matches, it carries a contract and every obligation holds; a pin mismatch, an
uncontracted binding or one failed obligation establishes nothing.

**What the audit publishes.** On the subcommand route and the checker-package
route alike, an established binding is `Pending`, an unestablished one and a
failed run `Unknown`; nothing is published `Safe` or provisional `Unsafe`.

**Composition.** A caller is never below its host value, an `Unsafe` host
stays `Unsafe`, a caller that applies a bound contract is never `Safe`, an
unbound caller (a typecheck stub's) keeps the host value, a `Safe` host over
established bindings is `Pending`, and the reasoned join keeps every reason.

The Rust boundary is pinned by `crates/quirin-audit-plugin-libcurl`'s tests. -/

-- The checker's decision.
#print axioms QCore.LibcurlContractPlugin.established_iff
#print axioms QCore.LibcurlContractPlugin.a_pin_mismatch_establishes_nothing
#print axioms QCore.LibcurlContractPlugin.an_uncontracted_binding_is_not_established
#print axioms QCore.LibcurlContractPlugin.a_failed_obligation_establishes_nothing

-- What the audit publishes.
#print axioms QCore.LibcurlContractPlugin.the_routes_agree
#print axioms QCore.LibcurlContractPlugin.never_published_safe
#print axioms QCore.LibcurlContractPlugin.never_published_unsafe
#print axioms QCore.LibcurlContractPlugin.an_established_binding_is_pending
#print axioms QCore.LibcurlContractPlugin.an_unestablished_binding_is_unknown
#print axioms QCore.LibcurlContractPlugin.a_failed_run_is_unknown
#print axioms QCore.LibcurlContractPlugin.a_pin_mismatch_is_unknown
#print axioms QCore.LibcurlContractPlugin.published_is_pending_or_unknown

-- Composition.
#print axioms QCore.LibcurlContractPlugin.caller_never_below_host
#print axioms QCore.LibcurlContractPlugin.an_unsafe_host_stays_unsafe
#print axioms QCore.LibcurlContractPlugin.a_bound_caller_is_never_safe
#print axioms QCore.LibcurlContractPlugin.an_unbound_caller_keeps_the_host_value
#print axioms QCore.LibcurlContractPlugin.a_safe_host_over_established_bindings_is_pending
#print axioms QCore.LibcurlContractPlugin.every_reason_is_kept
