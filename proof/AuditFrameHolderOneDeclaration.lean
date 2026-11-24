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

import QCore.FrameHolderOneDeclaration

/-! Card **FRAME-HOLDER-SHADOWED-NAME** (gateway rule 74o): a frame holder,
for a retention (rule 12t) or a callback registry, is the one automatic
object every reference of its name designates. Rule 74o's reading is a frame
holder only where rule 12t's was, so a `Safe` replay under it was `Safe`
before, and a shadowed name is refused. Which declaration a reference
designates is read off Clang (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/frame_holder_shadowed_name.rs`. -/

-- One declaration.
#print axioms QCore.FrameHolderOneDeclaration.a_frame_holder_was_one_before
#print axioms QCore.FrameHolderOneDeclaration.a_shadowed_name_is_not_a_frame_holder
#print axioms QCore.FrameHolderOneDeclaration.the_readings_agree_on_one_declaration
#print axioms QCore.FrameHolderOneDeclaration.a_shadowed_holder_is_undecided
-- Only refusals are added.
#print axioms QCore.FrameHolderOneDeclaration.step_mono
#print axioms QCore.FrameHolderOneDeclaration.lower_mono
#print axioms QCore.FrameHolderOneDeclaration.run_lowerAll_mono
#print axioms QCore.FrameHolderOneDeclaration.no_verdict_becomes_safe
-- The registry's frame key.
#print axioms QCore.FrameHolderOneDeclaration.a_registry_key_is_the_old_one_or_refused
#print axioms QCore.FrameHolderOneDeclaration.a_shadowed_registry_is_refused
-- Witness programs.
#print axioms QCore.FrameHolderOneDeclaration.probe_f1_was_safe_and_is_refused
#print axioms QCore.FrameHolderOneDeclaration.probe_f3_was_safe_and_is_refused
#print axioms QCore.FrameHolderOneDeclaration.probe_f4_was_a_frame_key_and_is_refused
#print axioms QCore.FrameHolderOneDeclaration.the_control_is_safe_under_both
