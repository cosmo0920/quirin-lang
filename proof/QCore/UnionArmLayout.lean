/-!
# Union places separated by layout (card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION)

The model of gateway rule 12n's union reading, amended in place: a place
inside a union keeps its arm path beside the union's key
(`rd_kafka_op_s.rko_u.*@leaders.cb`), and two places of one union alias only
where Clang's record layout puts them on a shared byte. Abstract rule
theorems, not an extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/callback_member_closure.rs`: `places_alias`,
`Tables::aliases`, `Tables::meets_held`, `Tables::meets_record`,
`merge_layout`, `MemberFacts::probe_layouts`).

The brief's preferred evidence was a discriminant; this card takes its
second option, a layout proof, and these theorems are the layout analogues
of the discriminant ones it names:

* **A separated write never changes the read.** A store over bytes disjoint
  from a read's never changes what the read sees
  (`a_disjoint_write_never_changes_the_read`); two places the rule keeps
  apart are known and disjoint (`separated_places_are_disjoint`), so a store
  into one never changes a read of the other
  (`a_separated_store_never_changes_the_read`).
* **An unplaced or disagreeing layout keeps the union shared.** The whole
  union, or a place whose bytes the layout does not give, aliases every place
  (`an_unplaced_place_aliases`, `an_ungiven_range_aliases`); two units that
  give one place different bytes leave it unplaced
  (`disagreeing_units_unplace`, `a_disagreeing_union_aliases_everything`).
  The same arm path always aliases (`the_same_arm_aliases`), and aliasing is
  symmetric (`places_alias_is_symmetric`).
* **The layout only narrows the shared reading.** Q3's sharing aliases every
  pair of one union's places; the layout's aliasing implies it, so the stores
  a member takes under the layout are among those it took before
  (`the_layout_takes_only_shared_stores`): the layout never adds a candidate
  or a reason to open.
* **Vectors.** `layout_vectors_hold` (Rust `LAYOUT_VECTORS`, arm paths
  numbered).
-/

namespace QCore.UnionArmLayout

/-- A byte range `(offset, size)` within a union. -/
abbrev Range := Nat × Nat

/-- Two ranges share a byte. -/
def overlap (a b : Range) : Bool :=
  decide (a.1 < b.1 + b.2) && decide (b.1 < a.1 + a.2)

/-- Some range of each list shares a byte. -/
def overlapsAny (as bs : List Range) : Bool :=
  as.any fun a => bs.any fun b => overlap a b

/-- Two places may share a byte: only two known, disjoint sets are apart. -/
def meet : Option (List Range) → Option (List Range) → Bool
  | some as, some bs => overlapsAny as bs
  | _, _ => true

/-- Rule 12n's union reading (Rust `places_alias`): two places of one union
(`none`: the whole union), with their ranges (`none`: not given). -/
def placesAlias (arm other : Option Nat) (r s : Option (List Range)) : Bool :=
  match arm, other with
  | some a, some b => a == b || meet r s
  | _, _ => true

/-! ## Bytes -/

/-- A union's bytes. -/
abbrev Bytes := Nat → Nat

/-- A store of the bytes `v` over the range `r`. -/
def write (m : Bytes) (r : Range) (v : Nat → Nat) : Bytes :=
  fun i => if r.1 ≤ i ∧ i < r.1 + r.2 then v i else m i

/-- What a read of the range `q` sees, byte by byte. -/
def read (m : Bytes) (q : Range) : Nat → Option Nat :=
  fun k => if k < q.2 then some (m (q.1 + k)) else none

/-- **A write over bytes disjoint from the read's never changes the read.** -/
theorem a_disjoint_write_never_changes_the_read (m : Bytes) (r q : Range) (v : Nat → Nat)
    (h : overlap r q = false) : read (write m r v) q = read m q := by
  funext k
  unfold read
  by_cases hk : k < q.2
  · have hout : ¬ (r.1 ≤ q.1 + k ∧ q.1 + k < r.1 + r.2) := by
      intro hin
      unfold overlap at h
      rcases Bool.and_eq_false_iff.mp h with h | h
      · have := of_decide_eq_false h
        omega
      · have := of_decide_eq_false h
        omega
    simp only [hk, ↓reduceIte, write, hout]
  · simp only [hk, ↓reduceIte]

theorem overlap_comm (a b : Range) : overlap a b = overlap b a := by
  unfold overlap
  exact Bool.and_comm _ _

theorem overlapsAny_false {as bs : List Range} (h : overlapsAny as bs = false) :
    ∀ a ∈ as, ∀ b ∈ bs, overlap a b = false := by
  intro a ha b hb
  unfold overlapsAny at h
  have h1 := List.any_eq_false.mp h a ha
  simp only [Bool.not_eq_true] at h1
  have h2 := List.any_eq_false.mp h1 b hb
  simpa using h2

/-- **Two places the rule keeps apart are known and disjoint.** -/
theorem separated_places_are_disjoint {arm other : Option Nat} {r s : Option (List Range)}
    (h : placesAlias arm other r s = false) :
    ∃ a b rs ss, arm = some a ∧ other = some b ∧ a ≠ b ∧ r = some rs ∧ s = some ss ∧
      ∀ x ∈ rs, ∀ y ∈ ss, overlap x y = false := by
  match arm, other, r, s, h with
  | some a, some b, some rs, some ss, h =>
    simp only [placesAlias, meet, Bool.or_eq_false_iff, beq_eq_false_iff_ne] at h
    exact ⟨a, b, rs, ss, rfl, rfl, h.1, rfl, rfl, overlapsAny_false h.2⟩
  | some a, some b, none, _, h => simp [placesAlias, meet] at h
  | some a, some b, some _, none, h => simp [placesAlias, meet] at h
  | none, _, _, _, h => simp [placesAlias] at h
  | some _, none, _, _, h => simp [placesAlias] at h

/-- **A store into a place the rule keeps apart never changes a read of the
other.** -/
theorem a_separated_store_never_changes_the_read {arm other : Option Nat}
    {rs ss : List Range} (h : placesAlias arm other (some rs) (some ss) = false)
    (m : Bytes) (v : Nat → Nat) :
    ∀ x ∈ rs, ∀ y ∈ ss, read (write m x v) y = read m y := by
  obtain ⟨_, _, rs', ss', _, _, _, hr, hs, hd⟩ := separated_places_are_disjoint h
  cases hr
  cases hs
  intro x hx y hy
  exact a_disjoint_write_never_changes_the_read m x y v (hd x hx y hy)

/-! ## Fail-closed edges -/

/-- **The whole union aliases every place.** -/
theorem an_unplaced_place_aliases (other : Option Nat) (r s : Option (List Range)) :
    placesAlias none other r s = true ∧ placesAlias other none s r = true := by
  constructor
  · rfl
  · cases other <;> rfl

/-- **A place whose bytes the layout does not give aliases every place.** -/
theorem an_ungiven_range_aliases (arm other : Option Nat) (s : Option (List Range)) :
    placesAlias arm other none s = true := by
  match arm, other with
  | some _, some _ => simp [placesAlias, meet]
  | none, _ => rfl
  | some _, none => rfl

/-- **The same arm path always aliases.** -/
theorem the_same_arm_aliases (a : Nat) (r s : Option (List Range)) :
    placesAlias (some a) (some a) r s = true := by
  simp [placesAlias]

theorem overlapsAny_comm (as bs : List Range) : overlapsAny as bs = overlapsAny bs as := by
  unfold overlapsAny
  cases h : as.any (fun a => bs.any fun b => overlap a b) <;>
    cases h' : bs.any (fun b => as.any fun a => overlap b a) <;> try rfl
  · exfalso
    obtain ⟨b, hb, hb'⟩ := List.any_eq_true.mp h'
    obtain ⟨a, ha, ha'⟩ := List.any_eq_true.mp hb'
    have := overlapsAny_false (as := as) (bs := bs) (by unfold overlapsAny; exact h) a ha b hb
    rw [overlap_comm] at this
    rw [this] at ha'
    exact Bool.false_ne_true ha'
  · exfalso
    obtain ⟨a, ha, ha'⟩ := List.any_eq_true.mp h
    obtain ⟨b, hb, hb'⟩ := List.any_eq_true.mp ha'
    have := overlapsAny_false (as := bs) (bs := as) (by unfold overlapsAny; exact h') b hb a ha
    rw [overlap_comm] at this
    rw [this] at hb'
    exact Bool.false_ne_true hb'

/-- **Aliasing is symmetric.** -/
theorem places_alias_is_symmetric (arm other : Option Nat) (r s : Option (List Range)) :
    placesAlias arm other r s = placesAlias other arm s r := by
  match arm, other, r, s with
  | some a, some b, some rs, some ss =>
    simp only [placesAlias, meet]
    rw [overlapsAny_comm, Bool.beq_comm]
  | some a, some b, none, _ => simp [placesAlias, meet]
  | some a, some b, some _, none => simp [placesAlias, meet]
  | none, none, _, _ => rfl
  | none, some _, _, _ => rfl
  | some _, none, _, _ => rfl

/-- One entry of two units' layouts merged: the same range, or the union
unplaced (`none`). -/
def mergeEntry (a b : Range) : Option Range :=
  if a = b then some a else none

/-- **Two units that give one place different bytes leave it unplaced.** -/
theorem disagreeing_units_unplace {a b : Range} (h : a ≠ b) : mergeEntry a b = none := by
  simp [mergeEntry, h]

/-- **An unplaced union aliases every pair of its places.** -/
theorem a_disagreeing_union_aliases_everything (arm other : Option Nat) :
    placesAlias arm other none none = true :=
  an_ungiven_range_aliases arm other none

/-! ## The layout only narrows the shared reading -/

/-- Q3's sharing: every pair of one union's places aliases. -/
def sharedAlias (_arm _other : Option Nat) : Bool := true

/-- The stores a member takes under an aliasing predicate. -/
def takenBy {α : Type} (p : α → Bool) (stores : List α) : List α := stores.filter p

/-- **The layout never adds a store a member takes**: every store it takes
the shared reading took, so no candidate or reason to open is added. -/
theorem the_layout_takes_only_shared_stores {α : Type} (arm : Option Nat)
    (place : α → Option Nat) (ranges : Option Nat → Option (List Range)) (stores : List α) :
    ∀ x ∈ takenBy (fun st => placesAlias arm (place st) (ranges arm) (ranges (place st))) stores,
      x ∈ takenBy (fun st => sharedAlias arm (place st)) stores := by
  intro x hx
  simp only [takenBy, List.mem_filter] at hx ⊢
  exact ⟨hx.1, rfl⟩

/-! ## Vectors -/

/-- `LAYOUT_VECTORS` (`callback_member_closure.rs`), the arm paths numbered
(`a.cb` 0, `b.cb` 1, `a` 2, `b` 3). -/
def layoutVectors : List (Option Nat × Option Nat × Option (List Range) × Option (List Range) × Bool) := [
  (some 0, some 0, none, none, true),
  (some 0, some 1, some [(0, 8)], some [(32, 8)], false),
  (some 0, some 1, some [(0, 8)], some [(4, 8)], true),
  (some 0, some 1, none, some [(32, 8)], true),
  (none, some 1, some [(0, 8)], some [(32, 8)], true),
  (some 0, some 1, some [(0, 8)], some [(8, 8)], false),
  (some 2, some 3, some [(0, 8), (40, 8)], some [(44, 4)], true)
]

theorem layout_vectors_hold :
    layoutVectors.all (fun (a, b, r, s, expected) =>
      placesAlias a b r s == expected && placesAlias b a s r == expected) = true := by
  decide

end QCore.UnionArmLayout
