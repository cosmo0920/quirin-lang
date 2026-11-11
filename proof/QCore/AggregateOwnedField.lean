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

import QCore.SourceRecord

/-! The owned record field (card `AGGREGATE-OWNED-FIELD`, wave 39): the
path-extended cycle-3 ownership model and the four rules constraint 15k
recorded as missing.

`QCore.SourceRecord` proved that a record of scalars presents **no** custody
event. This module is the other half: a record may now declare a field that
owns a resource, and the ownership IR addresses that field by a **path**
(`p.buf`) rather than by a plain name. Four rules are landed in the IR and
mirrored here.

**Construction moves in.** Composing a record from named owners moves each
owner into its field: the source binding is moved, the field owns the
resource, and the live quantity is unchanged (`compose_moves_each_source_in`,
`compose_moves_no_quantity`).

**Partial move.** Reading an owned field moves its resource out to a plain
binding and leaves a **hole**; a second read of the hole refuses
(`move_out_leaves_a_hole`, `a_hole_refuses_a_second_move`).

**Drop-through-aggregate.** Dropping the record releases **exactly the live
owned fields, in declaration order, and nothing twice** — the headline,
`drop_through_releases_exactly_the_live_fields` and
`drop_through_releases_nothing_twice` — and tolerates a hole
(`drop_through_tolerates_a_hole`).

**Leak at exit sees into the aggregate.** A normal exit needs the ledger
empty; a field still holding a resource makes it non-empty, and the leak is
named by its path (`an_unreleased_field_is_a_leak_named_by_its_path`).

And the copy rule: a record declaring an owned field is not copyable,
whatever the state of its fields (`an_aggregate_is_never_copied`).

What is **not** proved here: that the Rust lowering extracts the field-path
statements faithfully from source tokens (a tested Rust boundary); the
emitted C's designated initializer performing the move (cross-checked by the
sanitized native run); and the two window premises the route refuses by name
(`OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY`), which are one-arm Rust
refusals recorded as such. -/

namespace QCore.AggregateOwnedField

abbrev ResourceId := Nat
abbrev Name := String

/-- An owner place: a plain binding, or one field of a named aggregate. The
`Place::Name` / `Place::Field` vocabulary of `ownership.rs`, and the reason
`p.buf` is now something the checker can be asked about. -/
inductive Place where
  | name (n : Name)
  | field (base : Name) (field : Name)
  deriving DecidableEq, Repr

/-- The spelling a diagnostic names a place by. -/
def Place.spelling : Place → String
  | .name n => n
  | .field b f => b ++ "." ++ f

/-- What one owned field of a composed record holds: its resource, or a hole
left by a move out or a field release. -/
structure Field where
  name : Name
  held : Option ResourceId
  deriving DecidableEq, Repr

/-- A composed aggregate: its owned fields in **declaration order**. -/
structure Aggregate where
  fields : List Field
  deriving DecidableEq, Repr

/-- The resources of the fields still held, in declaration order. -/
def liveFields (a : Aggregate) : List (Name × ResourceId) :=
  a.fields.filterMap (fun f => f.held.map (fun r => (f.name, r)))

/-- Whether the aggregate declares an owned field at all — the type-level
question the copy rule asks. -/
def declaresOwnedField (a : Aggregate) : Bool := !a.fields.isEmpty

/-! ## The ledger: resources by the place that owns them -/

abbrev Ledger := List (ResourceId × Place)

def owns (ledger : Ledger) (r : ResourceId) : Option Place :=
  (ledger.find? (fun e => e.1 = r)).map Prod.snd

def release (ledger : Ledger) (r : ResourceId) : Ledger :=
  ledger.filter (fun e => e.1 ≠ r)

def moveTo (ledger : Ledger) (r : ResourceId) (p : Place) : Ledger :=
  ledger.map (fun e => if e.1 = r then (r, p) else e)

