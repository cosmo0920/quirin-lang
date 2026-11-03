/-!
# Declared integer result sets and the class a test reads (card CELL-GUARD-RESULT-RANGE-CHECKED)

USER DECISION 2026-11-03 (verbatim): "C: a new result-set form that allows
unions, such as result_set(-1 | [0, MAX]). It is the only option that can also
state open's and timerfd_create's results (needed by
RULE-38-VALIDITY-READING-RESULT-SET). It is also the largest grammar change. I
choose this because we must not loosen the custody of the contracts if we
cannot select other options." and (verbatim) "Also, we need to use
mathematical symbols on them and we can use XOR or AND or OR with the normal
symbols of programming languages."

Contract specification C09.4, grammar G05j, gateway rule 38e. Abstract
theorems and shared vectors, not an extraction of the Rust
(`crates/qcore-core/src/result_set.rs`: `SetExpr::eval`, `ResultSet::combine`,
`test_class`, `SET_VECTORS`, `MEMBER_VECTORS`, `CLASS_VECTORS`).

* **The operators' meaning** (`denote_or`, `denote_and`, `denote_xor`): over
  the integers of a C type's universe, `|` is union (∪), `&` intersection (∩)
  and `^` symmetric difference (△); a literal is one value and `[a, b]` the
  closed interval.
* **The class a test reads.** On a known set `w`, a test `t` reads a contract
  class `c` exactly (`HoldsOn`, `FailsOn`) when it agrees with `c` (or with
  its complement) on every member of `w`. Then each edge decides the class
  for every value the call can return
  (`holds_true_edge`, `holds_false_edge`, `fails_true_edge`,
  `fails_false_edge`), and every narrower known set reads it the same way
  (`narrowing_keeps_holds`, `narrowing_keeps_fails`).
* **Only a set in force is read** (`out_of_force_reads_the_universe`,
  `a_decision_out_of_force_is_exact_on_every_value`): a withdrawn, unbound,
  refuted, held or unchecked set is replaced by the whole universe, so any
  decision then taken is one an exact-on-every-value test takes; never a
  decision on an assumed set.
* **Vectors.** `set_vectors_hold`, `member_vectors_hold` (the interval
  implementation agrees with `denote` at each point),
  `class_vectors_hold` (Rust `SET_VECTORS`, `MEMBER_VECTORS`,
  `CLASS_VECTORS`).
-/

namespace QCore.ResultSet

/-- A closed interval `(lo, hi)`. -/
abbrev Interval := Int × Int

/-- Membership in a list of intervals. -/
def memI (s : List Interval) (x : Int) : Bool :=
  s.any (fun i => decide (i.1 ≤ x ∧ x ≤ i.2))

/-- Membership in a universe. -/
def inU (u : Interval) (x : Int) : Bool := decide (u.1 ≤ x ∧ x ≤ u.2)

/-! ## Written sets -/

inductive Bound where
  | lit (k : Int)
  | min
  | max
  deriving DecidableEq, Repr

def Bound.value (u : Interval) : Bound → Int
  | .lit k => k
  | .min => u.1
  | .max => u.2

inductive Expr where
  | val (b : Bound)
  | ival (a b : Bound)
  | or (a b : Expr)
  | and (a b : Expr)
  | xor (a b : Expr)
  deriving DecidableEq, Repr

/-- The meaning of a written set: membership of a value of the universe. -/
def denote (u : Interval) : Expr → Int → Bool
  | .val b, x => inU u x && x == b.value u
  | .ival a b, x => inU u x && decide (a.value u ≤ x ∧ x ≤ b.value u)
  | .or a b, x => denote u a x || denote u b x
  | .and a b, x => denote u a x && denote u b x
  | .xor a b, x => denote u a x != denote u b x

theorem denote_or (u : Interval) (a b : Expr) (x : Int) :
    denote u (.or a b) x = (denote u a x || denote u b x) := rfl

theorem denote_and (u : Interval) (a b : Expr) (x : Int) :
    denote u (.and a b) x = (denote u a x && denote u b x) := rfl

theorem denote_xor (u : Interval) (a b : Expr) (x : Int) :
    denote u (.xor a b) x = (denote u a x != denote u b x) := rfl

/-- Every member of a written set lies in its universe. -/
theorem denote_in_universe (u : Interval) (e : Expr) (x : Int)
    (h : denote u e x = true) : inU u x = true := by
  induction e with
  | val b => simp [denote] at h; exact h.1
  | ival a b => simp [denote] at h; simp [inU]; simp [inU] at h; exact h.1
  | or a b iha ihb =>
      simp [denote] at h
      cases h with
      | inl h => exact iha h
      | inr h => exact ihb h
  | and a b iha _ => simp [denote] at h; exact iha h.1
  | xor a b iha ihb =>
      simp [denote] at h
      cases ha : denote u a x
      · rw [ha] at h; simp at h; exact ihb (by simpa using h.symm)
      · exact iha ha

