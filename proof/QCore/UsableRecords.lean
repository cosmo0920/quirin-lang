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

import QCore.FfiStructImport

/-!
# Usable records: field writes, nested paths, records across calls, FFI files

Card **W48-QR-USABLE-RECORDS** (card 2 of the `.qr` language track;
implemented grammars §G07.13, §G02s and §G02u, language specification §L14.6
and §L21, constraint 15k's amendment). The model of

* `RecordValue::project_path` / `RecordValue::write_path`
  (`crates/qcore-core/src/source_record.rs`) — a record value is a tree whose
  inner nodes are records and whose leaves are field values;
* `Program::field_slice` (`crates/qcore-core/src/scalar_calls.rs`) — the same
  tree flattened into its leaves, a field a contiguous slice of them;
* the ownership IR's refill rule (`OwnershipOp::RefillField`,
  `crates/qcore-core/src/ownership.rs`);
* the by-value record argument of `source.scalar-calls.v1` and of an FFI
  helper, and the FFI bundle's file-mark gate
  (`crates/qcore-verification/src/driver.rs`, `compile_ffi_source`).

## What is proved

* **A field write changes exactly that field.** Writing at a path that exists
  makes the path read the written value (`write_changes_exactly_that_field`),
  and every path that diverges from it reads what it read before
  (`write_leaves_every_other_path`).
* **A nested path addresses exactly one sub-object.** Two diverging paths
  never see each other's writes (`a_nested_path_addresses_exactly_one_sub_object`),
  a path through a leaf addresses nothing (`a_path_through_a_leaf_addresses_nothing`),
  and in the flattened layout the slices of two fields of one record are
  disjoint and in declared order (`sibling_slices_are_disjoint`).
