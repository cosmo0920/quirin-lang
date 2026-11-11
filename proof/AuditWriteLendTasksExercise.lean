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

import QCore.WriteLendTasksExercise

-- Card `WRITE-LEND-TASKS-EXERCISE`: the worker exercises the range write
-- lease card `WRITE-LEND-TASKS` could only transfer and enforce. Wave 31
-- proved that no byte has two write *authorities*; this surface is about the
-- authority being used — the rebasing law that decides which relative index a
-- holder may name, and the exclusivity statement re-proved about the writes
-- themselves rather than about the permissions.

-- The vocabulary the card adds, printed so that a later change to any of it
-- is visible in this surface rather than only inside a proof.
#print axioms QCore.WriteLendTasksExercise.rebase
#print axioms QCore.WriteLendTasksExercise.admitsIndex
#print axioms QCore.WriteLendTasksExercise.footprintFits
#print axioms QCore.WriteLendTasksExercise.admitsWrite
#print axioms QCore.WriteLendTasksExercise.addedSurface

-- **The rebasing law.** A rebased index is inside its lease exactly when it
-- is below the lease's extent — the lease's own `low` cancels, which is why a
-- worker's body can be checked without knowing where its lease sits. And the
-- spawn site's single arithmetic comparison is *equivalent* to the whole
-- family of per-index containment facts, which is what licenses checking the
-- summary once instead of every write at every creation.
#print axioms QCore.WriteLendTasksExercise.admits_index_iff_below_extent
#print axioms QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law

-- The refusal of an unresolvable index is not vacuous: a nonempty lease
-- always has indices it does not admit, so "provably inside" and "inside"
-- are genuinely different questions on this route.
#print axioms QCore.WriteLendTasksExercise.some_index_is_always_outside

-- **Exclusivity survives the exercise.** An admitted write lies inside a
-- lease its own writer holds; and in any reachable ledger state two admitted
-- writes to one byte are the same writer's — so two threads never write one
-- byte. The owner is covered by the whole-payload suspension.
#print axioms QCore.WriteLendTasksExercise.an_admitted_write_lies_inside_its_writers_lease
#print axioms QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties
#print axioms QCore.WriteLendTasksExercise.the_owner_writes_no_byte_while_any_lease_is_out

-- What was added, as one list, so a widening changes a theorem rather than
-- slipping past.
#print axioms QCore.WriteLendTasksExercise.the_added_operations_are_exactly_two
