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
# Descriptor cells (card MK-EVENT-DESCRIPTOR-QUANT, gateway rule 38)

The model of contract specification C04.6.2 and gateway rule 38. USER
DECISION 2026-11-01 (verbatim): "I approve the changes of D1-D5." Abstract
rule theorems, not an extraction of the Rust analyzer
(`crates/gateway-rules/rule-38-descriptor-cells/src/cells.rs`: `Cells`;
`crates/qcore-contract/src/contract_descriptor_cell.rs`: the elaboration).

* **Cells are names.** A descriptor is an identity; a cell (a member of an
  object, or a slot's `int`) is a name of it. The table maps each name to its
  identity and status: `live`, `maybe` (a class no witness decided) or
  `stale`.
* **A fresh identity is owed once** (`a_fresh_identity_is_owed_once`): a
  release through its name leaves no live name of it, and a second release
  refuses (`a_stale_cell_never_releases_again`); through a second name
  (`#same`) the identity is still released once (`the_same_alias_releases_once`).
* **The witness decides the class** (`the_witness_decides_the_cell`); an
  untested witness never discharges (`an_untested_witness_never_discharges`).
* **An untracked cell decides nothing** (`an_untracked_cell_decides_nothing`)
  and **a live or undecided cell refuses at an exit**
  (`an_escaping_cell_refuses`).
* **The member-equality guard** is decided from identities only: equal when
  both sides name one live identity, distinct when they name two live ones,
  which is sound under the hypothesis that two open descriptors never share
  a number (`equal_identities_decide_eq`, `distinct_live_descriptors_decide_ne`);
  otherwise it is undecided, the call refuses and changes no cell
  (`an_undecided_guard_gives_the_join`, `the_join_never_releases`), and a
  guarded removal keeps its record and poisons it
  (`an_undecided_guarded_removal_keeps_and_poisons`). A removal and a cell
  release touch different records and commute
  (`removal_and_cell_release_commute`).
* **Two levels.** Each shorthand and its canonical parts elaborate to one
  canonical set (`each_shorthand_and_its_parts_mean_the_same`,
  `both_levels_are_one_canonical_set`).
* **Vectors.** `decision_vectors_hold` (Rust `cells::DECISION_VECTORS`).
* **The initial member** (card MK-EVENT-FIRST-MEMBER-CELL, C11 6.7.2.1p15):
  a pointer to a record whose first member is the event resolves to the
  event's own place (`the_initial_member_is_the_event_cell`,
  `the_initial_member_is_at_the_record_address`), so the cell table's results
  are unchanged under it
  (`the_cell_table_is_unchanged_under_the_initial_member`); every other
  record, a nested chain, an empty or incomplete record, a mix and any other
  type refuse (`every_other_record_refuses`, `a_nested_first_member_refuses`,
  `a_non_event_first_member_refuses`, `an_empty_or_incomplete_record_refuses`,
  `an_undecided_or_other_type_refuses`, `a_mixed_call_set_has_no_type`,
  `a_refused_view_creates_nothing`); `initial_member_vectors_hold` (Rust
  `initial_member::INITIAL_MEMBER_VECTORS`).
-/

namespace QCore.DescriptorCells

/-- What a name says of its identity. -/
inductive Status where
  | live
  | maybe
  | stale
  deriving DecidableEq, Repr

/-- A cell key (abstract). -/
abbrev Key := Nat

/-- One path's table: each name's identity and status. -/
abbrev Table := Key → Option (Nat × Status)

/-- The refusals. -/
inductive Refusal where
  | undecided
  | overwritten
  | doubleRelease
  deriving DecidableEq, Repr

def mayBeLive : Option (Nat × Status) → Bool
  | some (_, .live) => true
  | some (_, .maybe) => true
  | _ => false

def set (t : Table) (k : Key) (v : Nat × Status) : Table :=
  fun k' => if k' = k then some v else t k'

/-- `#fresh(K)` at `k`, identity `i`; `w`: a witness decides its class. -/
def create (t : Table) (k : Key) (i : Nat) (w : Bool) : Except Refusal Table :=
  if mayBeLive (t k) then .error .overwritten
  else .ok (set t k (i, if w then .maybe else .live))

/-- `#same(S)`: `k` becomes a second name of what `of` names. -/
def alias (t : Table) (k of : Key) : Except Refusal Table :=
  match t of with
  | none => .error .undecided
  | some v => if mayBeLive (t k) then .error .overwritten else .ok (set t k v)

