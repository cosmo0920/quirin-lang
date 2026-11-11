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

import QCore.ChainedTailActual

/-! Card NEIGHBOURS-CHAINED-TAIL: a bounded typed access chain at a **declared
variadic data tail** position.

Both directions are audited, and both matter.

**The declaration half is untouched.** No actual can buy itself a tail: with no
declared `resource_variadic_data_tail()` the whole call site refuses whatever
chains its actuals are, which is card O04-TRACE's arity repair carried through
this widening unchanged. Unread declarations admit nothing, as everywhere else.

**Widening, stated as such.** Everything card O04-TRACE's T1-T4 walker admitted
is still admitted — pointwise and lifted to whole sites over the un-widened
model's own admission — and the widening is *strict* on exactly the actual card
NEIGHBOURS isolated. The measured correction is audited beside it: a depth-one
**scalar** chain, squarely inside card C02's original whitelist, is widened by
the same disjunct, so the widening was never about a pointer-typed final field.

**Fail-closed.** Every `QCore.ChainedAccess` refusal lifts to the whole call
site: a bound base, a continued access, a chain past the inherited depth bound,
a `union`, incomplete, ambiguous, qualified or aggregate link, and an
intermediate link the reader did not type as exactly `struct POINTEE *`. An
actual that is not a *whole* chain keeps T2 unchanged, and an actual naming
something this body may bind is refused **under** the declaration exactly as it
was without one. A refused site refuses its statement and can never be classified
`safe`; refusal is never an accusation.

**The soundness sentence**: every admitted chained actual names a base this body
can never bind to a tracked resource identity and does not continue past its last
selection, so the value it reads is denoted by no place the projection tracks —
and its read premises are proved to be *exactly* the declared-`Borrowed`
admission's, so the position is the only difference and the position comes from
the callee.

**The admission removes only the spans it justified**: any other indirect access
still refuses the statement, and one risky sibling refuses the whole site, so
exempting an admitted tail span is never admitting the call.

Non-vacuity is audited with each premise separately flipped on the same witness.
-/

-- The declaration half.
#print axioms QCore.ChainedTailActual.the_declaration_is_read_from_the_callee_never_from_the_actual
#print axioms QCore.ChainedTailActual.unread_declarations_never_admit

-- Widening, stated as such.
#print axioms QCore.ChainedTailActual.every_previously_admitted_actual_is_still_data
#print axioms QCore.ChainedTailActual.every_previously_admitted_site_is_still_admitted
#print axioms QCore.ChainedTailActual.the_widening_is_strict
#print axioms QCore.ChainedTailActual.the_subject_site_is_admitted
#print axioms QCore.ChainedTailActual.a_depth_one_scalar_chain_is_widened_the_same_way

-- Fail-closed: every chain refusal lifts to the site.
#print axioms QCore.ChainedTailActual.a_refused_actual_refuses_the_site
#print axioms QCore.ChainedTailActual.a_refused_chain_refuses_the_site
#print axioms QCore.ChainedTailActual.a_bound_base_refuses_the_site
#print axioms QCore.ChainedTailActual.a_continued_access_refuses_the_site
#print axioms QCore.ChainedTailActual.a_chain_beyond_the_bound_refuses_the_site
#print axioms QCore.ChainedTailActual.a_union_link_refuses_the_site
#print axioms QCore.ChainedTailActual.an_incomplete_link_refuses_the_site
#print axioms QCore.ChainedTailActual.an_ambiguous_link_refuses_the_site
#print axioms QCore.ChainedTailActual.a_qualified_link_refuses_the_site
#print axioms QCore.ChainedTailActual.an_aggregate_link_refuses_the_site
#print axioms QCore.ChainedTailActual.a_non_continuable_intermediate_refuses_the_site
#print axioms QCore.ChainedTailActual.a_partial_chain_keeps_its_refusal
#print axioms QCore.ChainedTailActual.a_risky_name_still_refuses_under_the_declaration
#print axioms QCore.ChainedTailActual.every_admitted_actual_is_data

-- The soundness sentence, and the extension-not-replacement statement.
#print axioms QCore.ChainedTailActual.an_admitted_chain_names_no_bound_base
#print axioms QCore.ChainedTailActual.the_chain_half_is_exactly_the_borrowed_admission_at_depth_one

-- Bridging into the statement classification.
#print axioms QCore.ChainedTailActual.a_refused_site_refuses_the_statement
#print axioms QCore.ChainedTailActual.a_refused_site_is_never_safe
#print axioms QCore.ChainedTailActual.an_admitted_chain_does_not_rescue_the_statement
#print axioms QCore.ChainedTailActual.an_admitted_chain_beside_a_risky_sibling_still_refuses

-- Non-vacuity, with each premise separately flipped.
#print axioms QCore.ChainedTailActual.each_premise_is_load_bearing_on_the_subject
