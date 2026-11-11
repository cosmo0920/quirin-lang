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
# A callback's member-path release or move-out (card OP-CALLBACK-MEMBER-PATH-RELEASE)

The model of gateway rule 30 (contract specification C04.19.6, grammar
G05d): a callback candidate states what it leaves at a member path of a
formal's object, and a consumer composes that state with the release the
invoking declaration then applies there (C04.19.2's arms), joining over the
candidates. Abstract rule theorems, not an extraction of the Rust analyzer
(`crates/qcore-schemas/src/member_path.rs`: `compose_fate`, `join`,
`compose`, `join_all`, `complements`;
`crates/qcore-c-gateway/src/legacy_member_path_route.rs`: `apply`).

USER DECISION 2026-10-15: "B: state + set" and "Build it now". The value
keywords are `#released`, `#moved`, `#stale` and `#kept`; the set `{V,
#kept}` is one of the two on each path; a guard is absent or the complement
of the hand-off's.

* **The concrete effect.** On one reading of a fate, the object linked at the
  path is taken by the callback (`#released`, `#moved`, `#stale`) or not
  (`#kept`), and the arm then releases what the member still designates
  (nothing after a clear). `count` is how many times the object leaves the
  caller's custody (released or handed on) over the callback and the arm.
* **No double release, no missed release.** A path decided `gone` leaves
  exactly once on every reading the state allows and every arm the join
  allows (`gone_leaves_exactly_once`); a path decided `stays` never leaves
  (`stays_never_leaves`). A stale path is never decided
  (`a_stale_path_never_decides`); across candidates a `gone` join is `gone`
  for each (`a_gone_join_is_gone_for_every_candidate`), and one refusal
  refuses the join (`a_refusal_refuses_the_join`).
* **An unstated member never decides** (`an_unstated_member_never_decides`).
* **The set is the join of its members**
  (`the_set_is_the_join_of_its_members`): the composition of `{V, #kept}`
  is the join of the compositions of `V` and `#kept`.
* **The guard complement matches the hand-off guard**
  (`the_complement_partitions_the_results`): every result is in exactly one
  of the two classes.
* **Vectors.** `compose_vectors_hold` (Rust `COMPOSE_VECTORS`).
-/

namespace QCore.MemberPathRelease

/-- What a call leaves at a member path. -/
inductive Fate where
  | released
  | moved
  | stale
  | kept
  deriving DecidableEq, Repr

/-- A path's state: one fate, or (`maybe`) the set `{fate, #kept}`. -/
structure PathState where
  fate : Fate
  maybe : Bool
  deriving DecidableEq, Repr

/-- The join of the release's arms at the path. -/
inductive ArmJoin where
  | every
  | some
  | none
  deriving DecidableEq, Repr

inductive Refusal where
  | doubleRelease
  | stale
  | undecided
  | unstated
  deriving DecidableEq, Repr

inductive Outcome where
  | gone
  | stays
  | refused (r : Refusal)
  deriving DecidableEq, Repr

/-- Rust `compose_fate`. -/
def composeFate : Fate → ArmJoin → Outcome
  | .kept, .every => .gone
  | .kept, .none => .stays
  | .kept, .some => .refused .undecided
  | .released, _ => .gone
  | .moved, _ => .gone
  | .stale, .none => .refused .stale
  | .stale, _ => .refused .doubleRelease

/-- Rust `join`: equal outcomes stand, a refusal stands over anything,
`gone` against `stays` is undecided. -/
def join (a b : Outcome) : Outcome :=
  if a = b then a
  else
    match a, b with
    | .refused r, _ => .refused r
    | _, .refused r => .refused r
    | _, _ => .refused .undecided

/-- Rust `compose`, written as the table (not through `join`). -/
def compose : Option PathState → ArmJoin → Outcome
  | none, _ => .refused .unstated
  | some ⟨f, false⟩, a => composeFate f a
  | some ⟨.kept, true⟩, a => composeFate .kept a
  | some ⟨.stale, true⟩, .none => .refused .stale
  | some ⟨.stale, true⟩, _ => .refused .doubleRelease
  | some ⟨_, true⟩, .every => .gone
  | some ⟨_, true⟩, _ => .refused .undecided

/-- The fates a state allows. -/
def members (s : PathState) : List Fate :=
  if s.maybe then [s.fate, .kept] else [s.fate]

/-- The join of a list of outcomes (Rust `join_all`). -/
def joinAll : List Outcome → Option Outcome
  | [] => none
  | o :: os => some (os.foldl join o)

/-- Whether the arm may release the path, per join. -/
def armReleases : ArmJoin → List Bool
  | .every => [true]
  | .none => [false]
  | .some => [true, false]

/-- How many times the linked object leaves the caller's custody over the
callback and the arm on one reading: the callback takes it unless `kept`;
the arm releases what the member still designates (a cleared member
designates nothing). -/
def count (f : Fate) (armRel : Bool) : Nat :=
  match f with
  | .kept => if armRel then 1 else 0
  | .released | .moved => 1
  | .stale => if armRel then 2 else 1

/-- The set is the join of its members. -/
theorem the_set_is_the_join_of_its_members (s : PathState) (a : ArmJoin)
    (h : s.fate ≠ .kept ∨ s.maybe = false) :
    some (compose (some s) a) = joinAll ((members s).map (composeFate · a)) := by
  obtain ⟨f, m⟩ := s
  cases m <;> cases f <;> cases a <;> simp_all [compose, members, joinAll, composeFate, join]

/-- **No double release, no missed release** (gone): a path decided `gone`
leaves exactly once on every reading and every arm the join allows. -/
theorem gone_leaves_exactly_once (s : PathState) (a : ArmJoin)
    (h : compose (some s) a = .gone) :
    ∀ g ∈ members s, ∀ b ∈ armReleases a, count g b = 1 := by
  obtain ⟨f, m⟩ := s
  cases m <;> cases f <;> cases a <;>
    simp_all [compose, composeFate, members, armReleases, count]

/-- A path decided `stays` never leaves the caller's custody. -/
theorem stays_never_leaves (s : PathState) (a : ArmJoin)
    (h : compose (some s) a = .stays) :
    ∀ g ∈ members s, ∀ b ∈ armReleases a, count g b = 0 := by
  obtain ⟨f, m⟩ := s
  cases m <;> cases f <;> cases a <;>
    simp_all [compose, composeFate, members, armReleases, count]

/-- A stale path is never decided: some arm would release it again, or a
second invocation would take it again. -/
theorem a_stale_path_never_decides (m : Bool) (a : ArmJoin) :
    ∃ r, compose (some ⟨.stale, m⟩) a = .refused r := by
  cases m <;> cases a <;> simp [compose, composeFate]

/-- **An unstated member never decides.** -/
theorem an_unstated_member_never_decides (a : ArmJoin) :
    compose none a = .refused .unstated := rfl

theorem join_refused_left (r : Refusal) (b : Outcome) : ∃ r', join (.refused r) b = .refused r' := by
  unfold join
  split
  · exact ⟨r, rfl⟩
  · exact ⟨r, rfl⟩

theorem join_refused_right (a : Outcome) (r : Refusal) : ∃ r', join a (.refused r) = .refused r' := by
  unfold join
  split
  · rename_i h
    exact ⟨r, h ▸ rfl⟩
  · cases a <;> simp

theorem foldl_refused (os : List Outcome) (r : Refusal) :
    ∃ r', os.foldl join (.refused r) = .refused r' := by
  induction os generalizing r with
  | nil => exact ⟨r, rfl⟩
  | cons o os ih =>
    obtain ⟨r1, h1⟩ := join_refused_left r o
    simp only [List.foldl_cons, h1]
    exact ih r1

theorem foldl_keeps_refused (os : List Outcome) (acc : Outcome) (r : Refusal)
    (h : Outcome.refused r ∈ os) : ∃ r', os.foldl join acc = .refused r' := by
  induction os generalizing acc with
  | nil => simp at h
  | cons o os ih =>
    simp only [List.foldl_cons]
    rcases List.mem_cons.mp h with h | h
    · subst h
      obtain ⟨r1, h1⟩ := join_refused_right acc r
      rw [h1]
      exact foldl_refused os r1
    · exact ih _ h

/-- **One refusal refuses the join** over the candidates (a stale or
unstated candidate is never outvoted). -/
theorem a_refusal_refuses_the_join (os : List Outcome) (r : Refusal)
    (h : Outcome.refused r ∈ os) : ∃ r', joinAll os = some (.refused r') := by
  cases os with
  | nil => simp at h
  | cons o os =>
    rcases List.mem_cons.mp h with h | h
    · subst h
      obtain ⟨r', h'⟩ := foldl_refused os r
      exact ⟨r', by simp [joinAll, h']⟩
    · obtain ⟨r', h'⟩ := foldl_keeps_refused os o r h
      exact ⟨r', by simp [joinAll, h']⟩

