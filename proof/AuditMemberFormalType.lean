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

import QCore.MemberFormalType

/-! Card **LOG-PRINT-BODY-CONTRACTS** (gateway rule 12n, a fail-closed repair):
a store of a formal that is not a function pointer opens the member. Only a
function-pointer formal forwards its callers' actuals; an open unit reading,
anywhere in the run, keeps the run's closure open and the invocation is
refused; the defect (a `void *` formal read as a function pointer) would have
closed the member empty. Which C spelling is a function pointer is the
reading's extraction premise (rule 28), not modelled here. -/

#print axioms QCore.MemberFormalType.a_non_function_formal_store_is_open
#print axioms QCore.MemberFormalType.an_open_unit_keeps_the_member_open
#print axioms QCore.MemberFormalType.a_non_function_formal_store_never_decides
#print axioms QCore.MemberFormalType.the_defect_would_close_the_member_empty
