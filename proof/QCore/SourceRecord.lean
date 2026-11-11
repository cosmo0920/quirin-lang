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

import QCore.StdBuffer

/-! `source.record.v1` — the first source-level aggregate type (card
`AGGREGATES-V1`).

This module proves the three claims the Rust construct rests on, and one it
deliberately does **not** make.

**Type formation decides distinctness.** A record type is admitted only if it
declares at least one field, no two fields share a name, and the list is one the
model enumerates. `admitType_iff` states that as one equivalence, so a widening
has to change a theorem rather than slip past one.

**Construction is total, and that is what makes projection total.** `construct`
succeeds exactly when the supplied assignment names every declared field exactly
once and nothing else, and `projection_is_total_on_declared_fields` is the
consequence: a constructed value has a value for every field it can be asked
for. There is no partial arm for a reader to reason about.

**Field access returns what construction supplied — and CT09's scenario is the
corollary.** `projection_returns_what_construction_supplied` is the headline;
`two_distinct_fields_read_their_own_values` is the same fact stated the way
CT09's "distinct ordinary struct fields" asks for it, and it is non-vacuous
because the constructed value is built in *declared* order out of the supplied
assignment rather than being the assignment itself.

**A record of scalars carries no custody, and that is proved rather than
asserted.** `a_record_item_presents_no_event` and
`a_record_only_program_leaves_the_ledger_unchanged` say the aggregate of this
module contributes nothing at all to the inherited cycle-3 ledger of
`QCore.StdBuffer` — which is the modelled form of card `AGGREGATES-V1`'s
field-custody decision. Card `AGGREGATE-OWNED-FIELD` (wave 39) landed the
*owned* field beside it, in `QCore.AggregateOwnedField`, with the four rules a
record that owns needs; the theorems here remain true of the scalar record
they are about, and that module's `a_record_of_scalars_composes_nothing` is
where the two halves meet.

What is **not** proved here: that the Rust lowering extracts a declaration and a
construction faithfully from source tokens (a tested Rust boundary, the
`SourceLoop` / `BranchJoin` precedent); that a C compiler's designated
initializer assigns each member the value the model says (that is what the
sanitized native byte comparison cross-checks); and anything about a field this
model gives no type, which is absent by construction — every field of a v1
record is an integer.
-/

namespace QCore.SourceRecord

open QCore.StdBuffer

abbrev FieldName := String

/-- A declared field of the **scalar** record this module models.

It carries a name and nothing else: every field of the record this module is
about is an integer. Since card `AGGREGATE-OWNED-FIELD` the Rust `FieldType`
has a second constructor, `Bytes`, whose model — the move in, the move out,
the drop-through and the leak that names a path, and the kind-for-kind
agreement a construction now checks — is `QCore.AggregateOwnedField`'s; this
module's theorems are the scalar half and are unchanged by it. -/
structure Field where
  name : FieldName
  deriving DecidableEq, Repr

/-- A declared record type. -/
structure RecordType where
  name : String
  fields : List Field
  deriving DecidableEq, Repr

/-- The largest field list this model enumerates. In the model because it is a
refusal the construct owns, not an implementation limit hidden behind one. -/
def maxFields : Nat := 32

/-- The declared field names, in declaration order. -/
def names (t : RecordType) : List FieldName := t.fields.map Field.name

/-- Whether a list of names has no repeat. Written here rather than imported, so
the distinctness rule this module proves is the one it also defines. -/
def nodup : List FieldName → Bool
  | [] => true
  | f :: rest => !(decide (f ∈ rest)) && nodup rest

/-! ## Type formation -/

/-- **Type formation.** Exactly the three conditions the Rust `admit_type`
decides. -/
def admitType (t : RecordType) : Bool :=
  !t.fields.isEmpty && decide (t.fields.length ≤ maxFields) && nodup (names t)

/-- The formation rule as one equivalence: a widening has to change this
theorem rather than slip past it (the `QCore.StdIo.the_operand_forms_are_exactly_these`
convention). -/
theorem admitType_iff (t : RecordType) :
    admitType t = true ↔
      (t.fields ≠ [] ∧ t.fields.length ≤ maxFields ∧ nodup (names t) = true) := by
  simp [admitType, and_assoc]

