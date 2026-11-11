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

import QCore.Ieee754Vectors32
import QCore.Ieee754Vectors64

-- Card `W48-QR-FLOAT-IEEE-LEAN-MODEL`: IEEE 754 binary32/binary64 encoding,
-- round to nearest ties to even, the operations, comparisons and conversions
-- `.qr` floats use, and the Rust kernel's vector tables proved against them.
#print axioms QCore.Ieee754.binary32_valid
#print axioms QCore.Ieee754.binary64_valid
#print axioms QCore.Ieee754.binary32_constants
#print axioms QCore.Ieee754.binary64_constants
#print axioms QCore.Ieee754.log2_mul_pow
#print axioms QCore.Ieee754.log2_lt_of_lt
#print axioms QCore.Ieee754.pow_pos_ne
#print axioms QCore.Ieee754.log2Search_spec
#print axioms QCore.Ieee754.ilog2_eq
#print axioms QCore.Ieee754.sig_lt
#print axioms QCore.Ieee754.sig_ge
#print axioms QCore.Ieee754.shift_of_lt
#print axioms QCore.Ieee754.sig_of_lt
#print axioms QCore.Ieee754.sig_mul_shift
#print axioms QCore.Ieee754.unbRep_iff
#print axioms QCore.Ieee754.shift_le_kmax
#print axioms QCore.Ieee754.decodeMag_encodeMag
#print axioms QCore.Ieee754.decodeMag_rep
#print axioms QCore.Ieee754.encodeMag_decodeMag
#print axioms QCore.Ieee754.encodeMag_lt
#print axioms QCore.Ieee754.quietNaN_binary32
#print axioms QCore.Ieee754.quietNaN_binary64
#print axioms QCore.Ieee754.infMag_lt_signBit
#print axioms QCore.Ieee754.quietNaN_lt_signBit
#print axioms QCore.Ieee754.signBit_pow
#print axioms QCore.Ieee754.decode_signPart
#print axioms QCore.Ieee754.decode_encode
#print axioms QCore.Ieee754.encode_decode
#print axioms QCore.Ieee754.decode_valid
#print axioms QCore.Ieee754.encode_lt
#print axioms QCore.Ieee754.fracField_eq
#print axioms QCore.Ieee754.decodeMag_zero_iff
#print axioms QCore.Ieee754.decodeMag_lt_iff
#print axioms QCore.Ieee754.classify_spec
#print axioms QCore.Ieee754.p_pos
#print axioms QCore.Ieee754.pow_p
#print axioms QCore.Ieee754.dist_of_le
#print axioms QCore.Ieee754.dist_of_lt
#print axioms QCore.Ieee754.dist_eq_natAbs
#print axioms QCore.Ieee754.quotient_bounds
#print axioms QCore.Ieee754.gap
#print axioms QCore.Ieee754.sig_grid
#print axioms QCore.Ieee754.sig_top
#print axioms QCore.Ieee754.encodeMag_even
#print axioms QCore.Ieee754.candidates
#print axioms QCore.Ieee754.roundUnb_cases
#print axioms QCore.Ieee754.roundUnb_unbRep
#print axioms QCore.Ieee754.competitor
#print axioms QCore.Ieee754.roundUnb_nearest
#print axioms QCore.Ieee754.roundUnb_tie
#print axioms QCore.Ieee754.roundUnb_exact
#print axioms QCore.Ieee754.midpoint_of_nearer
#print axioms QCore.Ieee754.midpoint_of_nearer'
#print axioms QCore.Ieee754.roundUnb_mono
#print axioms QCore.Ieee754.roundMag_some
#print axioms QCore.Ieee754.roundMag_rep
#print axioms QCore.Ieee754.roundMag_nearest
#print axioms QCore.Ieee754.roundMag_tie_even
#print axioms QCore.Ieee754.sig_maxN
#print axioms QCore.Ieee754.maxN_unbRep
#print axioms QCore.Ieee754.top_unbRep
#print axioms QCore.Ieee754.roundMag_none_iff
#print axioms QCore.Ieee754.roundMag_mono
#print axioms QCore.Ieee754.roundMag_mono_none
#print axioms QCore.Ieee754.roundMag_exact
#print axioms QCore.Ieee754.rshift_small
#print axioms QCore.Ieee754.nan_propagates
#print axioms QCore.Ieee754.inf_sub_inf
#print axioms QCore.Ieee754.add_inf
#print axioms QCore.Ieee754.zero_mul_inf
#print axioms QCore.Ieee754.inf_mul
#print axioms QCore.Ieee754.div_by_zero
#print axioms QCore.Ieee754.div_inf
#print axioms QCore.Ieee754.add_zeros
#print axioms QCore.Ieee754.add_neg_self
#print axioms QCore.Ieee754.neg_zero_sub_zero
#print axioms QCore.Ieee754.round_sign
#print axioms QCore.Ieee754.mul_div_sign
#print axioms QCore.Ieee754.zero_mul
#print axioms QCore.Ieee754.neg_neg
#print axioms QCore.Ieee754.add_comm
#print axioms QCore.Ieee754.mul_comm
#print axioms QCore.Ieee754.round_fin
#print axioms QCore.Ieee754.round_inf_iff
#print axioms QCore.Ieee754.add_exact
#print axioms QCore.Ieee754.rank_fin_bounds
#print axioms QCore.Ieee754.flt_fin
#print axioms QCore.Ieee754.cmp_of_not_nan
#print axioms QCore.Ieee754.nan_unordered
#print axioms QCore.Ieee754.feq_self
#print axioms QCore.Ieee754.neg_zero_eq_zero
#print axioms QCore.Ieee754.trichotomy
#print axioms QCore.Ieee754.fle_iff
#print axioms QCore.Ieee754.feq_symm_trans
#print axioms QCore.Ieee754.flt_irrefl_trans
#print axioms QCore.Ieee754.gated_relations
#print axioms QCore.Ieee754.partial_not_total
#print axioms QCore.Ieee754.toInt_bool
#print axioms QCore.Ieee754.floatTypes_toInt_bool_differs
#print axioms QCore.Ieee754.roundUnb_scaled
#print axioms QCore.Ieee754.ofInt_roundNat
#print axioms QCore.Ieee754.ofInt_value
#print axioms QCore.Ieee754.ofInt_exact_small
#print axioms QCore.Ieee754.convert_fin
#print axioms QCore.Ieee754.round_of_some
#print axioms QCore.Ieee754.widen_exact
#print axioms QCore.Ieee754.truncDyadic_fin
#print axioms QCore.Ieee754.toInt_some_iff
#print axioms QCore.Ieee754.toInt_none_iff
#print axioms QCore.Ieee754.decode_negBits
#print axioms QCore.Ieee754.negBits_lt
#print axioms QCore.Ieee754.Vectors32.arith_hold
#print axioms QCore.Ieee754.Vectors32.neg_hold
#print axioms QCore.Ieee754.Vectors32.cmp_hold
#print axioms QCore.Ieee754.Vectors32.ofInt_hold
#print axioms QCore.Ieee754.Vectors32.toInt_hold
#print axioms QCore.Ieee754.Vectors32.convert_hold
#print axioms QCore.Ieee754.Vectors64.arith_hold
#print axioms QCore.Ieee754.Vectors64.neg_hold
#print axioms QCore.Ieee754.Vectors64.cmp_hold
#print axioms QCore.Ieee754.Vectors64.ofInt_hold
#print axioms QCore.Ieee754.Vectors64.toInt_hold
#print axioms QCore.Ieee754.Vectors64.convert_hold
#print axioms QCore.Ieee754.Vectors64.effects_hold
