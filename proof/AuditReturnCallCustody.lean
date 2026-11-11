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

import QCore.ReturnCallCustody

-- Card `C-GATEWAY-RETURN-CALL-W4`: the custody content of a modeled call in
-- `return` position. Model of `crates/qcore-c-gateway/src/legacy_return_call.rs`
-- and the `Statement::Return` lowering in `legacy_c.rs`, answering the
-- soundness ground on which wave 33's `C-GATEWAY-RETURN-POSITION` withdrew this
-- sub-shape. Behavioural pins:
-- `crates/qcore-c-gateway/tests/return_position_modeled_call.rs` and the
-- site-counter row in `crates/qcore-c-gateway/tests/unit/legacy_c.rs`.
-- Record: `docs/reports/c-gateway-return-call-w4/README.md`.

-- The rule is a desugaring: the lowering IS the two-statement form the analyzer
-- already decides, which is why `observe_exit` needs no arm for a call result
-- and why the parameter modes are applied by the position that always applied
-- them.
#print axioms QCore.ReturnCallCustody.the_lowering_is_the_desugaring

-- The accusing direction: a `none` result escapes nothing, so an obligation
-- still held at that exit survives to `report_leaks`, which skips an unmodeled
-- exit and therefore used to hide it.
#print axioms QCore.ReturnCallCustody.a_none_result_escapes_nothing

-- Freshness is structural. The produced site is the counter's value, and the
-- counter's invariant says the pre-state records nothing about it.
#print axioms QCore.ReturnCallCustody.a_produced_site_is_absent_from_the_pre_state

-- The theorem the card rests on, and its sharper form for the one mode that can
-- produce custody: an admitted return discharges nothing the body already owed,
-- and under `owned` exactly one site moves — the one the call itself produced.
#print axioms QCore.ReturnCallCustody.admitted_modes_discharge_nothing_prior
#print axioms QCore.ReturnCallCustody.an_owned_result_escapes_only_the_site_it_produced

-- Non-vacuity of the refusal (constraint 15n's established shape): the
-- reallocating arm is refused because admitting it really would have discharged
-- the replaced parameter -- a site of the pre-state, and a LIVE one, both stated
-- in the theorem so the claim cannot be read as weaker than it is.
#print axioms QCore.ReturnCallCustody.a_reallocated_result_would_have_discharged_its_parameter

-- The gate refuses exactly that arm, which closes the finite mode vocabulary:
-- admitted if and only if nothing prior is discharged.
#print axioms QCore.ReturnCallCustody.only_reallocation_is_refused
