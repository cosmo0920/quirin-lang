import QCore.SourceLoop
import QCore.WriteLend

/-! Loan windows over iterations (card `LOOPS-ON-COMPOSED-ROUTES`).

A loan's window on the byte-buffer and composed string routes is an interval of
**top-level statement indices**: it opens at the lend or the borrow and runs to
the last statement naming the loan. Before this card both routes refused a
`for` beside a loan, because the window was computed over top-level events only
and a `for` body contributed nothing to it — so a loan used only inside a loop
would have closed before the loop opened, and every owner write in the body
would have been admitted as "outside every live loan" (`STABILITY-REEVAL-3` §4,
`SLICE-PERMISSIONS-LOOP-WINDOW`, `STRING-CONCAT-LOOP-WINDOW`).

This module states the replacement — the **window function over iterations**
— and proves it sound relative to `LOOP-V1`'s unrolling.

**The unrolled program, tagged with origins.** A program is a list of
statements, each a route event or a loop over a body of the same. `tagged`
unrolls it exactly as `QCore.SourceLoop.unroll` does — a loop is its body
`count` times over, at every depth — and tags every unrolled event with the
**top-level index** it is attributed to. `the_unrolling_is_loop_v1s` proves the
untagged list is `LOOP-V1`'s own unrolling, so nothing below is about a second
unrolling.

**The window function.** `lastUse uses prog` is the largest origin of any
unrolled event `uses` holds of — the last top-level statement that names the
loan at any depth. It is what the Rust `last_uses` computes, and it is defined
over the tagged unrolling so that the soundness statement is about the program
the checker decides.

**Soundness relative to the unrolling.** `every_use_lies_inside_the_window`:
every use in the unrolled program has an origin at or before the window's end.
`origins_are_monotone`: unrolled positions are attributed to non-decreasing
top-level indices. Together,
`the_unrolled_window_lies_inside_the_iteration_window`: whatever lies between
the loan's opening and any of its uses **in the unrolled program** is attributed
inside the iteration window. The iteration window therefore holds every loan
live at every unrolled position the exact window would — an over-approximation
in the **refusing** direction, because the IR's owner-access decision refuses
no less under a longer live set (`a_longer_live_set_refuses_no_less`), and a
second lend is admitted no more (`a_longer_live_set_admits_no_more_lends`).

**Equal where there is no loop.** `the_window_is_exact_over_a_flat_program`:
on a loop-free program the tagged unrolling is the program itself with its own
indices, so the iteration window *is* the flat window the routes always
computed — no admitted flat program moves.

**Non-vacuous.** `a_top_level_only_window_misses_a_body_use`: the replaced
computation — over top-level statements only — closes before a loop whose body
is the loan's only use, so a use exists that lies outside it. That is the
admitting arm the card's Rust mutation re-exposes (`mutations.log`, M1).

What is **not** proved here, carried over from the family's boundary: that the
Rust lowering extracts these events faithfully from source tokens (a tested
Rust boundary); that the IR's per-access decision is applied at every unrolled
position (the Rust projection unrolls the body under the live loans, a tested
boundary); and anything about a loan *opened inside* a body, which is absent
from this model by construction because it is refused by name
(`LOOP-PER-ITERATION-LOAN`). -/

namespace QCore.LoopWindows

open QCore.StdBuffer
open QCore.SourceLoop

/-! ## 1. Statements, and the tagged unrolling -/

/-- A statement of a route that admits the counted loop: one of that route's
own events, or a loop whose body is a list of the same. Generic in the event
type, because the window is a statement about which events *name* a loan and
nothing else; §4 instantiates it at `QCore.StdBuffer.Event`. -/
inductive Stmt (α : Type) where
  | leaf (a : α)
  | loop (count : Nat) (body : List (Stmt α))

/-- `count` copies of `xs`, concatenated — `QCore.SourceLoop.repeatAppend`'s
shape, generic in the element type. -/
def copies {β : Type} (count : Nat) (xs : List β) : List β :=
  match count with
  | 0 => []
  | n + 1 => xs ++ copies n xs

mutual
  /-- The unrolled events of one statement, each tagged with `origin`: a leaf is
  itself, once; a loop is its body's tagged unrolling `count` times over, every
  copy attributed to the **same** origin, because the loop is one top-level
  statement however many times its body runs. -/
  def tagFrom {α : Type} (origin : Nat) : Stmt α → List (Nat × α)
    | .leaf a => [(origin, a)]
    | .loop count body => copies count (tagBody origin body)

  /-- The tagged unrolling of a body, all at one origin. -/
  def tagBody {α : Type} (origin : Nat) : List (Stmt α) → List (Nat × α)
    | [] => []
    | s :: rest => tagFrom origin s ++ tagBody origin rest
