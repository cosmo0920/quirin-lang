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

import QCore.ExternCall
import QCore.ModuleImports

-- Card `source.extern-call.v1`, extended by card `EXTERN-F1` with the
-- `resource_out` and `resource_stored` call modes and by card `EXTERN-MODES`
-- with `resource_released` and `resource_return_reallocated`. Model of
-- `crates/qcore-core/src/extern_call.rs`; the source lowering, the C backend
-- and the recorded native evidence are separate Rust boundaries, recorded in
-- `docs/reports/extern-call-v1/README.md`,
-- `docs/reports/extern-call-f1/README.md` and
-- `docs/reports/remaining-extern-modes/README.md`.

-- Refusal completeness: every resource-contract v1 mode still outside the
-- admitted table. `out_parameter_refuses` and `stored_parameter_refuses` are
-- deliberately retired by `EXTERN-F1`, which admits those two modes, and
-- `released_parameter_refuses` and `reallocated_return_refuses` by
-- `EXTERN-MODES`; each is replaced below by the narrower refusals that survive.
#print axioms QCore.ExternCall.nullable_released_parameter_refuses
#print axioms QCore.ExternCall.releasing_other_than_a_flat_mutable_pointer_refuses
#print axioms QCore.ExternCall.removed_parameter_refuses
#print axioms QCore.ExternCall.nullable_consumed_parameter_refuses
#print axioms QCore.ExternCall.scalar_custody_refuses
#print axioms QCore.ExternCall.borrowing_a_pointer_to_pointer_refuses
#print axioms QCore.ExternCall.consuming_a_const_pointer_refuses
#print axioms QCore.ExternCall.nonnull_owned_return_refuses
#print axioms QCore.ExternCall.nonnull_reallocated_return_refuses
#print axioms QCore.ExternCall.a_reallocation_subject_out_of_range_refuses
#print axioms QCore.ExternCall.a_reallocation_subject_that_is_not_owned_refuses
#print axioms QCore.ExternCall.a_reallocated_result_on_other_than_a_flat_mutable_pointer_refuses
#print axioms QCore.ExternCall.a_nullable_reallocation_subject_refuses
#print axioms QCore.ExternCall.a_reallocation_subject_on_other_than_a_flat_mutable_pointer_refuses
#print axioms QCore.ExternCall.owned_result_requires_a_mutable_pointer

-- `resource_owned` is admitted only where a `resource_stored` parameter
-- designates it, which is the position contract spec §C04.2 forces.
#print axioms QCore.ExternCall.an_owned_parameter_outside_owner_position_refuses
#print axioms QCore.ExternCall.owned_parameter_alone_refuses
#print axioms QCore.ExternCall.a_nullable_owner_refuses
#print axioms QCore.ExternCall.an_owner_on_other_than_a_flat_mutable_pointer_refuses

-- `resource_out`: admitted nullable, on the pointer-to-pointer projection, at
-- most one per declaration and never beside an owned return.
#print axioms QCore.ExternCall.a_nonnull_out_parameter_refuses
#print axioms QCore.ExternCall.an_out_parameter_on_other_than_the_pointer_to_pointer_projection_refuses
#print axioms QCore.ExternCall.two_out_slots_refuse
#print axioms QCore.ExternCall.an_out_slot_beside_an_owned_return_refuses

-- `resource_stored`: admitted nonnull, on a flat mutable pointer, beneath a
-- distinct declared owner, and with no acquisition in the same call.
#print axioms QCore.ExternCall.a_nullable_stored_parameter_refuses
#print axioms QCore.ExternCall.a_stored_parameter_on_other_than_a_flat_mutable_pointer_refuses
#print axioms QCore.ExternCall.a_stored_parameter_beneath_itself_refuses
#print axioms QCore.ExternCall.a_stored_parameter_beneath_a_non_owned_parameter_refuses
#print axioms QCore.ExternCall.a_stored_parameter_beside_an_owned_return_refuses

-- The admitted tables are not empty, so those refusals are not vacuous.
#print axioms QCore.ExternCall.the_admitted_table_is_inhabited
#print axioms QCore.ExternCall.the_out_table_is_inhabited
#print axioms QCore.ExternCall.the_store_table_is_inhabited

