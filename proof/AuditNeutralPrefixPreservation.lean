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

import QCore.NeutralPrefixPreservation

/-! Card W4-RECURSIVE-SUMMARIES (wave 30): the N-prefix preservation
induction — the soundness argument of
`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` §4.3 stated compositionally
over a finite execution prefix of a call stack, completed and still-running
inner calls included. Per-frame validation under the all-neutral assumption
map propagates to the whole trace of every prefix; the same per-frame
discipline for an assumed *effect* constrains no divergent prefix (the
measured `purge`/`mint` counterexample, cut at a finite prefix) while a
returned prefix does discharge it, so divergence is the exit comparison's
only gap and the optimistic start is admissible at the bottom element and
nowhere above it. No placeholder, no project axiom. -/

-- The induction and its well-foundedness.
#print axioms QCore.NeutralPrefixPreservation.neutral_prefix_preservation
#print axioms QCore.NeutralPrefixPreservation.a_completed_call_is_a_strictly_shorter_sub_execution
#print axioms QCore.NeutralPrefixPreservation.validated_frames_satisfy_the_neutral_claim
#print axioms QCore.NeutralPrefixPreservation.a_validated_prefix_exists

-- The effectful contrast at the same granularity.
#print axioms QCore.NeutralPrefixPreservation.an_assumed_effect_appears_in_no_divergent_prefix
#print axioms QCore.NeutralPrefixPreservation.a_returned_prefix_discharges_the_exit_claim
