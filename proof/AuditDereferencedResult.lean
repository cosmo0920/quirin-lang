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

import QCore.DereferencedResult

/-! Card CONSTRUCTOR-RESIDUALS: a **dereference of a neutral call's result**,
and the borrow-liveness repair that came with it.

Both directions are audited, and both matter.

**The base family is untouched.** With the bare wrapper the widened rule *is*
card O04-TRACE's `admitsNested`, pointwise and for every pointee reading, so no
statement that card admitted can move. The new premise N10 is consulted only
under a dereference, which is audited separately.

**Refinement, not violation.** Card W4-EXPR-POS's admitted families are proved
under *an admitted expression contains no pointer-valued operand*, and a call
whose result is dereferenced is pointer-valued by construction.
`the_earlier_sentence_does_not_cover_this` states that rather than stretching
it, and `the_refined_sentence_holds` proves the replacement — *binds, releases,
escapes and addresses nothing this projection tracks* — from the admission,
with each conjunct discharged by a premise already required (N6, N7, N9, N10).

**Fail-closed.** Every `admitsNested` refusal lifts unchanged; N10 refuses every
non-scalar and every unread pointee in both spellings; every other wrapper shape
refuses whatever the nested call is. A refused candidate leaves card
O04-B09B10's nested refusal exactly where it was, so refusal is never an
accusation.

**The repair.** The borrow-liveness pre-pass now reads the admitted nested
call's actuals too. It is audited as a set inclusion — strictly more refusing,
identical where there is no nested call — together with the measured fail-open
it closes and the control that must not move.

Non-vacuity is audited with each premise separately flipped on the real
subject, `flb_errno_print((*__errno_location ()), …)`.
-/

-- The base family is unchanged.
#print axioms QCore.DereferencedResult.the_bare_wrapper_is_exactly_the_base_admission
#print axioms QCore.DereferencedResult.every_previously_admitted_nested_call_is_still_admitted
#print axioms QCore.DereferencedResult.the_pointee_is_read_only_under_a_dereference
#print axioms QCore.DereferencedResult.the_bare_subject_is_unaffected_by_the_new_premise

-- Fail-closed: every O04-TRACE premise still refuses on its own.
#print axioms QCore.DereferencedResult.a_refused_nested_call_refuses_the_candidate
#print axioms QCore.DereferencedResult.unread_declarations_never_admit
#print axioms QCore.DereferencedResult.a_two_level_nested_call_is_never_admitted
#print axioms QCore.DereferencedResult.a_call_inside_the_nested_arguments_is_never_admitted
#print axioms QCore.DereferencedResult.an_uncontracted_nested_callee_is_never_admitted
#print axioms QCore.DereferencedResult.an_uncontracted_outer_callee_never_admits
#print axioms QCore.DereferencedResult.a_nested_arity_mismatch_is_never_admitted
#print axioms QCore.DereferencedResult.an_undenotable_inner_actual_is_never_admitted
#print axioms QCore.DereferencedResult.an_owned_nested_result_is_never_admitted
#print axioms QCore.DereferencedResult.a_custody_bearing_nested_parameter_is_never_admitted
#print axioms QCore.DereferencedResult.a_custody_slot_never_admits
#print axioms QCore.DereferencedResult.a_candidate_beyond_the_named_vector_needs_a_declared_tail

-- The wrapper half.
#print axioms QCore.DereferencedResult.every_other_wrapper_shape_refuses
#print axioms QCore.DereferencedResult.both_spellings_answer_alike

-- The N10 half.
#print axioms QCore.DereferencedResult.a_non_scalar_pointee_refuses
#print axioms QCore.DereferencedResult.an_unread_pointee_refuses

-- The refined soundness sentence, and the honest statement of what it replaces.
#print axioms QCore.DereferencedResult.the_refined_sentence_holds
#print axioms QCore.DereferencedResult.the_earlier_sentence_does_not_cover_this
#print axioms QCore.DereferencedResult.an_admitted_candidate_is_neutral

-- The subject, strictness and non-vacuity.
#print axioms QCore.DereferencedResult.the_widening_is_strict
#print axioms QCore.DereferencedResult.each_premise_is_load_bearing_on_the_subject

-- The borrow-liveness repair.
#print axioms QCore.DereferencedResult.the_repair_only_adds_refusals
#print axioms QCore.DereferencedResult.a_site_with_no_nested_call_is_unchanged
#print axioms QCore.DereferencedResult.the_measured_fail_open_is_closed
#print axioms QCore.DereferencedResult.the_control_is_unmoved
