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

import QCore.MemberPathRelease

/-!
# Rule 30 at a direct call, and the scalar guard (card FB-TEARDOWN-DESTROY-DECLARATIONS)

The model of gateway rule 30's direct-call reading (contract specification
C04.19.6, amended) and of the parameter guard of decision 2 (2026-10-16,
"Yes, allow it"). Abstract rule theorems, not an extraction of the Rust
analyzer (`crates/qcore-schemas/src/member_path.rs`: `compose_direct`,
`ParameterGuard::decide`, `state_under`;
`crates/qcore-c-gateway/src/legacy_member_path_route.rs`: `apply_direct`).

* **The direct call.** A direct call runs the declaration exactly once and no
  arm follows it. On one reading of a fate the object linked at the path is
  taken by the call (`#released`, `#moved`, `#stale`) or not (`#kept`), and
  the member is cleared (`#released`, `#moved`) or still designates it
  (`#stale`). `composeDirect` decides `gone` (taken once, the link dropped),
  `stays`, `taken` (taken once, the link kept: a later release by the caller
  would be the second) or refuses.
* **The direct-call state is sound** (`the_direct_call_state_is_sound`):
  `gone` takes the object exactly once on every reading and clears the
  member; `stays` never takes it; `taken` takes it exactly once and the
  member still designates it, so a release after the call is a second
  release (`a_release_after_taken_is_a_double_release`). A set and an
  unstated path never decide (`a_set_never_decides_at_a_direct_call`,
  `an_unstated_path_never_decides`).
* **The guard.** `when #PARAM OP CONST`: a constant actual in range selects
  its arm (`a_constant_selects_its_arm`); an unknown actual gives the
  cautious join of both arms (`an_unknown_value_gives_the_join`: every
  value's reading is among the join's), which never releases
  (`an_unknown_value_never_releases`) and decides only what every value
  decides (`the_join_is_cautious`).
* **Vectors.** `direct_vectors_hold`, `guard_vectors_hold` (Rust
  `DIRECT_VECTORS`, `GUARD_VECTORS`).
-/

namespace QCore.MemberPathDirectCall

open QCore.MemberPathRelease (Fate PathState Refusal members fateOf)

/-- What a direct call does to a tracked object the caller linked at a path. -/
inductive DirectOutcome where
  | gone
  | stays
  | taken
  | refused (r : Refusal)
  deriving DecidableEq, Repr

/-- Rust `compose_direct`. -/
def composeDirect : Option PathState → DirectOutcome
  | none => .refused .unstated
  | some ⟨.kept, _⟩ => .stays
  | some ⟨.released, false⟩ => .gone
  | some ⟨.moved, false⟩ => .gone
  | some ⟨.stale, false⟩ => .taken
  | some ⟨_, true⟩ => .refused .undecided

/-- How many times the call takes the object linked at the path, on one
reading. -/
def takes : Fate → Nat
  | .kept => 0
  | _ => 1

/-- Whether the member reads cleared after the call, on one reading. -/
def cleared : Fate → Bool
  | .released | .moved => true
  | _ => false

/-- **The direct-call state is sound.** -/
theorem the_direct_call_state_is_sound (s : PathState) :
    (composeDirect (some s) = .gone →
        ∀ g ∈ members s, takes g = 1 ∧ cleared g = true) ∧
    (composeDirect (some s) = .stays → ∀ g ∈ members s, takes g = 0) ∧
    (composeDirect (some s) = .taken →
        ∀ g ∈ members s, takes g = 1 ∧ cleared g = false) := by
  obtain ⟨f, m⟩ := s
  cases f <;> cases m <;> simp [composeDirect, members, takes, cleared]

/-- A release by the caller after a `taken` path is the object's second
release on every reading. -/
theorem a_release_after_taken_is_a_double_release (s : PathState)
    (h : composeDirect (some s) = .taken) : ∀ g ∈ members s, takes g + 1 = 2 := by
  obtain ⟨f, m⟩ := s
  cases f <;> cases m <;> simp_all [composeDirect, members, takes]

/-- A set never decides at a direct call: it is gone on one reading and
kept on the other. -/
theorem a_set_never_decides_at_a_direct_call (s : PathState) (hm : s.maybe = true)
    (hf : s.fate ≠ .kept) : ∃ r, composeDirect (some s) = .refused r := by
  obtain ⟨f, m⟩ := s
  simp only at hm hf
  subst hm
  cases f <;> simp_all [composeDirect]

/-- **An unstated path never decides.** -/
theorem an_unstated_path_never_decides : composeDirect none = .refused .unstated := rfl

/-! ### The parameter guard -/

inductive Op where
  | eq
  | ne
  | lt
  | gt
  | le
  | ge
  deriving DecidableEq, Repr

def Op.holds : Op → Int → Int → Bool
  | .eq, v, k => v == k
  | .ne, v, k => v != k
  | .lt, v, k => decide (v < k)
  | .gt, v, k => decide (v > k)
  | .le, v, k => decide (v ≤ k)
  | .ge, v, k => decide (v ≥ k)

/-- `when #PARAM OP CONST`, the constant resolved, with the parameter's
range. -/
structure Guard where
  op : Op
  constant : Int
  low : Int
  high : Int
  deriving DecidableEq, Repr

inductive Decision where
  | holds
  | fails
  | undecided
  deriving DecidableEq, Repr

def inRange (g : Guard) (v : Int) : Bool :=
  decide (g.low ≤ v ∧ v ≤ g.high ∧ g.low ≤ g.constant ∧ g.constant ≤ g.high)

