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

import QCore.CrossUnitElementRelease

/-! Card **LERI-CROSS-UNIT-ELEMENTS-RELEASED**: a released-element summary
(gateway rule 43) carried to a caller in another unit (rules 4c and 40).

**Transport keeps the clause.** An import that passes the certificate's
whole-contract comparison states exactly the replayed home's clauses; one
that dropped or added a clause is refused; the home's gate holds of it.

**Every consumer applies the clause or refuses.** No consumer reads a summary
with a clause as a borrow: one that applies container effects forgets the
caller's published objects, one that cannot refuses. The import reads as the
home's own summary, so rule 43's caller-side soundness holds across units,
and a dropped clause would leave a released object trusted.

**No import, no reading; one crossing.** A refused home leaves the call
refused, an incomplete definition census imports nothing (the run keeps its
first-phase rows), and a caller carrying the clause on is never exported.

**The Rust vectors.** `outcome` agrees with `CROSS_UNIT_VECTORS`. -/

-- Transport.
#print axioms QCore.CrossUnitElementRelease.transport_keeps_the_clause
#print axioms QCore.CrossUnitElementRelease.a_changed_clause_is_refused
#print axioms QCore.CrossUnitElementRelease.the_gate_crosses_with_the_summary

-- Consumers.
#print axioms QCore.CrossUnitElementRelease.no_consumer_borrows_a_released_summary
#print axioms QCore.CrossUnitElementRelease.a_consumer_that_cannot_apply_refuses
#print axioms QCore.CrossUnitElementRelease.an_applying_consumer_forgets
#print axioms QCore.CrossUnitElementRelease.the_import_reads_as_the_home
#print axioms QCore.CrossUnitElementRelease.forgetting_holds_across_units
#print axioms QCore.CrossUnitElementRelease.a_dropped_clause_would_trust_a_released_object

-- No import; one crossing.
#print axioms QCore.CrossUnitElementRelease.a_missing_import_is_refused
#print axioms QCore.CrossUnitElementRelease.an_incomplete_census_imports_nothing
#print axioms QCore.CrossUnitElementRelease.a_carried_clause_crosses_no_second_unit

-- The Rust vectors.
#print axioms QCore.CrossUnitElementRelease.vectors_hold