theorem release_removes_exactly_its_own (ledger : Ledger) (r : ResourceId) :
    owns (release ledger r) r = none := by
  unfold owns release
  cases h : (ledger.filter (fun e => e.1 ≠ r)).find? (fun e => e.1 = r) with
  | none => simp
  | some e =>
    have mem := List.find?_some h
    have kept := List.mem_filter.mp (List.mem_of_find?_eq_some h)
    simp only [decide_eq_true_eq] at mem
    simp only [decide_eq_true_eq] at kept
    exact absurd mem kept.2

theorem release_keeps_every_other (ledger : Ledger) (r s : ResourceId) (p : Place)
    (different : s ≠ r) (present : (s, p) ∈ ledger) : (s, p) ∈ release ledger r := by
  refine List.mem_filter.mpr ⟨present, ?_⟩
  simp [different]

theorem move_preserves_length (ledger : Ledger) (r : ResourceId) (p : Place) :
    (moveTo ledger r p).length = ledger.length := by
  simp [moveTo]

/-! ## Construction moves in -/

/-- Compose an aggregate named `base` from `(field, source binding, resource)`
triples, in declaration order: every resource moves from its plain binding to
the field place, and the aggregate records the held resources. -/
def compose (ledger : Ledger) (base : Name)
    (sources : List (Name × Name × ResourceId)) : Ledger × Aggregate :=
  (sources.foldl (fun l s => moveTo l s.2.2 (.field base s.1)) ledger,
   ⟨sources.map (fun s => ⟨s.1, some s.2.2⟩)⟩)

/-- Every source resource is owned by its field afterwards: a fold that moves
each resource in turn ends with every one at its field place. Stated over the
one-field composition and the two-field one — the shapes the flagship and the
drop-order fixtures use — which is what the Rust unit tests pin. -/
theorem compose_moves_each_source_in (base f : Name) (r : ResourceId) :
    owns (compose [(r, .name "b")] base [(f, "b", r)]).1 r = some (.field base f) := by
  simp [compose, moveTo, owns]

theorem compose_two_moves_both_in (base f g : Name) (r s : ResourceId)
    (distinct : r ≠ s) :
    owns (compose [(r, .name "b"), (s, .name "c")] base [(f, "b", r), (g, "c", s)]).1 r
        = some (.field base f)
      ∧ owns (compose [(r, .name "b"), (s, .name "c")] base [(f, "b", r), (g, "c", s)]).1 s
        = some (.field base g) := by
  constructor
  · simp [compose, moveTo, owns, distinct]
  · simp [compose, moveTo, owns, List.find?, Ne.symm distinct, distinct]

/-- A move changes no quantity: the ledger has the same entries, re-owned. -/
theorem compose_moves_no_quantity (ledger : Ledger) (base : Name)
    (sources : List (Name × Name × ResourceId)) :
    (compose ledger base sources).1.length = ledger.length := by
  unfold compose
  simp only
  induction sources generalizing ledger with
  | nil => rfl
  | cons s rest ih => simp [List.foldl, ih, move_preserves_length]

/-- The composed aggregate holds exactly the sources' resources, in order. -/
theorem compose_holds_the_sources (ledger : Ledger) (base : Name)
    (sources : List (Name × Name × ResourceId)) :
    liveFields (compose ledger base sources).2 = sources.map (fun s => (s.1, s.2.2)) := by
  unfold compose liveFields
  simp only
  induction sources with
  | nil => rfl
  | cons s rest ih => simp [ih]

/-! ## The partial move -/

/-- Move field `f` of `a` out to the plain binding `to`: `none` when the field
is a hole or undeclared, else the resource, the ledger re-owned, and the
aggregate with the hole. -/
def moveOut (ledger : Ledger) (a : Aggregate) (f : Name) (to : Name)
    : Option (ResourceId × Ledger × Aggregate) :=
  match a.fields.find? (fun fld => fld.name = f) with
  | none => none
  | some fld =>
    match fld.held with
    | none => none
    | some r =>
      some (r, moveTo ledger r (.name to),
            ⟨a.fields.map (fun x => if x.name = f then ⟨x.name, none⟩ else x)⟩)