/-- Stale every name of identity `i`. -/
def staleAll (t : Table) (i : Nat) : Table :=
  fun k => match t k with
    | some (j, s) => if j = i then some (j, .stale) else some (j, s)
    | none => none

/-- A release through `k`; `others`: whether some recorded name may be the
cell when `k` names nothing. -/
def release (t : Table) (k : Key) (others : Bool) : Except Refusal Table :=
  match t k with
  | some (i, .live) => .ok (staleAll t i)
  | some (_, .stale) => .error .doubleRelease
  | some (_, .maybe) => .error .undecided
  | none => if others then .error .undecided else .ok t

/-- The witness of identity `i` decided: `holds` makes its undecided names
live, the complement stale. -/
def witness (t : Table) (i : Nat) (holds : Bool) : Table :=
  fun k => match t k with
    | some (j, .maybe) =>
      if j = i then some (j, if holds then .live else .stale) else some (j, .maybe)
    | v => v

/-- Whether some name among `keys` may still be live: the exit refuses. -/
def escapes (t : Table) (keys : List Key) : Bool :=
  keys.any (fun k => mayBeLive (t k))

/-- **A fresh identity is owed once.** -/
theorem a_fresh_identity_is_owed_once (t t' : Table) (k : Key) (i : Nat)
    (h : create t k i false = .ok t') :
    release t' k false = .ok (staleAll t' i) ∧
      ∀ k', (staleAll t' i) k' ≠ some (i, .live) := by
  unfold create at h
  split at h
  · cases h
  · cases h
    refine ⟨?_, ?_⟩
    · simp [release, set]
    · intro k'
      simp only [staleAll]
      split
      · rename_i j s _
        by_cases hj : j = i
        · simp [hj]
        · simp [hj]
      · simp

/-- **A stale cell never releases again.** -/
theorem a_stale_cell_never_releases_again (t : Table) (k : Key) (i : Nat)
    (h : t k = some (i, .stale)) (others : Bool) :
    release t k others = .error .doubleRelease := by
  simp [release, h]

/-- **A release through a second name releases the identity once.** -/
theorem the_same_alias_releases_once (t : Table) (k d : Key) (i : Nat)
    (hk : t k = some (i, .live)) (hd : t d = some (i, .live)) :
    release (staleAll t i) k false = .error .doubleRelease ∧
      release t d false = .ok (staleAll t i) := by
  constructor
  · simp [release, staleAll, hk]
  · simp [release, hd]

/-- **The witness decides the cell.** -/
theorem the_witness_decides_the_cell (t : Table) (k : Key) (i : Nat)
    (h : t k = some (i, .maybe)) :
    witness t i true k = some (i, .live) ∧ witness t i false k = some (i, .stale) := by
  simp [witness, h]

/-- **An untested witness never discharges.** -/
theorem an_untested_witness_never_discharges (t : Table) (k : Key) (i : Nat)
    (h : t k = some (i, .maybe)) (others : Bool) :
    release t k others = .error .undecided := by
  simp [release, h]

/-- **An untracked cell decides nothing.** -/
theorem an_untracked_cell_decides_nothing (t : Table) (k : Key) (h : t k = none) :
    release t k false = .ok t ∧ release t k true = .error .undecided := by
  simp [release, h]

/-- **A live or undecided cell refuses at an exit.** -/
theorem an_escaping_cell_refuses (t : Table) (keys : List Key) (k : Key)
    (hk : k ∈ keys) (h : mayBeLive (t k) = true) : escapes t keys = true := by
  unfold escapes
  exact List.any_eq_true.mpr ⟨k, hk, h⟩

/-- The member-equality guard's decision. -/
inductive Equality where
  | equal
  | distinct
  | undecided
  deriving DecidableEq, Repr

def equality : Option (Nat × Status) → Option (Nat × Status) → Equality
  | some (i, .live), some (j, .live) => if i = j then .equal else .distinct
  | _, _ => .undecided

