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

import QCore.Frontend

-- Print the foundational dependencies of every Cycle 10 front-end theorem.
#print axioms QCore.Frontend.elaborate_implies_wellFormed
#print axioms QCore.Frontend.elaborate_preserves_source
#print axioms QCore.Frontend.elaborate_preserves_declaration_count
#print axioms QCore.Frontend.elaborate_preserves_spans
#print axioms QCore.Frontend.elaborate_has_unique_declarations
#print axioms QCore.Frontend.elaborate_resolves_references
#print axioms QCore.Frontend.elaborate_produces_valid_spans
#print axioms QCore.Frontend.elaborate_symbol_count
#print axioms QCore.Frontend.elaborate_symbols_unique
#print axioms QCore.Frontend.invalid_program_is_rejected