theorem a_record_declaring_no_field_is_refused (name : String) :
    admitType { name := name, fields := [] } = false := by
  rfl

theorem a_repeated_field_name_is_refused :
    admitType { name := "Point", fields := [⟨"x"⟩, ⟨"x"⟩] } = false := by
  rfl

theorem a_record_of_distinct_fields_is_admitted :
    admitType { name := "Point", fields := [⟨"x"⟩, ⟨"y"⟩] } = true := by
  rfl

/-- The one consequence of formation the construction rule needs. -/
theorem admitType_nodup {t : RecordType} (admitted : admitType t = true) :
    nodup (names t) = true := by
  unfold admitType at admitted
  simp only [Bool.and_eq_true] at admitted
  exact admitted.2

/-! ## Assignments -/

/-- A supplied field list: a name and the integer supplied for it. -/
abbrev Assignment := List (FieldName × Int)

/-- The value supplied for `f`, or `none`. -/
def value : Assignment → FieldName → Option Int
  | [], _ => none
  | (n, v) :: rest, f => if n = f then some v else value rest f

/-- The names an assignment supplies. -/
def supplied (a : Assignment) : List FieldName := a.map Prod.fst

/-! ## Construction -/

/-- A constructed record value.

Its field list is in **declared** order rather than in the order the source
supplied them, which is what the Rust certificate publishes and what makes the
projection theorem below a theorem rather than a restatement. -/
structure Value where
  typeName : String
  fields : Assignment
  deriving DecidableEq, Repr

/-- Every declared field is supplied, exactly once, and nothing else is. -/
def complete (t : RecordType) (a : Assignment) : Bool :=
  nodup (supplied a) &&
    (supplied a).all (fun f => decide (f ∈ names t)) &&
    (names t).all (fun f => decide (f ∈ supplied a))

/-- **Construction totality.** Build a value, in declared order, or refuse.

The `0` default is unreachable for an admitted construction — `complete`
guarantees every declared name is supplied — and `constructed_field_is_supplied`
below is the statement that it is never taken. -/
def construct (t : RecordType) (a : Assignment) : Option Value :=
  if admitType t && complete t a then
    some { typeName := t.name,
           fields := (names t).map (fun f => (f, (value a f).getD 0)) }
  else
    none

/-- **Field access.** -/
def project (v : Value) (f : FieldName) : Option Int := value v.fields f

/-! ## What construction and projection guarantee -/

/-- A name in the declared list is a name the constructed value carries. -/
theorem constructed_value_carries_every_declared_field
    (t : RecordType) (a : Assignment) (v : Value) (f : FieldName)
    (built : construct t a = some v) (declared : f ∈ names t) :
    (f, (value a f).getD 0) ∈ v.fields := by
  unfold construct at built
  split at built
  · cases built
    exact List.mem_map_of_mem declared
  · cases built

/-- `value` finds a name its list carries. -/
theorem value_isSome_of_mem (a : Assignment) (f : FieldName)
    (present : f ∈ supplied a) : (value a f).isSome = true := by
  induction a with
  | nil => cases present
  | cons entry rest ih =>
    unfold supplied at present
    simp only [List.map_cons, List.mem_cons] at present
    unfold value
    split
    · rfl
    · rename_i disagrees
      rcases present with head | tail
      · exact absurd head.symm disagrees
      · exact ih tail

/-- **Projection is total on the declared fields.** A constructed value has a
value for every field it can be asked for, which is what makes the field-access
rule a function rather than a partial one with an error arm. -/
theorem projection_is_total_on_declared_fields
    (t : RecordType) (a : Assignment) (v : Value) (f : FieldName)
    (built : construct t a = some v) (declared : f ∈ names t) :
    (project v f).isSome = true := by
  have carried := constructed_value_carries_every_declared_field t a v f built declared
  have : f ∈ supplied v.fields := by
    unfold supplied
    exact List.mem_map_of_mem (f := Prod.fst) carried
  exact value_isSome_of_mem v.fields f this

