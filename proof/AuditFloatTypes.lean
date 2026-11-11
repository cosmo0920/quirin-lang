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

import QCore.FloatTypes

-- Card `W48-QR-FLOATING-POINT`: the comparison effects `partial_eq` and
-- `partial_ord`, float -> integer truncation, and integer -> float rounding.
#print axioms QCore.FloatTypes.reached_is_declared
#print axioms QCore.FloatTypes.no_undeclared_float_comparison
#print axioms QCore.FloatTypes.caller_declares_callee_effects
#print axioms QCore.FloatTypes.no_equality_without_partial_eq
#print axioms QCore.FloatTypes.no_ordering_without_partial_ord
#print axioms QCore.FloatTypes.partial_ord_does_not_admit_equality
#print axioms QCore.FloatTypes.undeclared_callee_effect_is_refused
#print axioms QCore.FloatTypes.integer_comparisons_need_no_effect
#print axioms QCore.FloatTypes.toInt_succeeds_iff
#print axioms QCore.FloatTypes.toInt_nan
#print axioms QCore.FloatTypes.toInt_inf
#print axioms QCore.FloatTypes.toInt_in_range
#print axioms QCore.FloatTypes.truncDyadic_exact
#print axioms QCore.FloatTypes.toInt_needs_target
#print axioms QCore.FloatTypes.roundNat_exact_small
#print axioms QCore.FloatTypes.roundNat_error_le_half
#print axioms QCore.FloatTypes.roundInt_exact_small
#print axioms QCore.FloatTypes.roundInt_neg
#print axioms QCore.FloatTypes.rounding_vectors_hold
#print axioms QCore.FloatTypes.truncation_vectors_hold
