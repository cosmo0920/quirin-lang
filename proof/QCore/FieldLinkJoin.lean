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
# Partial field links (card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED)

The abstract rule implemented by `crates/qcore-c-gateway/src/field_link_join.rs`
(`join_link`, `escape_step`, `exit_refuses`) and its route-G adapter
`legacy_field_link_join.rs` (`join`, `escape_cascade`, `exit_backstop`).

An admitted aggregate field store records a *field link* `owner -> field ->
child` (O00 blocker B08). Releasing the owner does not discharge the child
(premise P-B08-SHALLOW); escaping the owner carries a live linked child with
it. The join of two paths used to record every child linked on one side only
as an unresolved effect and nothing else: no diagnostic reported it, the exit's
leak report skipped the child, and the escape cascade escaped it anyway. So
`if (n) b->item = it; else k_release(it); k_drop(b);` was `Safe` although the
arm that linked `it` leaks it, and `if (n) b->item = it; return b;` was `Safe`
although the other arm leaks it (`the_base_flow_hides_the_leak`).

The repair:

* the join adds **no** unresolved effect (`the_join_adds_no_unresolved`); it
  records whether each link holds on every path (`joinLink`), so an owner's
  release leaves a possibly-live child to be reported at the exit
  (`a_live_child_at_exit_is_reported`);
* an escape decides each child it reaches (`escapeStep`): it escapes only an
  exactly-live child linked on every path, and refuses a possibly-live child
  that is not, or whose status is mixed. A decided escape leaves no concrete
  path's child live (`a_decided_escape_leaves_no_live_child`), and a
  possibly-live child the escape does not carry on every path is refused
  (`an_escape_over_a_one_sided_live_child_refuses`);
* as a floor for every other rule, a settled exit on a path no refusal reached
  that withholds a possibly-live identity with an unresolved effect reports a
  refusal (`exitRefuses`), so such an exit is never `Safe`
  (`an_unresolved_possibly_live_obligation_at_exit_is_never_safe`).

## What this is not

A model of one child under one owner across two joined paths, and of the
exit's verdict. The rest of the flow (bindings, containment, generations,
collections) is the existing one. That every other rule reports a refusal on a
path where it leaves an identity unresolved (so the path is *unmodeled*) is a
premise of the Rust code, reviewed, not proved; the floor makes a violation of
it on a modeled path a refusal rather than a `Safe`. P-B08-SHALLOW and faithful
extraction of C (trust boundary 28) are stated premises. `linkVectors`,
`escapeVectors`, `exitVectors` and `shapeVectors` bind the Rust tables and the
gateway's verdicts on the pinned shapes mechanically
(`tests/unit/field_link_join.rs`, `tests/field_link_join.rs`).
-/

namespace QCore.FieldLinkJoin

/-! ## Links and their join -/

inductive Link where
  | absent
  | must
  | «partial»
  deriving DecidableEq, Repr

/-- `join_link`: a link holds on every path only when both sides hold it. -/
def joinLink : Link → Link → Link
  | .absent, .absent => .absent
  | .must, .must => .must
  | _, _ => .«partial»

/-- `LINK_VECTORS` of `field_link_join.rs`: `(one, other, joined)`. -/
def linkVectors : List (Link × Link × Link) := [
  (.absent, .absent, .absent),
  (.absent, .must, .«partial»),
  (.absent, .«partial», .«partial»),
  (.must, .absent, .«partial»),
  (.must, .must, .must),
  (.must, .«partial», .«partial»),
  (.«partial», .absent, .«partial»),
  (.«partial», .must, .«partial»),
  (.«partial», .«partial», .«partial»)
]

theorem link_vectors_hold :
    linkVectors.all (fun (a, b, j) => joinLink a b == j) = true := by
  decide

theorem join_link_comm (a b : Link) : joinLink a b = joinLink b a := by
  cases a <;> cases b <;> rfl

/-- A link joined with a side that lacks it never holds on every path. -/
theorem a_one_sided_link_is_partial (a : Link) (h : a ≠ .absent) :
    joinLink a .absent = .«partial» := by
  cases a <;> simp_all [joinLink]

/-! ## Statuses -/

/-- The flow's status bits of one identity; the join is bitwise or. -/
structure Status where
  live : Bool
  released : Bool
  escaped : Bool
  contained : Bool
  deriving DecidableEq, Repr

def Status.join (a b : Status) : Status :=
  ⟨a.live || b.live, a.released || b.released, a.escaped || b.escaped,
    a.contained || b.contained⟩

def liveOnly : Status := ⟨true, false, false, false⟩
def escapedOnly : Status := ⟨false, false, true, false⟩

/-! ## The escape step -/

inductive Step where
  | keep
  | escape
  | withdraw
  | refuse
  deriving DecidableEq, Repr

