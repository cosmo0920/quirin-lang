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
# The cross-unit import scope and its per-unit replay budget

Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended). Rule 4c's import route
replays every home a caller holds, at certificate issue and again at the
certificate check, and each replay is a whole-unit W4 analysis of the home.
A caller holds only the homes it may reach, and at most a stated budget.

Card **LINK-STANDING-IMPORT-LIMITERS** (rules 4c and 40, amended):

* above the budget a caller no longer holds none: it holds a subset of at
  most the budget (`select`): whole per-function home sets, smallest first,
  then the remaining reachable homes in name order;
* under a derived resolution (rule 40) a home only the run's member facts
  name, whose name has no token in the caller's unit, is not reachable
  (`reachable` with `probe = true`): Clang cannot type a name the unit never
  declares, so rule 40's condition (14) cannot hold for it.

Model of `crates/qcore-c-gateway/src/legacy_cross_unit.rs` (`scope`,
`select`, `for_caller`, `for_linked_caller`, the budget refusal in
`check_imports`). Whether the caller's analysis can look a name up only when
its unit uses the name (outside a file-scope prototype) or the run's member
facts name it as a value is a Rust/tool boundary recorded in
`docs/reports/current-run-worker-spin/README.md`; here it is the hypothesis
`consulted`. The order of the function sets (size, then function position)
and of the fill (home names) is computed by the Rust and given here as the
ordered inputs `groups` and `order`.
-/

namespace QCore.CrossUnitReplayBudget

abbrev Name := String

/-- A home is reachable when the caller does not define its name (local
precedence) and either spells it or the run's member facts name it and it
can be probed in the caller: always on a stated map (`probe = false`), and
under a derived resolution only when its token is `present` in the unit. -/
def reachable (probe : Bool) (present localDefs spelled named homes : List Name) :
    List Name :=
  homes.filter (fun h => !(localDefs.contains h) &&
    (spelled.contains h || (named.contains h && (!probe || present.contains h))))

/-- The member-named homes a derived run leaves unprobed: not defined, not
spelled, named, and without a token in the unit. -/
def unprobed (probe : Bool) (present localDefs spelled named homes : List Name) :
    List Name :=
  homes.filter (fun h => !(localDefs.contains h) && !(spelled.contains h) &&
    named.contains h && probe && !(present.contains h))

/-! ## The budget's subset -/

/-- The union of `acc` and the members of `g` it lacks. -/
def union (acc g : List Name) : List Name :=
  acc ++ g.filter (fun x => !(acc.contains x))

/-- Whole sets, in the given order, each added only when the union fits. -/
def addGroups (budget : Nat) : List Name → List (List Name) → List Name
  | acc, [] => acc
  | acc, g :: gs =>
    if (union acc g).length ≤ budget then addGroups budget (union acc g) gs
    else addGroups budget acc gs

/-- The remaining names, in the given order, while room is left. -/
def fill (budget : Nat) : List Name → List Name → List Name
  | acc, [] => acc
  | acc, x :: xs =>
    if acc.length < budget && !(acc.contains x) then fill budget (acc ++ [x]) xs
    else fill budget acc xs

/-- The homes a caller holds of its reachable `r`: all within the budget;
above it, the ordered function sets (restricted to `r`) then the ordered
fill (restricted to `r`). -/
def select (budget : Nat) (r : List Name) (groups : List (List Name))
    (order : List Name) : List Name :=
  if r.length ≤ budget then r
  else fill budget (addGroups budget [] (groups.map (fun g => g.filter r.contains)))
    (order.filter r.contains)

/-- The held homes of a caller. -/
def held (budget : Nat) (probe : Bool) (present localDefs spelled named homes : List Name)
    (groups : List (List Name)) (order : List Name) : List Name :=
  select budget (reachable probe present localDefs spelled named homes) groups order

/-- The reachable homes exceed the budget. -/
def exhausted (budget : Nat) (probe : Bool) (present localDefs spelled named homes : List Name) :
    Bool :=
  decide (budget < (reachable probe present localDefs spelled named homes).length)

/-! ## Work bound -/

theorem union_length_ge (acc g : List Name) : acc.length ≤ (union acc g).length := by
  simp [union]

theorem addGroups_length (budget : Nat) :
    ∀ (gs : List (List Name)) (acc : List Name), acc.length ≤ budget →
      (addGroups budget acc gs).length ≤ budget
  | [], acc, h => by simpa [addGroups] using h
  | g :: gs, acc, h => by
    unfold addGroups
    split
    · exact addGroups_length budget gs _ (by assumption)
    · exact addGroups_length budget gs acc h

