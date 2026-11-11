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

import QCore.LiteralTailActual

/-! Card **W44-LIBC-RESIDUALS**, `LIBC-PRINTF-TAIL-ACTUALS`: T6, the one tail
admission that needs neither the enclosing function's typed declarations nor
C02-P's closure. A tail actual that consists only of literals and C11 6.4.2.2
predefined identifiers is data whatever those say.

Four directions are audited. **Conservativity**: the widening only adds, a read
site with no literal-only actual is decided exactly as before, and the declared
tail is still required. **Soundness**: a literal-only actual carries no
identity under every assignment of "may bind" to names the declarations or the
closure could have made, and every other admitted actual is T1-T5 data. **What
still refuses**: a name, a punctuator, an empty actual. **Non-vacuity**: the
measured `RDUT:` tail, in an unread function and in a read one whose closure
marked the literal, was refused and is admitted, and a name beside it keeps its
refusal.

What this is not: the tokenizer's collapse of literals and Clang's keyword
lexing of the predefined identifiers are premises (trust boundary 28). -/

-- Conservativity.
#print axioms QCore.LiteralTailActual.a_read_site_admits_literal_or_data
#print axioms QCore.LiteralTailActual.all_without_literal_actuals
#print axioms QCore.LiteralTailActual.a_read_site_without_literal_actuals_is_unchanged
#print axioms QCore.LiteralTailActual.the_widening_only_adds
#print axioms QCore.LiteralTailActual.the_declaration_is_still_required

-- Soundness.
#print axioms QCore.LiteralTailActual.a_literal_actual_carries_no_identity_under_any_declarations
#print axioms QCore.LiteralTailActual.an_unread_site_admits_only_literal_actuals
#print axioms QCore.LiteralTailActual.every_actual_admitted_unread_carries_no_identity
#print axioms QCore.LiteralTailActual.every_actual_admitted_read_is_literal_or_data

-- What still refuses.
#print axioms QCore.LiteralTailActual.an_unread_name_still_refuses
#print axioms QCore.LiteralTailActual.an_unread_punctuator_still_refuses
#print axioms QCore.LiteralTailActual.an_empty_actual_still_refuses

-- Non-vacuity.
#print axioms QCore.LiteralTailActual.the_rdut_tail_was_refused_and_is_admitted
#print axioms QCore.LiteralTailActual.a_user_tail_beside_it_is_still_refused
#print axioms QCore.LiteralTailActual.the_closure_risky_literal_tail_was_refused_and_is_admitted
#print axioms QCore.LiteralTailActual.a_risky_name_beside_it_is_still_refused
