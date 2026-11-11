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

import QCore.RefusedWrites

/-! Card AOM-BINDING-POISON: what a refused statement destroys in the binding
environment.

Card AOM-DESTROY-SIBLINGS recorded, as the open residual
`AOM-SIBLINGS-BINDING-POISON`, that every refused statement set `unknown` and
`may_be_null` on **every** binding in the state, so an acquisition's name did
not survive an unrelated refused store and twelve declared releases could only
refuse. This audit covers the repair, in both directions.

* **Nothing was replaced** — the pre-repair blanket poison is the `unbounded`
  case of the new rule, on every state, and so is a bounded refusal that spares
  nothing.
* **The three premises of a spared place** — a plain name, frame-protected, and
  not written by the refused text — stated as a conjunction and then as five
  separate refusals: a written name, a `static` local, a file-scope object, an
  address-taken name and a name the reader never recognised are each poisoned
  whatever the other two premises say. Member and out-parameter places are never
  spared at all.
* **Monotonicity** — the repair can only *remove* poison, never add it, and a
  weaker frame spares strictly less, which is what makes every fail-closed
  default (an unreadable body yields the empty frame) safe by construction.
* **The refusal itself is untouched** — a function carrying a refusal never
  certifies, and the poison moves no resource state in either direction. These
  are the two facts that make the repair unable to promote anything.
* **Non-vacuity** — card AOM-DESTROY-SIBLINGS' own fixture, decided both ways:
  `result` survives `result[index].id = read_id;` and the address-taken
  aggregate beside it does not.

The readers' faithfulness — that the declaration runs, the static-storage names,
the file-scope objects, the address-of operands and the write positions are
extracted correctly from preprocessed C — is a premise here, exactly as in
`QCore.PublicationCustody` and `QCore.BorrowedActual`, as is the
no-undefined-behaviour reading of the C object model the frame premise rests on. -/

-- Nothing was replaced.
#print axioms QCore.RefusedWrites.unbounded_is_the_blanket_poison
#print axioms QCore.RefusedWrites.an_empty_sparing_list_is_also_the_blanket_poison

-- The three premises of a spared place.
#print axioms QCore.RefusedWrites.spared_places_are_unwritable
#print axioms QCore.RefusedWrites.a_selection_place_is_never_spared
#print axioms QCore.RefusedWrites.an_indirect_place_is_never_spared
#print axioms QCore.RefusedWrites.a_written_name_is_never_spared
#print axioms QCore.RefusedWrites.a_static_local_is_never_spared
#print axioms QCore.RefusedWrites.a_file_scope_object_is_never_spared
#print axioms QCore.RefusedWrites.an_address_taken_name_is_never_spared
#print axioms QCore.RefusedWrites.an_unrecognised_name_is_never_spared

-- Monotonicity.
#print axioms QCore.RefusedWrites.the_repair_only_removes_poison
#print axioms QCore.RefusedWrites.a_weaker_frame_spares_less

-- The refusal itself is untouched.
#print axioms QCore.RefusedWrites.a_refusal_never_certifies
#print axioms QCore.RefusedWrites.the_poison_moves_no_resource_state

-- Non-vacuity, on the card's own fixture.
#print axioms QCore.RefusedWrites.the_acquisition_survives_the_identifier_loop
#print axioms QCore.RefusedWrites.the_address_taken_aggregate_does_not
#print axioms QCore.RefusedWrites.the_written_member_is_still_poisoned
#print axioms QCore.RefusedWrites.the_fixture_state_spares_exactly_one_binding
