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

import QCore.OutParameterScalarFramePremise

/-! Audited companion to OUT-PARAMETER-SCALAR-FRAME-PREMISE (gateway rule 11r(7)). -/

#print axioms QCore.OutParameterScalarFramePremise.wide_store_keeps_pointer_object
#print axioms QCore.OutParameterScalarFramePremise.character_store_may_write
#print axioms QCore.OutParameterScalarFramePremise.no_premise_may_write
#print axioms QCore.OutParameterScalarFramePremise.spares_sound
#print axioms QCore.OutParameterScalarFramePremise.character_refuses
#print axioms QCore.OutParameterScalarFramePremise.floating_refuses
#print axioms QCore.OutParameterScalarFramePremise.opaque_refuses
#print axioms QCore.OutParameterScalarFramePremise.no_strict_aliasing_refuses