theorem fill_length (budget : Nat) :
    ∀ (xs acc : List Name), acc.length ≤ budget → (fill budget acc xs).length ≤ budget
  | [], acc, h => by simpa [fill] using h
  | x :: xs, acc, h => by
    unfold fill
    split
    · rename_i hc
      have : acc.length < budget := by simp at hc; exact hc.1
      exact fill_length budget xs _ (by simp; omega)
    · exact fill_length budget xs acc h

/-- The subset never exceeds the budget. -/
theorem select_within_budget (budget : Nat) (r : List Name) (gs : List (List Name))
    (o : List Name) : (select budget r gs o).length ≤ budget := by
  unfold select
  split
  · assumption
  · exact fill_length budget _ _ (addGroups_length budget _ [] (by simp))

/-- A caller replays at most `budget` homes per check. -/
theorem held_within_budget (budget : Nat) (p : Bool) (pr l s n hs : List Name)
    (gs : List (List Name)) (o : List Name) :
    (held budget p pr l s n hs gs o).length ≤ budget :=
  select_within_budget budget _ gs o

/-- One caller's inputs: probe, present, local, spelled, named, groups, order. -/
abbrev Caller :=
  Bool × List Name × List Name × List Name × List Name × List (List Name) × List Name

/-- The import work of a run: the homes each caller holds, summed. -/
def replayWork (budget : Nat) (callers : List Caller) (homes : List Name) : Nat :=
  (callers.map (fun c =>
    (held budget c.1 c.2.1 c.2.2.1 c.2.2.2.1 c.2.2.2.2.1 homes c.2.2.2.2.2.1
      c.2.2.2.2.2.2).length)).sum

/-- The run's home replays per check are at most `callers × budget`,
whatever the number of homes: the route's import work is linear in its
units, never `units × homes`. -/
theorem replay_work_bound (budget : Nat) (callers : List Caller) (homes : List Name) :
    replayWork budget callers homes ≤ callers.length * budget := by
  unfold replayWork
  induction callers with
  | nil => simp
  | cons c cs ih =>
    simp only [List.map_cons, List.sum_cons, List.length_cons]
    have := held_within_budget budget c.1 c.2.1 c.2.2.1 c.2.2.2.1 c.2.2.2.2.1 homes
      c.2.2.2.2.2.1 c.2.2.2.2.2.2
    rw [Nat.succ_mul]
    omega

/-! ## Fail-closed -/

theorem addGroups_mem (budget : Nat) (r : List Name) :
    ∀ (gs : List (List Name)) (acc : List Name), (∀ x, x ∈ acc → x ∈ r) →
      (∀ g, g ∈ gs → ∀ x, x ∈ g → x ∈ r) →
      ∀ x, x ∈ addGroups budget acc gs → x ∈ r
  | [], acc, hacc, _, x, hx => by simp [addGroups] at hx; exact hacc x hx
  | g :: gs, acc, hacc, hgs, x, hx => by
    unfold addGroups at hx
    have hrest : ∀ g', g' ∈ gs → ∀ y, y ∈ g' → y ∈ r :=
      fun g' hg' => hgs g' (by simp [hg'])
    split at hx
    · refine addGroups_mem budget r gs _ ?_ hrest x hx
      intro y hy
      simp [union, List.mem_append, List.mem_filter] at hy
      rcases hy with hy | hy
      · exact hacc y hy
      · exact hgs g (by simp) y hy.1
    · exact addGroups_mem budget r gs acc hacc hrest x hx

theorem fill_mem (budget : Nat) (r : List Name) :
    ∀ (xs acc : List Name), (∀ x, x ∈ acc → x ∈ r) → (∀ x, x ∈ xs → x ∈ r) →
      ∀ x, x ∈ fill budget acc xs → x ∈ r
  | [], acc, hacc, _, x, hx => by simp [fill] at hx; exact hacc x hx
  | y :: ys, acc, hacc, hxs, x, hx => by
    unfold fill at hx
    have hrest : ∀ z, z ∈ ys → z ∈ r := fun z hz => hxs z (by simp [hz])
    split at hx
    · refine fill_mem budget r ys _ ?_ hrest x hx
      intro z hz
      simp [List.mem_append] at hz
      rcases hz with hz | hz
      · exact hacc z hz
      · exact hz ▸ hxs y (by simp)
    · exact fill_mem budget r ys acc hacc hrest x hx

