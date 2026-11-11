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

import QCore.TaskRecordCaptures

-- Card `W48-QR-TASK-RECORD-CAPTURES`: task workers receive and return records.
-- A record crosses spawn and join as its leaves; the transferability
-- judgment's aggregate kind is decided from them.

-- The aggregate transfer judgment is the conjunction of its leaves, and its
-- boundary is decided exactly once over every leaf list and over the union of
-- the flat and aggregate vocabularies; a `write` capture is still refused by
-- name, for every flat kind and every record.
#print axioms QCore.Transferability.firstFailing_none_iff
#print axioms QCore.Transferability.firstFailing_names_a_failing_leaf
#print axioms QCore.Transferability.admitsOrRefusesEachKind
#print axioms QCore.Transferability.theAggregateJudgmentIsTheConjunctionOfItsLeaves
#print axioms QCore.Transferability.everyAdmittedAggregateTransferIsBacked
#print axioms QCore.Transferability.aWriteCaptureOfARecordIsRefusedByName
#print axioms QCore.Transferability.noRecordIsSharedWhole
#print axioms QCore.Transferability.theAggregateJudgmentIsDecidedExactlyOnce
#print axioms QCore.Transferability.theAggregateBoundaryIsDecidedExactlyOnce
#print axioms QCore.Transferability.aRecordIntoAnotherRecordTypeIsRefused
#print axioms QCore.Transferability.theBoundaryIsDecidedExactlyOnceOverRecords
#print axioms QCore.Transferability.aWriteCaptureIsRefusedByNameOverRecords
#print axioms QCore.Transferability.theTaskRoutesRecordsCrossAsTheirLeavesDecide
-- The flat theorems the aggregate ones extend, unchanged and still audited.
#print axioms QCore.Transferability.theBoundaryIsDecidedExactlyOnce
#print axioms QCore.Transferability.aWriteCaptureIsRefusedByName

-- The task route's records: a record crosses exactly when every leaf may.
#print axioms QCore.TaskRecordCaptures.a_record_crosses_exactly_when_every_leaf_may_cross

-- Custody is conserved across spawn and join for records; the scalar leaves
-- move no custody; a collected record is unpacked with every carried identity
-- in exactly one owned leaf.
#print axioms QCore.TaskRecordCaptures.transfer_then_transfer
#print axioms QCore.TaskRecordCaptures.custody_is_conserved_across_spawn_and_join_for_records
#print axioms QCore.TaskRecordCaptures.a_scalar_leaf_moves_no_custody
#print axioms QCore.TaskRecordCaptures.unpacking_places_every_carried_identity_once
#print axioms QCore.TaskRecordCaptures.unpacking_a_pair_needs_exactly_its_one_identity

-- Spawn failure restores the record exactly; while the reservation stands no
-- owned leaf is the parent's.
#print axioms QCore.TaskRecordCaptures.spawn_failure_restores_the_record_exactly
#print axioms QCore.TaskRecordCaptures.a_reserved_record_leaf_is_unavailable_to_its_parent

-- No byte has two write authorities with per-leaf lends, and two fields leased
-- at once is a reachable state.
#print axioms QCore.TaskRecordCaptures.every_leaf_is_reachable
#print axioms QCore.TaskRecordCaptures.no_byte_has_two_write_authorities_per_leaf
#print axioms QCore.TaskRecordCaptures.two_fields_leased_at_once_is_reachable

-- A refill is admitted exactly into a hole, on a leaf slot.
#print axioms QCore.TaskRecordCaptures.a_leaf_refill_is_admitted_exactly_into_a_hole
