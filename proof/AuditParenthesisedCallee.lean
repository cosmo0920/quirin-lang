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

import QCore.ParenthesisedCallee

/-! Card **W45-PARENTHESISED-CALLEE**: a statement `(f)(args);` was silently
skipped by the legacy-C gateway, because every call reader keyed on the pair
`NAME (`, and a statement with no visible call fell through to `NoOp`.

Four directions are audited.

**Rule 1 preserves what C calls.** Removing parentheses around a callee
changes neither what it designates nor, therefore, the lowering: `(f)(a)` and
`f(a)` have the same effect, and an admitted call is a call of exactly the
function C calls.

**No unrecorded no-op.** Every call is a classified effect or a refusal; a
`NoOp` lowering is only ever a statement recognised as call-free; a callee that
designates no name is refused, and keeps its body out of `Safe`.

**Rules 2 and 3 only refuse.** The hook over an arbitrary classification, and
the `for`-clause hook, never admit what the base classification refused.

**Non-vacuity.** Before the card the parenthesised call and every other callee
expression lowered to `NoOp`.

Faithful token extraction and the Rust correspondence are trust boundary 28's
premises, pinned by `crates/qcore-c-gateway/tests/parenthesised_callee.rs`. -/

-- Rule 1 preserves what C calls.
#print axioms QCore.ParenthesisedCallee.norm_designates
#print axioms QCore.ParenthesisedCallee.norm_parens
#print axioms QCore.ParenthesisedCallee.norm_idempotent
#print axioms QCore.ParenthesisedCallee.norm_is_name_iff
#print axioms QCore.ParenthesisedCallee.a_parenthesised_name_is_the_name
#print axioms QCore.ParenthesisedCallee.parentheses_never_change_a_lowering
#print axioms QCore.ParenthesisedCallee.an_admitted_call_is_the_designated_call
#print axioms QCore.ParenthesisedCallee.a_bare_name_is_unchanged

-- No unrecorded no-op.
#print axioms QCore.ParenthesisedCallee.a_call_is_never_a_no_op
#print axioms QCore.ParenthesisedCallee.a_no_op_is_call_free
#print axioms QCore.ParenthesisedCallee.an_opaque_callee_is_refused
#print axioms QCore.ParenthesisedCallee.an_opaque_call_keeps_the_body_out_of_safe

-- Rules 2 and 3 only refuse.
#print axioms QCore.ParenthesisedCallee.guard_never_admits_a_refusal
#print axioms QCore.ParenthesisedCallee.guard_admits_only_what_the_base_admits
#print axioms QCore.ParenthesisedCallee.guard_refuses_an_opaque_admission
#print axioms QCore.ParenthesisedCallee.guard_is_the_identity_without_an_opaque_call
#print axioms QCore.ParenthesisedCallee.a_grouped_for_clause_is_never_admitted
#print axioms QCore.ParenthesisedCallee.an_ungrouped_for_clause_is_unchanged

-- Non-vacuity.
#print axioms QCore.ParenthesisedCallee.the_parenthesised_call_was_a_no_op
#print axioms QCore.ParenthesisedCallee.the_parenthesised_call_is_now_the_call
#print axioms QCore.ParenthesisedCallee.an_opaque_call_was_a_no_op
#print axioms QCore.ParenthesisedCallee.the_double_release_is_no_longer_hidden