/-- `escape_step`: `none` is an identity the flow tracks no status for. -/
def escapeStep (isPartial : Bool) : Option Status → Step
  | none => .keep
  | some s =>
      if s = escapedOnly then .keep
      else if s = liveOnly && !isPartial then .escape
      else if s.live then .refuse
      else .withdraw

/-- `ESCAPE_VECTORS` of `field_link_join.rs`: `(partial, tracked, live,
released, escaped, contained, step)`. -/
def escapeVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Step) := [
  (false, false, false, false, false, false, .keep),
  (true, false, false, false, false, false, .keep),
  (false, true, true, false, false, false, .escape),
  (true, true, true, false, false, false, .refuse),
  (false, true, true, true, false, false, .refuse),
  (true, true, true, true, false, false, .refuse),
  (false, true, true, false, true, false, .refuse),
  (false, true, true, false, false, true, .refuse),
  (false, true, false, false, true, false, .keep),
  (true, true, false, false, true, false, .keep),
  (false, true, false, true, false, false, .withdraw),
  (true, true, false, true, false, false, .withdraw),
  (false, true, false, true, true, false, .withdraw),
  (false, true, false, false, false, true, .withdraw),
  (true, true, false, false, false, true, .withdraw),
  (false, true, false, true, false, true, .withdraw)
]

def vectorStatus (tracked live released escaped contained : Bool) : Option Status :=
  if tracked then some ⟨live, released, escaped, contained⟩ else none

theorem escape_vectors_hold :
    escapeVectors.all (fun (p, t, l, r, e, c, step) =>
      escapeStep p (vectorStatus t l r e c) == step) = true := by
  decide

/-- The escape discharges only an exactly-live child linked on every path. -/
theorem escape_only_an_exactly_live_must_child (isPartial : Bool) (s : Option Status)
    (h : escapeStep isPartial s = .escape) : isPartial = false ∧ s = some liveOnly := by
  cases s with
  | none => simp [escapeStep] at h
  | some s =>
    rcases s with ⟨a, b, c, d⟩
    cases isPartial <;> cases a <;> cases b <;> cases c <;> cases d <;> revert h <;> decide

/-- A possibly-live child is never withdrawn silently or kept: the escape
either carries it or refuses. -/
theorem a_possibly_live_child_is_escaped_or_refused (isPartial : Bool) (s : Status)
    (h : s.live = true) :
    escapeStep isPartial (some s) = .escape ∨ escapeStep isPartial (some s) = .refuse := by
  rcases s with ⟨a, b, c, d⟩
  cases isPartial <;> cases a <;> cases b <;> cases c <;> cases d <;> revert h <;> decide

/-- A partial link never lets the escape discharge a possibly-live child. -/
theorem an_escape_over_a_one_sided_live_child_refuses (s : Status) (h : s.live = true) :
    escapeStep true (some s) = .refuse := by
  rcases a_possibly_live_child_is_escaped_or_refused true s h with e | r
  · exact absurd (escape_only_an_exactly_live_must_child true (some s) e).1 (by decide)
  · exact r

/-! ## Two concrete paths and their abstraction -/

/-- The child's concrete state on one path. -/
inductive Conc where
  | absent
  | live
  | released
  | escaped
  deriving DecidableEq, Repr

/-- One path: whether it linked the child under the owner, and the child. -/
structure Path where
  linked : Bool
  child : Conc
  deriving DecidableEq, Repr

def Conc.status : Conc → Option Status
  | .absent => none
  | .live => some liveOnly
  | .released => some ⟨false, true, false, false⟩
  | .escaped => some escapedOnly

def joinStatus : Option Status → Option Status → Option Status
  | none, s => s
  | s, none => s
  | some a, some b => some (a.join b)

def Path.link (p : Path) : Link := if p.linked then .must else .absent

/-- The flow's joined view of the child. -/
def statusOf (p q : Path) : Option Status := joinStatus p.child.status q.child.status
def partialOf (p q : Path) : Bool := joinLink p.link q.link == .«partial»
def reachedOf (p q : Path) : Bool := p.linked || q.linked

/-- The concrete effect of the owner's escape on one path: a live linked child
goes with it. -/
def Path.escapeOwner (p : Path) : Conc :=
  if p.linked && p.child == .live then .escaped else p.child

/-- **A decided escape leaves no concrete path's child live**: whenever the
escape step does not refuse, no path still owes the child after the escape. -/
theorem a_decided_escape_leaves_no_live_child (p q : Path) (hr : reachedOf p q = true)
    (h : escapeStep (partialOf p q) (statusOf p q) ≠ .refuse) :
    p.escapeOwner ≠ .live ∧ q.escapeOwner ≠ .live := by
  rcases p with ⟨l1, c1⟩
  rcases q with ⟨l2, c2⟩
  cases l1 <;> cases c1 <;> cases l2 <;> cases c2 <;> revert hr h <;> decide

