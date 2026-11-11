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

import QCore.RetentionGuard

/-! Card **W43-ABI-ENUM-AND-CONST-PP** (gateway rule 12w): guarded retention
clauses, `resource_retained_by(P, H) when X OP K` (contract specification
§C04.14.1).

Audited directions. **Guard semantics**: only a decided value in range that
fails takes a clause out of force; an undecided actual keeps it. **Cautious
join**: every reading lies within the cautious one, and an undecided call
reads exactly the cautious union. **Never favourable**: for a verdict more
retention never improves, every reading is bounded by the cautious one, and
an undecided guard is `Safe` only where the cautious reading is. **Partition**:
an admitted guard has a satisfying and a refuting value in its range. **Body
check**: a validated body keeps nothing when the guard fails. **Controls**:
the Rust unit tests' vectors. -/

-- Guard semantics.
#print axioms QCore.RetentionGuard.only_a_decided_failure_is_out_of_force
#print axioms QCore.RetentionGuard.a_failure_is_a_decided_value_in_range_that_fails
#print axioms QCore.RetentionGuard.an_undecided_actual_keeps_the_clause

-- The cautious join, and never a favourable undecided reading.
#print axioms QCore.RetentionGuard.every_reading_is_within_the_cautious_one
#print axioms QCore.RetentionGuard.an_undecided_call_reads_the_cautious_union
#print axioms QCore.RetentionGuard.the_cautious_reading_bounds_every_reading
#print axioms QCore.RetentionGuard.an_undecided_guard_is_safe_only_where_the_cautious_reading_is

-- Partition and the body check.
#print axioms QCore.RetentionGuard.an_admitted_guard_partitions_its_range
#print axioms QCore.RetentionGuard.a_validated_body_keeps_nothing_when_the_guard_fails

-- Controls.
#print axioms QCore.RetentionGuard.decision_controls
#print axioms QCore.RetentionGuard.partition_controls
