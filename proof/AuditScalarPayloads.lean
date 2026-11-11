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

import QCore.ScalarPayloads

-- Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-PAYLOADS`): typed Buffer and
-- protected payloads, whose byte extent is the payload type's size.

-- The extent: each type's size, four for `i32`, none for a target width
-- without the record, and one to eight bytes otherwise.
#print axioms QCore.ScalarPayloads.size_is_bits_over_eight
#print axioms QCore.ScalarPayloads.an_i32_payload_is_four_bytes
#print axioms QCore.ScalarPayloads.a_target_width_payload_needs_the_target
#print axioms QCore.ScalarPayloads.sizes_are_one_to_eight

-- Why an integer payload's byte view is total and a `bool`'s is not, and what
-- a `protected bool` write stores.
#print axioms QCore.ScalarPayloads.every_integer_pattern_is_a_value
#print axioms QCore.ScalarPayloads.a_bool_byte_is_mostly_not_a_bool
#print axioms QCore.ScalarPayloads.a_bool_fill_is_a_bool

-- `WRITE-LEND-TASKS` over typed extents: the typed ledger is a sub-relation of
-- the untyped one, so no byte has two write authorities; every held byte is a
-- payload byte; a fitting worker footprint touches only payload bytes; and at
-- `i32` the admission is the four-byte rule it always was.
#print axioms QCore.ScalarPayloads.a_typed_open_is_an_open
#print axioms QCore.ScalarPayloads.typed_reachable_is_reachable
#print axioms QCore.ScalarPayloads.no_byte_has_two_write_authorities_typed
#print axioms QCore.ScalarPayloads.every_lease_ends_inside_the_payload
#print axioms QCore.ScalarPayloads.every_held_byte_is_a_payload_byte
#print axioms QCore.ScalarPayloads.a_fitting_footprint_touches_only_payload_bytes
#print axioms QCore.ScalarPayloads.an_i32_lease_is_decided_as_before

-- The vectors the Rust extent replays.
#print axioms QCore.ScalarPayloads.sizeAgreement_holds