/-- A live bit in the joined status names a concrete path whose child is
live. -/
theorem the_live_bit_names_a_concrete_live_path (p q : Path) :
    (∃ s, statusOf p q = some s ∧ s.live = true) ↔ (p.child = .live ∨ q.child = .live) := by
  rcases p with ⟨l1, c1⟩
  rcases q with ⟨l2, c2⟩
  cases c1 <;> cases c2 <;>
    simp [statusOf, joinStatus, Conc.status, Status.join, liveOnly, escapedOnly]

/-! ## The join adds no unresolved effect -/

/-- The unresolved effect the join records for the child: the base recorded
one for every one-sided link; the repair records none. -/
def baseJoinUnresolved (p q : Path) : Bool := p.linked != q.linked
def joinUnresolved (_ _ : Path) : Bool := false

theorem the_join_adds_no_unresolved (p q : Path) : joinUnresolved p q = false := rfl

/-- The exit's leak report after the owner's release (P-B08-SHALLOW: the
release does not reach the child): a possibly-live child with no unresolved
effect. -/
def leakReported (unresolved : Bool) (p q : Path) : Bool :=
  match statusOf p q with
  | some s => s.live && !unresolved
  | none => false

/-- **A child live on some path at the exit is reported** by the repaired join,
whatever the links. -/
theorem a_live_child_at_exit_is_reported (p q : Path)
    (h : p.child = .live ∨ q.child = .live) :
    leakReported (joinUnresolved p q) p q = true := by
  obtain ⟨s, hs, hl⟩ := (the_live_bit_names_a_concrete_live_path p q).2 h
  simp [leakReported, hs, hl, joinUnresolved]

/-! ## Verdicts -/

inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

def Verdict.rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- `Safe < Pending < Unknown < Unsafe`. -/
def Verdict.join (a b : Verdict) : Verdict := if a.rank ≤ b.rank then b else a

def fold (vs : List Verdict) : Verdict := vs.foldl Verdict.join .safe

theorem join_safe_iff (a b : Verdict) : a.join b = .safe ↔ a = .safe ∧ b = .safe := by
  cases a <;> cases b <;> decide

theorem foldl_safe (acc : Verdict) (vs : List Verdict) :
    vs.foldl Verdict.join acc = .safe ↔ acc = .safe ∧ ∀ v ∈ vs, v = .safe := by
  induction vs generalizing acc with
  | nil => simp
  | cons v vs ih =>
    simp only [List.foldl_cons, ih, join_safe_iff, List.mem_cons, forall_eq_or_imp]
    constructor
    · rintro ⟨⟨a, b⟩, c⟩; exact ⟨a, b, c⟩
    · rintro ⟨a, b, c⟩; exact ⟨⟨a, b⟩, c⟩

/-- A function is `Safe` only when every diagnostic is. -/
theorem fold_safe_iff (vs : List Verdict) : fold vs = .safe ↔ ∀ v ∈ vs, v = .safe := by
  simp [fold, foldl_safe]

/-! ## The settled-exit floor -/

/-- `exit_refuses`. -/
def exitRefuses (withheld modeled : Bool) : Bool := withheld && modeled

/-- `EXIT_VECTORS` of `field_link_join.rs`: `(withheld, modeled, refuses)`. -/
def exitVectors : List (Bool × Bool × Bool) := [
  (false, false, false),
  (false, true, false),
  (true, false, false),
  (true, true, true)
]

theorem exit_vectors_hold :
    exitVectors.all (fun (w, m, r) => exitRefuses w m == r) = true := by
  decide

/-- One settled exit: whether it withholds a possibly-live identity with an
unresolved effect, whether no refusal reached its path, and whether the leak
report accused it. -/
structure Exit where
  withheld : Bool
  modeled : Bool
  leak : Bool

/-- The diagnostics of a function with this exit. `poison` is the verdict of
the refusal an unmodeled path passed (the premise: an unmodeled path carries a
path-poisoning diagnostic, whose verdict is never `Safe`). -/
def diagnostics (poison : Verdict) (e : Exit) : List Verdict :=
  (if e.leak then [.«unsafe»] else []) ++
  (if exitRefuses e.withheld e.modeled then [.unknown] else []) ++
  (if e.modeled then [] else [poison])

/-- **An unresolved possibly-live obligation at exit is never `Safe`**: on a
modeled path the floor refuses, on an unmodeled one the refusal the path
passed stands. -/
theorem an_unresolved_possibly_live_obligation_at_exit_is_never_safe
    (poison : Verdict) (hpoison : poison ≠ .safe) (e : Exit) (h : e.withheld = true) :
    fold (diagnostics poison e) ≠ .safe := by
  rw [Ne, fold_safe_iff]
  intro hall
  cases hm : e.modeled
  · exact hpoison (hall poison (by simp [diagnostics, hm]))
  · exact absurd (hall .unknown (by simp [diagnostics, exitRefuses, h, hm])) (by decide)

