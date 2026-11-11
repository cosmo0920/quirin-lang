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

import QCore.SubcommandPlugins

/-! Card **W45-AUDIT-SUBCOMMAND-PLUGINS**: external `quirin-audit` subcommand
plugins (contract spec §C11.2). A row of the result is the join of the host's
verdict, the run's (`Unknown` for a failure) and the claims the checker-plugin
firewall of `QCore.VerdictSemigroup` classified.

Four directions are audited.

**No plugin output is `Safe` without an accepted certificate.** A row is `Safe`
exactly when the host's verdict is, the run completed and every claim carries
a certificate the audit's own checker accepted; any other plugin output leaves
it non-`Safe`. With no checker, as on the command line, only a `Safe` host row
the plugin claimed nothing about stays `Safe`, and an uncertified `Safe` or
`Unsafe` claim makes it `Pending`.

**A failed plugin is `Unknown`**, or the host's `Unsafe`.

**Monotonicity.** The join never lowers the host's verdict, an `Unsafe` stays
`Unsafe`, adding a claim never lowers a row, a higher host verdict never gives
a lower row, and the result is `Safe` only when every row is and the run
completed.

The Rust boundary — that `crates/qcore-audit/src/subcommand_plugins.rs`
classifies runs and claims as `Run` and `firewall` do — is pinned by
`crates/qcore-audit/tests/subcommand_plugins.rs` and the module's unit tests. -/

#print axioms QCore.SubcommandPlugins.join_eq_safe_iff

-- No plugin output is Safe without an accepted certificate.
#print axioms QCore.SubcommandPlugins.a_row_is_safe_only_with_accepted_certificates
#print axioms QCore.SubcommandPlugins.no_plugin_output_is_safe_without_an_accepted_certificate
#print axioms QCore.SubcommandPlugins.without_a_checker_only_an_unclaimed_safe_row_is_safe
#print axioms QCore.SubcommandPlugins.an_uncertified_claim_makes_a_safe_row_pending

-- A failed plugin is Unknown.
#print axioms QCore.SubcommandPlugins.a_failed_plugin_is_unknown

-- Monotonicity.
#print axioms QCore.SubcommandPlugins.the_join_never_lowers_the_host
#print axioms QCore.SubcommandPlugins.no_claim_lowers_an_existing_unsafe
#print axioms QCore.SubcommandPlugins.adding_a_claim_never_lowers
#print axioms QCore.SubcommandPlugins.a_row_is_monotone_in_the_host
#print axioms QCore.SubcommandPlugins.the_result_is_safe_only_when_every_row_is
