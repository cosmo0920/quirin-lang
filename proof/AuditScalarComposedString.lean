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

import QCore.ScalarComposedString

-- Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`): the fixed-width
-- tier on the composed string/buffer route.

-- The value decision is the buffer route's, per statement and per iteration.
#print axioms QCore.ScalarComposedString.the_composed_statement_is_the_buffer_statement
#print axioms QCore.ScalarComposedString.a_composed_loop_statement_is_decided_per_iteration

-- The borrow window is blind to the typed tier, at every iteration, and still
-- refuses a store inside a live loan.
#print axioms QCore.ScalarComposedString.a_typed_statement_never_conflicts
#print axioms QCore.ScalarComposedString.the_window_ignores_typed_statements
#print axioms QCore.ScalarComposedString.skeleton_unroll
#print axioms QCore.ScalarComposedString.an_unrolled_window_ignores_typed_statements
#print axioms QCore.ScalarComposedString.a_store_inside_a_live_loan_still_conflicts
#print axioms QCore.ScalarComposedString.a_store_outside_every_live_loan_is_admitted