/-- **Equal identities decide equal numbers**: the guard is equal only when
both sides name one live identity, whose number is then one. -/
theorem equal_identities_decide_eq (m a : Option (Nat × Status))
    (h : equality m a = .equal) :
    ∃ i, m = some (i, .live) ∧ a = some (i, .live) := by
  match m, a with
  | some (i, .live), some (j, .live) =>
    simp only [equality] at h
    split at h
    · rename_i hij
      exact ⟨i, rfl, by rw [hij]⟩
    · cases h
  | some (_, .maybe), _ => simp [equality] at h
  | some (_, .stale), _ => simp [equality] at h
  | none, _ => simp [equality] at h
  | some (_, .live), some (_, .maybe) => simp [equality] at h
  | some (_, .live), some (_, .stale) => simp [equality] at h
  | some (_, .live), none => simp [equality] at h

/-- **Two distinct live descriptors decide distinct numbers**, under the
hypothesis that two open descriptors never share a number (`unique`). -/
theorem distinct_live_descriptors_decide_ne (number : Nat → Int)
    (unique : ∀ i j, number i = number j → i = j)
    (m a : Option (Nat × Status)) (h : equality m a = .distinct) :
    ∃ i j, m = some (i, .live) ∧ a = some (j, .live) ∧ number i ≠ number j := by
  match m, a with
  | some (i, .live), some (j, .live) =>
    simp only [equality] at h
    split at h
    · cases h
    · rename_i hij
      exact ⟨i, j, rfl, rfl, fun hn => hij (unique i j hn)⟩
  | some (_, .maybe), _ => simp [equality] at h
  | some (_, .stale), _ => simp [equality] at h
  | none, _ => simp [equality] at h
  | some (_, .live), some (_, .maybe) => simp [equality] at h
  | some (_, .live), some (_, .stale) => simp [equality] at h
  | some (_, .live), none => simp [equality] at h

/-- The guarded teardown: equal releases both ends, distinct does nothing,
undecided refuses. -/
def teardown (t : Table) (m a w : Key) : Except Refusal Table :=
  match equality (t m) (t a) with
  | .equal => (release t a false).bind (fun t => release t w false)
  | .distinct => .ok t
  | .undecided => .error .undecided

/-- **An undecided guard gives the join**: anything but two live names
refuses. -/
theorem an_undecided_guard_gives_the_join (t : Table) (m a w : Key)
    (h : equality (t m) (t a) = .undecided) : teardown t m a w = .error .undecided := by
  simp [teardown, h]

