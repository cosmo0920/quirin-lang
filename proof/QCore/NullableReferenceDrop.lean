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

import QCore.ReferenceCount

/-!
# The NULL-accepting reference drop (card REFERENCE-DROP-NULL-ACCEPTING, rule 13b)

The abstract rule implemented by `crates/qcore-c-gateway/src/reference_count.rs`
(`DropActual`, `NullableDrop`, `decide_drop_actual`, `NULLABLE_DROP_VECTORS`)
and its route-G adapter (`legacy_reference_count.rs`, `drop_actual`),
contract specification §C04.17: `resource_reference_dropped(p, nullable)`.

OpenSSL documents that its frees of reference-counted objects do nothing on a
NULL argument (`X509_free`, `EVP_PKEY_free`, `SSL_free`, `SSL_CTX_free`,
`BIO_free`, `ENGINE_free`); `QCore.LibraryVersionRange` proves the premise
(`null_release_is_noop`, `null_drop_discharges_nothing`). Here a drop clause
carries that nullability, over `QCore.ReferenceCount`'s tables.

## The model

* `DropActual`: what a drop's actual denotes. `plain b` is any binding the
  non-NULL classes describe (`QCore.ReferenceCount.Binding`); `null` a
  definitely-NULL actual; `oneOrNull held handed` a name bound to exactly one
  live family or NULL.
* `decideDropActual a nullable`: a drop **not** marked nullable decides both
  NULL classes as the `other` binding (refused, as before the card); a
  NULL-accepting drop reads `null` as `noOp` and `oneOrNull` as that family's
  drop; `plain` is today's table either way.
* `NState`: a replay state of `QCore.ReferenceCount` plus the names that are
  definitely NULL. `nstep` replays `QCore.ReferenceCount.step` for every base
  event, binds a name to NULL, and applies a NULL-accepting drop: on a NULL
  name it is the identity, on any other name exactly `.drop`.

## The theorems

* `a_plain_actual_decides_as_today`: the non-NULL edge is exactly rule 13b's
  drop, nullable or not.
* `a_non_nullable_drop_refuses_a_possibly_null_actual` (negative control 1).
* `a_nullable_drop_of_an_unheld_reference_is_refused` (negative control 2: a
  NULL-accepting drop of a non-NULL reference still needs the reference).
* `only_the_null_edge_is_a_no_op`, `a_nullable_drop_of_null_is_a_no_op`,
  `the_one_or_null_edge_is_the_family_drop`.
* `the_null_edge_changes_nothing`, `the_null_edge_discharges_and_creates_nothing`
  (families, acquisitions, the next family, the refusal and violation flags
  and the outcome are all unchanged), `the_non_null_edge_is_the_drop`.
* `nullable_drop_vectors_hold`: the table shared with Rust
  (`crates/qcore-c-gateway/tests/reference_drop_null_accepting.rs` checks that
  the Rust constants are these rows).

## What this is not

A model over abstract names. That the gateway reads a definitely-NULL actual
and a one-family-or-NULL actual as these classes (`drop_actual`), and the
correspondence with the compiled Rust, are review premises (trust boundary
28) pinned by the shared vectors and the route-G controls. That the callee
does nothing on NULL is the declared assumption (constraint 4) its
documentation states and rule 13c checks structurally where the body is
defined.
-/

namespace QCore.NullableReferenceDrop

open QCore.ReferenceCount

inductive DropActual where
  | plain (b : Binding)
  | null
  | oneOrNull (held : Held) (handed : Bool)
  deriving DecidableEq, Repr

inductive NullableDrop where
  | noOp
  | drop (d : DropDecision)
  deriving DecidableEq, Repr

/-- Rule 13b's drop with the clause's nullability
(`reference_count::decide_drop_actual`). -/
def decideDropActual : DropActual → Bool → NullableDrop
  | .plain b, _ => .drop (decideDrop b)
  | .null, true => .noOp
  | .oneOrNull h hd, true => .drop (decideDrop (.family h hd))
  | .null, false => .drop (decideDrop .other)
  | .oneOrNull _ _, false => .drop (decideDrop .other)

/-- **The non-NULL edge is today's drop**, whatever the clause's nullability. -/
theorem a_plain_actual_decides_as_today (b : Binding) (n : Bool) :
    decideDropActual (.plain b) n = .drop (decideDrop b) := by
  cases n <;> rfl

/-- **Negative control 1**: a drop not marked nullable refuses a possibly-NULL
actual, exactly as the `other` binding did before the card. -/
theorem a_non_nullable_drop_refuses_a_possibly_null_actual (h : Held) (hd : Bool) :
    decideDropActual .null false = .drop (.refuse .unheld) ∧
      decideDropActual (.oneOrNull h hd) false = .drop (.refuse .unheld) ∧
      decideDropActual .null false = .drop (decideDrop .other) ∧
      decideDropActual (.oneOrNull h hd) false = .drop (decideDrop .other) :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- **Negative control 2**: a NULL-accepting drop of a non-NULL reference the
function does not hold still refuses. -/
theorem a_nullable_drop_of_an_unheld_reference_is_refused :
    decideDropActual (.plain .unbound) true = .drop (.refuse .unheld) ∧
      decideDropActual (.plain .other) true = .drop (.refuse .unheld) ∧
      decideDropActual (.plain .invalid) true = .drop (.refuse .dropped) :=
  ⟨rfl, rfl, rfl⟩

theorem a_nullable_drop_of_null_is_a_no_op : decideDropActual .null true = .noOp := rfl

/-- Only a definitely-NULL actual under a NULL-accepting clause is a no-op. -/
theorem only_the_null_edge_is_a_no_op (a : DropActual) (n : Bool)
    (h : decideDropActual a n = .noOp) : a = .null ∧ n = true := by
  cases a <;> cases n <;> simp_all [decideDropActual]

