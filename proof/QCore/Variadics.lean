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

import QCore.VariadicNested

/-!
# Variadics: the `va_list` protocol and the trailing data actuals

Card **CG26-VARIADICS** (gateway rule 11w), gap entries K08 and K09.

## The `va_list` protocol (K09)

C11 7.16 gives a variable argument list a protocol: `va_start` (or `va_copy`
into it) initialises it, `va_arg` reads it, handing it whole to a callee makes
it indeterminate until `va_end` (7.16p3), and `va_end` must run before the
function returns (7.16.1.3p2). `va_list_protocol.rs` reads one function's
lists and their events from Clang's AST into a control-flow graph, computes
the per-list abstract states before every node as a fixed point over sets of
`ListState`, and admits the function's `va_*` statements as custody-neutral
only when every node's events **step** and the exit's states are **accepted**.

This module proves that check sound. A graph is a list of nodes, each with the
operations it performs on one list and its successors; an abstract assignment
`a` gives every node a set of states. `certify` is the check the Rust
`decide` makes (entry contains the initial state, every node with a
non-empty set steps and its result is contained in each successor's set, the
exit's set is accepted). `Reach g o entry n x` says some path from `entry`
arrives at `n` with the concrete list in state `x`. Then:

* `certified_invariant` — every reachable concrete state is in `a`;
* `certified_no_violation` — no reachable node's operations fail;
* `certified_exit` — every concrete state reaching the exit is accepted: a
  local list is uninitialised or ended there, so a missing `va_end` is never
  admitted;
* the transition table `step` is the pointwise lifting of the concrete
  `stepOne` (`step_sound`, `step_refuses`), and `stepVectors` are the Rust
  `STEP_VECTORS`, row by row (`step_vectors_hold`, bound by a shared-vector
  test).

The lists of one function are decided one at a time: an event on one list
never reads another list's state (`va_copy (D, S)` is a read of `S` and a
start of `D`), so the per-list projection is exact.

## The trailing actuals (K08)

`QCore.VariadicNested` models the declared data tail (T1-T6). Card CG26 adds
one admission (W46-FCNTL-SETFD-ADDRESS-TAIL): the address of a plain scalar
local that may hold no tracked identity. It denotes no identity (`none` in the
modeled argument vector), so reading every tail position as `borrowed` is the
identity on the abstract state (`borrowed_tail_is_the_identity`); what the
callee may write through it is the scalar's value only, which the lowering
forgets (`ForgetScalar`), and no custody fact is a scalar's value.

## What this is not

Clang's AST extraction (node kinds, declaration ids, `variadic`) and the
graph's construction from it are trust boundary 28 premises; the graph-reading
rules (which forms refuse) are pinned by Rust tests. Nothing here is about
which arguments a `va_arg` reads or their types: format safety (K10) is not
claimed.
-/

namespace QCore.Variadics

/-! ## The protocol of one list -/

inductive ListState where
  | uninitialised
  | active
  | passed
  | ended
  deriving DecidableEq, Repr

/-- A list is the function's own local, or a parameter it received. -/
inductive Origin where
  | own
  | param
  deriving DecidableEq, Repr

inductive Operation where
  | start
  | arg
  | pass
  | copyFrom
  | end_
  deriving DecidableEq, Repr

/-- The concrete protocol: `none` is a violation. -/
def stepOne : Origin → ListState → Operation → Option ListState
  | .param, _, .start => none
  | .param, _, .end_ => none
  | _, .uninitialised, .start => some .active
  | _, .ended, .start => some .active
  | _, _, .start => none
  | _, .active, .arg => some .active
  | _, _, .arg => none
  | _, .active, .copyFrom => some .active
  | _, _, .copyFrom => none
  | _, .active, .pass => some .passed
  | _, _, .pass => none
  | _, .active, .end_ => some .ended
  | _, .passed, .end_ => some .ended
  | _, _, .end_ => none

def initialOne : Origin → ListState
  | .own => .uninitialised
  | .param => .active