end

/-- The tagged unrolling of a program whose first statement has index `i`. -/
def taggedFrom {α : Type} (i : Nat) : List (Stmt α) → List (Nat × α)
  | [] => []
  | s :: rest => tagFrom i s ++ taggedFrom (i + 1) rest

/-- **The tagged unrolling** of a program: statement `i` at the top level
contributes its unrolled events, every one tagged `i`. -/
def tagged {α : Type} (prog : List (Stmt α)) : List (Nat × α) := taggedFrom 0 prog

/-! ## 2. The window function over iterations -/

/-- The larger of `acc` and `p`'s origin when `uses` holds of `p`'s event. -/
def extend {α : Type} (uses : α → Bool) (acc : Nat) (p : Nat × α) : Nat :=
  if uses p.2 then max acc p.1 else acc

/-- **The window function over iterations.** The last top-level index at which
`uses` holds of some unrolled event — a loan touched anywhere inside a loop's
body is touched at that loop's own index, at every iteration. `0` when no event
uses the loan at all; the Rust `lend_windows` then falls back to the loan's own
index (`unwrap_or(index)`), a window of one statement that contains no other
statement, and this module says nothing about that case because there is no
use for it to cover. -/
def lastUse {α : Type} (uses : α → Bool) (prog : List (Stmt α)) : Nat :=
  (tagged prog).foldl (extend uses) 0

theorem foldl_extend_ge {α : Type} (uses : α → Bool) (l : List (Nat × α)) (init : Nat) :
    init ≤ l.foldl (extend uses) init := by
  induction l generalizing init with
  | nil => exact Nat.le_refl init
  | cons p rest ih =>
      refine Nat.le_trans ?_ (ih (extend uses init p))
      unfold extend
      split
      · exact Nat.le_max_left init p.1
      · exact Nat.le_refl init

theorem foldl_extend_covers {α : Type} (uses : α → Bool) (l : List (Nat × α)) (init : Nat)
    (p : Nat × α) (mem : p ∈ l) (used : uses p.2 = true) :
    p.1 ≤ l.foldl (extend uses) init := by
  induction l generalizing init with
  | nil => exact absurd mem (List.not_mem_nil)
  | cons q rest ih =>
      rcases List.mem_cons.mp mem with here | there
      · subst here
        refine Nat.le_trans ?_ (foldl_extend_ge uses rest (extend uses init p))
        simp only [extend, used, ite_true]
        exact Nat.le_max_right init p.1
      · exact ih (extend uses init q) there

/-- **Soundness, first half: every use of the unrolled program lies inside the
window.** Whatever top-level statement an unrolled use of the loan is attributed
to — a flat statement, or a loop at whatever depth and whatever iteration — it
is at or before the window's end. -/
theorem every_use_lies_inside_the_window {α : Type} (uses : α → Bool)
    (prog : List (Stmt α)) (p : Nat × α) (mem : p ∈ tagged prog) (used : uses p.2 = true) :
    p.1 ≤ lastUse uses prog :=
  foldl_extend_covers uses (tagged prog) 0 p mem used

/-! ## 3. Origins are monotone, so the exact window sits inside the iteration one -/

mutual
  theorem tagFrom_origin {α : Type} (origin : Nat) (s : Stmt α) :
      ∀ p ∈ tagFrom origin s, p.1 = origin := by
    cases s with
    | leaf a =>
        intro p mem
        simp [tagFrom] at mem
        rw [mem]
    | loop count body =>
        intro p mem
        have inBody : ∀ q ∈ tagBody origin body, q.1 = origin := tagBody_origin origin body
        simp only [tagFrom] at mem
        induction count with
        | zero => simp [copies] at mem
        | succ n ih =>
            simp only [copies, List.mem_append] at mem
            rcases mem with here | there
            · exact inBody p here
            · exact ih there

  theorem tagBody_origin {α : Type} (origin : Nat) (body : List (Stmt α)) :
      ∀ p ∈ tagBody origin body, p.1 = origin := by
    cases body with
    | nil => intro p mem; simp [tagBody] at mem
    | cons s rest =>
        intro p mem
        simp only [tagBody, List.mem_append] at mem
        rcases mem with here | there
        · exact tagFrom_origin origin s p here
        · exact tagBody_origin origin rest p there
