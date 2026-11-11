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

import QCore.ElementDestroyer
#print axioms QCore.ElementDestroyer.a_publication_keeps_its_borrowed_entry
#print axioms QCore.ElementDestroyer.an_unlink_sits_beside_a_borrow_or_a_release_only
#print axioms QCore.ElementDestroyer.destroying_an_admitted_element_is_safe
#print axioms QCore.ElementDestroyer.destroying_an_unadmitted_element_is_refused
#print axioms QCore.ElementDestroyer.the_release_never_precedes_the_unlink
#print axioms QCore.ElementDestroyer.destroying_twice_is_never_safe
#print axioms QCore.ElementDestroyer.a_release_after_the_destroyer_is_a_violation
#print axioms QCore.ElementDestroyer.an_unlink_before_the_destroyer_is_refused
#print axioms QCore.ElementDestroyer.destroying_a_published_object_is_safe
#print axioms QCore.ElementDestroyer.a_destroying_body_is_withheld
#print axioms QCore.ElementDestroyer.the_modelled_destroy_is_an_unlink_then_a_release
#print axioms QCore.ElementDestroyer.the_modelled_destroy_of_an_admitted_element_is_safe
#print axioms QCore.ElementDestroyer.a_modelled_cleanup_unlinks_and_retires
