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

import QCore.ReservedSpelling

/-! Card **W45-SR5-SOUNDNESS** (`HELD-PREFIX-SOURCE-FORGES-PENDING`,
`crates/qcore-c-gateway/src/legacy_reserved_spelling.rs`): after the escape of
every source-spelled reserved prefix, a marked name was minted from the withheld
set, so `Pending` arises only from a tool-minted hold, and a genuine hold stays
`Pending`. Premise: the escape prefix diverges from every marker (the Rust
constants do). Not a proof about the Rust lexer (trust boundary 28). -/

#print axioms QCore.ReservedSpelling.diverge_append_not_startsWith
#print axioms QCore.ReservedSpelling.an_escaped_name_carries_no_marker
#print axioms QCore.ReservedSpelling.an_escaped_stream_carries_no_marker
#print axioms QCore.ReservedSpelling.a_source_spelled_marker_is_not_held
#print axioms QCore.ReservedSpelling.the_escape_keeps_every_other_name
#print axioms QCore.ReservedSpelling.startsWith_append
#print axioms QCore.ReservedSpelling.a_marked_name_was_minted
#print axioms QCore.ReservedSpelling.pending_only_from_a_minted_hold
#print axioms QCore.ReservedSpelling.a_genuine_hold_is_pending
#print axioms QCore.ReservedSpelling.witness_is_well_separated
#print axioms QCore.ReservedSpelling.the_measured_forgery_was_pending
#print axioms QCore.ReservedSpelling.the_measured_forgery_is_unknown