/-! ## The pinned shapes -/

inductive Action where
  | release
  | escape
  deriving DecidableEq, Repr

/-- Whether a concrete path owes the child at the exit, after the owner's
action. -/
def concreteLeak (p q : Path) : Action → Bool
  | .release => p.child == .live || q.child == .live
  | .escape => p.escapeOwner == .live || q.escapeOwner == .live

/-- The repaired flow's verdict of one shape at its exit. -/
def shapeModel (p q : Path) : Action → Verdict
  | .release => if leakReported (joinUnresolved p q) p q then .«unsafe» else .safe
  | .escape =>
      if !reachedOf p q then
        (if leakReported false p q then .«unsafe» else .safe)
      else match escapeStep (partialOf p q) (statusOf p q) with
        | .refuse => .unknown
        | _ => .safe

/-- The base flow: the join marked a one-sided child unresolved without a
diagnostic, and the escape cascade escaped an exactly-live child regardless. -/
def baseModel (p q : Path) : Action → Verdict
  | .release => if leakReported (baseJoinUnresolved p q) p q then .«unsafe» else .safe
  | .escape => if leakReported false p q && !reachedOf p q then .«unsafe» else .safe

/-- **The repaired model never answers `Safe` over a concrete leak**, and
accuses only a concrete leak. -/
theorem the_model_is_sound (p q : Path) (a : Action) (h : shapeModel p q a = .safe) :
    concreteLeak p q a = false := by
  rcases p with ⟨l1, c1⟩
  rcases q with ⟨l2, c2⟩
  cases a <;> cases l1 <;> cases c1 <;> cases l2 <;> cases c2 <;> revert h <;> decide

theorem the_model_accuses_only_concrete_leaks (p q : Path) (a : Action)
    (h : shapeModel p q a = .«unsafe») : concreteLeak p q a = true := by
  rcases p with ⟨l1, c1⟩
  rcases q with ⟨l2, c2⟩
  cases a <;> cases l1 <;> cases c1 <;> cases l2 <;> cases c2 <;> revert h <;> decide

def linkedLive : Path := ⟨true, .live⟩
def linkedReleased : Path := ⟨true, .released⟩
def unlinkedLive : Path := ⟨false, .live⟩
def unlinkedReleased : Path := ⟨false, .released⟩

/-- **The base flow hid both leaks**: the ledger's branch fixture and the
one-sided escape were `Safe` while a concrete path leaks the child. -/
theorem the_base_flow_hides_the_leak :
    baseModel linkedLive unlinkedReleased .release = .safe
      ∧ concreteLeak linkedLive unlinkedReleased .release = true
      ∧ baseModel linkedLive unlinkedLive .escape = .safe
      ∧ concreteLeak linkedLive unlinkedLive .escape = true := by
  decide

/-- The ledger's fixture reports the same leak as its straight-line form. -/
theorem the_branch_reports_the_straight_line_leak :
    shapeModel linkedLive unlinkedReleased .release = .«unsafe»
      ∧ shapeModel linkedLive linkedLive .release = .«unsafe» := by
  decide

/-- `SHAPE_VECTORS` of `tests/field_link_join.rs`: the gateway's verdict on
each pinned fixture. -/
def shapeVectors : List (String × Verdict) := [
  ("straight-line-release", .«unsafe»),
  ("branch-release", .«unsafe»),
  ("both-arms-member-released", .safe),
  ("both-arms-escape", .safe),
  ("one-sided-escape", .unknown),
  ("released-arm-escape", .unknown),
  ("mixed-linked-escape", .unknown)
]

/-- The model's verdict of each pinned shape: its two paths and the owner's
action. -/
def shapeVerdict : String → Verdict
  | "straight-line-release" => shapeModel linkedLive linkedLive .release
  | "branch-release" => shapeModel linkedLive unlinkedReleased .release
  | "both-arms-member-released" => shapeModel linkedReleased linkedReleased .release
  | "both-arms-escape" => shapeModel linkedLive linkedLive .escape
  | "one-sided-escape" => shapeModel linkedLive unlinkedLive .escape
  | "released-arm-escape" => shapeModel linkedLive unlinkedReleased .escape
  | "mixed-linked-escape" => shapeModel linkedLive linkedReleased .escape
  | _ => .unknown

theorem shape_vectors_hold :
    shapeVectors.all (fun (s, v) => shapeVerdict s == v) = true := by
  decide

end QCore.FieldLinkJoin