end

theorem taggedFrom_origin_ge {α : Type} (i : Nat) (prog : List (Stmt α)) :
    ∀ p ∈ taggedFrom i prog, i ≤ p.1 := by
  induction prog generalizing i with
  | nil => intro p mem; simp [taggedFrom] at mem
  | cons s rest ih =>
      intro p mem
      simp only [taggedFrom, List.mem_append] at mem
      rcases mem with here | there
      · rw [tagFrom_origin i s p here]
        exact Nat.le_refl i
      · exact Nat.le_trans (Nat.le_succ i) (ih (i + 1) p there)

theorem pairwise_of_one_origin {α : Type} (i : Nat) (l : List (Nat × α))
    (one : ∀ p ∈ l, p.1 = i) : l.Pairwise (fun p q => p.1 ≤ q.1) := by
  induction l with
  | nil => exact List.Pairwise.nil
  | cons p rest ih =>
      refine List.Pairwise.cons ?_ (ih (fun q mem => one q (List.mem_cons_of_mem p mem)))
      intro q mem
      rw [one p (List.mem_cons_self ..), one q (List.mem_cons_of_mem p mem)]
      exact Nat.le_refl i

/-- **Soundness, second half: unrolled positions are attributed to
non-decreasing top-level indices.** Unrolling never moves an event in front of
one that stood before it at the top level. -/
theorem origins_are_monotone {α : Type} (i : Nat) (prog : List (Stmt α)) :
    (taggedFrom i prog).Pairwise (fun p q => p.1 ≤ q.1) := by
  induction prog generalizing i with
  | nil => exact List.Pairwise.nil
  | cons s rest ih =>
      simp only [taggedFrom]
      refine List.pairwise_append.mpr ⟨?_, ih (i + 1), ?_⟩
      · exact pairwise_of_one_origin i (tagFrom i s) (tagFrom_origin i s)
      · intro p memP q memQ
        rw [tagFrom_origin i s p memP]
        exact Nat.le_trans (Nat.le_succ i) (taggedFrom_origin_ge (i + 1) rest q memQ)

/-- **The headline.** Split the unrolled program at the loan's opening and at
any one of its uses: everything between them — every unrolled position the
*exact* window contains — is attributed inside the iteration window
`[opening, lastUse]`. So a loan the exact window holds live at an unrolled
position is held live there by the iteration window too, which is the
direction invariant 1 permits. -/
theorem the_unrolled_window_lies_inside_the_iteration_window {α : Type} (uses : α → Bool)
    (prog : List (Stmt α)) (pre mid post : List (Nat × α)) (opening use : Nat × α)
    (split : tagged prog = pre ++ opening :: (mid ++ use :: post))
    (used : uses use.2 = true) :
    ∀ q ∈ mid, opening.1 ≤ q.1 ∧ q.1 ≤ lastUse uses prog := by
  intro q mem
  have sorted : (tagged prog).Pairwise (fun p q => p.1 ≤ q.1) := origins_are_monotone 0 prog
  rw [split] at sorted
  have tail := (List.pairwise_append.mp sorted).2.1
  have fromOpening := (List.pairwise_cons.mp tail).1
  have afterOpening := (List.pairwise_cons.mp tail).2
  have beforeUse := (List.pairwise_append.mp afterOpening).2.2
  constructor
  · exact fromOpening q (List.mem_append.mpr (Or.inl mem))
  · refine Nat.le_trans (beforeUse q mem use (List.mem_cons_self ..)) ?_
    refine every_use_lies_inside_the_window uses prog use ?_ used
    rw [split]
    exact List.mem_append.mpr (Or.inr (List.mem_cons_of_mem opening
      (List.mem_append.mpr (Or.inr (List.mem_cons_self ..)))))

/-! ## 4. The unrolling is `LOOP-V1`'s -/

mutual
  /-- The untagged unrolling, written independently of `tagged`. -/
  def unrollStmt {α : Type} : Stmt α → List α
    | .leaf a => [a]
    | .loop count body => copies count (unrollStmts body)

  def unrollStmts {α : Type} : List (Stmt α) → List α
    | [] => []
    | s :: rest => unrollStmt s ++ unrollStmts rest
end