/-- A no-repeat list gives `value` back exactly the pair it carries. -/
theorem value_of_mem_nodup (a : Assignment) (f : FieldName) (x : Int)
    (distinct : nodup (supplied a) = true) (carried : (f, x) ∈ a) :
    value a f = some x := by
  induction a with
  | nil => cases carried
  | cons entry rest ih =>
    unfold supplied at distinct
    rw [List.map_cons] at distinct
    unfold nodup at distinct
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_false_iff_not] at distinct
    obtain ⟨fresh, tail_distinct⟩ := distinct
    simp only [List.mem_cons] at carried
    unfold value
    split
    · rename_i agrees
      rcases carried with head | tail
      · cases head; rfl
      · exfalso
        have mem : f ∈ List.map Prod.fst rest := List.mem_map_of_mem (f := Prod.fst) tail
        rw [agrees] at fresh
        exact fresh mem
    · rename_i disagrees
      rcases carried with head | tail
      · cases head; exact absurd rfl disagrees
      · exact ih tail_distinct tail

/-- Rebuilding an assignment in declared order supplies exactly the declared
names. -/
theorem supplied_rebuild (fs : List FieldName) (a : Assignment) :
    supplied (fs.map (fun f => (f, (value a f).getD 0))) = fs := by
  induction fs with
  | nil => rfl
  | cons f rest ih => simp [supplied] at ih ⊢; exact ih

/-- The declared-order rebuild has no repeat, because the declared names have
none. -/
theorem constructed_fields_are_distinct
    (t : RecordType) (a : Assignment) (v : Value)
    (built : construct t a = some v) : nodup (supplied v.fields) = true := by
  unfold construct at built
  split at built
  · rename_i admitted
    cases built
    simp only [Bool.and_eq_true] at admitted
    show nodup (supplied ((names t).map (fun f => (f, (value a f).getD 0)))) = true
    rw [supplied_rebuild]
    exact admitType_nodup admitted.1
  · cases built

/-- **The headline.** Reading a field of a constructed value gives back the
value the construction supplied for that field — not for any other. -/
theorem projection_returns_what_construction_supplied
    (t : RecordType) (a : Assignment) (v : Value) (f : FieldName) (x : Int)
    (built : construct t a = some v) (declared : f ∈ names t)
    (assigned : value a f = some x) :
    project v f = some x := by
  have carried := constructed_value_carries_every_declared_field t a v f built declared
  rw [assigned] at carried
  exact value_of_mem_nodup v.fields f x (constructed_fields_are_distinct t a v built) carried

/-- **CT09's scenario.** Two *distinct* declared fields read back the two values
supplied for them. A model that let one field's value answer for another's would
falsify this, and so would a lowering that gave two fields one member — which is
what the sanitized native byte comparison cross-checks. -/
theorem two_distinct_fields_read_their_own_values
    (t : RecordType) (a : Assignment) (v : Value) (f g : FieldName) (x y : Int)
    (built : construct t a = some v)
    (f_declared : f ∈ names t) (g_declared : g ∈ names t)
    (f_given : value a f = some x) (g_given : value a g = some y) :
    project v f = some x ∧ project v g = some y :=
  ⟨projection_returns_what_construction_supplied t a v f x built f_declared f_given,
   projection_returns_what_construction_supplied t a v g y built g_declared g_given⟩

/-- A worked positive control, so the two theorems above are not vacuous. -/
theorem a_two_field_record_constructs_and_projects :
    (construct { name := "Point", fields := [⟨"x"⟩, ⟨"y"⟩] }
        [("y", 66), ("x", 65)]).map (fun v => (project v "x", project v "y"))
      = some (some 65, some 66) := by
  rfl

/-! ## The refusals, as one list -/

/-- A construction is refused **exactly** when the type is not one this model
admits or the assignment is not complete — there is no fifth way and no silent
one. -/
theorem refused_constructions_are_exactly_these (t : RecordType) (a : Assignment) :
    (construct t a).isNone = !(admitType t && complete t a) := by
  unfold construct
  cases admitType t && complete t a <;> rfl

/-- And completeness is exactly the three conditions the Rust `construct`
decides: no repeat, nothing undeclared, nothing missing. A widening has to
change this theorem rather than slip past it. -/
theorem complete_iff (t : RecordType) (a : Assignment) :
    complete t a = true ↔
      (nodup (supplied a) = true
        ∧ (∀ f ∈ supplied a, f ∈ names t)
        ∧ (∀ f ∈ names t, f ∈ supplied a)) := by
  simp [complete, and_assoc]

theorem a_missing_field_is_refused :
    construct { name := "Point", fields := [⟨"x"⟩, ⟨"y"⟩] } [("x", 1)] = none := by
  rfl