/-- A local list must be uninitialised or ended at an exit; a parameter list
    owes nothing (its `va_end` is the caller's). -/
def exitOkOne : Origin → ListState → Bool
  | .param, _ => true
  | .own, .uninitialised => true
  | .own, .ended => true
  | .own, _ => false

/-- A set of states, one flag per state (the Rust mask's bits 0-3). -/
structure States where
  u : Bool
  a : Bool
  p : Bool
  e : Bool
  deriving DecidableEq, Repr

def States.empty : States := ⟨false, false, false, false⟩

def States.mem (s : States) : ListState → Bool
  | .uninitialised => s.u
  | .active => s.a
  | .passed => s.p
  | .ended => s.e

def States.single : ListState → States
  | .uninitialised => ⟨true, false, false, false⟩
  | .active => ⟨false, true, false, false⟩
  | .passed => ⟨false, false, true, false⟩
  | .ended => ⟨false, false, false, true⟩

def States.isEmpty (s : States) : Bool := !s.u && !s.a && !s.p && !s.e

/-- `s ⊆ t`. -/
def States.sub (s t : States) : Bool :=
  (!s.u || t.u) && (!s.a || t.a) && (!s.p || t.p) && (!s.e || t.e)

/-- Every member of `s` is allowed. -/
def States.within (s : States) (u a p e : Bool) : Bool :=
  (!s.u || u) && (!s.a || a) && (!s.p || p) && (!s.e || e)

/-- The Rust `step`: the abstract transition over a set. -/
def step : Origin → States → Operation → Option States
  | .param, _, .start => none
  | .param, _, .end_ => none
  | _, s, .start =>
      if s.within true false false true then some (States.single .active) else none
  | _, s, .arg =>
      if s.within false true false false then some (States.single .active) else none
  | _, s, .copyFrom =>
      if s.within false true false false then some (States.single .active) else none
  | _, s, .pass =>
      if s.within false true false false then some (States.single .passed) else none
  | _, s, .end_ =>
      if s.within false true true false then some (States.single .ended) else none

def exitOk (o : Origin) (s : States) : Bool :=
  match o with
  | .param => true
  | .own => s.within true false false true

def initial (o : Origin) : States := States.single (initialOne o)

/-! ### The table is the pointwise lifting of the protocol -/

theorem step_sound (o : Origin) (s t : States) (op : Operation) (x : ListState)
    (h : step o s op = some t) (hx : s.mem x = true) :
    ∃ y, stepOne o x op = some y ∧ t.mem y = true := by
  cases o <;> cases op <;> cases x <;> cases s with
  | mk u a p e =>
    cases u <;> cases a <;> cases p <;> cases e <;>
      simp_all [step, stepOne, States.within, States.mem, States.single] <;>
      (subst_vars; rfl)

theorem step_refuses (o : Origin) (s : States) (op : Operation) (x : ListState)
    (hx : s.mem x = true) (bad : stepOne o x op = none) : step o s op = none := by
  cases o <;> cases op <;> cases x <;> cases s with
  | mk u a p e =>
    cases u <;> cases a <;> cases p <;> cases e <;>
      simp_all [step, stepOne, States.within, States.mem]

theorem exit_sound (o : Origin) (s : States) (x : ListState)
    (h : exitOk o s = true) (hx : s.mem x = true) : exitOkOne o x = true := by
  cases o <;> cases x <;> cases s with
  | mk u a p e =>
    cases u <;> cases a <;> cases p <;> cases e <;>
      simp_all [exitOk, exitOkOne, States.within, States.mem]

theorem sub_mem (s t : States) (x : ListState) (h : s.sub t = true) (hx : s.mem x = true) :
    t.mem x = true := by
  cases x <;> cases s with
  | mk u a p e =>
    cases t with
    | mk u' a' p' e' =>
      cases u <;> cases a <;> cases p <;> cases e <;>
        simp_all [States.sub, States.mem]

theorem mem_nonempty (s : States) (x : ListState) (hx : s.mem x = true) :
    s.isEmpty = false := by
  cases x <;> cases s with
  | mk u a p e => simp_all [States.isEmpty, States.mem]

/-! ### The mask encoding and the shared vectors -/

def ofMask (n : Nat) : States := ⟨n.testBit 0, n.testBit 1, n.testBit 2, n.testBit 3⟩

def toMask (s : States) : Nat :=
  (if s.u then 1 else 0) + (if s.a then 2 else 0) + (if s.p then 4 else 0) + (if s.e then 8 else 0)

/-- The Rust `STEP_VECTORS`: origin, incoming mask, operation, outgoing mask
    or `none` for a violation. -/
def stepVectors : List (Origin × Nat × Operation × Option Nat) := [
  (.own, 1, .start, some 2),
  (.own, 8, .start, some 2),
  (.own, 9, .start, some 2),
  (.own, 2, .start, none),
  (.own, 3, .start, none),
  (.own, 2, .arg, some 2),
  (.own, 3, .arg, none),
  (.own, 4, .arg, none),
  (.own, 8, .arg, none),
  (.own, 2, .pass, some 4),
  (.own, 4, .pass, none),
  (.own, 2, .copyFrom, some 2),
  (.own, 10, .copyFrom, none),
  (.own, 2, .end_, some 8),
  (.own, 4, .end_, some 8),
  (.own, 6, .end_, some 8),
  (.own, 1, .end_, none),
  (.own, 8, .end_, none),
  (.own, 10, .end_, none),
  (.param, 2, .arg, some 2),
  (.param, 2, .pass, some 4),
  (.param, 4, .arg, none),
  (.param, 2, .copyFrom, some 2),
  (.param, 2, .start, none),
  (.param, 2, .end_, none)
]

theorem step_vectors_hold :
    stepVectors.all (fun (o, i, op, out) => (step o (ofMask i) op).map toMask == out) = true := by
  decide

/-! ## Graphs, paths and the certificate -/

/-- One node: the operations it performs on the list, then its successors. -/
structure Node where
  ops : List Operation
  succ : List Nat
  deriving DecidableEq, Repr

def nodeAt (g : List Node) (n : Nat) : Node := g.getD n ⟨[], []⟩

def stAt (a : List States) (n : Nat) : States := a.getD n States.empty

def runOne (o : Origin) : List Operation → ListState → Option ListState
  | [], x => some x
  | op :: ops, x => (stepOne o x op).bind (runOne o ops)

def runAbs (o : Origin) : List Operation → States → Option States
  | [], s => some s
  | op :: ops, s => (step o s op).bind (runAbs o ops)

/-- Some path from `entry` reaches `n` with the list in state `x`, before
    `n`'s own operations. -/
inductive Reach (g : List Node) (o : Origin) (entry : Nat) : Nat → ListState → Prop
  | init : Reach g o entry entry (initialOne o)
  | next {n m : Nat} {x y : ListState} :
      Reach g o entry n x → runOne o (nodeAt g n).ops x = some y →
      m ∈ (nodeAt g n).succ → Reach g o entry m y

/-- One node's obligation: unreachable (empty), or its operations step and the
    result is contained in every successor's set. -/
def nodeOk (o : Origin) (g : List Node) (a : List States) (n : Nat) : Bool :=
  (stAt a n).isEmpty ||
    match runAbs o (nodeAt g n).ops (stAt a n) with
    | none => false
    | some t => (nodeAt g n).succ.all (fun m => t.sub (stAt a m))

/-- The check `decide` makes, over one list. -/
def certify (o : Origin) (g : List Node) (a : List States) (entry exit : Nat) : Bool :=
  (initial o).sub (stAt a entry) &&
    (List.range g.length).all (nodeOk o g a) &&
    exitOk o (stAt a exit)

theorem run_sound (o : Origin) :
    ∀ (ops : List Operation) (s t : States) (x : ListState),
      runAbs o ops s = some t → s.mem x = true →
      ∃ y, runOne o ops x = some y ∧ t.mem y = true
  | [], s, t, x, h, hx => by
    simp [runAbs] at h
    exact ⟨x, rfl, h ▸ hx⟩
  | op :: ops, s, t, x, h, hx => by
    simp only [runAbs, Option.bind_eq_some_iff] at h
    obtain ⟨s', hs', hrest⟩ := h
    obtain ⟨y, hy, hy'⟩ := step_sound o s s' op x hs' hx
    obtain ⟨z, hz, hz'⟩ := run_sound o ops s' t y hrest hy'
    exact ⟨z, by simp [runOne, hy, hz], hz'⟩

theorem node_beyond (g : List Node) (n : Nat) (h : ¬ n < g.length) : nodeAt g n = ⟨[], []⟩ := by
  simp [nodeAt, List.getD_eq_getElem?_getD, List.getElem?_eq_none (Nat.le_of_not_lt h)]

theorem node_ok_of (o : Origin) (g : List Node) (a : List States) (entry exit : Nat)
    (ok : certify o g a entry exit = true) (n : Nat) (hn : n < g.length) :
    nodeOk o g a n = true := by
  simp only [certify, Bool.and_eq_true, List.all_eq_true, List.mem_range] at ok
  exact ok.1.2 n hn

/-- **The invariant**: every concrete state some path reaches is in `a`. -/
theorem certified_invariant (o : Origin) (g : List Node) (a : List States) (entry exit : Nat)
    (ok : certify o g a entry exit = true) {n : Nat} {x : ListState}
    (r : Reach g o entry n x) : (stAt a n).mem x = true := by
  induction r with
  | init =>
    have h : (initial o).sub (stAt a entry) = true := by
      simp only [certify, Bool.and_eq_true] at ok
      exact ok.1.1
    exact sub_mem _ _ _ h (by cases o <;> rfl)
  | @next n m x y _ hrun hm ih =>
    by_cases hn : n < g.length
    · have hnode := node_ok_of o g a entry exit ok n hn
      have hne := mem_nonempty _ _ ih
      simp only [nodeOk, hne, Bool.false_or] at hnode
      cases habs : runAbs o (nodeAt g n).ops (stAt a n) with
      | none => simp [habs] at hnode
      | some t =>
        simp only [habs, List.all_eq_true] at hnode
        obtain ⟨y', hy', ht⟩ := run_sound o _ _ _ _ habs ih
        rw [hrun] at hy'
        cases hy'
        exact sub_mem _ _ _ (hnode m hm) ht
    · rw [node_beyond g n hn] at hm
      simp at hm

/-- **No violation**: no reachable node's operations fail the protocol. -/
theorem certified_no_violation (o : Origin) (g : List Node) (a : List States) (entry exit : Nat)
    (ok : certify o g a entry exit = true) {n : Nat} {x : ListState}
    (r : Reach g o entry n x) : ∃ y, runOne o (nodeAt g n).ops x = some y := by
  have hx := certified_invariant o g a entry exit ok r
  by_cases hn : n < g.length
  · have hnode := node_ok_of o g a entry exit ok n hn
    have hne := mem_nonempty _ _ hx
    simp only [nodeOk, hne, Bool.false_or] at hnode
    cases habs : runAbs o (nodeAt g n).ops (stAt a n) with
    | none => simp [habs] at hnode
    | some t =>
      obtain ⟨y, hy, _⟩ := run_sound o _ _ _ _ habs hx
      exact ⟨y, hy⟩
  · rw [node_beyond g n hn]
    exact ⟨x, rfl⟩

/-- **The exit**: every concrete state reaching the exit is accepted, so a
    local list reaching it not ended (a missing `va_end`, or a `va_copy`
    without its own) is never certified. -/
theorem certified_exit (o : Origin) (g : List Node) (a : List States) (entry exit : Nat)
    (ok : certify o g a entry exit = true) {x : ListState}
    (r : Reach g o entry exit x) : exitOkOne o x = true := by
  have hx := certified_invariant o g a entry exit ok r
  have h : exitOk o (stAt a exit) = true := by
    simp only [certify, Bool.and_eq_true] at ok
    exact ok.2
  exact exit_sound o _ x h hx

/-! ### What is refused, concretely -/

/-- `va_arg` after `va_end` is a violation. -/
theorem an_arg_after_end_is_refused (o : Origin) : stepOne o .ended .arg = none := by
  cases o <;> rfl

/-- A second pass of a passed list is a violation (7.16p3). -/
theorem a_second_pass_is_refused (o : Origin) : stepOne o .passed .pass = none := by
  cases o <;> rfl

/-- A local list still active, or passed, at an exit is a missing `va_end`. -/
theorem an_active_local_at_exit_is_refused : exitOkOne .own .active = false := rfl
theorem a_passed_local_at_exit_is_refused : exitOkOne .own .passed = false := rfl

/-- A parameter list is never started or ended here. -/
theorem a_parameter_is_never_started (x : ListState) : stepOne .param x .start = none := by
  cases x <;> rfl
theorem a_parameter_is_never_ended (x : ListState) : stepOne .param x .end_ = none := by
  cases x <;> rfl

/-! ### Non-vacuity: two graphs, one certified, one not -/

/-- entry 1 → start 2 → end 3 → exit 0. -/
def straight : List Node :=
  [⟨[], []⟩, ⟨[], [2]⟩, ⟨[.start], [3]⟩, ⟨[.end_], [0]⟩]

def straightStates : List States :=
  [States.single .ended, States.single .uninitialised, States.single .uninitialised,
   States.single .active]

theorem the_straight_protocol_is_certified :
    certify .own straight straightStates 1 0 = true := by decide

/-- The same graph with an edge from `start` straight to the exit (a `return`
    between `va_start` and `va_end`): the exit may see the list active, and
    no assignment certifies it. -/
def skipping : List Node :=
  [⟨[], []⟩, ⟨[], [2]⟩, ⟨[.start], [3, 0]⟩, ⟨[.end_], [0]⟩]

theorem the_skipping_path_reaches_the_exit_active :
    Reach skipping .own 1 0 .active :=
  Reach.next (n := 2) (Reach.next (n := 1) Reach.init rfl (by decide)) rfl (by decide)

theorem the_skipping_protocol_is_never_certified (a : List States) :
    certify .own skipping a 1 0 = false := by
  cases h : certify .own skipping a 1 0 with
  | false => rfl
  | true =>
    have := certified_exit .own skipping a 1 0 h the_skipping_path_reaches_the_exit_active
    simp [exitOkOne] at this

/-! ## The trailing actuals -/

open QCore.VariadicNested

/-- Reading every tail position as `borrowed`, with the modeled argument
    vector extended by what the tail actuals denote, is the identity on the
    abstract state: the extension adds only `borrowed` effects. -/
theorem borrowed_tail_is_the_identity (c : Contract) (args tail : List (Option Nat))
    (o : Outcome) (arity : args.length = c.modes.length) :
    applyCall { c with modes := c.modes ++ List.replicate tail.length ParamMode.borrowed }
        (args ++ tail) o
      = applyCall c args o := by
  simp only [applyCall]
  congr 1
  rw [List.zip_append (by simp [arity]), List.foldl_append]
  have hb : ∀ (l : List (ParamMode × Option Nat)) (acc : Outcome),
      (∀ p ∈ l, p.1 = ParamMode.borrowed) →
      List.foldl (fun acc (p : ParamMode × Option Nat) => paramEffect p.1 p.2 acc) acc l = acc := by
    intro l
    induction l with
    | nil => intro acc _; rfl
    | cons p rest ih =>
      intro acc h
      simp only [List.foldl_cons]
      have hp : p.1 = ParamMode.borrowed := h p (by simp)
      have : paramEffect p.1 p.2 acc = acc := by rw [hp]; rfl
      rw [this]
      exact ih acc (fun q hq => h q (by simp [hq]))
  apply hb
  intro p hp
  have := List.of_mem_zip hp
  exact List.eq_of_mem_replicate this.1

/-- The address of a scalar local carries no identity: the modeled argument
    is `none`, so even at a custody-taking position it would demand and change
    nothing (it is admitted only in a declared tail, where the position is
    `borrowed` anyway). -/
theorem a_scalar_address_demands_nothing (m : ParamMode) (o : Outcome)
    (notOut : m ≠ ParamMode.out) : paramEffect m none o = o := by
  cases m <;> simp_all [paramEffect]

end QCore.Variadics