-- Summary derivation is a function of the contract, so a certificate consumer
-- can repeat it instead of believing it.
#print axioms QCore.ExternCall.admission_is_deterministic
#print axioms QCore.ExternCall.an_out_slot_carries_no_caller_custody
#print axioms QCore.ExternCall.a_store_and_its_owner_both_carry_custody

-- The derived effect row (design §3.4), with both acquisition sources.
#print axioms QCore.ExternCall.every_derived_row_contains_io
#print axioms QCore.ExternCall.alloc_is_derived_exactly_for_an_acquisition
#print axioms QCore.ExternCall.alloc_is_derived_for_an_out_slot
#print axioms QCore.ExternCall.mut_is_derived_exactly_for_a_mutating_parameter
#print axioms QCore.ExternCall.a_const_loan_of_a_scalar_call_derives_no_mutation
#print axioms QCore.ExternCall.an_empty_declared_row_admits_no_extern_call
#print axioms QCore.ExternCall.a_row_without_alloc_refuses_an_acquiring_call
#print axioms QCore.ExternCall.a_row_without_alloc_refuses_an_out_writing_call

-- Adapter totality: an acquiring call has exactly two exits, whichever source
-- establishes the custody (§3.3).
#print axioms QCore.ExternCall.adapter_totality
#print axioms QCore.ExternCall.a_data_call_never_binds_an_identity
#print axioms QCore.ExternCall.a_data_call_never_takes_the_error_arm
#print axioms QCore.ExternCall.a_store_binds_no_identity
#print axioms QCore.ExternCall.a_failed_acquisition_binds_nothing
#print axioms QCore.ExternCall.a_successful_acquisition_binds_exactly_the_fresh_identity
#print axioms QCore.ExternCall.an_out_write_is_an_acquisition_only_on_success

-- Conservation: applying a derived summary changes the ledger by exactly the
-- declared delta, given the contract premise.
#print axioms QCore.ExternCall.the_custody_delta_is_exactly_the_declared_one
#print axioms QCore.ExternCall.a_departing_identity_does_not_survive
#print axioms QCore.ExternCall.a_consumed_identity_does_not_survive
#print axioms QCore.ExternCall.a_stored_identity_does_not_survive
#print axioms QCore.ExternCall.the_owner_survives_a_store

-- Card `EXTERN-ERR-REL`: the adapter's error-arm disposition discipline.
-- Conservation first — under an admitted arm every live identity is released,
-- and an identity that is not is refused. The v1 abandonment refusal is
-- narrowed to empty arms, not deleted.
#print axioms QCore.ExternCall.error_arm_conservation
#print axioms QCore.ExternCall.an_abandoned_identity_refuses
#print axioms QCore.ExternCall.an_admitted_error_exit_is_empty
#print axioms QCore.ExternCall.an_empty_arm_with_live_custody_refuses
#print axioms QCore.ExternCall.an_empty_arm_with_nothing_live_is_admitted

-- A disposition is a release and nothing else, exactly once per live identity,
-- in the reverse acquisition order the established cleanup edge performs.
#print axioms QCore.ExternCall.an_acquiring_disposition_refuses
#print axioms QCore.ExternCall.a_storing_disposition_refuses
#print axioms QCore.ExternCall.an_arm_of_the_wrong_length_refuses
#print axioms QCore.ExternCall.a_repeated_disposition_refuses
#print axioms QCore.ExternCall.a_disposition_of_a_departed_identity_refuses
#print axioms QCore.ExternCall.an_arm_in_the_wrong_order_refuses
#print axioms QCore.ExternCall.two_dispositions_must_run_in_reverse_acquisition_order

-- The arm is a path: the success arm keeps everything, and totality survives.
#print axioms QCore.ExternCall.the_success_arm_keeps_every_dispositioned_identity
#print axioms QCore.ExternCall.adapter_with_arm_totality

-- STORE-REACH, non-vacuity: the second acquisition an arm makes admissible is
-- exactly what the admitted store needs, and is still refused without the arm.
#print axioms QCore.ExternCall.the_release_callee_is_a_disposition
#print axioms QCore.ExternCall.the_attach_callee_is_not_a_disposition
#print axioms QCore.ExternCall.a_second_acquisition_is_admitted_when_its_arm_releases_the_first
#print axioms QCore.ExternCall.the_same_acquisition_with_an_empty_arm_refuses
#print axioms QCore.ExternCall.the_store_is_reachable_from_two_live_identities