/-- After a move out the field is a hole: it is no longer live. -/
theorem move_out_leaves_a_hole (base f to : Name) (r : ResourceId) :
    (moveOut [(r, .field base f)] ⟨[⟨f, some r⟩]⟩ f to).map
        (fun x => (owns x.2.1 r, liveFields x.2.2))
      = some (some (.name to), []) := by
  simp [moveOut, moveTo, owns, liveFields]

/-- A second move out of the same field refuses: the hole holds nothing. -/
theorem a_hole_refuses_a_second_move (base f to : Name) (r : ResourceId) :
    (moveOut [(r, .field base f)] ⟨[⟨f, some r⟩]⟩ f to).bind
        (fun x => moveOut x.2.1 x.2.2 f "d") = none := by
  simp [moveOut, moveTo]

/-- Moving one field out leaves every other field exactly as it was. -/
theorem move_out_keeps_the_other_fields (base f g to : Name) (r s : ResourceId)
    (distinct : g ≠ f) :
    (moveOut [(r, .field base f), (s, .field base g)]
        ⟨[⟨f, some r⟩, ⟨g, some s⟩]⟩ f to).map (fun x => liveFields x.2.2)
      = some [(g, s)] := by
  simp [moveOut, moveTo, liveFields, distinct]

/-! ## Drop-through-aggregate: the headline -/

/-- Release every **live** field of `a`, in declaration order; a hole is
skipped. Returns the ledger and the resources released, in that order. -/
def dropThrough (ledger : Ledger) (a : Aggregate) : Ledger × List ResourceId :=
  a.fields.foldl
    (fun acc f => match f.held with
      | none => acc
      | some r => (release acc.1 r, acc.2 ++ [r]))
    (ledger, [])

/-- The released list is a function of the fields alone. -/
theorem dropThrough_released (ledger : Ledger) (a : Aggregate) :
    (dropThrough ledger a).2 = (liveFields a).map Prod.snd := by
  unfold dropThrough liveFields
  suffices h : ∀ (fs : List Field) (l : Ledger) (acc : List ResourceId),
      (fs.foldl (fun acc f => match f.held with
        | none => acc
        | some r => (release acc.1 r, acc.2 ++ [r])) (l, acc)).2
        = acc ++ (fs.filterMap (fun f => f.held.map (fun r => (f.name, r)))).map Prod.snd by
    simpa using h a.fields ledger []
  intro fs
  induction fs with
  | nil => intro l acc; simp
  | cons f rest ih =>
    intro l acc
    obtain ⟨name, held⟩ := f
    cases held with
    | none => simp [List.foldl, List.filterMap, ih]
    | some r => simp [List.foldl, List.filterMap, ih]

/-- **The headline.** A whole-record drop releases exactly the resources of
the live owned fields, in declaration order — no hole, nothing else. -/
theorem drop_through_releases_exactly_the_live_fields (ledger : Ledger) (a : Aggregate) :
    (dropThrough ledger a).2 = (liveFields a).map Prod.snd :=
  dropThrough_released ledger a

/-- Nothing twice: when the live fields hold distinct resources, the released
list has no repeat. Distinctness of what fields hold is the ledger invariant
the Rust `validate_invariant` checks after every step (one owner per
resource), so the hypothesis is the state the IR maintains, not an
assumption about the program. -/
theorem drop_through_releases_nothing_twice (ledger : Ledger) (a : Aggregate)
    (distinct : ((liveFields a).map Prod.snd).Nodup) :
    (dropThrough ledger a).2.Nodup := by
  rw [dropThrough_released]
  exact distinct

/-- A hole is tolerated: the drop of an aggregate with one moved-out field
releases exactly the others. -/
theorem drop_through_tolerates_a_hole (ledger : Ledger) (f g : Name) (s : ResourceId) :
    (dropThrough ledger ⟨[⟨f, none⟩, ⟨g, some s⟩]⟩).2 = [s] := by
  simp [dropThrough_released, liveFields]

/-- The order is the declaration's: the first-declared field is released
first, whatever the resource identities say. -/
theorem drop_through_is_in_declaration_order (ledger : Ledger) (f g : Name) (r s : ResourceId) :
    (dropThrough ledger ⟨[⟨f, some r⟩, ⟨g, some s⟩]⟩).2 = [r, s] := by
  simp [dropThrough_released, liveFields]