/-- One family or NULL: the family's drop on its non-NULL edge. -/
theorem the_one_or_null_edge_is_the_family_drop (h : Held) (hd : Bool) :
    decideDropActual (.oneOrNull h hd) true = decideDropActual (.plain (.family h hd)) true :=
  rfl

/-- A possibly-final drop needs the family's last reference, NULL edge or not. -/
theorem a_final_nullable_drop_is_the_last_reference (a : DropActual)
    (h : decideDropActual a true = .drop .final) :
    a = .plain (.family (.known 1) false) ∨ a = .oneOrNull (.known 1) false := by
  cases a with
  | plain b =>
      left
      simp only [decideDropActual, NullableDrop.drop.injEq] at h
      rw [a_final_drop_is_the_last_reference b h]
  | null => simp [decideDropActual] at h
  | oneOrNull held handed =>
      right
      simp only [decideDropActual, NullableDrop.drop.injEq] at h
      have := a_final_drop_is_the_last_reference _ h
      simp only [Binding.family.injEq] at this
      rw [this.1, this.2]

/-! ## The NULL edge on the replay -/

/-- A replay state and the names that are definitely NULL. -/
structure NState where
  st : State
  nulls : List Nat
  deriving DecidableEq, Repr

inductive NEvent where
  | base (e : Event)
  /-- `n = NULL;` -/
  | null (n : Nat)
  /-- A NULL-accepting drop of `n`. -/
  | dropNullable (n : Nat)
  deriving DecidableEq, Repr

/-- The names a base event binds. -/
def written : Event → List Nat
  | .acquire n => [n]
  | .keepTo d _ => [d]
  | .alias d _ => [d]
  | _ => []

def nstep (s : NState) : NEvent → NState
  | .base e => ⟨step s.st e, s.nulls.filter (fun m => !(written e).contains m)⟩
  | .null n => ⟨setName s.st n none, n :: s.nulls⟩
  | .dropNullable n => if s.nulls.contains n then s else ⟨step s.st (.drop n), s.nulls⟩

/-- **The NULL edge changes nothing.** -/
theorem the_null_edge_changes_nothing (s : NState) (n : Nat) (h : s.nulls.contains n = true) :
    nstep s (.dropNullable n) = s := by
  simp only [nstep, h, ↓reduceIte]

/-- **It discharges nothing and creates nothing**: no family, acquisition or
name is added or removed, the refusal and violation flags and the outcome are
unchanged. -/
theorem the_null_edge_discharges_and_creates_nothing (s : NState) (n : Nat)
    (h : s.nulls.contains n = true) :
    (nstep s (.dropNullable n)).st.families = s.st.families ∧
      (nstep s (.dropNullable n)).st.acquired = s.st.acquired ∧
      (nstep s (.dropNullable n)).st.names = s.st.names ∧
      (nstep s (.dropNullable n)).st.next = s.st.next ∧
      (nstep s (.dropNullable n)).st.refused = s.st.refused ∧
      (nstep s (.dropNullable n)).st.violation = s.st.violation ∧
      outcome (nstep s (.dropNullable n)).st = outcome s.st := by
  rw [the_null_edge_changes_nothing s n h]
  exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- **The non-NULL edge is the drop**: on a name that is not NULL, a
NULL-accepting drop is exactly `QCore.ReferenceCount`'s `.drop`. -/
theorem the_non_null_edge_is_the_drop (s : NState) (n : Nat) (h : s.nulls.contains n = false) :
    (nstep s (.dropNullable n)).st = step s.st (.drop n) := by
  simp only [nstep, h, Bool.false_eq_true, ↓reduceIte]

def nrun (es : List NEvent) : NState := es.foldl nstep ⟨State.empty, []⟩

/-- A NULL pointer dropped: safe. An acquired reference dropped through the
NULL-accepting drop: safe. A NULL drop beside an acquired reference: that
reference is still owed (a leak candidate). -/
theorem nullable_drop_traces :
    outcome (nrun [.null 0, .dropNullable 0]).st = .safe ∧
      outcome (nrun [.base (.acquire 0), .dropNullable 0]).st = .safe ∧
      outcome (nrun [.null 1, .base (.acquire 0), .dropNullable 1]).st = .leak ∧
      outcome (nrun [.base (.acquire 0), .dropNullable 0, .dropNullable 0]).st = .violation ∧
      outcome (nrun [.dropNullable 0]).st = .refused := by
  decide

/-! ## Shared vectors -/

/-- `NULLABLE_DROP_VECTORS` of `reference_count.rs`, row by row. -/
def nullableDropVectors : List (DropActual × Bool × NullableDrop) := [
  (.null, true, .noOp),
  (.null, false, (.drop (.refuse .unheld))),
  ((.oneOrNull (.known 1) false), true, (.drop .final)),
  ((.oneOrNull (.known 2) false), true, (.drop .decrement)),
  ((.oneOrNull (.known 1) true), true, (.drop (.refuse .unheld))),
  ((.oneOrNull .undecided false), true, (.drop (.refuse .undecided))),
  ((.oneOrNull (.known 1) false), false, (.drop (.refuse .unheld))),
  ((.plain .unbound), true, (.drop (.refuse .unheld))),
  ((.plain .other), true, (.drop (.refuse .unheld))),
  ((.plain .invalid), true, (.drop (.refuse .dropped))),
  ((.plain .freed), true, (.drop .overDrop)),
  ((.plain (.family (.known 1) false)), true, (.drop .final))
]

theorem nullable_drop_vectors_hold :
    nullableDropVectors.all (fun (a, n, d) => decideDropActual a n == d) = true := by
  decide

end QCore.NullableReferenceDrop
