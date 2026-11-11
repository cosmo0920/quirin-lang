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

import QCore.CheckedSpecification

/-! Card **W45-FB-BACKLOG-CUSTODY**: checked specifications (contract spec
§C06.3, `crates/qcore-c-gateway/src/legacy_checked_spec.rs`).

Three directions are audited.

**Soundness of the descent.** Everything left in force is established under
exactly what is left in force, settling only withdraws, a specification with
no body (or in a recursive component) is never in force, and the descent
reaches a fixed point within the round bound the Rust uses.

**Never trusted.** A defining function's row is `Safe` only when its
specification is in force, and so only when its body established it.

**Non-vacuity.** A failing specification is withdrawn; an established one
stays.

That the check is the W4 gate over the unit's own body, and that call sites
apply exactly the settled environment, are Rust boundaries pinned by
`crates/qcore-c-gateway/tests/intrusive_list_iteration.rs`. -/

-- Soundness of the descent.
#print axioms QCore.CheckedSpecification.settle_sound
#print axioms QCore.CheckedSpecification.settle_subset
#print axioms QCore.CheckedSpecification.never_without_a_body
#print axioms QCore.CheckedSpecification.settle_is_a_fixed_point_within_its_bound

-- Never trusted.
#print axioms QCore.CheckedSpecification.a_row_is_safe_only_if_established

-- Non-vacuity.
#print axioms QCore.CheckedSpecification.a_failing_specification_is_withdrawn
#print axioms QCore.CheckedSpecification.an_established_specification_stays
