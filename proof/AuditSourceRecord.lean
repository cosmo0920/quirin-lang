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

import QCore.SourceRecord

-- Card `AGGREGATES-V1`: `source.record.v1`, the first source-level aggregate.

-- Type formation, and the distinctness it decides. A record declares at least
-- one field, no two fields share a name, and the list is one the model
-- enumerates — stated as one equivalence, so a widening has to change a theorem
-- rather than slip past one.
#print axioms QCore.SourceRecord.admitType_iff
#print axioms QCore.SourceRecord.a_record_declaring_no_field_is_refused
#print axioms QCore.SourceRecord.a_repeated_field_name_is_refused
#print axioms QCore.SourceRecord.a_record_of_distinct_fields_is_admitted
#print axioms QCore.SourceRecord.admitType_nodup

-- Construction totality, and the projection totality that follows from it: a
-- constructed value has a value for every field it can be asked for, so the
-- field-access rule is a function rather than a partial one with an error arm.
#print axioms QCore.SourceRecord.constructed_value_carries_every_declared_field
#print axioms QCore.SourceRecord.value_isSome_of_mem
#print axioms QCore.SourceRecord.projection_is_total_on_declared_fields
#print axioms QCore.SourceRecord.supplied_rebuild
#print axioms QCore.SourceRecord.constructed_fields_are_distinct
#print axioms QCore.SourceRecord.value_of_mem_nodup

-- Field access soundness — the headline — and CT09's "distinct ordinary struct
-- fields" as its corollary, with a worked positive control so neither is
-- vacuous.
#print axioms QCore.SourceRecord.projection_returns_what_construction_supplied
#print axioms QCore.SourceRecord.two_distinct_fields_read_their_own_values
#print axioms QCore.SourceRecord.a_two_field_record_constructs_and_projects

-- The refusals as one list: a construction is refused exactly when the type is
-- not admitted or the assignment is not complete, and completeness is exactly
-- the three conditions the Rust reader decides.
#print axioms QCore.SourceRecord.refused_constructions_are_exactly_these
#print axioms QCore.SourceRecord.complete_iff
#print axioms QCore.SourceRecord.a_missing_field_is_refused
#print axioms QCore.SourceRecord.an_undeclared_field_is_refused
#print axioms QCore.SourceRecord.a_repeated_field_is_refused

-- The field-custody decision, proved rather than asserted: the aggregate
-- contributes nothing at all to the inherited cycle-3 ledger, so it can open no
-- obligation and leak nothing. An owned field would make these false, which is
-- exactly why it is a separate card.
#print axioms QCore.SourceRecord.a_record_item_presents_no_event
#print axioms QCore.SourceRecord.a_field_read_presents_no_event
#print axioms QCore.SourceRecord.a_record_only_program_presents_no_event
#print axioms QCore.SourceRecord.a_record_only_program_leaves_the_ledger_unchanged

-- Neutrality, against a predicate written independently of the erasure, with a
-- theorem making the claim non-vacuous: a program that declares no record is
-- unchanged by this construct.
#print axioms QCore.SourceRecord.a_record_free_sequence_presents_itself
#print axioms QCore.SourceRecord.a_record_item_is_not_record_free

-- Effects: a record operation derives nothing, and the contrast that makes the
-- claim informative rather than a fact about the function.
#print axioms QCore.SourceRecord.a_construction_derives_nothing
#print axioms QCore.SourceRecord.a_field_read_derives_nothing
#print axioms QCore.SourceRecord.an_acquisition_beside_it_still_derives_alloc
