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

import QCore.FrameHelperTask

-- Card `P04-COMPOSE-FRAME`: the frame helper's verification task. Model of
-- `qcore_verification::frame_source`. The evaluator is unedited; what is
-- modelled here is task identity, fail-closed re-establishment, and the
-- composition of a derived task with the existing `QCore.ChunkSource`
-- frame-preservation theorems.
--
-- Outside every statement below, and recorded in
-- `docs/reports/remaining-p04-frame/README.md`: whether the C body Clang
-- parsed is the body that runs, cross-unit record ABI equality, live storage
-- and exclusive access. Those stay the pinned contract's declared premises and
-- the audit's own; the task publishes them as assumptions.

-- Every bound input is load-bearing in the model: moving any input alone
-- gives a different task. Rust derivation repeatability is tested separately.
#print axioms QCore.FrameHelperTask.equal_digests_are_equal_tasks
#print axioms QCore.FrameHelperTask.a_changed_summary_retires_the_task
#print axioms QCore.FrameHelperTask.a_changed_import_identity_retires_the_task
#print axioms QCore.FrameHelperTask.a_changed_contract_retires_the_task
#print axioms QCore.FrameHelperTask.a_changed_binding_retires_the_task
#print axioms QCore.FrameHelperTask.a_changed_profile_selection_retires_the_task
#print axioms QCore.FrameHelperTask.every_bound_input_is_load_bearing

-- The recorded boundary `P04-FRAME-OBLIGATION-ID`, proved rather than assumed:
-- the obligation identifier is blind to the bound inputs, and the three-field
-- triple is what separates two tasks the identifier cannot.
#print axioms QCore.FrameHelperTask.the_obligation_identifier_does_not_read_the_bound_inputs
#print axioms QCore.FrameHelperTask.the_triple_separates_what_the_identifier_cannot

-- Re-establishment is fail-closed. Identity makes a task nameable; it never
-- makes it discharged, and a refusing core evaluation is never established.
#print axioms QCore.FrameHelperTask.an_incomplete_identity_is_refused
#print axioms QCore.FrameHelperTask.a_foreign_identity_is_refused
#print axioms QCore.FrameHelperTask.a_refusing_evaluation_is_never_established
#print axioms QCore.FrameHelperTask.every_prerequisite_field_is_load_bearing
#print axioms QCore.FrameHelperTask.the_derived_identity_is_established

-- What `derive` decides about the frame itself, in both directions.
#print axioms QCore.FrameHelperTask.an_empty_write_set_is_not_derivable
#print axioms QCore.FrameHelperTask.an_empty_imported_protection_is_not_derivable
#print axioms QCore.FrameHelperTask.an_uncontained_evaluated_protection_is_not_derivable
#print axioms QCore.FrameHelperTask.derivable_implies_containment
#print axioms QCore.FrameHelperTask.a_derivable_helper_has_a_nonempty_write_set

-- The tie to the unedited `QCore.ChunkSource` frame-preservation theorems: a
-- derivable helper is admitted by `checkedFrameWrites` and preserves every
-- imported protected field, and the containment condition carries that
-- preservation to the fields the selected evaluator actually checks.
#print axioms QCore.FrameHelperTask.a_derivable_helper_preserves_every_imported_protected_field
#print axioms QCore.FrameHelperTask.a_derivable_helper_preserves_every_evaluated_protected_field
#print axioms QCore.FrameHelperTask.a_write_to_an_imported_protected_field_is_refused

-- Non-vacuity: the real `cio_error_reset` shape derives, preserves both
-- protected fields, and really writes its metadata field.
#print axioms QCore.FrameHelperTask.the_real_chunkio_helper_is_derivable
#print axioms QCore.FrameHelperTask.the_real_chunkio_helper_preserves_the_lock_field
#print axioms QCore.FrameHelperTask.the_real_chunkio_helper_preserves_the_busy_field
#print axioms QCore.FrameHelperTask.the_real_chunkio_helper_writes_its_metadata_field
