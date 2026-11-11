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

import QCore.Compiler

-- Print the foundational dependencies of every Cycle 12 compiler theorem.
#print axioms QCore.Compiler.compile_preserves_checked_source
#print axioms QCore.Compiler.compile_implies_whole_program_accepts
#print axioms QCore.Compiler.compile_selects_named_entry
#print axioms QCore.Compiler.compile_retains_entry_cfg
#print axioms QCore.Compiler.compile_emits_origin_mapping
#print axioms QCore.Compiler.emitted_c11_matches_reference