* **The owner path over a tree** (the IR's `ComposeNested` and drop-through):
  moving an aggregate under a field conserves the resources it holds
  (`nested_composition_conserves_custody`), the drop-through releases exactly
  the held leaves and leaves holes (`drop_through_tree_releases_exactly_the_live_leaves`),
  and with distinct field names at every level no two owned leaves share a
  path (`compose_keeps_paths_distinct`).
* **The refill never overwrites a live field**: a whole write of an owned
  field is admitted exactly into a hole, and then moves custody rather than
  creating it (`a_refill_is_admitted_exactly_into_a_hole`,
  `a_refill_moves_custody`).
* **Custody is conserved across a record-passing call.** A call admits a
  record argument only when it owns nothing, so the copy the call makes adds
  no resource to any ledger (`an_admitted_record_argument_owns_nothing`,
  `a_record_passing_call_conserves_custody`), and a record that owns is never
  admitted as an argument (`a_record_that_owns_is_never_passed`).
* **An FFI record never leaves an FFI file without the assumption.** A bundle
  is admitted only when every file holding a function takes the mark
  (`an_admitted_ffi_bundle_marks_every_function_file`), and every result it
  publishes carries the unsafe-FFI assumption and passes the tripwire
  (`every_ffi_bundle_result_carries_the_assumption`).

## What this is not

That the Rust reader extracts paths and writes from source tokens faithfully,
and that the emitted C's member assignment is the model's write, are pinned
by `compiler/tests/usable_records.rs` (including sanitized native runs), not
proved here. The leaf-list model of the nested aggregate mirrors the Rust
IR's aggregate binding (its owned leaves as paths); that the Rust walks agree
with it is pinned by the IR's unit tests, not extracted.
-/

namespace QCore.UsableRecords

/-! ## Record values as trees -/

/-- A record value: a leaf field value, or a record of named fields. -/
inductive Val where
  | leaf (n : Int)
  | node (fields : List (String × Val))
  deriving Inhabited

/-- The first field named `f`. -/
def lookup : List (String × Val) → String → Option Val
  | [], _ => none
  | (g, v) :: tl, f => if g = f then some v else lookup tl f

/-- `project_path`: the value at a path, each component but the last a record
field. -/
def project : Val → List String → Option Val
  | v, [] => some v
  | .leaf _, _ :: _ => none
  | .node fs, f :: rest => (lookup fs f).bind (fun v => project v rest)

/-- `write_path`: replace the value at a path; a path the value does not have
changes nothing. -/
def write : Val → List String → Val → Val
  | _, [], w => w
  | .leaf n, _ :: _, _ => .leaf n
  | .node fs, f :: rest, w =>
    .node (fs.map (fun e => if e.1 = f then (e.1, write e.2 rest w) else e))

/-- Two paths that part: they agree on a prefix and then name different
fields, so neither is a prefix of the other. -/
inductive Diverge : List String → List String → Prop
  | here {f g : String} {rest rest' : List String} (h : f ≠ g) :
      Diverge (f :: rest) (g :: rest')
  | later {f : String} {rest rest' : List String} (h : Diverge rest rest') :
      Diverge (f :: rest) (f :: rest')

theorem lookup_map_other (fs : List (String × Val)) (f g : String) (h : f ≠ g)
    (upd : Val → Val) :
    lookup (fs.map (fun e => if e.1 = f then (e.1, upd e.2) else e)) g = lookup fs g := by
  induction fs with
  | nil => rfl
  | cons e tl ih =>
    obtain ⟨k, v⟩ := e
    by_cases hk : k = f
    · subst hk
      simp [lookup, h, ih]
    · by_cases hkg : k = g
      · subst hkg
        simp [lookup, hk]
      · simp [lookup, hk, hkg, ih]

theorem lookup_map_same (fs : List (String × Val)) (f : String) (upd : Val → Val) :
    lookup (fs.map (fun e => if e.1 = f then (e.1, upd e.2) else e)) f
      = (lookup fs f).map upd := by
  induction fs with
  | nil => rfl
  | cons e tl ih =>
    obtain ⟨k, v⟩ := e
    by_cases hk : k = f
    · subst hk
      simp [lookup]
    · simp [lookup, hk, ih]

/-- **A field write changes that field**: writing at a path the value has
makes the path read the written value. -/
theorem write_changes_exactly_that_field :
    ∀ (p : List String) (v w old : Val), project v p = some old →
      project (write v p w) p = some w := by
  intro p
  induction p with
  | nil => intro v w old _; simp [project, write]
  | cons f rest ih =>
    intro v w old h
    cases v with
    | leaf n => simp [project] at h
    | node fs =>
      simp only [project] at h
      cases hl : lookup fs f with
      | none => simp [hl] at h
      | some inner =>
        simp only [hl, Option.bind_some] at h
        simp only [write, project]
        rw [lookup_map_same fs f (fun e => write e rest w), hl]
        simp only [Option.map_some, Option.bind_some]
        exact ih inner w old h

/-- **…and nothing else**: every path that diverges from the written one reads
what it read before. -/
theorem write_leaves_every_other_path :
    ∀ (p q : List String), Diverge p q → ∀ (v w : Val),
      project (write v p w) q = project v q := by
  intro p q hd
  induction hd with
  | @here f g rest rest' hfg =>
    intro v w
    cases v with
    | leaf n => simp [write, project]
    | node fs =>
      simp only [write, project]
      rw [lookup_map_other fs f g hfg (fun e => write e rest w)]
  | @later f rest rest' _ ih =>
    intro v w
    cases v with
    | leaf n => simp [write, project]
    | node fs =>
      simp only [write, project]
      rw [lookup_map_same fs f (fun e => write e rest w)]
      cases hl : lookup fs f with
      | none => simp
      | some inner =>
        simp only [Option.map_some, Option.bind_some]
        exact ih inner w

/-- **A nested path addresses exactly one sub-object**: of two diverging
paths that both exist, a write at one is seen at it and not at the other. -/
theorem a_nested_path_addresses_exactly_one_sub_object
    (p q : List String) (hd : Diverge p q) (v w old other : Val)
    (hp : project v p = some old) (hq : project v q = some other) :
    project (write v p w) p = some w ∧ project (write v p w) q = some other :=
  ⟨write_changes_exactly_that_field p v w old hp,
   (write_leaves_every_other_path p q hd v w).trans hq⟩

/-- A path continuing through a field that is not a record addresses nothing
— the `PathThroughNonRecord` refusal. -/
theorem a_path_through_a_leaf_addresses_nothing (n : Int) (f : String)
    (rest : List String) : project (.leaf n) (f :: rest) = none := rfl

/-- Non-vacuity: a record holding a record, a write at `inner.a`, and the
sibling `inner.b` and the outer `n` unchanged. -/
def sample : Val :=
  .node [("inner", .node [("a", .leaf 1), ("b", .leaf 2)]), ("n", .leaf 3)]

theorem the_sample_write_is_seen_only_at_its_path :
    project (write sample ["inner", "a"] (.leaf 9)) ["inner", "a"] = some (.leaf 9)
      ∧ (project (write sample ["inner", "a"] (.leaf 9)) ["inner", "b"]).isSome
      ∧ (project (write sample ["inner", "a"] (.leaf 9)) ["n"]).isSome := by
  refine ⟨?_, ?_, ?_⟩
  · exact write_changes_exactly_that_field ["inner", "a"] sample (.leaf 9) (.leaf 1) rfl
  · rw [write_leaves_every_other_path ["inner", "a"] ["inner", "b"]
      (.later (.here (by decide))) sample (.leaf 9)]
    rfl
  · rw [write_leaves_every_other_path ["inner", "a"] ["n"] (.here (by decide)) sample
      (.leaf 9)]
    rfl

/-! ## The nested aggregate as the IR holds it

The ownership IR keeps an aggregate's owned **leaves**, each a path below the
record and what it holds (`Binding::Aggregate { fields }` with a field
binding per leaf). `ComposePart::Record` moves a whole aggregate under a field:
each leaf's path gains the field as its head and keeps its state. -/

/-- An aggregate's owned leaves: a path below the record and its resource, or
a hole. -/
abbrev Leaves := List (List String × Option Nat)

/-- A whole aggregate moved under field `f`. -/
def under (f : String) (a : Leaves) : Leaves := a.map (fun l => (f :: l.1, l.2))

/-- `ComposeNested`: the parts in declaration order, each under its field. -/
def compose (parts : List (String × Leaves)) : Leaves := parts.flatMap (fun p => under p.1 p.2)

/-- The resources an aggregate holds, in leaf order. -/
def held (a : Leaves) : List Nat := a.filterMap (·.2)

/-- `drop_through`: every leaf becomes a hole, and the held resources are
released in leaf order. -/
def dropThrough (a : Leaves) : Leaves × List Nat := (a.map (fun l => (l.1, none)), held a)

theorem under_held (f : String) (a : Leaves) : held (under f a) = held a := by
  induction a with
  | nil => rfl
  | cons l tl ih =>
    obtain ⟨p, r⟩ := l
    cases r <;> simp_all [held, under]

/-- **A nested composition conserves custody**: the composed aggregate holds
exactly its parts' resources, in declaration order — none gained, none lost. -/
theorem nested_composition_conserves_custody (parts : List (String × Leaves)) :
    held (compose parts) = parts.flatMap (fun p => held p.2) := by
  induction parts with
  | nil => rfl
  | cons p tl ih =>
    simp only [compose, List.flatMap_cons] at ih ⊢
    rw [← ih, ← under_held p.1 p.2]
    simp [held, List.filterMap_append]

/-- **The drop-through over a tree** releases exactly the held leaves, in
leaf order, and leaves every leaf a hole, so a second drop releases nothing. -/
theorem drop_through_tree_releases_exactly_the_live_leaves (a : Leaves) :
    (dropThrough a).2 = held a ∧ held (dropThrough a).1 = [] := by
  refine ⟨rfl, ?_⟩
  induction a with
  | nil => rfl
  | cons l tl ih => simp_all [dropThrough, held]

theorem under_paths (f : String) (a : Leaves) :
    (under f a).map (·.1) = (a.map (·.1)).map (f :: ·) := by
  simp [under]

/-- Moving an aggregate under a field keeps its leaf paths distinct. -/
theorem under_keeps_paths_distinct (f : String) (a : Leaves) (h : (a.map (·.1)).Nodup) :
    ((under f a).map (·.1)).Nodup := by
  rw [under_paths]
  exact List.Pairwise.map (fun q => f :: q)
    (fun _ _ hne heq => hne (List.cons.inj heq).2) h

/-- Every path of `under f a` starts with `f`. -/
theorem under_path_head (f : String) (a : Leaves) (p : List String)
    (hp : p ∈ (under f a).map (·.1)) : p.head? = some f := by
  rw [under_paths] at hp
  obtain ⟨q, _, rfl⟩ := List.mem_map.mp hp
  rfl

/-- Every path of a composition starts with one of its parts' fields. -/
theorem compose_path_head (parts : List (String × Leaves)) (p : List String)
    (hp : p ∈ (compose parts).map (·.1)) : ∃ g ∈ parts.map (·.1), p.head? = some g := by
  simp only [compose, List.map_flatMap, List.mem_flatMap] at hp
  obtain ⟨part, hpart, hin⟩ := hp
  exact ⟨part.1, List.mem_map_of_mem hpart, under_path_head part.1 part.2 p hin⟩

/-- **No two owned leaves of a nested aggregate share a path**: with the
parts' fields distinct and each part's leaf paths distinct, every leaf path of
the composition is distinct — so a path names one owned leaf, and the
drop-through releases each once. -/
theorem compose_keeps_paths_distinct :
    ∀ (parts : List (String × Leaves)), (parts.map (·.1)).Nodup →
      (∀ part ∈ parts, (part.2.map (·.1)).Nodup) → ((compose parts).map (·.1)).Nodup := by
  intro parts
  induction parts with
  | nil => intro _ _; simp [compose]
  | cons p tl ih =>
    intro hnames hparts
    have htl : (tl.map (·.1)).Nodup := (List.nodup_cons.mp hnames).2
    have hfresh : p.1 ∉ tl.map (·.1) := (List.nodup_cons.mp hnames).1
    have hrest := ih htl (fun part hpart => hparts part (List.mem_cons_of_mem p hpart))
    have hhead := under_keeps_paths_distinct p.1 p.2 (hparts p (List.mem_cons_self))
    have hcompose : compose (p :: tl) = under p.1 p.2 ++ compose tl := by
      simp [compose, List.flatMap_cons]
    rw [hcompose, List.map_append]
    refine List.nodup_append.mpr ⟨hhead, hrest, ?_⟩
    intro x hx y hy hxy
    subst hxy
    obtain ⟨g, hg, hgx⟩ := compose_path_head tl x hy
    have hfx := under_path_head p.1 p.2 x hx
    rw [hfx] at hgx
    cases hgx
    exact hfresh hg

/-! ## The flattened layout (`Program::field_slice`) -/

/-- The first leaf of field `i` of a record whose fields have widths `ws`. -/
def start (ws : List Nat) (i : Nat) : Nat := (ws.take i).sum

theorem start_succ (ws : List Nat) (i : Nat) (hi : i < ws.length) :
    start ws (i + 1) = start ws i + ws[i] := by
  unfold start
  rw [List.take_add_one, List.sum_append]
  simp [List.getElem?_eq_getElem hi]

theorem start_mono (ws : List Nat) : ∀ i j, i ≤ j → start ws i ≤ start ws j := by
  intro i j hij
  unfold start
  obtain ⟨k, rfl⟩ := Nat.exists_eq_add_of_le hij
  rw [List.take_add, List.sum_append]
  exact Nat.le_add_right _ _

/-- **Sibling slices are disjoint and in declared order**: field `i`'s slice
`[start i, start i + w i)` ends at or before field `j`'s begins, for `i < j`,
so no leaf belongs to two fields and a field path names one slice. -/
theorem sibling_slices_are_disjoint (ws : List Nat) (i j : Nat) (hij : i < j)
    (hi : i < ws.length) : start ws i + ws[i] ≤ start ws j := by
  rw [← start_succ ws i hi]
  exact start_mono ws (i + 1) j hij

/-! ## The refill rule (`OwnershipOp::RefillField`) -/

/-- An owned field: holding a resource, or a hole. -/
abbrev Held := Option Nat

/-- The refill: admitted exactly into a hole, moving `r` in. -/
def refill (field : Held) (r : Nat) : Option Held :=
  match field with
  | none => some (some r)
  | some _ => none

/-- The refill is admitted exactly into a hole: over a live field it refuses,
so the buffer that field holds never loses its release. -/
theorem a_refill_is_admitted_exactly_into_a_hole (field : Held) (r : Nat) :
    (refill field r).isSome = true ↔ field = none := by
  cases field <;> simp [refill]

/-- The refill moves custody: the resources held by the source and the field
together are the same before and after. -/
theorem a_refill_moves_custody (r : Nat) (after : Held) (h : refill none r = some after) :
    after = some r := by
  simp [refill] at h
  exact h.symm

/-! ## Custody across a record-passing call -/

/-- A record argument's type: its owned leaves (the resources it would carry).
A record of scalars — every record a call admits — has none. -/
structure RecordArg where
  owned : List Nat

/-- The call-side admission (`source.scalar-calls.v1`, an FFI helper): a
record crosses a call only when it owns nothing. -/
def admitsAsArgument (r : RecordArg) : Bool := r.owned.isEmpty

/-- What a by-value call does to the caller's ledger: the argument is copied,
so every resource it owns would be held twice. -/
def afterCopy (ledger : List Nat) (r : RecordArg) : List Nat := ledger ++ r.owned

theorem an_admitted_record_argument_owns_nothing (r : RecordArg)
    (h : admitsAsArgument r = true) : r.owned = [] := by
  simpa [admitsAsArgument] using h

/-- **Custody is conserved across a record-passing call**: an admitted copy
adds no resource to the ledger. -/
theorem a_record_passing_call_conserves_custody (ledger : List Nat) (r : RecordArg)
    (h : admitsAsArgument r = true) : afterCopy ledger r = ledger := by
  simp [afterCopy, an_admitted_record_argument_owns_nothing r h]

/-- And the refusing side: a record that owns a resource is never admitted as
an argument, so no copy can duplicate a release duty. -/
theorem a_record_that_owns_is_never_passed (r : RecordArg) (x : Nat) (hx : x ∈ r.owned) :
    admitsAsArgument r = false := by
  cases h : r.owned with
  | nil => rw [h] at hx; simp at hx
  | cons y ys => simp [admitsAsArgument, h]

/-! ## FFI records stay in FFI files -/

/-- A file of a bundle, marked when it imports `std.ffi.unsafe`. -/
structure File where
  marked : Bool

/-- `compile_ffi_source`'s gate: every file holding a function takes the mark. -/
def admitsBundle (functionFiles : List File) : Bool :=
  functionFiles.all (·.marked)

/-- An admitted FFI bundle marks every file its code — and so every imported
record it uses — lives in. -/
theorem an_admitted_ffi_bundle_marks_every_function_file (functionFiles : List File)
    (h : admitsBundle functionFiles = true) : ∀ f ∈ functionFiles, f.marked = true := by
  intro f hf
  exact (List.all_eq_true.mp h) f hf

/-- Every result an admitted FFI bundle publishes carries the unsafe-FFI
assumption, passes the publication tripwire and is never plain `Safe`
(`QCore.FfiStructImport.publish`). -/
theorem every_ffi_bundle_result_carries_the_assumption (v : QCore.VerdictSemigroup.V)
    (base : List String) :
    QCore.FfiStructImport.tripwire (QCore.FfiStructImport.publish true v base) = true
      ∧ ¬ QCore.FfiStructImport.plainSafe (QCore.FfiStructImport.publish true v base) :=
  ⟨QCore.FfiStructImport.an_ffi_publication_passes_the_tripwire v base,
   QCore.FfiStructImport.an_ffi_file_never_publishes_plain_safe v base⟩

/-- Non-vacuity: a bundle with an unmarked function file is refused. -/
theorem an_unmarked_function_file_is_refused :
    admitsBundle [⟨true⟩, ⟨false⟩] = false := rfl

end QCore.UsableRecords
