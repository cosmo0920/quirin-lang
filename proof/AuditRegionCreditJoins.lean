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

import QCore.RegionCreditJoins

-- Card `REGION-CREDIT-JOINS` (wave 28): the composition of a flat-epoch credit
-- plan with the quantitative error region — CT22's scenario, "quant opening
-- discards parent-local unused credit after joins", made expressible on
-- `source.epoch.tasks.v1`.

-- The composed body's admitted shapes, as one list, and its embedding into the
-- sequential route's admitted body.
#print axioms QCore.RegionCreditJoins.admitted_parent_shapes_are_exactly_these

-- The pack is parent-local: a body of quantities accumulates no cleanup duty,
-- so the sealed pack holds only the remainder.
#print axioms QCore.RegionCreditJoins.a_parent_local_body_builds_no_duty
#print axioms QCore.RegionCreditJoins.the_pack_after_joins_holds_only_the_quantity

-- The CT22 property: the discharge restores the baseline, runs no action,
-- abandons exactly the parent-local remainder, and consumes the capability;
-- the empty-pack boundary still consumes.
#print axioms QCore.RegionCreditJoins.the_opening_abandons_exactly_the_parents_remainder
#print axioms QCore.RegionCreditJoins.discharging_an_empty_pack_still_consumes

-- The erasure: the composed region contributes no acquisition and no release
-- to the task program — the modelled form of the byte-identical emitted C.
#print axioms QCore.RegionCreditJoins.a_parent_local_body_emits_no_acquisition
#print axioms QCore.RegionCreditJoins.a_parent_local_body_emits_no_release
#print axioms QCore.RegionCreditJoins.the_region_erases_from_the_task_program

-- The re-derived quantity: the reader's fold and the model's remainder are one
-- function on every body (the driver's cross-check is sound), the overspend
-- still refuses, and the flagship's own figures are non-vacuous.
#print axioms QCore.RegionCreditJoins.the_readers_fold_is_the_models_remainder
#print axioms QCore.RegionCreditJoins.overspending_refuses_here
#print axioms QCore.RegionCreditJoins.the_flagship_abandons_three
#print axioms QCore.RegionCreditJoins.the_boundary_abandons_nothing

-- After the joins, no child holds anything: a spawn answered by its join
-- leaves the outstanding set unchanged, and the sequential spawn-then-join
-- discipline the checked scoped-task ledger enforces leaves no child at the
-- opening.
#print axioms QCore.RegionCreditJoins.a_join_returns_its_spawn
#print axioms QCore.RegionCreditJoins.the_sequential_spawn_join_discipline_leaves_no_child
