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

import QCore.NativeLibraryContract

-- Card `NATIVE-LIBRARY-CONTRACT-SLICE`: the contracted half of the
-- `source.native-library.v1` manifest — level L3 of the linkage spectrum, where
-- a library's pointer symbols belong. Model of the parts this card added to
-- `crates/qcore-core/src/native_library.rs` and `extern_call.rs`; the manifest's
-- concrete syntax, its total-or-reject loader, the driver's diagnostics and the
-- CLI's premise printing are separate Rust boundaries, recorded in
-- `docs/reports/wave20-native-library-contract-slice/README.md`.

-- **The premises stack, and the contract premise is never replaced.** A
-- manifest-bound contracted call publishes what the code does *and* which code
-- it is; an unmanifested one is byte-unchanged; and no admitted outcome rests on
-- nothing.
#print axioms QCore.NativeLibraryContract.an_unmanifested_contract_keeps_its_one_premise
#print axioms QCore.NativeLibraryContract.a_manifest_bound_call_stacks_both_premises
#print axioms QCore.NativeLibraryContract.the_contract_premise_is_never_replaced
#print axioms QCore.NativeLibraryContract.there_is_no_premise_free_path
#print axioms QCore.NativeLibraryContract.a_resident_call_contributes_exactly_two_premises
#print axioms QCore.NativeLibraryContract.the_artifact_premise_names_the_contract_and_the_build
#print axioms QCore.NativeLibraryContract.a_contracted_premise_waives_nothing
#print axioms QCore.NativeLibraryContract.a_contracted_residence_names_its_module
#print axioms QCore.NativeLibraryContract.a_contract_less_residence_names_none

-- **Withdrawal, and why the direction differs from level L1.** A contradicted
-- binding withdraws; the withdrawal admits nothing and publishes nothing; and a
-- manifest can only ever remove an admission. The closing theorem states the one
-- rule both cards obey — withdraw only where the weaker reading refuses — by
-- exhibiting the two weaker readings side by side.
#print axioms QCore.NativeLibraryContract.a_contradicted_binding_withdraws
#print axioms QCore.NativeLibraryContract.a_withdrawal_admits_nothing
#print axioms QCore.NativeLibraryContract.a_withdrawal_publishes_no_premise
#print axioms QCore.NativeLibraryContract.the_unmanifested_reading_is_always_admitted
#print axioms QCore.NativeLibraryContract.a_manifest_can_only_withdraw
#print axioms QCore.NativeLibraryContract.withdrawal_only_removes_admissions
#print axioms QCore.NativeLibraryContract.the_weaker_reading_refuses_here_and_is_admitted_at_level_one

-- **Binding totality, in both directions of design §4.1**, and the measured
-- non-addition that follows from it: a manifest-contracted symbol is always
-- declared by the environment, so the waiver mirror is a theorem of the existing
-- gates rather than a fourth diagnostic code.
#print axioms QCore.NativeLibraryContract.an_unresolved_entry_is_not_total
#print axioms QCore.NativeLibraryContract.an_entry_in_another_module_is_not_total
#print axioms QCore.NativeLibraryContract.an_unbound_contract_in_the_slice_is_not_total
#print axioms QCore.NativeLibraryContract.a_declaration_outside_the_slice_is_not_demanded
#print axioms QCore.NativeLibraryContract.a_matching_slice_is_total
#print axioms QCore.NativeLibraryContract.a_manifest_contracted_symbol_is_declared_by_the_environment

-- **Closed allocator custody** (design §4.4), the check that was vacuous while
-- every entry was contract-less and becomes real here: a slice the check admits,
-- and that can acquire at all, contains somewhere to discharge.
#print axioms QCore.NativeLibraryContract.an_admitted_slice_discharges_what_it_can_acquire
#print axioms QCore.NativeLibraryContract.an_open_custody_slice_is_refused
#print axioms QCore.NativeLibraryContract.a_discharging_slice_is_closed
#print axioms QCore.NativeLibraryContract.a_slice_that_cannot_acquire_is_closed

-- Non-vacuity: the route admits something reachable, it is
-- exactly where a pointer symbol belongs, and an unbound symbol keeps the single
-- premise beside it.
#print axioms QCore.NativeLibraryContract.a_pointer_symbol_binds_here_and_never_as_a_waiver
#print axioms QCore.NativeLibraryContract.the_contract_slice_route_admits_and_stacks
#print axioms QCore.NativeLibraryContract.an_unbound_symbol_keeps_the_single_premise
