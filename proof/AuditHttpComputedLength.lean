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

import QCore.HttpComputedLength

-- Card `HTTP-V1-COMPUTED-LENGTH`: a **computed** `Content-Length` inside
-- `source.http.v1`. The claim is not that a new check was added — it is that
-- widening a message component to a value the entry computes leaves every check
-- `HTTP-QUIRIN-V1` decided exactly where it was.

-- The decimal roundtrip. A `Content-Length` the compiler rendered from a length
-- it computed is a number the framing gate can read back, which is what makes a
-- computed length decidable at all rather than a premise.
#print axioms QCore.HttpComputedLength.fromDigits_snoc
#print axioms QCore.HttpComputedLength.fromDigits_digits
#print axioms QCore.HttpComputedLength.digits_lt_ten
#print axioms QCore.HttpComputedLength.digits_ne_nil
#print axioms QCore.HttpComputedLength.digitByte_toNat
#print axioms QCore.HttpComputedLength.decBytes_ne_nil
#print axioms QCore.HttpComputedLength.decBytes_all_digits
#print axioms QCore.HttpComputedLength.decBytes_digitValues
#print axioms QCore.HttpComputedLength.a_rendering_parses_back_to_the_number_it_rendered

-- The injection wall, over composed bytes, in **both** directions: composing
-- cannot launder a `CR` into a head and cannot spoil a head built from clean
-- pieces. A rendered component settles outright — decimal digits are field
-- content, so the wall is unreachable there rather than merely satisfied.
#print axioms QCore.HttpComputedLength.a_rendering_carries_no_framing_byte
#print axioms QCore.HttpComputedLength.a_composition_is_framing_free_exactly_when_its_pieces_are
#print axioms QCore.HttpComputedLength.a_framing_byte_in_a_piece_survives_the_composition

-- The length arithmetic, and that a composition is the flattening of its
-- resolved pieces. Standing constraint 15h as an equation: a length derived
-- from a composed payload is a check-time constant exactly as a literal's own
-- length is.
#print axioms QCore.HttpComputedLength.compose_eq_flatten
#print axioms QCore.HttpComputedLength.the_composed_length_is_the_sum_of_the_piece_lengths

-- **The card's own theorems**, each over `QCore.StdHttp.admitFraming` itself,
-- unedited. Proved in both directions so the check stays a gate rather than a
-- description, and stated both for a bare rendering and for a rendering of the
-- sum of a composition's piece lengths — the shape the published example has.
#print axioms QCore.HttpComputedLength.a_rendered_content_length_is_admitted
#print axioms QCore.HttpComputedLength.a_rendered_length_that_is_not_the_content_length_is_refused
#print axioms QCore.HttpComputedLength.a_composed_content_is_framed_by_the_sum_of_its_pieces
#print axioms QCore.HttpComputedLength.a_composed_content_with_a_wrong_declared_length_is_refused

-- The added vocabulary: two statements, neither deriving an effect, and a
-- composition of fewer than two pieces refused. Written as closed lists so a
-- third statement or a widened admission changes a theorem rather than slipping
-- past one.
#print axioms QCore.HttpComputedLength.no_computed_statement_derives_an_effect
#print axioms QCore.HttpComputedLength.the_added_statement_forms_are_exactly_two
#print axioms QCore.HttpComputedLength.a_composition_of_one_is_refused
#print axioms QCore.HttpComputedLength.a_composition_of_none_is_refused
#print axioms QCore.HttpComputedLength.a_composition_of_two_is_admitted