theorem copies_map {α β : Type} (f : α → β) (count : Nat) (xs : List α) :
    (copies count xs).map f = copies count (xs.map f) := by
  induction count with
  | zero => rfl
  | succ n ih => simp [copies, List.map_append, ih]

mutual
  theorem tagFrom_untagged {α : Type} (origin : Nat) (s : Stmt α) :
      (tagFrom origin s).map Prod.snd = unrollStmt s := by
    cases s with
    | leaf a => rfl
    | loop count body =>
        simp only [tagFrom, unrollStmt, copies_map]
        rw [tagBody_untagged origin body]

  theorem tagBody_untagged {α : Type} (origin : Nat) (body : List (Stmt α)) :
      (tagBody origin body).map Prod.snd = unrollStmts body := by
    cases body with
    | nil => rfl
    | cons s rest =>
        simp only [tagBody, unrollStmts, List.map_append]
        rw [tagFrom_untagged origin s, tagBody_untagged origin rest]
end

theorem taggedFrom_untagged {α : Type} (i : Nat) (prog : List (Stmt α)) :
    (taggedFrom i prog).map Prod.snd = unrollStmts prog := by
  induction prog generalizing i with
  | nil => rfl
  | cons s rest ih =>
      simp only [taggedFrom, unrollStmts, List.map_append]
      rw [tagFrom_untagged i s, ih (i + 1)]

/-- Forgetting the tags gives the untagged unrolling: the window is computed
over exactly the events the checker decides. -/
theorem tagged_forgets_to_the_unrolling {α : Type} (prog : List (Stmt α)) :
    (tagged prog).map Prod.snd = unrollStmts prog :=
  taggedFrom_untagged 0 prog

mutual
  /-- A statement over the buffer route's events, as `LOOP-V1`'s own item. The
  index name is not part of the unrolling and is dropped. -/
  def toItem : Stmt Event → Item
    | .leaf e => .event e
    | .loop count body => .loop "" count (toItems body)

  def toItems : List (Stmt Event) → List Item
    | [] => []
    | s :: rest => toItem s :: toItems rest
end

theorem copies_is_repeatAppend (count : Nat) (xs : List Event) :
    copies count xs = repeatAppend count xs := by
  induction count with
  | zero => rfl
  | succ n ih => simp [copies, repeatAppend, ih]

mutual
  theorem toItem_unrolls (s : Stmt Event) : unroll (toItem s) = unrollStmt s := by
    cases s with
    | leaf e => rfl
    | loop count body =>
        simp only [toItem, unroll, unrollStmt, copies_is_repeatAppend]
        rw [toItems_unroll body]

  theorem toItems_unroll (prog : List (Stmt Event)) :
      unrollList (toItems prog) = unrollStmts prog := by
    cases prog with
    | nil => rfl
    | cons s rest =>
        simp only [toItems, unrollList, unrollStmts]
        rw [toItem_unrolls s, toItems_unroll rest]
end

/-- **The unrolling the window is stated over is `LOOP-V1`'s.** On the buffer
route's events, the untagged unrolling is `QCore.SourceLoop.unrollList` of the
same program read as that module's items — so `every_use_lies_inside_the_window`
and its consequences are statements about the flat list the inherited cycle-3
checker is handed, and not about a second unrolling that happens to agree. -/
theorem the_unrolling_is_loop_v1s (prog : List (Stmt Event)) :
    (tagged prog).map Prod.snd = unrollList (toItems prog) := by
  rw [tagged_forgets_to_the_unrolling, toItems_unroll]

/-! ## 5. Equal where there is no loop -/

/-- A flat program, tagged with its own indices — the window the routes have
always computed, written independently of `tagged`. -/
def flatTagged {α : Type} (i : Nat) : List α → List (Nat × α)
  | [] => []
  | a :: rest => (i, a) :: flatTagged (i + 1) rest

/-- On a loop-free program the tagged unrolling **is** the program with its own
indices, so `lastUse` is the flat last-use scan the routes ran before this card,
and no admitted flat program's window moves. -/
theorem the_window_is_exact_over_a_flat_program {α : Type} (i : Nat) (events : List α) :
    taggedFrom i (events.map Stmt.leaf) = flatTagged i events := by
  induction events generalizing i with
  | nil => rfl
  | cons a rest ih => simp [taggedFrom, tagFrom, flatTagged, ih]

/-! ## 6. The replaced computation, and why it admitted -/

