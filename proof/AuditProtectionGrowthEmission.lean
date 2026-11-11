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

import QCore.ProtectionGrowthEmission

/-! Card CT34-SANITIZED: rendering an admitted protection growth.

Card CT34-GROWTH's boundary `CT34-GROWTH-NATIVE` recorded that the C backend
refused every `GrowProtection` event, so the growth route was check-only. This
module models the decision that replaces that refusal, and this audit covers it
in both directions.

* **What the rendering is.** A footprint is a declared shape and a C mutex has
  no extent to widen, so a growth performs no mutex operation at all: the mutex
  operations of a rendering are exactly the ones the trace's own events call for,
  and deleting every growth from a trace moves none of them.
* **The two renderings agree.** The emitted program's observation is the
  reference execution, on every trace, refusals included — the model of the
  line-for-line comparison the CT harness performs on a real compiled and
  sanitizer-run program. That a `printf` prints its own argument is the model's
  one premise about C.
* **Fail-closed, which is the half the old boundary leaves standing.** Only a
  growth the supplied certificate admits renders: an unminted guard, an absent
  record, an edited record and a published shape that hides the growth each
  refuse, a refusal anywhere refuses the whole program, and a growth that did
  render was admitted and rendered as exactly one bookkeeping line naming the
  generation the trace really locked.
* **Independence.** A trace that never grows renders the same under every
  certificate, so this admission cannot move a module that does not use it.
* **Decidable scenarios** over the CT34 sanitized control's own trace: it
  renders, its observation is pinned in full, it performs exactly six mutex
  operations, and five certificate edits each refuse it.

The model does not claim that a C mutex protects the grown region; no `mtx_t`
can express an extent. It says what the emitted program does, and what the two
renderings agree on. -/

-- The rendering, and what a growth costs the mutex
#print axioms QCore.ProtectionGrowthEmission.a_growth_statement_performs_no_mutex_operation
#print axioms QCore.ProtectionGrowthEmission.step_operations_are_the_events_own
#print axioms QCore.ProtectionGrowthEmission.operations_are_exactly_the_traces_own_mutex_calls
#print axioms QCore.ProtectionGrowthEmission.growth_contributes_no_mutex_operation

-- The emitted program and the reference execution agree
#print axioms QCore.ProtectionGrowthEmission.step_renderings_agree
#print axioms QCore.ProtectionGrowthEmission.emitted_program_observes_the_reference
#print axioms QCore.ProtectionGrowthEmission.emitted_and_reference_renderings_agree

-- Only an admitted growth renders
#print axioms QCore.ProtectionGrowthEmission.growth_requires_a_minted_guard
#print axioms QCore.ProtectionGrowthEmission.unadmitted_growth_is_refused
#print axioms QCore.ProtectionGrowthEmission.growth_requires_a_record_at_this_index
#print axioms QCore.ProtectionGrowthEmission.growth_requires_the_published_shape
#print axioms QCore.ProtectionGrowthEmission.head_refusal_refuses_the_program
#print axioms QCore.ProtectionGrowthEmission.tail_refusal_refuses_the_program
#print axioms QCore.ProtectionGrowthEmission.a_refused_growth_refuses_the_whole_program
#print axioms QCore.ProtectionGrowthEmission.a_rendered_growth_was_admitted

-- A module that never grows is untouched
#print axioms QCore.ProtectionGrowthEmission.a_growthless_trace_renders_independently_of_the_certificate

-- The CT34 sanitized control's own trace
#print axioms QCore.ProtectionGrowthEmission.the_fixture_renders_and_agrees
#print axioms QCore.ProtectionGrowthEmission.the_fixture_observation_is_pinned
#print axioms QCore.ProtectionGrowthEmission.the_fixture_performs_six_mutex_operations
#print axioms QCore.ProtectionGrowthEmission.deleting_the_growth_moves_no_mutex_operation
#print axioms QCore.ProtectionGrowthEmission.the_fixture_with_no_admitted_growth_is_refused
#print axioms QCore.ProtectionGrowthEmission.the_fixture_with_a_relocated_record_is_refused
#print axioms QCore.ProtectionGrowthEmission.the_fixture_with_an_edited_record_is_refused
#print axioms QCore.ProtectionGrowthEmission.the_fixture_with_a_hidden_shape_is_refused
#print axioms QCore.ProtectionGrowthEmission.a_growth_under_an_unminted_guard_is_refused