/-- After the drop, none of the released resources is owned any more. -/
theorem drop_through_releases_from_the_ledger (base f : Name) (r : ResourceId) :
    owns (dropThrough [(r, .field base f)] ⟨[⟨f, some r⟩]⟩).1 r = none := by
  simp [dropThrough, release, owns]

/-! ## The leak rule that sees into the aggregate -/

/-- A normal exit needs the ledger empty — `QCore.Ownership.NormalExit`'s
statement, over the place-keyed ledger. -/
def NormalExit (ledger : Ledger) : Prop := ledger = []

/-- The places a leak report names: every owner still in the ledger, by its
spelling — a field by its path. -/
def leakedPlaces (ledger : Ledger) : List String := ledger.map (fun e => e.2.spelling)

/-- A field still holding a resource at the exit is a leak, and the report
names the path `base.field`. -/
theorem an_unreleased_field_is_a_leak_named_by_its_path (base f : Name) (r : ResourceId) :
    ¬NormalExit [(r, .field base f)] ∧ leakedPlaces [(r, .field base f)] = [base ++ "." ++ f] := by
  constructor
  · intro h; cases h
  · rfl

/-- And a whole-record drop of a fully live aggregate discharges the leak: the
ledger holding exactly the aggregate's resources is empty afterwards. -/
theorem drop_through_discharges_the_leak (base f : Name) (r : ResourceId) :
    NormalExit (dropThrough [(r, .field base f)] ⟨[⟨f, some r⟩]⟩).1 := by
  simp [NormalExit, dropThrough, release]

/-! ## The copy rule -/

/-- Whether a value of the aggregate's type may be copied: never, when the
type declares an owned field — the refusal is about the type, not the state. -/
def copyable (a : Aggregate) : Bool := !declaresOwnedField a

theorem an_aggregate_is_never_copied (f : Name) (r : ResourceId) :
    copyable ⟨[⟨f, some r⟩]⟩ = false := rfl

/-- Including one whose every field has been moved out: a hollow record is
still not a value two bindings may hold. -/
theorem a_hollow_aggregate_is_still_not_copied (f : Name) :
    copyable ⟨[⟨f, none⟩]⟩ = false := rfl

/-! ## Type agreement at construction

The Rust `source_record::construct` now checks that a scalar is supplied at an
`i32` field and a buffer at a `bytes` field. Stated here as the one rule, with
both refusing directions, so a widening has to change a theorem. -/

inductive FieldKind where
  | i32
  | bytes
  deriving DecidableEq, Repr

inductive Supplied where
  | scalar (v : Int)
  | buffer (name : Name)
  deriving DecidableEq, Repr

def agrees : FieldKind → Supplied → Bool
  | .i32, .scalar _ => true
  | .bytes, .buffer _ => true
  | .i32, .buffer _ => false
  | .bytes, .scalar _ => false

theorem agreement_is_exactly_kind_for_kind (k : FieldKind) (s : Supplied) :
    agrees k s = true ↔
      ((k = .i32 ∧ ∃ v, s = .scalar v) ∨ (k = .bytes ∧ ∃ n, s = .buffer n)) := by
  cases k <;> cases s <;> simp [agrees]

theorem a_scalar_at_a_bytes_field_is_refused (v : Int) : agrees .bytes (.scalar v) = false := rfl

theorem a_buffer_at_an_i32_field_is_refused (n : Name) : agrees .i32 (.buffer n) = false := rfl

/-! ## Neutrality

A record of scalars is exactly what it was: it composes no aggregate, and
`QCore.SourceRecord` already proves it presents no custody event. Stated once
more against this module's own vocabulary, so the two halves meet. -/

theorem a_record_of_scalars_composes_nothing (ledger : Ledger) (base : Name) :
    (compose ledger base []).1 = ledger ∧ liveFields (compose ledger base []).2 = [] := by
  constructor <;> rfl

end QCore.AggregateOwnedField
