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

import QCore.RetentionNullConstant

/-! Card **RETENTION-NULL-CONSTANT-ACTUAL** (carried by
OPENSSL-UNSTATED-CALLBACK-SLOTS; gateway rule 12t, amended in place): a null
pointer constant at a retained position retains nothing.

The null reading decides outright with no record and no refusal in every
holder class; every other actual is decided exactly as `QCore.RetentionPath`
decides it, so the refusals of a non-static actual in an unknown or `global`
holder stay; a null retention is the identity on every path's records and
never changes a replay. The Rust boundary is pinned by `retained_by.rs`'s
`NULL_DECISION_VECTORS` / `NULL_SPELLING_VECTORS` and
`crates/qcore-c-gateway/tests/retention_null_constant.rs`. -/

#print axioms QCore.RetentionNullConstant.a_null_constant_retains_nothing
#print axioms QCore.RetentionNullConstant.a_null_constant_never_records
#print axioms QCore.RetentionNullConstant.a_null_constant_never_refuses
#print axioms QCore.RetentionNullConstant.every_other_actual_is_decided_as_before
#print axioms QCore.RetentionNullConstant.a_non_static_actual_in_an_unknown_holder_still_refuses
#print axioms QCore.RetentionNullConstant.an_unreadable_actual_still_refuses
#print axioms QCore.RetentionNullConstant.a_null_retention_is_the_identity_on_records
#print axioms QCore.RetentionNullConstant.a_null_retention_never_changes_a_replay
#print axioms QCore.RetentionNullConstant.null_decision_vectors_hold
#print axioms QCore.RetentionNullConstant.the_path_vectors_hold_through_the_reading