theorem join_gone {a b : Outcome} (h : join a b = .gone) : a = .gone ∧ b = .gone := by
  unfold join at h
  split at h
  · rename_i e
    exact ⟨h, e ▸ h⟩
  · cases a <;> cases b <;> simp_all

theorem foldl_gone (os : List Outcome) (acc : Outcome) (h : os.foldl join acc = .gone) :
    acc = .gone ∧ ∀ o ∈ os, o = .gone := by
  induction os generalizing acc with
  | nil => exact ⟨h, by simp⟩
  | cons o os ih =>
    simp only [List.foldl_cons] at h
    obtain ⟨h1, h2⟩ := ih _ h
    obtain ⟨ha, ho⟩ := join_gone h1
    refine ⟨ha, ?_⟩
    intro x hx
    rcases List.mem_cons.mp hx with hx | hx
    · exact hx ▸ ho
    · exact h2 x hx

/-- **A `gone` join is `gone` for every candidate**: together with
`gone_leaves_exactly_once`, every candidate's every reading takes the object
out of the caller's custody exactly once. -/
theorem a_gone_join_is_gone_for_every_candidate (os : List Outcome)
    (h : joinAll os = some .gone) : ∀ o ∈ os, o = .gone := by
  cases os with
  | nil => simp [joinAll] at h
  | cons o os =>
    simp only [joinAll, Option.some.injEq] at h
    obtain ⟨h1, h2⟩ := foldl_gone os o h
    intro x hx
    rcases List.mem_cons.mp hx with hx | hx
    · exact hx ▸ h1
    · exact h2 x hx