/-- **The join never releases**: a teardown that changes the table decided
its guard equal. -/
theorem the_join_never_releases (t t' : Table) (m a w : Key)
    (h : teardown t m a w = .ok t') (hne : t' ≠ t) : equality (t m) (t a) = .equal := by
  unfold teardown at h
  split at h
  · assumption
  · cases h; exact absurd rfl hne
  · cases h

/-- A retention record's status under removals (rule 12t). -/
structure Record where
  held : Bool
  poisoned : Bool
  deriving DecidableEq, Repr

/-- A removal; `holder = none`: undecided. -/
def remove (r : Record) (named : Option Bool) : Record :=
  match named with
  | some true => if r.poisoned then r else { r with held := false }
  | some false => { r with poisoned := true }
  | none => { r with poisoned := true }

/-- **An undecided guarded removal keeps its record and poisons it.** -/
theorem an_undecided_guarded_removal_keeps_and_poisons (r : Record) :
    (remove r none).held = r.held ∧ (remove r none).poisoned = true := by
  simp [remove]

/-- **A removal and a cell release commute**: they touch different parts of
the state. -/
def stepRemove (s : Record × Table) (m : Option Bool) : Record × Table :=
  (remove s.1 m, s.2)

def stepRelease (s : Record × Table) (k : Key) : Except Refusal (Record × Table) :=
  (release s.2 k false).map (fun t => (s.1, t))

theorem removal_and_cell_release_commute (s : Record × Table) (k : Key) (m : Option Bool) :
    stepRelease (stepRemove s m) k = (stepRelease s k).map (fun s' => stepRemove s' m) := by
  unfold stepRelease stepRemove
  cases release s.2 k false <;> rfl

/-! ## Two levels -/

/-- A value as written: the shorthand carries its kind, the lower level
states it apart. -/
inductive Value where
  | fresh
  | stale
  | staleOrKept
  | same (slot : Nat)
  deriving DecidableEq, Repr

inductive Kind where
  | descriptor
  deriving DecidableEq, Repr

/-- The canonical parts of one cell: its kind and its per-class state. -/
structure Canonical where
  kind : Option Kind
  value : Value
  guard : Option Int
  deriving DecidableEq, Repr

/-- The shorthand `|-> V(K) [when ...]`. -/
def shorthand (v : Value) (k : Option Kind) (g : Option Int) : Canonical :=
  ⟨k, v, g⟩

/-- The lower level: a kind clause and the state clause, merged. -/
def lowerLevel (kindClause : Option Kind) (v : Value) (g : Option Int) : Canonical :=
  ⟨kindClause, v, g⟩

/-- An agreeing overlap of the two levels merges. -/
def merge (a b : Canonical) : Option Canonical :=
  if a = b then some a else none

theorem each_shorthand_and_its_parts_mean_the_same (v : Value) (g : Option Int) :
    shorthand v (some .descriptor) g = lowerLevel (some .descriptor) v g := rfl

theorem both_levels_are_one_canonical_set (v : Value) (g : Option Int) :
    merge (shorthand v (some .descriptor) g) (lowerLevel (some .descriptor) v g) =
      some (shorthand v (some .descriptor) g) := by
  simp [merge, shorthand, lowerLevel]

/-! ## The shared vectors -/

inductive Step where
  | fresh (k : Key)
  | witnessOk
  | witnessFail
  | stale (k : Key)
  | same (k : Key)
  | join
  deriving DecidableEq, Repr

inductive Outcome where
  | safe
  | escapes
  | undecided
  | overwritten
  | doubleRelease
  deriving DecidableEq, Repr

/-- A join with the empty path: a name live or undecided on one side only is
undecided; a stale one stays stale. -/
def joinEmpty (t : Table) : Table :=
  fun k => match t k with
    | some (_, .stale) => some (0, .stale)
    | some _ => some (0, .maybe)
    | none => none

def refusalOutcome : Refusal → Outcome
  | .undecided => .undecided
  | .overwritten => .overwritten
  | .doubleRelease => .doubleRelease

/-- Run a trace on cells `c = 0` and `d = 1` with identity `1`. -/
def run : List Step → Table → Outcome
  | [], t => if escapes t [0, 1] then .escapes else .safe
  | .fresh k :: rest, t => match create t k 1 true with
    | .ok t => run rest t
    | .error r => refusalOutcome r
  | .witnessOk :: rest, t => run rest (witness t 1 true)
  | .witnessFail :: rest, t => run rest (witness t 1 false)
  | .stale k :: rest, t => match release t k true with
    | .ok t => run rest t
    | .error r => if (List.range 2).any (fun k => (t k).isSome) then refusalOutcome r
      else run rest t
  | .same k :: rest, t => match alias t k 0 with
    | .ok t => run rest t
    | .error r => refusalOutcome r
  | .join :: rest, t => run rest (joinEmpty t)

/-- Rust `DECISION_VECTORS`, step for step (`c` is key 0, `d` key 1). -/
def decisionVectors : List (List Step × Outcome) := [
  ([.fresh 0, .witnessOk, .stale 0], .safe),
  ([.fresh 0, .witnessOk], .escapes),
  ([.fresh 0, .witnessFail], .safe),
  ([.fresh 0, .stale 0], .undecided),
  ([.fresh 0, .witnessOk, .stale 0, .stale 0], .doubleRelease),
  ([.fresh 0, .witnessOk, .same 1, .stale 1, .stale 0], .doubleRelease),
  ([.fresh 0, .witnessOk, .same 1, .stale 1], .safe),
  ([.fresh 0, .witnessOk, .join], .escapes),
  ([.stale 0], .safe),
  ([.fresh 0, .witnessOk, .fresh 0], .overwritten)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (trace, o) => run trace (fun _ => none) == o) = true := by
  decide

/-! ## The initial member (card MK-EVENT-FIRST-MEMBER-CELL)

Rule 38 (1). C11 6.7.2.1p15: a pointer to a structure object, suitably
converted, points to its initial member, and a structure has no padding at
its beginning. A member cell `#data: mk_event.fd` is read through a pointer
to `struct mk_event`; a pointer to a record whose one complete definition
declares its first member `struct mk_event m` is read as `&X->m`, the direct
reading's own place, and nothing else is read (Rust
`initial_member::member_view`). -/

/-- Clang's type of a member cell's actual, before the `void *` conversion. -/
inductive PtrType where
  /-- `struct mk_event *`. -/
  | event
  /-- `struct R *`. -/
  | record (r : Nat)
  /-- A union, `void *`, a pointer to a pointer, anything else. -/
  | other
  deriving DecidableEq, Repr