/-- The window computed over **top-level statements only** — what `lend_windows`
and `last_view_uses` computed before this card: a loop contributes nothing. -/
def shallowTagged {α : Type} (i : Nat) : List (Stmt α) → List (Nat × α)
  | [] => []
  | .leaf a :: rest => (i, a) :: shallowTagged (i + 1) rest
  | .loop _ _ :: rest => shallowTagged (i + 1) rest

def lastUseShallow {α : Type} (uses : α → Bool) (prog : List (Stmt α)) : Nat :=
  (shallowTagged 0 prog).foldl (extend uses) 0

/-- `STABILITY-REEVAL-3` §4's scenario: a loan opened at statement 0 and used
only inside the loop at statement 1. Event `1` is the loan's use. -/
def bodyOnlyUse : List (Stmt Nat) := [.leaf 0, .loop 2 [.leaf 1]]

/-- **Non-vacuous.** Over the replaced computation the window closes at
statement 0, before the loop opens; over the iteration window it runs through
the loop at statement 1; and the use inside the loop — present in the unrolled
program at origin 1 — lies outside the replaced window and inside the new one.
This is the admitting arm the Rust mutation re-exposes. -/
theorem a_top_level_only_window_misses_a_body_use :
    lastUseShallow (· == 1) bodyOnlyUse = 0
      ∧ lastUse (· == 1) bodyOnlyUse = 1
      ∧ (1, 1) ∈ tagged bodyOnlyUse
      ∧ ¬ (1 ≤ lastUseShallow (· == 1) bodyOnlyUse) := by
  decide

/-! ## 7. A longer live set refuses no less

The iteration window over-approximates the exact one, so at some unrolled
positions it holds a loan live that the exact window would not. The IR's
decisions are monotone in the live set in the refusing direction, which is what
makes an over-approximation of liveness safe. -/

/-- An owner access the IR refuses under some live loans it still refuses when
one more loan is live. -/
theorem a_longer_live_set_refuses_no_less (loans : List QCore.WriteLend.Loan)
    (extra : QCore.WriteLend.Loan) (resource : Nat) (write : Bool)
    (a : QCore.SlicePermissions.Range)
    (refused : QCore.WriteLend.ownerBlocked loans resource write a = true) :
    QCore.WriteLend.ownerBlocked (extra :: loans) resource write a = true := by
  simp only [QCore.WriteLend.ownerBlocked, List.any_cons] at refused ⊢
  rw [refused]
  exact Bool.or_true _

/-- A second lend admitted under a longer live set was admitted under the
shorter one: lengthening the live set admits no lend it would have refused. -/
theorem a_longer_live_set_admits_no_more_lends (loans : List QCore.WriteLend.Loan)
    (extra : QCore.WriteLend.Loan) (resource : Nat) (r : QCore.SlicePermissions.Range)
    (admitted : QCore.WriteLend.admitsLend (extra :: loans) resource r = true) :
    QCore.WriteLend.admitsLend loans resource r = true := by
  simp only [QCore.WriteLend.admitsLend, List.all_cons, Bool.and_eq_true] at admitted
  exact admitted.2

/-- A release blocked under some live loans is blocked under one more. -/
theorem a_longer_live_set_blocks_the_drop_no_less (loans : List QCore.WriteLend.Loan)
    (extra : QCore.WriteLend.Loan) (resource : Nat)
    (blocked : QCore.WriteLend.hasLoan loans resource = true) :
    QCore.WriteLend.hasLoan (extra :: loans) resource = true := by
  simp only [QCore.WriteLend.hasLoan, List.any_cons] at blocked ⊢
  rw [blocked]
  exact Bool.or_true _

/-! ## 8. What this card admits, as one list

A widening must change a theorem rather than slip past one. The shapes a loop
body may hold beside a loan are exactly these; a lend or a borrow inside a body
(`LOOP-PER-ITERATION-LOAN`) and an acquisition or a release inside one
(`LOOP-WINDOW-BODY-CUSTODY`) are outside the list and refused by name. -/

inductive AdmittedInBody where
  /-- A store, a load or an extent read — through the owner or through a loan. -/
  | access
  /-- A literal binding, a line, a nested loop. -/
  | plain
  deriving DecidableEq, Repr

def admittedInBody : List AdmittedInBody := [.access, .plain]

theorem the_admitted_body_shapes_are_exactly_these (s : AdmittedInBody) :
    s ∈ admittedInBody := by
  cases s <;> simp [admittedInBody]

end QCore.LoopWindows