theorem an_undeclared_field_is_refused :
    construct { name := "Point", fields := [⟨"x"⟩] } [("x", 1), ("z", 2)] = none := by
  rfl

theorem a_repeated_field_is_refused :
    construct { name := "Point", fields := [⟨"x"⟩] } [("x", 1), ("x", 2)] = none := by
  rfl

/-! ## The field-custody decision, proved rather than asserted

A v1 record is a **plain value**: it contributes nothing at all to the inherited
cycle-3 ledger of `QCore.StdBuffer`. That is what makes "scalar fields only"
a decision with a consequence rather than a restriction with none. -/

/-- A statement of a route that admits this construct: one of that route's own
events, or one of the aggregate's two. -/
inductive Item where
  | event (e : Event)
  | construct (binding : String) (typeName : String) (fields : Assignment)
  | project (binding : String) (record : FieldName) (field : FieldName)
  deriving Repr

/-- The events one item presents to the inherited ownership checker.

**The two aggregate items present none**, which is the whole of the v1
field-custody decision expressed where it can be checked. -/
def custody : Item → List Event
  | .event e => [e]
  | .construct _ _ _ => []
  | .project _ _ _ => []

def custodyList : List Item → List Event
  | [] => []
  | item :: rest => custody item ++ custodyList rest

theorem a_record_item_presents_no_event
    (binding typeName : String) (fields : Assignment) :
    custody (.construct binding typeName fields) = [] := rfl

theorem a_field_read_presents_no_event (binding record field : String) :
    custody (.project binding record field) = [] := rfl

/-- A program of nothing but record operations presents the inherited checker
with the empty operation list — so it can open no obligation and leak nothing. -/
theorem a_record_only_program_presents_no_event :
    custodyList [.construct "p" "Point" [("x", 1)], .project "a" "p" "x"] = [] := by
  rfl

/-- And therefore leaves the ledger exactly as it found it. -/
theorem a_record_only_program_leaves_the_ledger_unchanged (ledger : Ledger) :
    run ledger (custodyList [.construct "p" "Point" [("x", 1)], .project "a" "p" "x"])
      = ledger := by
  rfl

/-- Neutrality, against a predicate written independently of `custody`: a
sequence that holds no record presents exactly its own events, so every program
written before this card is unchanged by it. -/
def recordFree : Item → Bool
  | .event _ => true
  | .construct _ _ _ => false
  | .project _ _ _ => false

theorem a_record_free_sequence_presents_itself (items : List Item)
    (free : items.all recordFree = true) :
    custodyList items = items.filterMap (fun item =>
      match item with
      | .event e => some e
      | _ => none) := by
  induction items with
  | nil => rfl
  | cons item rest ih =>
    simp only [List.all_cons, Bool.and_eq_true] at free
    cases item with
    | event e =>
      simp [custodyList, custody, ih free.2]
    | construct _ _ _ => simp [recordFree] at free
    | project _ _ _ => simp [recordFree] at free

/-- Non-vacuity for the predicate above: a record item is exactly what it
excludes. -/
theorem a_record_item_is_not_record_free
    (binding typeName : String) (fields : Assignment) :
    recordFree (.construct binding typeName fields) = false := rfl

/-! ## Effects

A record operation derives **nothing**: constructing one allocates no resource
and reading a field mutates no object. Stated against `QCore.StdBuffer`'s own
effect vocabulary so it is the same `Effect` the route's declared row admits. -/

def itemEffects : Item → List Effect
  | .event e => eventEffects e
  | .construct _ _ _ => []
  | .project _ _ _ => []

theorem a_construction_derives_nothing
    (binding typeName : String) (fields : Assignment) :
    itemEffects (.construct binding typeName fields) = [] := rfl

theorem a_field_read_derives_nothing (binding record field : String) :
    itemEffects (.project binding record field) = [] := rfl

/-- The contrast that makes the claim informative: an acquisition on the
admitting route *does* derive `alloc`, so "derives nothing" is a fact about the
aggregate and not about this function. -/
theorem an_acquisition_beside_it_still_derives_alloc (name : String) (extent : Nat) :
    itemEffects (.event (.acquire name extent)) = [Effect.alloc] := rfl

end QCore.SourceRecord
