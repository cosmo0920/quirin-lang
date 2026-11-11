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
Before this card every caller held every home of the run, so the route did
`callers × homes` home analyses. A caller now holds only the homes it may
reach, and none when they exceed a stated budget.

Model of `crates/qcore-c-gateway/src/legacy_cross_unit.rs` (`scope`,
`for_caller`, the budget refusal in `check_imports`). Whether the caller's
analysis can look a name up only when its unit uses the name (outside a file-scope
prototype) or the run's member facts name it as a value is a Rust/tool boundary recorded in
`docs/reports/current-run-worker-spin/README.md`; here it is the hypothesis
`consulted`.
-/

namespace QCore.CrossUnitReplayBudget

abbrev Name := String

/-- A home is reachable when the caller does not define its name (local
precedence) and either spells it or the run's member facts name it. -/
def reachable (localDefs spelled named homes : List Name) : List Name :=
  homes.filter (fun h => !(localDefs.contains h) && (spelled.contains h || named.contains h))

/-- The caller holds every reachable home when they fit the budget, and
none otherwise. -/
def held (budget : Nat) (localDefs spelled named homes : List Name) : List Name :=
  if (reachable localDefs spelled named homes).length ≤ budget then
    reachable localDefs spelled named homes
  else []

/-- The budget is exhausted exactly when the reachable homes exceed it. -/
def exhausted (budget : Nat) (localDefs spelled named homes : List Name) : Bool :=
  decide (budget < (reachable localDefs spelled named homes).length)

/-! ## Work bound -/

/-- A caller replays at most `budget` homes per check. -/
theorem held_within_budget (budget : Nat) (l s n hs : List Name) :
    (held budget l s n hs).length ≤ budget := by
  unfold held
  split
  · assumption
  · simp

/-- The import work of a run: the homes each caller holds, summed. -/
def replayWork (budget : Nat) (callers : List (List Name × List Name × List Name))
    (homes : List Name) : Nat :=
  (callers.map (fun c => (held budget c.1 c.2.1 c.2.2 homes).length)).sum

/-- The run's home replays per check are at most `callers × budget`,
whatever the number of homes: the route's import work is linear in its
units, never `units × homes`. -/
theorem replay_work_bound (budget : Nat)
    (callers : List (List Name × List Name × List Name)) (homes : List Name) :
    replayWork budget callers homes ≤ callers.length * budget := by
  unfold replayWork
  induction callers with
  | nil => simp
  | cons c cs ih =>
    simp only [List.map_cons, List.sum_cons, List.length_cons]
    have := held_within_budget budget c.1 c.2.1 c.2.2 homes
    rw [Nat.succ_mul]
    omega

/-! ## Fail-closed -/

/-- Every held home is a run home the caller does not define. -/
theorem held_is_a_run_home (budget : Nat) (l s n hs : List Name) (x : Name)
    (h : x ∈ held budget l s n hs) : x ∈ hs ∧ ¬ x ∈ l := by
  unfold held at h
  split at h
  · simp [reachable, List.mem_filter] at h
    exact ⟨h.1, by simpa using h.2.1⟩
  · simp at h

/-- An exhausted budget holds no import. -/
theorem exhausted_holds_none (budget : Nat) (l s n hs : List Name)
    (h : exhausted budget l s n hs = true) : held budget l s n hs = [] := by
  unfold exhausted at h
  unfold held
  have : ¬ (reachable l s n hs).length ≤ budget := by
    simp at h; omega
  simp [this]

/-- Within the budget, a caller holds exactly its reachable homes. -/
theorem within_budget_holds_reachable (budget : Nat) (l s n hs : List Name)
    (h : exhausted budget l s n hs = false) :
    held budget l s n hs = reachable l s n hs := by
  unfold exhausted at h
  unfold held
  have : (reachable l s n hs).length ≤ budget := by
    simp at h; omega
  simp [this]

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

/-- At an exhausted budget a call whose only summary was an import reads
`Unknown`. -/
theorem exhausted_import_call_is_unknown {σ : Type} (declared imports : Name → Option σ)
    (budget : Nat) (l s n hs : List Name) (name : Name)
    (h : exhausted budget l s n hs = true) (undeclared : declared name = none) :
    callReading (lookup declared imports (held budget l s n hs) name) =
      .unknownUncontracted := by
  rw [exhausted_holds_none budget l s n hs h]
  simp [lookup, undeclared, callReading]

/-- Within the budget, every name the caller's analysis can consult (its
spelling or a run member fact, not locally defined) reads exactly what the
full, unrestricted run table supplies: the restriction changes no lookup the
analysis makes. -/
theorem consulted_lookup_unchanged {σ : Type} (declared imports : Name → Option σ)
    (budget : Nat) (l s n hs : List Name) (name : Name)
    (within : exhausted budget l s n hs = false)
    (consulted : ¬ name ∈ l ∧ (name ∈ s ∨ name ∈ n)) :
    lookup declared imports (held budget l s n hs) name =
      lookup declared imports (hs.filter (fun h => !(l.contains h))) name := by
  rw [within_budget_holds_reachable budget l s n hs within]
  unfold lookup
  cases declared name with
  | some c => rfl
  | none =>
    have hnl := consulted.1
    have hor := consulted.2
    simp [reachable, hnl, hor]

/-! ## Vectors bound to the Rust (`SCOPE_VECTORS`) -/

/-- `(homes, local, spelled, named, budget, held, reachable, exhausted)`. -/
def scopeVectors :
    List (List Name × List Name × List Name × List Name × Nat × List Name × Nat × Bool) := [
  -- spelled homes are held; an unspelled, unnamed home is not
  (["a", "b", "c"], [], ["a", "x"], [], 4, ["a"], 1, false),
  -- a member-fact name is held though unspelled
  (["a", "b", "c"], [], ["a"], ["c"], 4, ["a", "c"], 2, false),
  -- a local definition takes precedence over its home
  (["a", "b"], ["a"], ["a", "b"], [], 4, ["b"], 1, false),
  -- exactly at the budget, every reachable home is held
  (["a", "b"], [], ["a", "b"], [], 2, ["a", "b"], 2, false),
  -- above the budget, none is held
  (["a", "b", "c"], [], ["a", "b", "c"], [], 2, [], 3, true),
  -- no home reachable
  (["a"], [], [], [], 0, [], 0, false)
]

theorem scope_vectors_hold :
    scopeVectors.all (fun (hs, l, s, n, b, h, r, e) =>
      held b l s n hs == h && (reachable l s n hs).length == r &&
        exhausted b l s n hs == e) = true := by
  decide

end QCore.CrossUnitReplayBudget