/-- Rust `ParameterGuard::decide`. -/
def decideGuard (g : Guard) : Option Int → Decision
  | some v => if inRange g v then (if g.op.holds v g.constant then .holds else .fails)
      else .undecided
  | none => .undecided

def kept : PathState := ⟨.kept, false⟩

/-- Rust `state_under`. -/
def stateUnder (s : PathState) : Decision → PathState
  | .holds => s
  | .fails => kept
  | .undecided => if s.fate = .kept then kept else ⟨s.fate, true⟩

/-- **A guard on a constant selects its arm.** -/
theorem a_constant_selects_its_arm (g : Guard) (s : PathState) (v : Int)
    (h : inRange g v = true) :
    stateUnder s (decideGuard g (some v)) = if g.op.holds v g.constant then s else kept := by
  simp only [decideGuard, h, ite_true]
  split <;> rfl

/-- **An unknown value gives the join**: every value's reading of the path is
among the readings of the state an unknown actual gives. -/
theorem an_unknown_value_gives_the_join (g : Guard) (s : PathState) (a : Option Int) :
    ∀ f ∈ members (stateUnder s (decideGuard g a)),
      f ∈ members (stateUnder s (decideGuard g none)) := by
  intro f hf
  obtain ⟨sf, sm⟩ := s
  cases a with
  | none => exact hf
  | some v =>
    simp only [decideGuard] at hf ⊢
    split at hf
    · split at hf
      · cases sf <;> cases sm <;> simp_all [stateUnder, members, kept]
      · cases sf <;> cases sm <;> simp_all [stateUnder, members, kept]
    · exact hf

/-- An unknown value never releases: a stated release, move or stale state
read under an unknown actual refuses at a direct call. -/
theorem an_unknown_value_never_releases (g : Guard) (s : PathState) (h : s.fate ≠ .kept) :
    ∃ r, composeDirect (some (stateUnder s (decideGuard g none))) = .refused r := by
  obtain ⟨f, m⟩ := s
  simp only at h
  cases f <;> simp_all [decideGuard, stateUnder, composeDirect]

/-- **The join is cautious**: what an unknown actual decides, every value
decides; otherwise it refuses. -/
theorem the_join_is_cautious (g : Guard) (s : PathState) (o : DirectOutcome)
    (h : composeDirect (some (stateUnder s (decideGuard g none))) = o) :
    (∃ r, o = .refused r) ∨ ∀ v, composeDirect (some (stateUnder s (decideGuard g (some v)))) = o := by
  obtain ⟨f, m⟩ := s
  cases f
  all_goals first
    | (left
       cases m <;> simp [decideGuard, stateUnder, composeDirect] at h <;> exact ⟨_, h.symm⟩)
    | skip
  right
  intro v
  simp only [decideGuard, stateUnder, kept, ite_true] at h
  simp only [decideGuard]
  split
  · split <;> cases m <;> simp_all [stateUnder, composeDirect, kept]
  · simp_all [stateUnder, composeDirect, kept]

/-! ### Vectors -/

def DirectOutcome.word : DirectOutcome → String
  | .gone => "gone"
  | .stays => "stays"
  | .taken => "taken"
  | .refused .doubleRelease => "double_release"
  | .refused .stale => "stale"
  | .refused .undecided => "undecided"
  | .refused .unstated => "unstated"

/-- `DIRECT_VECTORS`: (fate word or `unstated`, maybe, outcome). -/
def directVectors : List (String × Bool × String) := [
  ("released", false, "gone"),
  ("moved", false, "gone"),
  ("stale", false, "taken"),
  ("kept", false, "stays"),
  ("released", true, "undecided"),
  ("moved", true, "undecided"),
  ("stale", true, "undecided"),
  ("unstated", false, "unstated")
]

theorem direct_vectors_hold :
    directVectors.all (fun (f, m, o) =>
      (composeDirect ((fateOf f).map (fun f => ⟨f, m⟩))).word == o) = true := by
  decide

def opOf : String → Op
  | "==" => .eq
  | "!=" => .ne
  | "<" => .lt
  | ">" => .gt
  | "<=" => .le
  | _ => .ge

def Fate.word : Fate → String
  | .released => "released"
  | .moved => "moved"
  | .stale => "stale"
  | .kept => "kept"

def spelling (s : PathState) : String :=
  if s.maybe then "{#" ++ Fate.word s.fate ++ ", #kept}" else "#" ++ Fate.word s.fate

/-- `GUARD_VECTORS`: (operator, actual, constant, stated fate, the state's
spelling at the call), over the `i32` range. -/
def guardVectors : List (String × Option Int × Int × String × String) := [
  ("!=", some 1, 0, "released", "#released"),
  ("!=", some 0, 0, "released", "#kept"),
  ("!=", none, 0, "released", "{#released, #kept}"),
  ("==", some 0, 0, "stale", "#stale"),
  ("==", some 2, 0, "stale", "#kept"),
  ("<", some (-1), 0, "moved", "#moved"),
  (">=", none, 3, "moved", "{#moved, #kept}"),
  ("==", none, 1, "kept", "#kept")
]

theorem guard_vectors_hold :
    guardVectors.all (fun (op, a, k, f, out) =>
      match fateOf f with
      | some f =>
        spelling (stateUnder ⟨f, false⟩
          (decideGuard ⟨opOf op, k, -2147483648, 2147483647⟩ a)) == out
      | none => false) = true := by
  decide

end QCore.MemberPathDirectCall