/-- Every held home is reachable: the subset is drawn from the reachable
homes only. -/
theorem select_sub_reachable (budget : Nat) (r : List Name) (gs : List (List Name))
    (o : List Name) (x : Name) (h : x ∈ select budget r gs o) : x ∈ r := by
  unfold select at h
  split at h
  · exact h
  · refine fill_mem budget r _ _ ?_ ?_ x h
    · exact addGroups_mem budget r _ [] (by simp) (by
        intro g hg y hy
        simp [List.mem_map] at hg
        obtain ⟨g', _, rfl⟩ := hg
        simp [List.mem_filter] at hy
        exact hy.2)
    · intro y hy
      simp [List.mem_filter] at hy
      exact hy.2

/-- Every held home is a run home the caller does not define. -/
theorem held_is_a_run_home (budget : Nat) (p : Bool) (pr l s n hs : List Name)
    (gs : List (List Name)) (o : List Name) (x : Name)
    (h : x ∈ held budget p pr l s n hs gs o) : x ∈ hs ∧ ¬ x ∈ l := by
  have hr := select_sub_reachable budget _ gs o x h
  simp [reachable, List.mem_filter] at hr
  exact ⟨hr.1, by simpa using hr.2.1⟩

/-- Within the budget, a caller holds exactly its reachable homes. -/
theorem within_budget_holds_reachable (budget : Nat) (p : Bool) (pr l s n hs : List Name)
    (gs : List (List Name)) (o : List Name) (h : exhausted budget p pr l s n hs = false) :
    held budget p pr l s n hs gs o = reachable p pr l s n hs := by
  unfold exhausted at h
  unfold held select
  have : (reachable p pr l s n hs).length ≤ budget := by
    simp at h; omega
  simp [this]

/-- Under a derived resolution, a member-named home with no token in the
caller's unit is never held, at any budget. -/
theorem unprobed_is_never_held (budget : Nat) (pr l s n hs : List Name)
    (gs : List (List Name)) (o : List Name) (x : Name) (hx : x ∈ unprobed true pr l s n hs) :
    ¬ x ∈ held budget true pr l s n hs gs o := by
  intro hh
  have hr := select_sub_reachable budget _ gs o x hh
  simp only [unprobed, reachable, List.mem_filter] at hx hr
  have a := hx.2
  have b := hr.2
  revert a b
  cases l.contains x <;> cases s.contains x <;> cases n.contains x <;>
    cases pr.contains x <;> simp

/-- Leaving the unprobed homes out only shrinks the scope: a derived run's
reachable homes are among the stated run's, so they count less toward the
budget, never more. -/
theorem probing_shrinks_reachable (pr l s n hs : List Name) (x : Name)
    (h : x ∈ reachable true pr l s n hs) : x ∈ reachable false pr l s n hs := by
  simp only [reachable, List.mem_filter] at h ⊢
  refine ⟨h.1, ?_⟩
  have a := h.2
  revert a
  cases l.contains x <;> cases s.contains x <;> cases n.contains x <;>
    cases pr.contains x <;> simp

/-- Rule 4c's lookup, restricted to a held set: a declaration wins, then an
import the caller holds. -/
def lookup {σ : Type} (declared : Name → Option σ) (imports : Name → Option σ)
    (holds : List Name) (name : Name) : Option σ :=
  match declared name with
  | some contract => some contract
  | none => if holds.contains name then imports name else none

/-- Holding fewer homes never supplies a summary the full table did not:
a restricted lookup is the full lookup or no import at all. -/
theorem restricted_lookup_is_full_or_absent {σ : Type} (declared imports : Name → Option σ)
    (holds all : List Name) (name : Name) (sub : ∀ x, x ∈ holds → x ∈ all) :
    lookup declared imports holds name = lookup declared imports all name ∨
      lookup declared imports holds name = declared name := by
  unfold lookup
  cases hd : declared name with
  | some c => left; rfl
  | none =>
    by_cases hh : name ∈ holds
    · left
      have hm : name ∈ all := sub name hh
      simp [hh, hm]
    · right
      simp [hh]

/-- A subset import discharges exactly its imported homes: a held home reads
what the full table supplies. -/
theorem held_lookup_is_full {σ : Type} (declared imports : Name → Option σ)
    (holds all : List Name) (name : Name) (hheld : name ∈ holds) (hall : name ∈ all) :
    lookup declared imports holds name = lookup declared imports all name := by
  unfold lookup
  cases declared name with
  | some c => rfl
  | none => simp [hheld, hall]

