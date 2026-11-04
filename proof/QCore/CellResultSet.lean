import QCore.CellProtocolPerEnd

/-!
# One set reading for every end guard (card CELL-RESULT-SET-ALIGNMENT)

USER REQUEST 2026-11-04 (verbatim): "We need to write down a prompt to align
the result set for our contracts." USER DECISIONS 2026-11-04 (verbatim):
"Basically, Q1(a) and Q2(i) But we must not handle descriptor 0 as a valid
handle. So, we need to exclude the 0 descriptor like: written_set(pipedes,
[1, MAX]); a live descriptor is in [1, MAX]"; "Write [1, MAX]"; Q3 "(α)
live_set suffix"; "Accept the leak".

Gateway rule 38f as generalised: a per-end sentinel's cell has a **live set**
(declared `live_set(SET)`, else rule 38's premise: every `int` but `-1`) and
an **absent set** (declared `absent_set(SET)`, else the guard's dead set). A
test's dead edge reads "absent" only when its values meet the live set
nowhere; its live edge reads "live" only when its values meet the absent set
nowhere; otherwise the edge leaves its names undecided. Abstract rule
theorems over the Rust reading (`crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs`:
`Guard::dead_edge_reads_absent`, `Guard::live_edge_reads_live`,
`Protocol::dead_edge_decides`, `Protocol::guard_follows_live`,
`Protocol::undecided_refusal`, `PER_END_SET_VECTORS`), not an extraction.

The reading is parametric in the sets. The declared `[1, MAX]` is a policy
the user chose, not POSIX's `[0, MAX]`: both are exercised here.

* **Soundness of each edge** (`meets_of_mem`, `the_dead_edge_reading_is_sound`,
  `the_live_edge_reading_is_sound`): a deciding edge holds no value of the
  set it excludes.
* **Parity** (`without_sets_the_reading_is_rule_38f`): with no declared set,
  the reading is `QCore.CellProtocolPerEnd`'s on `!= -1` and `> 0`.
* **The declared bound and its contrast**
  (`the_declared_bound_decides_the_positive_dead_edge`,
  `posix_leaves_descriptor_zero_on_the_positive_dead_edge`).
* **Vectors.** `set_vectors_hold` (Rust `protocol::PER_END_SET_VECTORS`).
-/

namespace QCore.CellResultSet

open QCore.DescriptorCells
open QCore.DescriptorCellProtocol (Guard GuardFact saysLive)
open QCore.CellProtocolPerEnd (Fact cells empty status allLive notLive noneLive adopt
  staleIdentity normalize join tableJoin)

/-! ## Value sets -/

def intMin : Int := -2147483648
def intMax : Int := 2147483647

/-- A value set: closed intervals over `int`. -/
abbrev VSet := List (Int × Int)

def mem (s : VSet) (v : Int) : Bool := s.any (fun p => decide (p.1 ≤ v ∧ v ≤ p.2))

/-- Whether two sets share a value. -/
def meets (a b : VSet) : Bool :=
  a.any (fun p => b.any (fun q => decide (p.1 ≤ q.2 ∧ q.1 ≤ p.2 ∧ p.1 ≤ p.2 ∧ q.1 ≤ q.2)))

/-- Each interval of `a` lies inside one of `b` (so `a ⊆ b`). -/
def within (a b : VSet) : Bool :=
  a.all (fun p => b.any (fun q => decide (q.1 ≤ p.1 ∧ p.2 ≤ q.2)))

/-- **A shared value is a meeting**: if no meeting, no value is in both. -/
theorem meets_of_mem (a b : VSet) (v : Int) (ha : mem a v = true) (hb : mem b v = true) :
    meets a b = true := by
  simp only [mem, meets, List.any_eq_true, decide_eq_true_eq] at *
  obtain ⟨p, hp, hp1, hp2⟩ := ha
  obtain ⟨q, hq, hq1, hq2⟩ := hb
  exact ⟨p, hp, q, hq, by omega⟩

/-! ## The guards' edges -/

def liveEdge : Guard → VSet
  | .notMinusOne => [(intMin, -2), (0, intMax)]
  | .positive => [(1, intMax)]
  | .equalOne => [(1, 1)]

def deadEdge : Guard → VSet
  | .notMinusOne => [(-1, -1)]
  | .positive => [(intMin, 0)]
  | .equalOne => [(intMin, 0), (2, intMax)]

/-- Rule 38's premise: a live descriptor is never `-1`. -/
def premiseLive : VSet := [(intMin, -2), (0, intMax)]

/-- A per-end sentinel with its declared sets (`none`: not declared). -/
structure Spec where
  guard : Guard
  live : Option VSet
  absent : Option VSet

def Spec.liveSet (s : Spec) : VSet := s.live.getD premiseLive
def Spec.absentSet (s : Spec) : VSet := s.absent.getD (deadEdge s.guard)

/-- Rust `Protocol::dead_edge_decides`. -/
def deadDecides (s : Spec) : Bool :=
  s.guard == .equalOne || !meets (deadEdge s.guard) s.liveSet

/-- Rust `Protocol::live_edge_decides`. -/
def liveDecides (s : Spec) : Bool :=
  s.guard == .equalOne || !meets (liveEdge s.guard) s.absentSet

/-- Rust `Protocol::guard_follows_live`: the guard says live whenever its cell
holds a live value. -/
def guardFollowsLive (s : Spec) : Bool :=
  s.guard != .equalOne && within s.liveSet (liveEdge s.guard)

/-- Rust `Protocol::undecided_refusal`: `true` names the zero reading. -/
def zeroReading (s : Spec) : Bool :=
  (mem (deadEdge s.guard) 0 && mem s.liveSet 0) || (mem (liveEdge s.guard) 0 && mem s.absentSet 0)

/-- **A deciding dead edge holds no live value.** -/
theorem the_dead_edge_reading_is_sound (s : Spec) (hs : s.guard ≠ .equalOne)
    (h : deadDecides s = true) (v : Int) (hd : mem (deadEdge s.guard) v = true) :
    mem s.liveSet v = false := by
  have hm : meets (deadEdge s.guard) s.liveSet = false := by
    cases hg : s.guard <;> simp_all [deadDecides]
  cases hv : mem s.liveSet v
  · rfl
  · exact absurd (meets_of_mem _ _ v hd hv) (by simp [hm])

/-- **A deciding live edge holds no absent value.** -/
theorem the_live_edge_reading_is_sound (s : Spec) (hs : s.guard ≠ .equalOne)
    (h : liveDecides s = true) (v : Int) (hl : mem (liveEdge s.guard) v = true) :
    mem s.absentSet v = false := by
  have hm : meets (liveEdge s.guard) s.absentSet = false := by
    cases hg : s.guard <;> simp_all [liveDecides]
  cases hv : mem s.absentSet v
  · rfl
  · exact absurd (meets_of_mem _ _ v hl hv) (by simp [hm])

/-- **Without declared sets the reading is rule 38f's.** -/
theorem without_sets_the_reading_is_rule_38f :
    deadDecides ⟨.notMinusOne, none, none⟩ = QCore.CellProtocolPerEnd.deadEdgeDecides .notMinusOne ∧
    deadDecides ⟨.positive, none, none⟩ = QCore.CellProtocolPerEnd.deadEdgeDecides .positive ∧
    liveDecides ⟨.notMinusOne, none, none⟩ = true ∧ liveDecides ⟨.positive, none, none⟩ = true ∧
    guardFollowsLive ⟨.notMinusOne, none, none⟩ = true ∧
    guardFollowsLive ⟨.positive, none, none⟩ = false := by
  decide

/-- **The declared bound decides the `> 0` dead edge** (USER DECISION
2026-11-04, "Write [1, MAX]"; a declared policy). -/
theorem the_declared_bound_decides_the_positive_dead_edge :
    deadDecides ⟨.positive, some [(1, intMax)], some [(0, 0)]⟩ = true ∧
    liveDecides ⟨.positive, some [(1, intMax)], some [(0, 0)]⟩ = true ∧
    guardFollowsLive ⟨.positive, some [(1, intMax)], some [(0, 0)]⟩ = true := by
  decide

/-- **POSIX's `[0, MAX]` leaves descriptor 0 on the `> 0` dead edge.** -/
theorem posix_leaves_descriptor_zero_on_the_positive_dead_edge :
    deadDecides ⟨.positive, some [(0, intMax)], none⟩ = false ∧
    zeroReading ⟨.positive, some [(0, intMax)], none⟩ = true := by
  decide

/-! ## The per-end group under declared sets -/

def introduced (s : Spec) (live : Bool) (i : Nat) : Fact :=
  let undecided := if live then !liveDecides s else !deadDecides s
  { guard := some (if live then .live else .dead)
    touched := false
    entry := fun k =>
      if k = 0 ∨ k = 1 then
        (if live && !undecided then some (i, .live)
         else if undecided then some (i, .maybe) else none)
      else none
    zero := undecided }

def reached : Fact := { guard := none, touched := true, entry := empty, zero := false }

def holds (s : Spec) (f : Fact) (t : Table) : Bool :=
  !f.touched
    || (allLive f t && (guardFollowsLive s || f.guard == some .live))
    || (noneLive f t && f.guard == some .dead)

/-- A store of `v` to `k` (the guard is `c`, key 0): only a value of the
absent set keeps the sentinel. -/
def store (s : Spec) (f : Fact) (t : Table) (k : Key) (v : Int) : Except Bool Fact :=
  let f := { f with touched := true }
  if !notLive (status f t k) then .error true
  else if k = 0 then
    if saysLive s.guard v || !mem s.absentSet v then .error false
    else .ok { f with guard := some .dead }
  else .ok f

inductive Outcome where
  | holds
  | broken
  | overwritten
  | doubleRelease
  | undecided
  | ownerReleased
  | zeroUndecided
  | edgeUndecided
  deriving DecidableEq, Repr

def refusalOutcome : Refusal → Outcome
  | .undecided => .undecided
  | .overwritten => .overwritten
  | .doubleRelease => .doubleRelease

def releaseHolder (s : Spec) (f : Fact) (t : Table) : Option Outcome :=
  if noneLive f t then none
  else if f.zero then some (if zeroReading s then .zeroUndecided else .edgeUndecided)
  else some .ownerReleased



/-- Run a trace as Rust `run_per_end_vector` does (identity 100). -/
def run (s : Spec) : List QCore.CellProtocolPerEnd.Step → Option Fact → Table → Bool → Outcome
  | [], none, _, _ => .holds
  | [], some f, t, _ => if holds s f t then .holds else .broken
  | .testLive :: rest, _, t, _ => run s rest (some (introduced s true 100)) t true
  | .testDead :: rest, _, t, _ => run s rest (some (introduced s false 100)) t false
  | .close k :: rest, f, t, live =>
    let (f, t) := adopt (f.getD reached) t k
    match t k with
    | none => .undecided
    | some (i, _) =>
      match release t k true with
      | .ok t => run s rest (some (staleIdentity f i)) t live
      | .error r => refusalOutcome r
  | .store k v :: rest, f, t, live =>
    match store s (f.getD reached) t k v with
    | .ok f => run s rest (some f) t live
    | .error true => .overwritten
    | .error false => .broken
  | .join :: rest, f, t, live =>
    match f with
    | none => .holds
    | some f =>
      let other := introduced s (!live) 100
      let (f, t') := normalize f t empty
      let (o, ot) := normalize other empty t'
      run s rest (some (join f o)) (tableJoin t' ot) live
  | .call :: rest, f, t, live =>
    match f with
    | some f => if holds s f t then run s rest none empty live else .broken
    | none => run s rest none empty live
  | .free :: rest, f, t, live =>
    match f with
    | some f =>
      match releaseHolder s f t with
      | none => run s rest none t live
      | some o => o
    | none => run s rest none t live

/-- The declared sets the vectors name: `[1, MAX]` (the decided bound),
POSIX's `[0, MAX]`, a set with a negative live value, and the absent sets. -/
def fromOne : VSet := [(1, intMax)]
def fromZero : VSet := [(0, intMax)]
def minusFive : VSet := [(-5, -5), (1, intMax)]
def absZero : VSet := [(0, 0)]
def absMinusOne : VSet := [(-1, -1)]
def absBoth : VSet := [(-1, 0)]

/-- Rust `PER_END_SET_VECTORS`, entry for entry. -/
def setVectors : List (Spec × List QCore.CellProtocolPerEnd.Step × Outcome) := [
  (⟨.positive, some fromOne, some absZero⟩, [.testDead, .free], .holds),
  (⟨.positive, some fromOne, none⟩, [.testDead, .free], .holds),
  (⟨.positive, some fromZero, none⟩, [.testDead, .free], .zeroUndecided),
  (⟨.positive, some fromOne, some absZero⟩, [.testDead, .close 0], .undecided),
  (⟨.positive, some fromOne, some absZero⟩, [.testLive, .close 0, .join, .free], .holds),
  (⟨.positive, some fromZero, none⟩, [.testLive, .close 0, .join, .free], .zeroUndecided),
  (⟨.positive, some fromOne, some absZero⟩, [.testDead, .store 0 0], .holds),
  (⟨.positive, some fromOne, some absZero⟩, [.testDead, .store 0 (-1)], .broken),
  (⟨.positive, some fromOne, some absZero⟩, [.testLive, .call], .holds),
  (⟨.positive, some fromOne, some absMinusOne⟩, [.testDead, .free], .holds),
  (⟨.positive, some fromOne, some absBoth⟩, [.testDead, .free], .holds),
  (⟨.positive, some fromOne, some absMinusOne⟩, [.testLive, .close 0, .store 0 (-1)], .holds),
  (⟨.positive, some fromOne, some absMinusOne⟩, [.testLive, .close 0, .store 0 0], .broken),
  (⟨.positive, some fromOne, some absMinusOne⟩, [.testLive, .close 0, .store 0 (-1), .join, .free], .holds),
  (⟨.positive, some fromOne, some absBoth⟩, [.testDead, .store 0 (-1)], .holds),
  (⟨.positive, some minusFive, none⟩, [.testDead, .free], .edgeUndecided),
  (⟨.notMinusOne, none, some absZero⟩, [.testLive, .free], .zeroUndecided),
  (⟨.notMinusOne, none, some absMinusOne⟩, [.testDead, .free], .holds)
]

theorem set_vectors_hold :
    setVectors.all (fun (s, trace, o) => run s trace none empty false == o) = true := by
  decide

end QCore.CellResultSet
