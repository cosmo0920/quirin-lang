import QCore.ExternCall
import QCore.ModuleImports

-- Card `source.extern-call.v1`: the bounded extern-call model. Model of
-- `crates/qcore-core/src/extern_call.rs`; the source lowering, the C backend
-- and the recorded native evidence are separate Rust boundaries, recorded in
-- `docs/reports/extern-call-v1/README.md`.

-- Refusal completeness: every resource-contract v1 mode outside design §3.1.
#print axioms QCore.ExternCall.owned_parameter_refuses
#print axioms QCore.ExternCall.released_parameter_refuses
#print axioms QCore.ExternCall.out_parameter_refuses
#print axioms QCore.ExternCall.stored_parameter_refuses
#print axioms QCore.ExternCall.removed_parameter_refuses
#print axioms QCore.ExternCall.nullable_consumed_parameter_refuses
#print axioms QCore.ExternCall.scalar_custody_refuses
#print axioms QCore.ExternCall.borrowing_a_pointer_to_pointer_refuses
#print axioms QCore.ExternCall.consuming_a_const_pointer_refuses
#print axioms QCore.ExternCall.nonnull_owned_return_refuses
#print axioms QCore.ExternCall.reallocated_return_refuses
#print axioms QCore.ExternCall.owned_result_requires_a_mutable_pointer

-- The admitted table is not empty, so those refusals are not vacuous.
#print axioms QCore.ExternCall.the_admitted_table_is_inhabited

-- Summary derivation is a function of the contract, so a certificate consumer
-- can repeat it instead of believing it.
#print axioms QCore.ExternCall.admission_is_deterministic

-- The derived effect row (design §3.4).
#print axioms QCore.ExternCall.every_derived_row_contains_io
#print axioms QCore.ExternCall.alloc_is_derived_exactly_for_an_owned_return
#print axioms QCore.ExternCall.mut_is_derived_exactly_for_a_mutating_parameter
#print axioms QCore.ExternCall.a_const_loan_of_a_scalar_call_derives_no_mutation
#print axioms QCore.ExternCall.an_empty_declared_row_admits_no_extern_call
#print axioms QCore.ExternCall.a_row_without_alloc_refuses_an_acquiring_call

-- Adapter totality: a nullable owned return has exactly two exits (§3.3).
#print axioms QCore.ExternCall.adapter_totality
#print axioms QCore.ExternCall.a_data_call_never_binds_an_identity
#print axioms QCore.ExternCall.a_data_call_never_takes_the_error_arm
#print axioms QCore.ExternCall.a_failed_acquisition_binds_nothing
#print axioms QCore.ExternCall.a_successful_acquisition_binds_exactly_the_fresh_identity

-- Conservation: applying a derived summary changes the ledger by exactly the
-- declared delta, given the contract premise.
#print axioms QCore.ExternCall.the_custody_delta_is_exactly_the_declared_one
#print axioms QCore.ExternCall.a_consumed_identity_does_not_survive

-- The file-local import gate is a second, independent opt-in.
#print axioms QCore.ModuleImports.empty_imports_grant_no_extern_calls
#print axioms QCore.ModuleImports.admitted_extern_calls_require_the_extern_import
#print axioms QCore.ModuleImports.the_concurrency_import_alone_grants_no_extern_call
#print axioms QCore.ModuleImports.sequential_code_needs_no_extern_import
#print axioms QCore.ModuleImports.both_imports_are_independent
