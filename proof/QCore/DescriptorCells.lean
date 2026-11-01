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

end QCore.DescriptorCells
