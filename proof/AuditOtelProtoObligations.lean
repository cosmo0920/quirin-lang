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

import QCore.OtelProtoObligations

/-! Audited companion to OTEL-PROTO-AUDIT (the OTP block: protobuf-c unpack
facts, site obligations, site classes and the pinned protocol identity). -/

#print axioms QCore.OtelProtoObligations.formula_otp_absent_message_is_null
#print axioms QCore.OtelProtoObligations.formula_otp_repeated_elements_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_repeated_array_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_selected_member_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_string_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_bytes_data_iff_length
#print axioms QCore.OtelProtoObligations.guarded_iff_tested
#print axioms QCore.OtelProtoObligations.guarded_holds
#print axioms QCore.OtelProtoObligations.formula_otp_unpack_result_tested
#print axioms QCore.OtelProtoObligations.formula_otp_deref_needs_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_subscript_needs_bound
#print axioms QCore.OtelProtoObligations.formula_otp_member_needs_case
#print axioms QCore.OtelProtoObligations.formula_otp_data_needs_length
#print axioms QCore.OtelProtoObligations.formula_otp_enum_index_needs_name
#print axioms QCore.OtelProtoObligations.formula_otp_bounded_element_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_selected_member_guarded_nonnull
#print axioms QCore.OtelProtoObligations.formula_otp_classes_partition
#print axioms QCore.OtelProtoObligations.formula_otp_candidate_is_wire_and_unguarded
#print axioms QCore.OtelProtoObligations.formula_otp_unguarded_off_wire_undetermined
#print axioms QCore.OtelProtoObligations.formula_otp_guard_is_cited
#print axioms QCore.OtelProtoObligations.formula_otp_undetermined_names_reason
#print axioms QCore.OtelProtoObligations.formula_otp_no_unsafe_claimed
#print axioms QCore.OtelProtoObligations.formula_otp_identity_refused_unless_pinned
#print axioms QCore.OtelProtoObligations.formula_otp_pinned_identity_admitted
#print axioms QCore.OtelProtoObligations.the_tables_are_counted
