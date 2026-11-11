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

import QCore.SourceValidation

#print axioms QCore.SourceValidation.admitted_return_has_declared_shape
#print axioms QCore.SourceValidation.unsupported_result_rejected
#print axioms QCore.SourceValidation.integer_cannot_satisfy_unit
#print axioms QCore.SourceValidation.bare_cannot_satisfy_integer
#print axioms QCore.SourceValidation.signed_literal_is_in_range
#print axioms QCore.SourceValidation.declaration_validation_independent_of_uses
#print axioms QCore.SourceValidation.accepted_declarations_are_valid

#print axioms QCore.SourceValidation.scalar_check_preserves_separate_custody
