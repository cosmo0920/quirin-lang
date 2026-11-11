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

import QCore.ScalarTypedErrors
#print axioms QCore.ScalarTypedErrors.checkedE_ok_iff
#print axioms QCore.ScalarTypedErrors.checkedE_raises_iff
#print axioms QCore.ScalarTypedErrors.checkedE_raises_classified
#print axioms QCore.ScalarTypedErrors.checkedE_in_range
#print axioms QCore.ScalarTypedErrors.checkedE_exact
#print axioms QCore.ScalarTypedErrors.division_by_zero_raises
#print axioms QCore.ScalarTypedErrors.min_div_minus_one_raises_overflow
#print axioms QCore.ScalarTypedErrors.convertE_raises_out_of_range
#print axioms QCore.ScalarTypedErrors.convertE_ok_iff_fits
#print axioms QCore.ScalarTypedErrors.total_conversion_never_raises
#print axioms QCore.ScalarTypedErrors.evalE_agrees
#print axioms QCore.ScalarTypedErrors.static_fault_is_dynamic_raise
#print axioms QCore.ScalarTypedErrors.static_value_is_dynamic_value
#print axioms QCore.ScalarTypedErrors.static_fault_variant
#print axioms QCore.ScalarTypedErrors.propagate_raise
#print axioms QCore.ScalarTypedErrors.propagate_value
#print axioms QCore.ScalarTypedErrors.handle_raise
#print axioms QCore.ScalarTypedErrors.handle_value
#print axioms QCore.ScalarTypedErrors.handled_never_raises
#print axioms QCore.ScalarTypedErrors.typed_agreement_holds
