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

import QCore.OpensslCalleeShapes

/-! Card **OPENSSL-PROVIDER-REMAINING-CALLEES**: the OpenSSL checker's
documented shapes and the dependency fold.

**The documented shapes.** The checker accepts exactly one clause set per
documented function; a counted object's release or constructor never states a
final release, and its returned reference carries the reference kind; a
borrowed result owns and releases nothing; a retention keeps its position
borrowed, states the retention and never releases it; a plain call states no
custody; every documented premise is in every accepted declaration.

**The dependency fold.** A function's published status never lowers its own,
is never `Safe` while it rests on a `Pending` binding, and folding every
dependency covers the `ContractCall` callees the earlier fold read.

The Rust boundary is pinned by `quirin-provider-openssl`'s checker tests and
`crates/qcore-audit/tests/user_contract_plugins.rs`. -/

-- The documented shapes.
#print axioms QCore.OpensslCalleeShapes.accepted_declarations_agree
#print axioms QCore.OpensslCalleeShapes.mem_expected
#print axioms QCore.OpensslCalleeShapes.premise_never_custody
#print axioms QCore.OpensslCalleeShapes.counted_never_released
#print axioms QCore.OpensslCalleeShapes.returned_reference_is_counted
#print axioms QCore.OpensslCalleeShapes.borrowed_result_owns_nothing
#print axioms QCore.OpensslCalleeShapes.retention_keeps_item_borrowed
#print axioms QCore.OpensslCalleeShapes.plain_takes_no_custody
#print axioms QCore.OpensslCalleeShapes.premise_is_stated

-- The dependency fold.
#print axioms QCore.OpensslCalleeShapes.published_never_lowers
#print axioms QCore.OpensslCalleeShapes.resting_on_pending_is_never_safe
#print axioms QCore.OpensslCalleeShapes.fold_le_of_subset
#print axioms QCore.OpensslCalleeShapes.dependencies_cover_applied
