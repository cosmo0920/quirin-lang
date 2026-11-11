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

import QCore.SubObjectStore

/-! Audited Lean companion to card W45-SUB-OBJECT-STORES.

A store into a sub-object (`R->F`, `R[I].F`, `(R)->F`, `S.F`, `(&S)->F`,
`A[I].F`) that the lexical gate refused is lowered, through a resolved typed
base, to a store decided on the state at the node; every failing premise is the
gate's refusal.

* **What an admitted store leaves** — the member forgotten, no other name
  rebound (constraint 5i), and no status, marker or custody map changed.
* **Base premises** — an unresolved base, or a tracked base under a pointer
  member, refuses.
* **A tracked identity is never stored** — any value bound to a target, and
  an unknown one, refuses; an unbound one is neutral; the classifier reads no
  formal's pointer and no call result carrying custody.
* **The fallback hands on** — it changes no status and marks the stored
  identity unresolved.
* **Non-vacuity** — each decision on a concrete state. -/

-- What an admitted store leaves.
#print axioms QCore.SubObjectStore.store_admitted
#print axioms QCore.SubObjectStore.an_admitted_store_moves_no_custody
#print axioms QCore.SubObjectStore.an_admitted_store_forgets_the_member
#print axioms QCore.SubObjectStore.an_admitted_store_rebinds_no_other_name

-- Base premises.
#print axioms QCore.SubObjectStore.an_unresolved_base_refuses
#print axioms QCore.SubObjectStore.a_pointer_store_through_a_tracked_base_refuses

-- A tracked identity is never stored.
#print axioms QCore.SubObjectStore.a_stored_identity_keeps_the_refusal
#print axioms QCore.SubObjectStore.an_unknown_value_refuses
#print axioms QCore.SubObjectStore.an_unbound_value_is_neutral
#print axioms QCore.SubObjectStore.a_formal_value_keeps_the_refusal
#print axioms QCore.SubObjectStore.a_custody_result_keeps_the_refusal

-- The fallback hands on.
#print axioms QCore.SubObjectStore.the_fallback_changes_no_status
#print axioms QCore.SubObjectStore.the_fallback_marks_the_stored_identity

-- Non-vacuity.
#print axioms QCore.SubObjectStore.the_formal_base_resolves
#print axioms QCore.SubObjectStore.the_owned_pointer_into_the_callers_record_is_refused
#print axioms QCore.SubObjectStore.an_unbound_pointer_stores_no_identity
#print axioms QCore.SubObjectStore.a_scalar_store_is_admitted
#print axioms QCore.SubObjectStore.a_none_result_call_is_read