-- Card `EXTERN-MODES`. `resource_released` is admitted nonnull on a flat
-- mutable pointer and is the mode the error arm was always describing;
-- `resource_return_reallocated` is admitted nullable over an `Owned` subject
-- and is the one departing mode whose failure arm the declaration states.
#print axioms QCore.ExternCall.the_release_table_is_inhabited
#print axioms QCore.ExternCall.the_reallocation_table_is_inhabited
#print axioms QCore.ExternCall.a_reallocation_beside_an_out_slot_refuses
#print axioms QCore.ExternCall.a_reallocating_callee_acquires
#print axioms QCore.ExternCall.alloc_is_derived_for_a_reallocation
#print axioms QCore.ExternCall.a_row_without_alloc_refuses_a_reallocating_call
#print axioms QCore.ExternCall.a_release_carries_its_own_custody
#print axioms QCore.ExternCall.a_reallocation_subject_carries_its_own_custody
#print axioms QCore.ExternCall.the_free_callee_is_a_disposition
#print axioms QCore.ExternCall.an_arm_rendered_by_a_declared_release_is_admitted
#print axioms QCore.ExternCall.reallocation_adapter_totality
#print axioms QCore.ExternCall.a_removed_identity_is_gone
#print axioms QCore.ExternCall.a_successful_reallocation_replaces_its_subject
#print axioms QCore.ExternCall.a_failed_reallocation_binds_nothing
#print axioms QCore.ExternCall.a_preserved_subject_is_released_by_the_arm
#print axioms QCore.ExternCall.a_failed_reallocation_whose_arm_ignores_the_subject_refuses
#print axioms QCore.ExternCall.a_departing_identity_is_absent_where_a_subject_is_present
#print axioms QCore.ExternCall.a_reallocation_round_trip_is_admitted
#print axioms QCore.ExternCall.a_failed_reallocation_round_trip_is_admitted
#print axioms QCore.ExternCall.a_failed_reallocation_round_trip_with_an_empty_arm_refuses

-- Card `PIPE-SCALAR-RESULT`: binding a non-acquiring call's scalar result.
-- Refusal completeness first — every acquisition source, `unit` and every
-- pointer keep the refusal they had, so the mandatory adapter remains the one
-- spelling of an acquisition.
#print axioms QCore.ExternCall.an_owned_return_binds_no_scalar
#print axioms QCore.ExternCall.a_reallocating_return_binds_no_scalar
#print axioms QCore.ExternCall.an_out_writing_call_binds_no_scalar
#print axioms QCore.ExternCall.an_acquiring_callee_binds_no_scalar
#print axioms QCore.ExternCall.a_unit_result_binds_no_scalar
#print axioms QCore.ExternCall.a_pointer_result_binds_no_scalar
#print axioms QCore.ExternCall.refused_scalar_bindings_are_exactly_these

-- The admitted table is not empty, and it is reached through admission rather
-- than only postulated.
#print axioms QCore.ExternCall.the_scalar_binding_table_is_inhabited
#print axioms QCore.ExternCall.a_bridged_scalar_returning_contract_binds_its_result

-- Non-interference: the value namespace and the ledger do not meet. This is
-- "a scalar binding must not launder a resource identity", proved.
#print axioms QCore.ExternCall.a_scalar_binding_leaves_the_ledger_unchanged
#print axioms QCore.ExternCall.a_scalar_binding_has_the_discarding_calls_ledger
#print axioms QCore.ExternCall.a_scalar_binding_introduces_no_identity
#print axioms QCore.ExternCall.a_scalar_binding_never_names_a_live_identity
#print axioms QCore.ExternCall.a_scalar_binding_over_a_live_identity_refuses
#print axioms QCore.ExternCall.an_acquisition_has_no_scalar_binding_step
#print axioms QCore.ExternCall.a_scalar_actual_carries_no_custody
#print axioms QCore.ExternCall.a_scalar_chain_leaves_one_live_identity

-- The file-local import gate is a second, independent opt-in.
#print axioms QCore.ModuleImports.empty_imports_grant_no_extern_calls
#print axioms QCore.ModuleImports.admitted_extern_calls_require_the_extern_import
#print axioms QCore.ModuleImports.the_concurrency_import_alone_grants_no_extern_call
#print axioms QCore.ModuleImports.sequential_code_needs_no_extern_import
#print axioms QCore.ModuleImports.both_imports_are_independent
