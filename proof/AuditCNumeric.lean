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

import QCore.CNumeric
#print axioms QCore.CNumeric.long_is_64_under_lp64
#print axioms QCore.CNumeric.long_is_32_under_llp64
#print axioms QCore.CNumeric.long_plus_uint_lp64
#print axioms QCore.CNumeric.long_plus_uint_llp64
#print axioms QCore.CNumeric.char_promotes_to_int
#print axioms QCore.CNumeric.uint_plus_int_is_uint
#print axioms QCore.CNumeric.promotion_vectors_hold
#print axioms QCore.CNumeric.usual_vectors_hold
#print axioms QCore.CNumeric.kernel_ok_iff_unreached
#print axioms QCore.CNumeric.kernel_error_reached
#print axioms QCore.CNumeric.unsigned_arithmetic_defined
#print axioms QCore.CNumeric.wrap_in_range
#print axioms QCore.CNumeric.signed_add_in_range
#print axioms QCore.CNumeric.division_by_zero_reached
#print axioms QCore.CNumeric.quotient_overflow_reached
#print axioms QCore.CNumeric.shift_count_reached
#print axioms QCore.CNumeric.negative_left_shift_reached
#print axioms QCore.CNumeric.int_div_zero
#print axioms QCore.CNumeric.int_min_div_neg_one
#print axioms QCore.CNumeric.one_shl_32
#print axioms QCore.CNumeric.uint_max_plus_one_wraps
#print axioms QCore.CNumeric.kernel_vectors_hold
#print axioms QCore.CNumeric.convert_unsigned_defined
#print axioms QCore.CNumeric.convert_fits_exact
#print axioms QCore.CNumeric.conversion_vectors_hold
#print axioms QCore.CNumeric.add_discharge_sound
#print axioms QCore.CNumeric.sub_discharge_sound
#print axioms QCore.CNumeric.mul_discharge_sound
#print axioms QCore.CNumeric.division_discharge_sound
#print axioms QCore.CNumeric.quotient_discharge_sound
#print axioms QCore.CNumeric.shift_count_discharge_sound
#print axioms QCore.CNumeric.negative_shift_discharge_sound
#print axioms QCore.CNumeric.shl_overflow_discharge_sound
#print axioms QCore.CNumeric.conversion_discharge_sound
#print axioms QCore.CNumeric.range_mono
#print axioms QCore.CNumeric.portable_in_every_model
#print axioms QCore.CNumeric.portable_long_holds_on_llp64