/-- A member's declared type. -/
inductive FieldType where
  /-- Exactly `struct mk_event`. -/
  | event
  /-- Another record (the next level of a nested chain). -/
  | record (r : Nat)
  /-- A pointer to or array of the event, a qualified event, an integer. -/
  | other
  deriving DecidableEq, Repr

/-- The unit's record facts: the first member (name, declared type) of the one
complete definition of each record, `some none` for a record with no member,
`none` for an incomplete or ambiguous one. -/
abbrev Layout := Nat → Option (Option (Nat × FieldType))

/-- How a member cell's actual names its object. -/
inductive View where
  | direct
  | initial (m : Nat)
  | refused
  deriving DecidableEq, Repr

/-- The view of an actual whose calls agree on `actual` (`none`: they do not,
or a type is unknown). -/
def memberView (actual : Option PtrType) (layout : Layout) : View :=
  match actual with
  | none => .refused
  | some .event => .direct
  | some (.record r) =>
    match layout r with
    | some (some (m, .event)) => .initial m
    | _ => .refused
  | some .other => .refused

/-- The one type every call of the callee in the function agrees on. -/
def agreed : List PtrType → Option PtrType
  | [] => none
  | t :: ts => if ts.all (fun u => u == t) then some t else none

/-- A pointer-valued place as spelled. -/
inductive Place where
  | name (n : Nat)
  /-- `p->m`. -/
  | member (p : Place) (m : Nat)
  /-- `&p->m`. -/
  | addr (p : Place) (m : Nat)
  deriving DecidableEq, Repr

/-- The place a cell is keyed on: the actual itself, its initial member, or
nothing. -/
def resolve : View → Place → Option Place
  | .direct, p => some p
  | .initial m, p => some (.addr p m)
  | .refused, _ => none

/-- A creation through a possibly unresolved key. -/
def createAt (t : Table) (k : Option Key) (i : Nat) (w : Bool) : Except Refusal Table :=
  match k with
  | none => .error .undecided
  | some k => create t k i w

/-- A release through a possibly unresolved key (`others`: some recorded name
may be the cell). -/
def releaseAt (t : Table) (k : Option Key) (others : Bool) : Except Refusal Table :=
  match k with
  | none => if others then .error .undecided else .ok t
  | some k => release t k others

/-- What a possibly unresolved key names. -/
def lookup (t : Table) : Option Key → Option (Nat × Status)
  | none => none
  | some k => t k

/-- **The initial member is the event's cell**: a record pointer whose first
member is the event resolves to the place `&p->m` resolves to directly. -/
theorem the_initial_member_is_the_event_cell (layout : Layout) (r m : Nat) (p : Place)
    (h : layout r = some (some (m, .event))) :
    resolve (memberView (some (.record r)) layout) p =
      resolve (memberView (some .event) layout) (.addr p m) := by
  simp [memberView, resolve, h]

/-- **The cell table's results are unchanged under it**: for every key
encoding, a creation, a release, a second name and the guard's lookup through
the record pointer give what they give through `&p->m`. -/
theorem the_cell_table_is_unchanged_under_the_initial_member (enc : Place → Key)
    (layout : Layout) (r m : Nat) (p : Place) (h : layout r = some (some (m, .event)))
    (t : Table) (i : Nat) (w others : Bool) (against : Option Key) :
    createAt t ((resolve (memberView (some (.record r)) layout) p).map enc) i w =
        createAt t ((resolve (memberView (some .event) layout) (.addr p m)).map enc) i w ∧
      releaseAt t ((resolve (memberView (some (.record r)) layout) p).map enc) others =
        releaseAt t ((resolve (memberView (some .event) layout) (.addr p m)).map enc) others ∧
      equality (lookup t ((resolve (memberView (some (.record r)) layout) p).map enc))
          (lookup t against) =
        equality (lookup t ((resolve (memberView (some .event) layout) (.addr p m)).map enc))
          (lookup t against) ∧
      (resolve (memberView (some (.record r)) layout) p).map enc = some (enc (.addr p m)) := by
  have hk := congrArg (Option.map enc) (the_initial_member_is_the_event_cell layout r m p h)
  refine ⟨?_, ?_, ?_, ?_⟩
  · rw [hk]
  · rw [hk]
  · rw [hk]
  · rw [hk]; simp [memberView, resolve]