/-! ## The interval implementation (the Rust `combine`) -/

def insertSorted (x : Int) : List Int → List Int
  | [] => [x]
  | y :: ys => if x ≤ y then x :: y :: ys else y :: insertSorted x ys

def sortInts : List Int → List Int
  | [] => []
  | x :: xs => insertSorted x (sortInts xs)

def dedupFrom (prev : Int) : List Int → List Int
  | [] => []
  | y :: ys => if y == prev then dedupFrom prev ys else y :: dedupFrom y ys

def dedupSorted : List Int → List Int
  | [] => []
  | x :: xs => x :: dedupFrom x xs

/-- Segments between consecutive cuts; the last runs to the universe's top. -/
def segmentsFrom (hi c : Int) : List Int → List Interval
  | [] => [(c, hi)]
  | d :: rest => (c, d - 1) :: segmentsFrom hi d rest

def segments (hi : Int) : List Int → List Interval
  | [] => []
  | c :: rest => segmentsFrom hi c rest

def mergeStep (acc : List Interval) (s : Interval) : List Interval :=
  match acc with
  | (a, b) :: rest => if b + 1 == s.1 then (a, s.2) :: rest else s :: acc
  | [] => [s]

def mergeAdjacent (l : List Interval) : List Interval :=
  (l.foldl mergeStep []).reverse

def cutsOf (u : Interval) (s : List Interval) : List Int :=
  s.foldr (fun i acc => i.1 :: (if i.2 < u.2 then (i.2 + 1) :: acc else acc)) []

/-- Cut the universe at every boundary of both sets, keep each segment whose
first point `f` admits, merge adjacent segments. -/
def combine (f : Bool → Bool → Bool) (u : Interval) (a b : List Interval) : List Interval :=
  let cuts := dedupSorted (sortInts (u.1 :: (cutsOf u a ++ cutsOf u b)))
  mergeAdjacent ((segments u.2 cuts).filter (fun s => f (memI a s.1) (memI b s.1)))

def union (u : Interval) := combine (fun a b => a || b) u
def inter (u : Interval) := combine (fun a b => a && b) u
def symm (u : Interval) := combine (fun a b => a != b) u
def diff (u : Interval) := combine (fun a b => a && !b) u

def clip (u : Interval) (lo hi : Int) : List Interval :=
  if max lo u.1 ≤ min hi u.2 then [(max lo u.1, min hi u.2)] else []

def nonempty (s : List Interval) : Option (List Interval) :=
  if s.isEmpty then none else some s

/-- The loader's reading: every operand and the whole set non-empty, every
literal inside the universe, every interval's bounds in order. -/
def eval (u : Interval) : Expr → Option (List Interval)
  | .val b =>
      let v := b.value u
      if inU u v then some [(v, v)] else none
  | .ival a b =>
      let lo := a.value u
      let hi := b.value u
      if inU u lo && inU u hi && decide (lo ≤ hi) then some [(lo, hi)] else none
  | .or a b => do nonempty (union u (← eval u a) (← eval u b))
  | .and a b => do nonempty (inter u (← eval u a) (← eval u b))
  | .xor a b => do nonempty (symm u (← eval u a) (← eval u b))

/-! ## The class a caller's test reads -/

inductive Cmp where
  | eq | ne | lt | le | gt | ge
  deriving DecidableEq, Repr

def comparisonSet (u : Interval) : Cmp → Int → List Interval
  | .eq, k => clip u k k
  | .ne, k => diff u [u] (clip u k k)
  | .lt, k => clip u u.1 (k - 1)
  | .le, k => clip u u.1 k
  | .gt, k => clip u (k + 1) u.2
  | .ge, k => clip u k u.2

/-- The meaning of a comparison `v OP k`. -/
def cmpHolds : Cmp → Int → Int → Bool
  | .eq, v, k => v == k
  | .ne, v, k => v != k
  | .lt, v, k => decide (v < k)
  | .le, v, k => decide (v ≤ k)
  | .gt, v, k => decide (v > k)
  | .ge, v, k => decide (v ≥ k)

inductive Class where
  | holds | fails | undecided
  deriving DecidableEq, Repr

/-- The class the test's true edge reads of `c` within `w` (the Rust
`test_class`). -/
def classOf (u : Interval) (w t c : List Interval) : Class :=
  let tested := inter u w t
  if tested == inter u w c then .holds
  else if tested == diff u w c then .fails
  else .undecided

/-! ## Semantic statements over membership -/

