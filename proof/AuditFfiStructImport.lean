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

import QCore.FfiStructImport

-- Card `W48-QR-C-STRUCT-IMPORT`: an imported C struct's listing against the
-- recorded compiler's layout, and the file-wide unsafe-FFI assumption.

-- Layout agreement: every listed field is a recorded member, at its recorded
-- offset, of a type the member admits; every admitted read is there; the
-- listing is in layout order.
#print axioms QCore.FfiStructImport.checked_fields_agree
#print axioms QCore.FfiStructImport.checked_names
#print axioms QCore.FfiStructImport.every_listed_read_is_at_the_recorded_offset
#print axioms QCore.FfiStructImport.checked_offsets_ascend

-- Opaque fields are unreachable.
#print axioms QCore.FfiStructImport.an_opaque_field_is_unreachable

-- No plain `Safe` from an FFI file, and the join keeps the assumption.
#print axioms QCore.FfiStructImport.an_ffi_file_never_publishes_plain_safe
#print axioms QCore.FfiStructImport.the_tripwire_rules_out_plain_safe
#print axioms QCore.FfiStructImport.an_ffi_publication_passes_the_tripwire
#print axioms QCore.FfiStructImport.join_keeps_the_assumption
#print axioms QCore.FfiStructImport.foldl_keeps
#print axioms QCore.FfiStructImport.every_composed_verdict_carries_the_assumption
#print axioms QCore.FfiStructImport.a_composition_with_an_ffi_part_is_never_plain_safe

-- Non-vacuity: the example's layout checks, and a mistyped, a reordered and an
-- unequivalent listing do not.
#print axioms QCore.FfiStructImport.the_example_listing_checks
#print axioms QCore.FfiStructImport.a_mistyped_listing_does_not_check
#print axioms QCore.FfiStructImport.a_reordered_listing_does_not_check
#print axioms QCore.FfiStructImport.an_unequivalent_member_cannot_be_listed
