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

import QCore.DeepAstTransports

/-! Audited companion to DEEP-AST-TRANSPORTS-AND-WALL-TIME: the filtered and closed-world Clang JSON transports read a document within rule 13o's bound (rule 13q), the others keep their limit (rule 13r), and the per-function memo and line tree answer as the readers they replace (rules 13s, 13t). -/

#print axioms QCore.DeepAstTransports.reads_iff_within_the_bound
#print axioms QCore.DeepAstTransports.the_band_keeps_every_default_reading
#print axioms QCore.DeepAstTransports.nothing_beyond_the_bound
#print axioms QCore.DeepAstTransports.a_default_transport_is_unchanged
#print axioms QCore.DeepAstTransports.the_band_transports
#print axioms QCore.DeepAstTransports.streamDepth_le_iff
#print axioms QCore.DeepAstTransports.a_stream_is_read_iff_every_value_is
#print axioms QCore.DeepAstTransports.fits_at_the_bound_fits_within_it
#print axioms QCore.DeepAstTransports.lookup_mem
#print axioms QCore.DeepAstTransports.the_memo_answers_as_the_function
#print axioms QCore.DeepAstTransports.asking_keeps_the_memo_sound
#print axioms QCore.DeepAstTransports.the_empty_memo_is_sound
#print axioms QCore.DeepAstTransports.every_ask_is_answered_by_the_function
#print axioms QCore.DeepAstTransports.annotated_walk
#print axioms QCore.DeepAstTransports.annotated_walkList
#print axioms QCore.DeepAstTransports.the_annotated_walk_is_the_walk