/-- On every member of `w`, the test agrees with the class. -/
def HoldsOn (w t c : Int → Bool) : Prop := ∀ x, w x = true → t x = c x

/-- On every member of `w`, the test agrees with the class's complement. -/
def FailsOn (w t c : Int → Bool) : Prop := ∀ x, w x = true → t x = !c x

theorem holds_true_edge {w t c : Int → Bool} (h : HoldsOn w t c) {x : Int}
    (hw : w x = true) (ht : t x = true) : c x = true := by
  rw [← h x hw]; exact ht

theorem holds_false_edge {w t c : Int → Bool} (h : HoldsOn w t c) {x : Int}
    (hw : w x = true) (ht : t x = false) : c x = false := by
  rw [← h x hw]; exact ht

theorem fails_true_edge {w t c : Int → Bool} (h : FailsOn w t c) {x : Int}
    (hw : w x = true) (ht : t x = true) : c x = false := by
  have e := h x hw
  rw [ht] at e
  cases hc : c x
  · rfl
  · rw [hc] at e; exact absurd e (by decide)

theorem fails_false_edge {w t c : Int → Bool} (h : FailsOn w t c) {x : Int}
    (hw : w x = true) (ht : t x = false) : c x = true := by
  have e := h x hw
  rw [ht] at e
  cases hc : c x
  · rw [hc] at e; exact absurd e (by decide)
  · rfl

theorem narrowing_keeps_holds {w n t c : Int → Bool}
    (sub : ∀ x, n x = true → w x = true) (h : HoldsOn w t c) : HoldsOn n t c :=
  fun x hn => h x (sub x hn)

theorem narrowing_keeps_fails {w n t c : Int → Bool}
    (sub : ∀ x, n x = true → w x = true) (h : FailsOn w t c) : FailsOn n t c :=
  fun x hn => h x (sub x hn)

/-- How a declared set stands where it is read. -/
inductive Standing where
  | inForce
  | held
  | withdrawn
  | refuted
  | unchecked
  | unbound
  deriving DecidableEq, Repr

/-- The set a reading uses: the declared one only while in force, otherwise
the whole universe. -/
def readingSet (u : Interval) (standing : Standing) (declared : Int → Bool) : Int → Bool :=
  if standing = .inForce then declared else inU u

theorem out_of_force_reads_the_universe (u : Interval) (standing : Standing)
    (declared : Int → Bool) (h : standing ≠ .inForce) :
    readingSet u standing declared = inU u := by
  simp [readingSet, h]

/-- A decision taken on a set out of force is one an exact test takes on
every value of the type: nothing is decided on an assumed set. -/
theorem a_decision_out_of_force_is_exact_on_every_value (u : Interval)
    (standing : Standing) (declared t c : Int → Bool) (h : standing ≠ .inForce)
    (d : HoldsOn (readingSet u standing declared) t c ∨
      FailsOn (readingSet u standing declared) t c) :
    HoldsOn (inU u) t c ∨ FailsOn (inU u) t c := by
  rw [out_of_force_reads_the_universe u standing declared h] at d
  exact d

/-! ## Vectors (shared with the Rust, entry for entry) -/

/-- `int`'s universe. -/
def INT : Interval := (-2147483648, 2147483647)

/-- `(written set, its intervals or none for a refusal)`: Rust `SET_VECTORS`. -/
def setVectors : List (Expr × Option (List Interval)) := [
  (.or (.val (.lit (-1))) (.val (.lit 0)), some [(-1, 0)]),
  (.or (.val (.lit (-1))) (.ival (.lit 0) .max), some [(-1, 2147483647)]),
  (.or (.ival .min (.lit (-2))) (.ival (.lit 1) .max), some [(-2147483648, -2), (1, 2147483647)]),
  (.and (.ival (.lit (-1)) (.lit 5)) (.ival (.lit 0) .max), some [(0, 5)]),
  (.xor (.ival (.lit (-1)) (.lit 0)) (.val (.lit 0)), some [(-1, -1)]),
  (.xor (.ival .min .max) (.val (.lit 0)), some [(-2147483648, -1), (1, 2147483647)]),
  (.or (.val (.lit (-1))) (.and (.ival (.lit 0) (.lit 3)) (.ival (.lit 2) (.lit 9))), some [(-1, -1), (2, 3)]),
  (.and (.or (.val (.lit (-1))) (.ival (.lit 0) (.lit 3))) (.ival (.lit 2) (.lit 9)), some [(2, 3)]),
  (.or (.val (.lit (-1))) (.xor (.val (.lit 0)) (.val (.lit 0))), none),
  (.xor (.or (.val (.lit (-1))) (.val (.lit 0))) (.val (.lit 0)), some [(-1, -1)]),
  (.xor (.ival (.lit 0) (.lit 9)) (.and (.ival (.lit 5) (.lit 12)) (.ival (.lit 0) (.lit 7))), some [(0, 4), (8, 9)]),
  (.or (.val .min) (.val .max), some [(-2147483648, -2147483648), (2147483647, 2147483647)]),
  (.and (.ival (.lit 0) (.lit 5)) (.ival (.lit 6) (.lit 9)), none),
  (.xor (.val (.lit 0)) (.val (.lit 0)), none),
  (.ival (.lit 5) (.lit 1), none),
  (.val (.lit 2147483648), none),
  (.or (.ival (.lit 0) .max) (.and (.ival (.lit 3) (.lit 4)) (.val (.lit 9))), none)
]