/-- **Every other record refuses**: unless its first member is the event. -/
theorem every_other_record_refuses (layout : Layout) (r : Nat)
    (h : ∀ m, layout r ≠ some (some (m, .event))) :
    memberView (some (.record r)) layout = .refused := by
  cases hl : layout r with
  | none => simp [memberView, hl]
  | some first =>
    cases first with
    | none => simp [memberView, hl]
    | some pair =>
      obtain ⟨m, ft⟩ := pair
      cases ft with
      | event => exact absurd hl (h m)
      | record s => simp [memberView, hl]
      | other => simp [memberView, hl]

/-- **A nested first-member chain refuses at its first level.** -/
theorem a_nested_first_member_refuses (layout : Layout) (r m s : Nat)
    (h : layout r = some (some (m, .record s))) :
    memberView (some (.record r)) layout = .refused :=
  every_other_record_refuses layout r (by intro m'; simp [h])

/-- **A first member of another type refuses** (a pointer to or array of the
event, a qualified event, an integer). -/
theorem a_non_event_first_member_refuses (layout : Layout) (r m : Nat)
    (h : layout r = some (some (m, .other))) :
    memberView (some (.record r)) layout = .refused :=
  every_other_record_refuses layout r (by intro m'; simp [h])

/-- **A record with no member, or no one complete definition, refuses.** -/
theorem an_empty_or_incomplete_record_refuses (layout : Layout) (r : Nat)
    (h : layout r = some none ∨ layout r = none) :
    memberView (some (.record r)) layout = .refused := by
  apply every_other_record_refuses layout r
  intro m'
  rcases h with h | h <;> simp [h]

/-- **An unknown type, a mix, a union or any other pointer refuses.** -/
theorem an_undecided_or_other_type_refuses (layout : Layout) :
    memberView none layout = .refused ∧ memberView (some .other) layout = .refused :=
  ⟨rfl, rfl⟩

/-- **A mix of types has no agreed type.** -/
theorem a_mixed_call_set_has_no_type (t u : PtrType) (ts : List PtrType) (hu : u ∈ ts)
    (hne : u ≠ t) : agreed (t :: ts) = none := by
  simp only [agreed, ite_eq_right_iff, reduceCtorEq, imp_false, List.all_eq_true,
    beq_iff_eq]
  intro hall
  exact hne (hall u hu)

/-- **A refused view reads no cell**: the creation refuses. -/
theorem a_refused_view_creates_nothing (enc : Place → Key) (p : Place) (t : Table) (i : Nat)
    (w : Bool) : createAt t ((resolve .refused p).map enc) i w = .error .undecided := rfl

/-- Member offsets of a record laid out from its beginning (name, size). -/
def offsetOf : List (Nat × Nat) → Nat → Option Nat
  | [], _ => none
  | (n, size) :: rest, m => if n = m then some 0 else (offsetOf rest m).map (· + size)

/-- **The initial member is at the record's address** (C11 6.7.2.1p15: no
padding at the beginning), so the converted pointer and `&p->m` are one
address. -/
theorem the_initial_member_is_at_the_record_address (m size : Nat) (rest : List (Nat × Nat)) :
    offsetOf ((m, size) :: rest) m = some 0 := by
  simp [offsetOf]

/-- Rust `INITIAL_MEMBER_VECTORS`, entry for entry (record `ins` is 1,
`outer` 2; member `event` is 0, `fd` 1, `events` 2, `ins` 3). -/
def initialMemberVectors : List (Option PtrType × Option (Option (Nat × FieldType)) × View) := [
  (some .event, none, .direct),
  (some (.record 1), some (some (0, .event)), .initial 0),
  (some (.record 1), some (some (1, .other)), .refused),
  (some (.record 1), some (some (0, .other)), .refused),
  (some (.record 1), some (some (2, .other)), .refused),
  (some (.record 1), some (some (0, .other)), .refused),
  (some (.record 2), some (some (3, .record 1)), .refused),
  (some (.record 1), some none, .refused),
  (some (.record 1), none, .refused),
  (some .other, none, .refused),
  (some .other, none, .refused),
  (some .other, none, .refused),
  (none, none, .refused)
]

theorem initial_member_vectors_hold :
    initialMemberVectors.all (fun (a, first, v) => memberView a (fun _ => first) == v) = true := by
  decide

end QCore.DescriptorCells
