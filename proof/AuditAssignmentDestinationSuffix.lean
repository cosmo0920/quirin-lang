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

import QCore.AssignmentDestinationSuffix

/-! Audited Lean companion to card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE.

`direct_assignment_destination` named a left-hand side by its first
place-spelling suffix, so `recs [ i ] . name` was named `name` and a body that
leaks one object and frees another certified `Safe`. The repaired reader names
a suffix only when it is the whole left-hand side or follows a declarator
prefix; every other such left-hand side is refused by the lexical gate.

* **Refinement and preservation** — the repaired reader names nothing new, and
  an exact place or a declaration keeps its name.
* **What a name now means** — a named destination follows no selection,
  subscript, call or parenthesis.
* **The verdict direction** — a body the repaired classifier certifies was
  lowered exactly as before and was certified before.
* **Non-vacuity** — the card's fixture, `( * p ) - > f`, a declaration and a
  whole member place, and a two-location store model in which the old name
  releases the wrong object. -/

-- Refinement and preservation.
#print axioms QCore.AssignmentDestinationSuffix.the_repair_names_nothing_new
#print axioms QCore.AssignmentDestinationSuffix.firstFrom_self
#print axioms QCore.AssignmentDestinationSuffix.an_exact_place_keeps_its_name
#print axioms QCore.AssignmentDestinationSuffix.a_declaration_keeps_its_name
#print axioms QCore.AssignmentDestinationSuffix.the_readers_differ_only_on_sub_objects

-- What a name now means.
#print axioms QCore.AssignmentDestinationSuffix.firstFrom_ge
#print axioms QCore.AssignmentDestinationSuffix.a_named_destination_follows_no_selection

-- The verdict direction.
#print axioms QCore.AssignmentDestinationSuffix.newLower_eq_old_or_refused
#print axioms QCore.AssignmentDestinationSuffix.the_repair_never_promotes

-- Non-vacuity.
#print axioms QCore.AssignmentDestinationSuffix.the_old_reader_names_the_member_as_the_local
#print axioms QCore.AssignmentDestinationSuffix.the_repaired_reader_names_nothing
#print axioms QCore.AssignmentDestinationSuffix.the_fixture_is_a_sub_object_store
#print axioms QCore.AssignmentDestinationSuffix.the_fixture_is_refused
#print axioms QCore.AssignmentDestinationSuffix.a_pointer_declaration_is_still_named
#print axioms QCore.AssignmentDestinationSuffix.a_whole_member_place_is_still_named
#print axioms QCore.AssignmentDestinationSuffix.a_parenthesised_dereference_is_a_sub_object
#print axioms QCore.AssignmentDestinationSuffix.the_old_name_releases_the_wrong_object
