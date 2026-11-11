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

import QCore.EventRemovalWrapper

#print axioms QCore.EventRemovalWrapper.disable_ends_a_fresh_exact_registration
#print axioms QCore.EventRemovalWrapper.disable_keeps_an_already_poisoned_registration
#print axioms QCore.EventRemovalWrapper.disable_does_not_destroy_its_holder
