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

import QCore.NullPointerConstantPremise

/-! Card **CURRENT-CONTEXT-NEW-TRIAGE** (D3; user decision 2026-11-16 "Admit
(void *) 0 (Recommended)"; contract specification §C04.5.1): the C11
6.3.2.3p3 null pointer constant `(void *) 0` meets a pointer premise of `0`.
Audited: scalar premises and other literals unchanged; the widening is
strict; an applied contract is still the declared one; non-vacuity. The
structural recogniser is `legacy_literal_premise::void_pointer_null_constant`,
pinned by its unit tests (trust boundary 28). -/

#print axioms QCore.NullPointerConstantPremise.a_scalar_premise_is_unchanged
#print axioms QCore.NullPointerConstantPremise.another_literal_is_unchanged
#print axioms QCore.NullPointerConstantPremise.the_old_spellings_still_satisfy
#print axioms QCore.NullPointerConstantPremise.an_applied_contract_is_still_the_declared_one
#print axioms QCore.NullPointerConstantPremise.a_poisoned_environment_still_applies_nothing
#print axioms QCore.NullPointerConstantPremise.the_cast_meets_only_a_pointer_premise
