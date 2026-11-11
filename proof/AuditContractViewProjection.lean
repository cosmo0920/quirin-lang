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

import QCore.ContractViewProjection

-- Card `CONTRACT-VIEW-PROJECTION-CHECK`: checked relations between the
-- declarations of one function in several contract modules. Model of
-- `qcore_contract::contract_views`; report
-- `docs/reports/contract-view-projection-check/README.md`.
--
-- Outside every statement: that the Lean clause models transcribe the `.qcore`
-- text (review), that the Rust enumeration decides the fragment exactly
-- (`CONTRACT-VIEW-SMALL-MODEL-UNPROVED`), and the premises each relation is
-- pinned as assuming.

-- Projection: the native-evaluated allocator contract entails the v1 view,
-- non-vacuously, and the mutated contracts the Rust controls use do not.
#print axioms QCore.ContractViewProjection.malloc_entails_owned_nullable
#print axioms QCore.ContractViewProjection.calloc_entails_owned_nullable
#print axioms QCore.ContractViewProjection.free_entails_released_nullable
#print axioms QCore.ContractViewProjection.malloc_contract_has_a_null_and_a_live_model
#print axioms QCore.ContractViewProjection.calloc_contract_has_a_live_model
#print axioms QCore.ContractViewProjection.the_mutated_malloc_contract_does_not_entail_the_view
#print axioms QCore.ContractViewProjection.the_mutated_free_contract_does_not_entail_the_view

-- Projection: the ChunkIO close lifecycles entail the v1 view; the memory
-- close contract does not outside its memory-backend premise (the registered
-- finding `CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`).
#print axioms QCore.ContractViewProjection.memory_close_entails_released_nonnull
#print axioms QCore.ContractViewProjection.releasing_only_the_name_does_not_entail_the_view
#print axioms QCore.ContractViewProjection.outside_the_memory_backend_the_view_is_not_implied
#print axioms QCore.ContractViewProjection.chunk_destructor_entails_released_nonnull

-- Renamed equivalence of the two `cio_error_reset` frame policies, its
-- condition, and that both real protected sets meet it.
#print axioms QCore.ContractViewProjection.the_frame_evaluators_agree_under_the_condition
#print axioms QCore.ContractViewProjection.the_condition_is_load_bearing
#print axioms QCore.ContractViewProjection.both_real_protected_sets_satisfy_the_condition

-- Agreement of recorded C signatures with typed declarations.
#print axioms QCore.ContractViewProjection.an_arity_mismatch_is_refused
#print axioms QCore.ContractViewProjection.the_monkey_wrapper_signatures_agree
#print axioms QCore.ContractViewProjection.a_mistagged_pointer_is_refused
#print axioms QCore.ContractViewProjection.a_const_pointer_declared_mutable_is_refused

-- The census guard, and why dropping premises is sound.
#print axioms QCore.ContractViewProjection.an_unregistered_pair_is_refused
#print axioms QCore.ContractViewProjection.a_stale_registration_is_refused
#print axioms QCore.ContractViewProjection.an_exactly_registered_census_is_accepted
#print axioms QCore.ContractViewProjection.dropping_premises_is_sound
