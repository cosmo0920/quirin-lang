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

import QCore.QuantRegionSource

-- Card `QUANT-REGION-SOURCE`: `source.region.v1`, the first source path to the
-- kernel's quantitative cleanup-pack opening.

-- The pack a source region seals is the frame its body built: the duties in
-- acquisition order and the region's own quantity.
#print axioms QCore.QuantRegionSource.sealed_pack_carries_the_bodys_duties
#print axioms QCore.QuantRegionSource.sealed_pack_carries_the_regions_quantity

-- Pairing soundness. A handler is admitted exactly when it consumes the
-- capability once; zero is the kernel's `CleanupPackNotDischarged` (`Q305`) and
-- two its `CleanupPackDuplicated` (`Q304`).
#print axioms QCore.QuantRegionSource.handler_admits_iff_one_discharge
#print axioms QCore.QuantRegionSource.a_handler_that_never_discharges_is_refused
#print axioms QCore.QuantRegionSource.a_handler_that_discharges_twice_is_refused
#print axioms QCore.QuantRegionSource.one_discharge_is_admitted

-- Quantity totality. The remainder is computed by a total function; an
-- overspend is an explicit refusal rather than a wrapped subtraction.
#print axioms QCore.QuantRegionSource.remainder_is_total_on_admitted_bodies
#print axioms QCore.QuantRegionSource.overspending_refuses
#print axioms QCore.QuantRegionSource.spending_the_whole_credit_leaves_nothing

-- Faithfulness of the erasure: the releases the emitted straight-line program
-- performs **are** the cleanup trace the kernel's own `discharge` produces for
-- the pack this region sealed, and the emitted program is the hand-written flat
-- twin statement for statement.
#print axioms QCore.QuantRegionSource.emit_releases_are_the_packs_duties
#print axioms QCore.QuantRegionSource.an_admitted_region_emits_its_flat_twin
#print axioms QCore.QuantRegionSource.two_duties_are_released_last_first
#print axioms QCore.QuantRegionSource.an_explicit_release_removes_its_duty

-- A credit is not a resource, which is why a discharge may abandon one and may
-- not discard the other.
#print axioms QCore.QuantRegionSource.credit_contributes_no_duty
#print axioms QCore.QuantRegionSource.effort_contributes_no_duty
#print axioms QCore.QuantRegionSource.a_release_erases_an_earlier_duty

-- The admitted shapes as one list each, so a widening must change a theorem
-- rather than slip past one.
#print axioms QCore.QuantRegionSource.admitted_body_shapes_are_exactly_these
#print axioms QCore.QuantRegionSource.admitted_handler_shapes_are_exactly_these
