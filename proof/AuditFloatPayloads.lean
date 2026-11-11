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

import QCore.FloatPayloads

-- Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `f32`/`f64` payloads and float values on
-- the byte-buffer route.

-- The extent: a float payload is its IEEE format's size; the integer table's
-- sizes are unchanged; every size is one to eight bytes.
#print axioms QCore.FloatPayloads.a_float_payload_is_its_format_size
#print axioms QCore.FloatPayloads.integer_payload_sizes_are_unchanged
#print axioms QCore.FloatPayloads.payload_sizes_are_one_to_eight

-- The byte view of a float payload is total: bytes round-trip, every byte
-- sequence decodes to a pattern of its width, a byte write keeps a value.
#print axioms QCore.FloatPayloads.toBytes_length
#print axioms QCore.FloatPayloads.fromBytes_lt
#print axioms QCore.FloatPayloads.fromBytes_toBytes
#print axioms QCore.FloatPayloads.a_byte_write_keeps_a_float_value
#print axioms QCore.FloatPayloads.every_float_pattern_is_its_bytes
#print axioms QCore.FloatPayloads.float_patterns_are_every_byte_pattern

-- `WRITE-LEND-TASKS` at float extents: no byte has two write authorities,
-- every held byte is a payload byte, a lease sized for the other float is
-- refused, and the native rows' halves are admitted.
#print axioms QCore.FloatPayloads.no_byte_has_two_write_authorities_payload
#print axioms QCore.FloatPayloads.every_held_byte_is_a_payload_byte_payload
#print axioms QCore.FloatPayloads.float_lease_ledgers
#print axioms QCore.FloatPayloads.an_f64_sized_lease_on_an_f32_is_refused
#print axioms QCore.FloatPayloads.the_float_halves_are_admitted

-- Comparisons on the byte-buffer route: its one call-free entry is admitted
-- exactly when every float comparison is declared; unrolling adds none.
#print axioms QCore.FloatPayloads.buffer_entry_admitted_iff
#print axioms QCore.FloatPayloads.buffer_entry_without_effect_compares_nothing
#print axioms QCore.FloatPayloads.unrolled_comparisons_are_the_body's
#print axioms QCore.FloatPayloads.a_loop_body_comparison_needs_its_effect

-- The vectors the Rust extent replays.
#print axioms QCore.FloatPayloads.floatSizeAgreement_holds