/-! ### The guard complement -/

inductive GuardOp where
  | eq
  | ne
  deriving DecidableEq, Repr

/-- `when #result OP @K`, the constant resolved to its value. -/
structure Guard where
  op : GuardOp
  value : Int
  deriving DecidableEq, Repr

def holds (g : Guard) (r : Int) : Bool :=
  match g.op with
  | .eq => r == g.value
  | .ne => r != g.value

/-- Rust `complements`: the same constant, the other operator. -/
def complements (g h : Guard) : Bool :=
  g.value == h.value && g.op != h.op

/-- **The guard complement matches the hand-off guard**: a member-path
state's guard that complements the hand-off's holds of exactly the results
the hand-off's does not, so every result is in exactly one class. -/
theorem the_complement_partitions_the_results (g h : Guard) (c : complements g h = true)
    (r : Int) : holds g r = !holds h r := by
  obtain ⟨go, gv⟩ := g
  obtain ⟨ho, hv⟩ := h
  simp only [complements, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq] at c
  obtain ⟨hv', hop⟩ := c
  subst hv'
  cases go <;> cases ho <;> simp_all [holds, bne]

/-! ### Vectors -/

def fateOf : String → Option Fate
  | "released" => some .released
  | "moved" => some .moved
  | "stale" => some .stale
  | "kept" => some .kept
  | _ => none

def armOf : String → ArmJoin
  | "every" => .every
  | "some" => .some
  | _ => .none

def Outcome.word : Outcome → String
  | .gone => "gone"
  | .stays => "stays"
  | .refused .doubleRelease => "double_release"
  | .refused .stale => "stale"
  | .refused .undecided => "undecided"
  | .refused .unstated => "unstated"

/-- `COMPOSE_VECTORS`: (fate word, maybe, arm join, outcome). -/
def composeVectors : List (String × Bool × String × String) := [
  ("kept", false, "every", "gone"),
  ("kept", false, "none", "stays"),
  ("kept", false, "some", "undecided"),
  ("released", false, "every", "gone"),
  ("released", false, "none", "gone"),
  ("moved", false, "some", "gone"),
  ("stale", false, "every", "double_release"),
  ("stale", false, "some", "double_release"),
  ("stale", false, "none", "stale"),
  ("moved", true, "every", "gone"),
  ("moved", true, "none", "undecided"),
  ("released", true, "some", "undecided"),
  ("stale", true, "every", "double_release"),
  ("stale", true, "none", "stale"),
  ("unstated", false, "every", "unstated"),
  ("unstated", false, "none", "unstated")
]

theorem compose_vectors_hold :
    composeVectors.all (fun (f, m, a, o) =>
      (compose ((fateOf f).map (fun f => ⟨f, m⟩)) (armOf a)).word == o) = true := by
  decide

end QCore.MemberPathRelease
