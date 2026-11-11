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

import QCore.TranslationValidation

#print axioms QCore.TranslationValidation.validation_preserves_pass_boundary
#print axioms QCore.TranslationValidation.validation_source_matches_cfg
#print axioms QCore.TranslationValidation.validation_cfg_matches_target
#print axioms QCore.TranslationValidation.validation_source_matches_target
#print axioms QCore.TranslationValidation.widened_legacy_state_is_never_safe
