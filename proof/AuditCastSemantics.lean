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

import QCore.CastSemantics

/-! Audited companion to CAST-FORMALIZATION (gateway rule 12h). -/
#print axioms QCore.CastSemantics.function_object_refused
#print axioms QCore.CastSemantics.view_rows
#print axioms QCore.CastSemantics.classes_unique
#print axioms QCore.CastSemantics.agrees
#print axioms QCore.CastSemantics.view_preserves_identity
#print axioms QCore.CastSemantics.round_trip_same_pointer
#print axioms QCore.CastSemantics.pointer_to_integer_loses_provenance
#print axioms QCore.CastSemantics.tracked_pointer_to_integer_refused
#print axioms QCore.CastSemantics.integer_to_pointer_no_identity
#print axioms QCore.CastSemantics.null_pointer_no_identity
#print axioms QCore.CastSemantics.refused_classes_refuse
#print axioms QCore.CastSemantics.refused_rows
#print axioms QCore.CastSemantics.admitted_access_without_premise
#print axioms QCore.CastSemantics.view_access_needs_the_premise
#print axioms QCore.CastSemantics.punned_access_refused
#print axioms QCore.CastSemantics.cast_keeps_events
#print axioms QCore.CastSemantics.discard_is_effect_only
#print axioms QCore.CastSemantics.classify_vectors_hold
#print axioms QCore.CastSemantics.cast_transparent
#print axioms QCore.CastSemantics.star_through_cast_is_two
#print axioms QCore.CastSemantics.sr5_vectors_hold