theorem set_vectors_hold :
    setVectors.all (fun (e, r) => eval INT e == r) = true := by
  decide +kernel

/-- `(written set, value, member)`: Rust `MEMBER_VECTORS`. Both the meaning
and the implementation agree with the expected membership. -/
def memberVectors : List (Expr × Int × Bool) := [
  (.or (.val (.lit (-1))) (.val (.lit 0)), -1, true),
  (.or (.val (.lit (-1))) (.val (.lit 0)), 0, true),
  (.or (.val (.lit (-1))) (.val (.lit 0)), 1, false),
  (.or (.val (.lit (-1))) (.val (.lit 0)), -2, false),
  (.or (.val (.lit (-1))) (.ival (.lit 0) .max), 2147483647, true),
  (.or (.val (.lit (-1))) (.ival (.lit 0) .max), -2, false),
  (.and (.ival (.lit (-1)) (.lit 5)) (.ival (.lit 0) .max), -1, false),
  (.and (.ival (.lit (-1)) (.lit 5)) (.ival (.lit 0) .max), 5, true),
  (.xor (.ival (.lit (-1)) (.lit 0)) (.val (.lit 0)), 0, false),
  (.xor (.ival (.lit (-1)) (.lit 0)) (.val (.lit 0)), -1, true),
  (.xor (.ival .min .max) (.val (.lit 0)), 0, false),
  (.xor (.ival .min .max) (.val (.lit 0)), -2147483648, true),
  (.xor (.ival (.lit 0) (.lit 9)) (.and (.ival (.lit 5) (.lit 12)) (.ival (.lit 0) (.lit 7))), 6, false),
  (.xor (.ival (.lit 0) (.lit 9)) (.and (.ival (.lit 5) (.lit 12)) (.ival (.lit 0) (.lit 7))), 8, true)
]

theorem member_vectors_hold :
    memberVectors.all (fun (e, x, m) =>
      denote INT e x == m && (eval INT e).any (fun s => memI s x == m)) = true := by
  decide +kernel

/-- The known set: `none` is no set in force (the universe). -/
def known (w : Option Expr) : List Interval :=
  match w with
  | none => [INT]
  | some e => (eval INT e).getD []

/-- `(known set, test operator, test literal, contract class, expected)`:
Rust `CLASS_VECTORS`. -/
def classVectors : List (Option Expr × Cmp × Int × Expr × Class) := [
  (none, .ne, 0, .val (.lit 0), .fails),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .ne, 0, .val (.lit 0), .fails),
  (none, .eq, -1, .val (.lit 0), .undecided),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .eq, -1, .val (.lit 0), .fails),
  (none, .lt, 0, .val (.lit 0), .undecided),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .lt, 0, .val (.lit 0), .fails),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .ge, 0, .val (.lit 0), .holds),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .eq, 0, .val (.lit 0), .holds),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .eq, -2, .val (.lit 0), .undecided),
  (some (.or (.val (.lit (-1))) (.val (.lit 0))), .ne, -2, .val (.lit 0), .undecided),
  (some (.or (.val (.lit (-1))) (.ival (.lit 0) .max)), .gt, 0, .or (.ival .min (.lit (-2))) (.ival (.lit 0) .max), .undecided),
  (some (.or (.val (.lit (-1))) (.ival (.lit 0) .max)), .ge, 0, .or (.ival .min (.lit (-2))) (.ival (.lit 0) .max), .holds),
  (some (.or (.val (.lit (-1))) (.ival (.lit 0) .max)), .lt, 0, .or (.ival .min (.lit (-2))) (.ival (.lit 0) .max), .fails),
  (none, .eq, -1, .or (.ival .min (.lit (-2))) (.ival (.lit 0) .max), .fails),
  (none, .lt, 0, .or (.ival .min (.lit (-2))) (.ival (.lit 0) .max), .undecided)
]

theorem class_vectors_hold :
    classVectors.all (fun (w, op, k, c, r) =>
      classOf INT (known w) (comparisonSet INT op k) ((eval INT c).getD []) == r) = true := by
  decide +kernel

end QCore.ResultSet