/-- A call with no summary in force is uncontracted, and an uncontracted
call reads `Unknown`, never `Safe` (rule 4c's fail-closed import). -/
inductive Reading where
  | summarised
  | unknownUncontracted
  deriving DecidableEq, Repr

def callReading {σ : Type} (summary : Option σ) : Reading :=
  match summary with
  | some _ => .summarised
  | none => .unknownUncontracted

/-- A withheld home stays a refusal: a call whose only summary was the
import of a home the caller does not hold reads `Unknown`. -/
theorem withheld_import_call_is_unknown {σ : Type} (declared imports : Name → Option σ)
    (holds : List Name) (name : Name) (withheld : ¬ name ∈ holds)
    (undeclared : declared name = none) :
    callReading (lookup declared imports holds name) = .unknownUncontracted := by
  simp [lookup, undeclared, withheld, callReading]

/-- Within the budget, every name the caller's analysis can consult (its
spelling, or a run member fact it can probe, not locally defined) reads
exactly what the full, unrestricted run table supplies: the restriction
changes no lookup the analysis makes. -/
theorem consulted_lookup_unchanged {σ : Type} (declared imports : Name → Option σ)
    (budget : Nat) (p : Bool) (pr l s n hs : List Name) (gs : List (List Name))
    (o : List Name) (name : Name) (inRun : name ∈ hs)
    (within : exhausted budget p pr l s n hs = false)
    (consulted : ¬ name ∈ l ∧ (name ∈ s ∨ (name ∈ n ∧ (p = false ∨ name ∈ pr)))) :
    lookup declared imports (held budget p pr l s n hs gs o) name =
      lookup declared imports hs name := by
  rw [within_budget_holds_reachable budget p pr l s n hs gs o within]
  apply held_lookup_is_full _ _ _ _ _ _ inRun
  obtain ⟨hnl, hor⟩ := consulted
  simp only [reachable, List.mem_filter]
  refine ⟨inRun, ?_⟩
  rcases hor with h | ⟨h1, h2⟩
  · simp [hnl, h]
  · rcases h2 with h2 | h2 <;> simp [hnl, h1, h2]

/-! ## Vectors bound to the Rust (`SCOPE_VECTORS`) -/

/-- `(homes, local, spelled, named, probe, present, groups, budget, held,
reachable, unprobed, exhausted)`; the homes are in name order, which is the
fill order. -/
def scopeVectors :
    List (List Name × List Name × List Name × List Name × Bool × List Name ×
      List (List Name) × Nat × List Name × Nat × Nat × Bool) := [
  -- spelled homes are held; an unspelled, unnamed home is not
  (["a", "b", "c"], [], ["a", "x"], [], false, [], [], 4, ["a"], 1, 0, false),
  -- a member-fact name is held though unspelled
  (["a", "b", "c"], [], ["a"], ["c"], false, [], [], 4, ["a", "c"], 2, 0, false),
  -- a local definition takes precedence over its home
  (["a", "b"], ["a"], ["a", "b"], [], false, [], [], 4, ["b"], 1, 0, false),
  -- exactly at the budget, every reachable home is held
  (["a", "b"], [], ["a", "b"], [], false, [], [], 2, ["a", "b"], 2, 0, false),
  -- above the budget: the smaller whole function set first, then by name
  (["a", "b", "c"], [], ["a", "b", "c"], [], false, [], [["c"], ["a", "b"]], 2,
    ["c", "a"], 3, 0, true),
  -- no home reachable
  (["a"], [], [], [], false, [], [], 0, [], 0, 0, false),
  -- derived run: a member-named home with no token in the unit is unprobed
  (["a", "b", "c"], [], ["a"], ["b", "c"], true, ["a", "b"], [], 4, ["a", "b"], 2, 1, false),
  -- derived run: the unprobed homes do not count toward the budget
  (["a", "b", "c"], [], ["a"], ["b", "c"], true, ["a"], [], 1, ["a"], 1, 2, false),
  -- equal-size function sets: the earlier function wins the tie
  (["a", "b", "c", "d"], [], ["a", "b", "c", "d"], [], false, [], [["c", "d"], ["a", "b"]], 2,
    ["c", "d"], 4, 0, true),
  -- a function set larger than the budget is never held whole
  (["a", "b", "c", "d"], [], ["a", "b", "c", "d"], [], false, [], [["d"], ["a", "b", "c"]], 2,
    ["d", "a"], 4, 0, true)
]

theorem scope_vectors_hold :
    scopeVectors.all (fun (hs, l, s, n, p, pr, gs, b, h, r, u, e) =>
      held b p pr l s n hs gs hs == h && (reachable p pr l s n hs).length == r &&
        (unprobed p pr l s n hs).length == u && exhausted b p pr l s n hs == e) = true := by
  decide

end QCore.CrossUnitReplayBudget
