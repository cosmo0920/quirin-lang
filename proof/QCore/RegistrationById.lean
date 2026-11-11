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

/-!
# A registration forwarded through an id
(card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, rule 12n)

The abstract rule implemented by
`crates/qcore-c-gateway/src/registration_by_id.rs` (`validates`, `VECTORS`)
and its hook in `legacy_callback_route::settle`, contract specification
§C04.19.3.

USER DECISION 2026-10-16, "Result + list":

    ensures normal resource_callback_member_registered(#id, TAG.FIELD, F) where #id = #result in P.LIST;

`#id` is the setter's returned id, naming the record linked into `P->LIST`; a
later call taking that id resolves the same record. In a unit that defines the
setter the clause stands only when the body creates exactly one record, links
it into that list, stores `F` into the member and returns that record's id;
anything else withdraws the declaration.

## The model

A body is read as the effects the checker recognises (`Effect`); `validates`
is the check. `stores` is what the body contributes to the run's closure of
the member (rule 12n): each callback stored into it.

## The theorems

* `a_forwarded_registration_is_the_in_body_store`: a validated body contributes
  exactly the callback the clause names, the store a direct registration
  `X->FIELD = F` contributes.
* `a_validated_body_returns_only_its_record`: every return is the one
  record's id or the failure value.
* `two_records_never_validate`, `an_unlinked_record_never_validates`.
* `an_unresolved_or_ambiguous_id_never_decides`: an id that names no record,
  or two, resolves to nothing, so nothing is decided through it.
* `vectors_hold`: the Rust controls (`VECTORS`).

## What this is not

That the checker's token reading recognises these effects exactly (a helper
followed one level, a view `&r->H` reaching that member only, premise
`P-PATH-MEMBER-VIEW`) is a review premise (trust boundary 28), pinned by the
vectors and `crates/qcore-c-gateway/tests/event_loop_retention.rs`.
-/

namespace QCore.RegistrationById

/-- What the checker recognises in a body. Records, lists and callbacks are
abstract names. -/
inductive Effect where
  | create (record : Nat)
  | link (record list : Nat)
  | store (record callback : Nat)
  | returnId (record : Nat)
  | returnFailure
  | escape (record : Nat)
  deriving DecidableEq, Repr

def creates : List Effect → List Nat
  | [] => []
  | .create r :: es => r :: creates es
  | _ :: es => creates es

def links : List Effect → List (Nat × Nat)
  | [] => []
  | .link r l :: es => (r, l) :: links es
  | _ :: es => links es

def storesOf : List Effect → List (Nat × Nat)
  | [] => []
  | .store r c :: es => (r, c) :: storesOf es
  | _ :: es => storesOf es

/-- The callbacks the body stores into the member: its contribution to the
run's closure. -/
def stores (es : List Effect) : List Nat := (storesOf es).map Prod.snd

def returnsFine (r : Nat) : Effect → Bool
  | .returnId r' => r' == r
  | .returnFailure => true
  | .escape _ => false
  | _ => true

def returnsId (r : Nat) : Effect → Bool
  | .returnId r' => r' == r
  | _ => false

/-- Rule 12n's forwarding check over the recognised effects. -/
def validates (callback list : Nat) (es : List Effect) : Bool :=
  match creates es with
  | [r] =>
      links es == [(r, list)] && storesOf es == [(r, callback)] && es.all (returnsFine r)
        && es.any (returnsId r)
  | _ => false

theorem storesOf_of_validates (c l : Nat) (es : List Effect) (h : validates c l es = true) :
    ∃ r, storesOf es = [(r, c)] := by
  unfold validates at h
  split at h
  · rename_i r _
    simp only [Bool.and_eq_true, beq_iff_eq, and_assoc] at h
    exact ⟨r, h.2.1⟩
  · simp at h

/-- **A forwarded registration is the in-body store**: a validated body
contributes exactly the callback its clause names. -/
theorem a_forwarded_registration_is_the_in_body_store (c l : Nat) (es : List Effect)
    (h : validates c l es = true) : stores es = [c] := by
  obtain ⟨r, hs⟩ := storesOf_of_validates c l es h
  simp [stores, hs]

/-- A direct registration, `X->FIELD = F`, contributes that `F`. -/
theorem a_direct_store_contributes_its_callback (r c : Nat) :
    stores [.store r c] = [c] := rfl

/-- **A validated body returns only its record's id or the failure value.** -/
theorem a_validated_body_returns_only_its_record (c l : Nat) (es : List Effect)
    (h : validates c l es = true) : ∃ r, creates es = [r] ∧ es.all (returnsFine r) = true := by
  unfold validates at h
  split at h
  · rename_i r hr
    simp only [Bool.and_eq_true, and_assoc] at h
    exact ⟨r, hr, h.2.2.1⟩
  · simp at h

theorem two_records_never_validate (c l r s : Nat) (es : List Effect)
    (h : creates es = [r, s]) : validates c l es = false := by
  simp [validates, h]

theorem an_unlinked_record_never_validates (c l r : Nat) (es : List Effect)
    (hc : creates es = [r]) (hl : links es = []) : validates c l es = false := by
  simp [validates, hc, hl]

/-! ## Resolving an id -/

/-- The record an id names among `(id, record)` pairs: exactly one, or none. -/
def resolve (table : List (Nat × Nat)) (id : Nat) : Option Nat :=
  match table.filter (fun p => p.1 == id) with
  | [(_, r)] => some r
  | _ => none

/-- What a call taking the id decides through it: the record, or nothing. -/
def decideThrough (table : List (Nat × Nat)) (id : Nat) : Option Nat := resolve table id

/-- **An id that resolves to no record, or to an ambiguous one, never
decides.** -/
theorem an_unresolved_or_ambiguous_id_never_decides (table : List (Nat × Nat)) (id : Nat)
    (h : (table.filter fun p => p.1 == id).length ≠ 1) : decideThrough table id = none := by
  unfold decideThrough resolve
  split
  · rename_i r heq; rw [heq] at h; simp at h
  · rfl

theorem a_unique_id_resolves_its_record (id r : Nat) :
    decideThrough [(id, r)] id = some r := by
  simp [decideThrough, resolve]

/-! ## Shared vectors -/

/-- `VECTORS` of `registration_by_id.rs`, each body as its recognised effects
(callback 9, list 5), with whether the registration stands. -/
def vectors : List (List Effect × Bool) := [
  -- the helper form, read through the helper: one record, linked, stored,
  -- its id returned (and the failure value)
  ([.create 1, .store 1 9, .link 1 5, .returnFailure, .returnId 1], true),
  -- the direct form
  ([.create 1, .returnFailure, .store 1 9, .link 1 5, .returnId 1], true),
  -- two records
  ([.create 1, .create 2, .returnFailure, .store 1 9, .link 1 5, .returnId 1], false),
  -- not linked
  ([.create 1, .returnFailure, .store 1 9, .returnId 1], false),
  -- the callback stored twice
  ([.create 1, .returnFailure, .store 1 9, .store 2 9, .link 1 5, .returnId 1], false),
  -- another value returned
  ([.create 1, .store 1 9, .link 1 5, .returnFailure], false),
  -- the record escapes whole
  ([.create 1, .store 1 9, .link 1 5, .returnFailure, .escape 1, .returnId 1], false)
]

theorem vectors_hold : vectors.all (fun (es, stands) => validates 9 5 es == stands) = true := by
  decide

end QCore.RegistrationById
