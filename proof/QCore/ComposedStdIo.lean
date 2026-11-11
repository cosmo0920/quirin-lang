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

import QCore.ExternCall

/-! `source.std-io-extern.v1` — the checked composition of
`source.extern-call.v1` and `source.std-io.v1` (card `STD-IO-COMPOSE`).

This is a model of `crates/qcore-core/src/composed_std_io.rs`, and — like that
module — it models the **join**, not a third checker. A composed program is two
event lists side by side plus the program order that interleaves them; neither
projection is constructed here, so nothing here can weaken a custody rule, an
adapter rule or a rendering rule. What is proved is what the join owns:

* **Composing added no custody path.** Only an extern event touches the ledger,
  and the ledger a whole composed run produces is *exactly* the one its extern
  projection produces alone.
* **Nothing is dropped at the join.** The composed effect row is the union of
  the two projections' rows, admitting the union admits each projection, and the
  order restricted to each projection recovers that projection's event list —
  nothing lost, nothing duplicated by the interleaving.
* **A rendering reads a value, never an identity**, and it reads only a value an
  *earlier* composed step bound and whose number some extern event determined.

Three things are deliberately **not** claimed. The order between standard output
and standard error is not modeled — `source.std-io.v1` already stated that
boundary, and C gives no order between two separately buffered streams; the two
streams are counted separately below and nothing relates them. The digits a
rendering publishes are not modeled either: a line carries the *number* that
crossed from the extern model into it, and the digits are what the Rust tests
compare against a native run. And the extern route's own premises are premises
still — composing neither discharges nor adds one. -/

namespace QCore.ComposedStdIo

open QCore.ExternCall (Ledger removeAll)

/-! ## The two event lists, the order, and the two namespaces -/

/-- A name in the **value** namespace. `Nat`, exactly as
`QCore.ExternCall.stepBind` models scalar names, and kept in a different field
from every custody identity for the same reason that model gives: nothing may
resolve one name in both namespaces. -/
abbrev Name := Nat

/-- A value of the extern model's value namespace. -/
abbrev Value := Nat

/-- A live custody identity. Members of `QCore.ExternCall.Ledger`. -/
abbrev Ident := Nat

/-- The digits are not modeled; the number is. See the module header. -/
abbrev Text := Int

/-- The rendering, as this composition sees it: which number reaches the line.
Which digits it is spelled with is `source.std-io.v1`'s `dec`/`hex`, checked on
that route and compared natively by the Rust tests. -/
def render (number : Int) : Text := number

/-- One call's binding of its scalar result: the value it names, and the number
the declaration states it succeeds with — if it states one. `outcome = none` is
the **undetermined** case, a bound call result no model computes a number for. -/
structure ScalarBinding where
  value : Value
  outcome : Option Int
  deriving DecidableEq, Repr

/-- `let NAME = INTEGER;`. It calls nothing, so it moves no custody at all. -/
structure LiteralBinding where
  value : Value
  number : Int
  deriving DecidableEq, Repr

/-- One extern call, in the two dimensions the join reads: what it binds in the
value namespace, and what it moves in the ledger. `acquired` is the identity an
admitted acquisition establishes and `departing` the identities the declaration
removes, exactly as `QCore.ExternCall.stepCall` uses them. -/
structure Call where
  binding : Option ScalarBinding
  acquired : Option Ident
  departing : List Ident
  deriving DecidableEq, Repr

inductive ExtEvent where
  | call (c : Call)
  | bind (b : LiteralBinding)
  | ret
  deriving DecidableEq, Repr

/-- **The rendering discipline, as a type.** The two bare constructors cannot
denote a custody identity: an operand is a literal written at the site, or a
name resolved in the *value* namespace. The third, card `STD-FMT-BORROW`'s
`borrow NAME`, resolves in the **custody** namespace instead and renders the
identity itself — never the object, its address or its extent, none of which
this model carries. Every custody theorem below is downstream of this, exactly
as `QCore.StdIo.Operand`'s are on its own route. -/
inductive Operand where
  | lit (number : Int)
  | scalar (name : Name)
  | borrowed (name : Name)
  deriving DecidableEq, Repr

inductive IoEvent where
  | print (operand : Operand)
  | log (operand : Operand)
  | flush
  | ret
  deriving DecidableEq, Repr

/-- One step of the composed program order. The two index constructors name an
event of a projection; `ret` is the one `return;` that is the last event of
**both** — one source statement, two models, one step. -/
inductive Step where
  | ext (index : Nat)
  | io (index : Nat)
  | ret
  deriving DecidableEq, Repr

/-- The premises the extern route records. They are listed rather than
summarised so that "composing adds no premise" is a statement about this list
and not about a count. -/
inductive Premise where
  | declaredContract (callee : Name)
  | noExtentTransport
  | linkageSuppliedByBuilder
  | uncheckedLinkage (callee : Name)
  deriving DecidableEq, Repr

/-- A complete composed module. -/
structure Program where
  externs : List ExtEvent
  io : List IoEvent
  order : List Step
  /-- The extern model's value behind each scalar name a rendering may read.
  Built from that model's own namespace, so a name meaning a custody identity is
  absent by construction. -/
  operandValues : List (Name × Value)
  /-- The custody identity behind each name a **borrowed** rendering reads (card
  `STD-FMT-BORROW`). A separate field from `operandValues`, so that "a borrowed
  render reads no value and a scalar render reads no identity" is a statement
  about which field a clause consults. -/
  operandIdentities : List (Name × Ident)
  /-- The premises the extern projection derived. -/
  externPremises : List Premise
  deriving DecidableEq, Repr

/-! ## Custody: only an extern event touches the ledger -/

/-- Stepping one extern event. This is the only clause of the composed step
function that is not the identity on the ledger. -/
def stepExt (live : Ledger) : ExtEvent → Ledger
  | .call c =>
      match c.acquired with
      | some x => x :: removeAll live c.departing
      | none => removeAll live c.departing
  | .bind _ => live
  | .ret => live

/-- Stepping one standard-IO event. One clause, and it is the identity: the
ledger is not something this route can write. -/
def stepIo (live : Ledger) (_event : IoEvent) : Ledger := live

/-- Stepping one composed step. An index naming no event steps nothing — the
interleaving check refuses such an order, and reading it as a custody move would
be exactly the optimistic fallback invariant 1 forbids. -/
def stepAt (p : Program) (live : Ledger) : Step → Ledger
  | .ext i => match p.externs[i]? with
      | some e => stepExt live e
      | none => live
  | .io i => match p.io[i]? with
      | some e => stepIo live e
      | none => live
  | .ret => live

/-- Running the whole composed program. -/
def run (p : Program) (live : Ledger) : Ledger := p.order.foldl (stepAt p) live

/-- Running the extern projection alone, with no composition anywhere near it. -/
def runExtern (externs : List ExtEvent) (live : Ledger) : Ledger := externs.foldl stepExt live

/-- **Stepping a rendering leaves the ledger byte-identical.** Whatever the
standard-IO event at that index is, and whether or not there is one. -/
theorem a_rendering_leaves_the_ledger_unchanged (p : Program) (live : Ledger) (i : Nat) :
    stepAt p live (.io i) = live := by
  cases h : p.io[i]? <;> simp [stepAt, stepIo, h]

/-- And so does the shared `return`. -/
theorem the_shared_return_leaves_the_ledger_unchanged (p : Program) (live : Ledger) :
    stepAt p live .ret = live := rfl

/-! ## The interleaving, and the projections recovered from it -/

def extIndices : List Step → List Nat
  | [] => []
  | .ext i :: rest => i :: extIndices rest
  | .io _ :: rest => extIndices rest
  | .ret :: rest => extIndices rest

def ioIndices : List Step → List Nat
  | [] => []
  | .ext _ :: rest => ioIndices rest
  | .io i :: rest => i :: ioIndices rest
  | .ret :: rest => ioIndices rest

def retCount : List Step → Nat
  | [] => 0
  | .ext _ :: rest => retCount rest
  | .io _ :: rest => retCount rest
  | .ret :: rest => retCount rest + 1

/-- The order restricted to the extern projection. An index naming no event
contributes nothing; the interleaving check refuses such an order. -/
def externSteps (p : Program) : List Step → List ExtEvent
  | [] => []
  | .ext i :: rest =>
      match p.externs[i]? with
      | some e => e :: externSteps p rest
      | none => externSteps p rest
  | .io _ :: rest => externSteps p rest
  | .ret :: rest => externSteps p rest

/-- The order restricted to the standard-IO projection. -/
def ioSteps (p : Program) : List Step → List IoEvent
  | [] => []
  | .ext _ :: rest => ioSteps p rest
  | .io i :: rest =>
      match p.io[i]? with
      | some e => e :: ioSteps p rest
      | none => ioSteps p rest
  | .ret :: rest => ioSteps p rest

/-- **The order is an interleaving.** Each projection ends in its own `return`,
the composed order ends in the one `return` they share and carries exactly one,
and each projection's events appear in the order's restriction in their own
order, each exactly once — the trailing `return` excepted, since it is the
shared step. A composed program whose order lost, duplicated or reordered an
event is refused rather than checked. -/
def admitsOrder (p : Program) : Bool :=
  p.externs.getLast? == some ExtEvent.ret &&
    p.io.getLast? == some IoEvent.ret &&
    p.order.getLast? == some Step.ret &&
    retCount p.order == 1 &&
    extIndices p.order == List.range (p.externs.length - 1) &&
    ioIndices p.order == List.range (p.io.length - 1)

private theorem filterMap_getElem?_range {α : Type} (l : List α) :
    ∀ n : Nat, (List.range n).filterMap (fun i => l[i]?) = l.take n := by
  intro n
  induction n with
  | zero => simp
  | succ n ih =>
    cases h : l[n]? <;>
      simp [List.range_succ, List.filterMap_append, ih, List.take_add_one, h]

private theorem extern_steps_are_indexed (p : Program) (order : List Step) :
    externSteps p order = (extIndices order).filterMap (fun i => p.externs[i]?) := by
  induction order with
  | nil => rfl
  | cons s rest ih =>
    cases s with
    | ext i => cases h : p.externs[i]? <;> simp [externSteps, extIndices, h, ih]
    | io i => simp [externSteps, extIndices, ih]
    | ret => simp [externSteps, extIndices, ih]

private theorem io_steps_are_indexed (p : Program) (order : List Step) :
    ioSteps p order = (ioIndices order).filterMap (fun i => p.io[i]?) := by
  induction order with
  | nil => rfl
  | cons s rest ih =>
    cases s with
    | ext i => simp [ioSteps, ioIndices, ih]
    | io i => cases h : p.io[i]? <;> simp [ioSteps, ioIndices, h, ih]
    | ret => simp [ioSteps, ioIndices, ih]

private theorem dropLast_append_getLast? {α : Type} (l : List α) (x : α)
    (h : l.getLast? = some x) : l.dropLast ++ [x] = l := by
  cases hl : l with
  | nil => rw [hl] at h; simp at h
  | cons a rest =>
    have hne : (a :: rest) ≠ [] := by simp
    have hlast : (a :: rest).getLast? = some ((a :: rest).getLast hne) :=
      List.getLast?_eq_some_getLast hne
    rw [hl, hlast] at h
    have hx : (a :: rest).getLast hne = x := by simpa using h
    calc (a :: rest).dropLast ++ [x] = (a :: rest).dropLast ++ [(a :: rest).getLast hne] := by
          rw [hx]
      _ = a :: rest := List.dropLast_concat_getLast hne

/-- **Nothing is lost or duplicated by the interleaving.** The composed order
restricted to the extern steps is exactly the extern event list, and likewise
for the standard-IO steps — the shared trailing `return` in each case being the
composed `ret` step. -/
theorem the_projections_are_recovered_from_the_order (p : Program) (h : admitsOrder p = true) :
    externSteps p p.order ++ [ExtEvent.ret] = p.externs
      ∧ ioSteps p p.order ++ [IoEvent.ret] = p.io := by
  simp only [admitsOrder, Bool.and_eq_true, beq_iff_eq] at h
  obtain ⟨⟨⟨⟨⟨hext, hio⟩, _⟩, _⟩, hextIdx⟩, hioIdx⟩ := h
  constructor
  · rw [extern_steps_are_indexed, hextIdx, filterMap_getElem?_range,
      ← List.dropLast_eq_take]
    exact dropLast_append_getLast? _ _ hext
  · rw [io_steps_are_indexed, hioIdx, filterMap_getElem?_range, ← List.dropLast_eq_take]
    exact dropLast_append_getLast? _ _ hio

/-! ## Composing added no custody path -/

/-- The composed run steps only its extern steps. No interleaving hypothesis:
a standard-IO step and the shared `return` are the identity on the ledger
whatever else is true of the order. -/
theorem the_order_runs_only_its_extern_steps (p : Program) (order : List Step) (live : Ledger) :
    order.foldl (stepAt p) live = (externSteps p order).foldl stepExt live := by
  induction order generalizing live with
  | nil => rfl
  | cons s rest ih =>
    cases s with
    | ext i => cases h : p.externs[i]? <;> simp [externSteps, stepAt, h, ih]
    | io i => cases h : p.io[i]? <;> simp [externSteps, stepAt, stepIo, h, ih]
    | ret => simp [externSteps, stepAt, ih]

/-- **The card's central claim.** The ledger a composed run produces is exactly
the ledger the extern projection produces on its own: composing the two routes
introduced no custody path, no acquisition, no release and no leak. -/
theorem the_composed_ledger_is_the_extern_projections_ledger (p : Program)
    (h : admitsOrder p = true) (live : Ledger) :
    run p live = runExtern p.externs live := by
  have hrec := (the_projections_are_recovered_from_the_order p h).1
  rw [run, the_order_runs_only_its_extern_steps, runExtern, ← hrec, List.foldl_append]
  simp [stepExt]

private theorem mem_removeAll (x : Nat) :
    ∀ (cs : List Nat) (l : Ledger), x ∈ removeAll l cs → x ∈ l := by
  intro cs
  induction cs with
  | nil => intro l h; simpa [removeAll] using h
  | cons _ rest ih =>
    intro l hm
    exact (List.mem_filter.mp (ih _ hm)).1

private theorem live_after_extern_events (evs : List ExtEvent) (live : Ledger) (x : Ident)
    (hx : x ∈ evs.foldl stepExt live) :
    x ∈ live ∨ ∃ c, ExtEvent.call c ∈ evs ∧ c.acquired = some x := by
  induction evs generalizing live with
  | nil => exact Or.inl (by simpa using hx)
  | cons e rest ih =>
    rcases ih (stepExt live e) (by simpa using hx) with h | ⟨c, hc, hac⟩
    · cases e with
      | call c =>
        cases hacq : c.acquired with
        | none =>
          rw [stepExt, hacq] at h
          exact Or.inl (mem_removeAll x c.departing live h)
        | some y =>
          rw [stepExt, hacq] at h
          rcases List.mem_cons.mp h with rfl | h
          · exact Or.inr ⟨c, List.mem_cons_self, hacq⟩
          · exact Or.inl (mem_removeAll x c.departing live h)
      | bind _ => exact Or.inl (by simpa [stepExt] using h)
      | ret => exact Or.inl (by simpa [stepExt] using h)
    · exact Or.inr ⟨c, List.mem_cons_of_mem _ hc, hac⟩

private theorem extern_steps_are_extern_events (p : Program) (order : List Step) (e : ExtEvent)
    (h : e ∈ externSteps p order) : e ∈ p.externs := by
  induction order with
  | nil => simp [externSteps] at h
  | cons s rest ih =>
    cases s with
    | ext i =>
      cases hg : p.externs[i]? with
      | none => exact ih (by simpa [externSteps, hg] using h)
      | some e' =>
        rcases List.mem_cons.mp (by simpa [externSteps, hg] using h) with rfl | hm
        · exact List.mem_of_getElem? hg
        · exact ih hm
    | io i => exact ih (by simpa [externSteps] using h)
    | ret => exact ih (by simpa [externSteps] using h)

/-- **The value namespace and the ledger never exchange a name.** Every identity
a composed run holds was either live before it or was established by an extern
*call's* declared acquisition. No rendering, no operand, no bound value and no
literal can put a name into the ledger — there is no step that does it. -/
theorem no_step_moves_a_name_between_the_namespaces (p : Program) (live : Ledger) (x : Ident)
    (hx : x ∈ run p live) :
    x ∈ live ∨ ∃ c, ExtEvent.call c ∈ p.externs ∧ c.acquired = some x := by
  rw [run, the_order_runs_only_its_extern_steps] at hx
  rcases live_after_extern_events _ live x hx with h | ⟨c, hc, hac⟩
  · exact Or.inl h
  · exact Or.inr ⟨c, extern_steps_are_extern_events p p.order _ hc, hac⟩

/-- **The operand forms are exactly these three**, exhibited as one list so that
a future widening changes this theorem rather than slipping past it. The first
two carry no `Ident` at all; the third carries a *name* the custody namespace
resolves, and even that never becomes a value — see
`a_borrowed_render_reads_no_value` below. -/
theorem an_operand_is_a_literal_a_value_name_or_a_borrow (o : Operand) :
    (∃ number, o = .lit number) ∨ (∃ name, o = .scalar name)
      ∨ ∃ name, o = .borrowed name := by
  cases o with
  | lit number => exact Or.inl ⟨number, rfl⟩
  | scalar name => exact Or.inr (Or.inl ⟨name, rfl⟩)
  | borrowed name => exact Or.inr (Or.inr ⟨name, rfl⟩)

/-! ## Determinacy: a rendering reads a number some model computed -/

def lookupValue : List (Name × Value) → Name → Option Value
  | [], _ => none
  | (n, v) :: rest, name => if n = name then some v else lookupValue rest name

/-- The custody namespace's own lookup (card `STD-FMT-BORROW`). A separate
function over a separate field, which is what makes "no rendering resolves one
name in both namespaces" a fact about the definitions rather than a comment. -/
def lookupIdent : List (Name × Ident) → Name → Option Ident
  | [], _ => none
  | (n, x) :: rest, name => if n = name then some x else lookupIdent rest name

/-- The ledger the emitted program is in when the composed step at `position`
runs: the extern projection's own ledger, folded over the steps before it from
the entry's empty starting state.

This is the liveness a borrowed render requires, and it is `stepAt`'s — the
composition derives no second notion of liveness, exactly as the Rust join reads
the extern certificate's published `live_after` rather than recomputing one. -/
def liveBefore (p : Program) (position : Nat) : Ledger :=
  (p.order.take position).foldl (stepAt p) []

/-- Whether an extern event binds this value at all, in the value namespace.
Kept apart from `determinedBy` below, because they are different rules: a call
result can be bound without being determined, and that program refuses. -/
def bindsValue : ExtEvent → Value → Bool
  | .bind b, v => b.value == v
  | .call c, v => match c.binding with
      | some s => s.value == v
      | none => false
  | .ret, _ => false

/-- The number an extern event **determines** for a value, if it determines one:
a literal binding determines its literal, a call binding determines the number
its declaration states it succeeds with, and a call binding whose declaration
states none determines nothing. -/
def determinedBy : ExtEvent → Value → Option Int
  | .bind b, v => if b.value = v then some b.number else none
  | .call c, v => match c.binding with
      | some s => if s.value = v then s.outcome else none
      | none => none
  | .ret, _ => none

def numberOf : List ExtEvent → Value → Option Int
  | [], _ => none
  | e :: rest, v => match determinedBy e v with
      | some n => some n
      | none => numberOf rest v

/-- **The determined values are exactly these**, and no others: a value is
determined when, and only when, some extern event of this program determines a
number for it. Widening what a program may render therefore changes this
theorem rather than slipping past it. -/
theorem the_determined_values_are_exactly_these (evs : List ExtEvent) (v : Value) :
    (numberOf evs v).isSome = true ↔ ∃ e ∈ evs, (determinedBy e v).isSome = true := by
  induction evs with
  | nil => simp [numberOf]
  | cons e rest ih =>
    cases h : determinedBy e v with
    | some n => simp [numberOf, h]
    | none => simp [numberOf, h, ih]

/-- A bound call result whose declaration states no outcome is exactly the
undetermined case: it binds a name and determines no number. -/
theorem a_call_without_a_declared_outcome_determines_nothing (c : Call) (v : Value)
    (h : c.binding = some ⟨v, none⟩) :
    determinedBy (.call c) v = none ∧ bindsValue (.call c) v = true := by
  constructor
  · simp [determinedBy, h]
  · simp [bindsValue, h]

/-- A literal binding determines its own literal, which is what lets a rendered
value originate in the module that renders it. -/
theorem a_literal_binding_determines_its_literal (b : LiteralBinding) :
    determinedBy (.bind b) b.value = some b.number := by
  simp [determinedBy]

/-! ## Ordering: a rendering reads a binding that is already behind it -/

/-- Whether this composed step binds the value. Only an extern step can. -/
def bindsHere (p : Program) (v : Value) : Step → Bool
  | .ext i => match p.externs[i]? with
      | some e => bindsValue e v
      | none => false
  | .io _ => false
  | .ret => false

def boundAtFrom (p : Program) (v : Value) (position : Nat) : List Step → Option Nat
  | [] => none
  | s :: rest => if bindsHere p v s then some position else boundAtFrom p v (position + 1) rest

/-- The composed position of the step that binds this value, if one does. -/
def boundAt (p : Program) (v : Value) : Option Nat := boundAtFrom p v 0 p.order

/-- What a rendering publishes, or `none` when it has no admitted step. Three
clauses refuse, and each is one of the join's rules: a name the value namespace
does not resolve, a value no composed step binds *earlier* than this rendering,
and a value whose number no extern event determines. -/
def renderOperand (p : Program) (position : Nat) : Operand → Option Text
  | .lit number => some (render number)
  | .scalar name =>
      match lookupValue p.operandValues name with
      | none => none
      | some v =>
          match boundAt p v with
          | none => none
          | some b => if b < position then (numberOf p.externs v).map render else none
  -- Card `STD-FMT-BORROW`. Two clauses refuse, and they are the join's own
  -- custody rules: a name the *custody* namespace does not resolve, and an
  -- identity the extern projection's ledger does not hold live where the
  -- rendering stands. Nothing else is consulted, and in particular no value.
  | .borrowed name =>
      match lookupIdent p.operandIdentities name with
      | none => none
      | some x => if x ∈ liveBefore p position then some (render (Int.ofNat x)) else none

/-- **A rendering reads only a value bound by an earlier composed step.** Every
admitted rendering of a scalar exhibits the value it resolved to, the earlier
step that bound it, and the number an extern event determined for it. -/
theorem a_rendering_reads_only_an_earlier_binding (p : Program) (position : Nat) (name : Name)
    (text : Text) (h : renderOperand p position (.scalar name) = some text) :
    ∃ v b n, lookupValue p.operandValues name = some v
      ∧ boundAt p v = some b ∧ b < position
      ∧ numberOf p.externs v = some n ∧ text = render n := by
  cases hv : lookupValue p.operandValues name with
  | none => simp [renderOperand, hv] at h
  | some v =>
    cases hb : boundAt p v with
    | none => simp [renderOperand, hv, hb] at h
    | some b =>
      by_cases hlt : b < position
      · cases hn : numberOf p.externs v with
        | none => simp [renderOperand, hv, hb, hlt, hn] at h
        | some n =>
          refine ⟨v, b, n, rfl, hb, hlt, hn, ?_⟩
          simp only [renderOperand, hv, hb, ite_eq_left hlt, hn, Option.map_some,
            Option.some.injEq] at h
          exact h.symm
      · simp [renderOperand, hv, hb, hlt] at h

/-- The reverse shape has no admitted step: a rendering placed before the step
that binds the value it names refuses. -/
theorem a_rendering_before_its_binding_has_no_admitted_step (p : Program) (position : Nat)
    (name : Name) (v : Value) (b : Nat)
    (hv : lookupValue p.operandValues name = some v) (hb : boundAt p v = some b)
    (hlt : ¬ b < position) : renderOperand p position (.scalar name) = none := by
  simp [renderOperand, hv, hb, hlt]

/-! ## The borrowed render (card `STD-FMT-BORROW`)

The custody argument, as theorems rather than as a comment. Three things have to
hold and each is one below: the resource must be live where the line is written;
the rendering must leave every obligation exactly as it found it; and the two
namespaces must stay two. -/

/-- **A borrowed render reads only a live identity.** Every admitted one
exhibits the identity it resolved to and the ledger membership that admitted it.
-/
theorem a_borrowed_render_reads_only_a_live_identity (p : Program) (position : Nat) (name : Name)
    (text : Text) (h : renderOperand p position (.borrowed name) = some text) :
    ∃ x, lookupIdent p.operandIdentities name = some x
      ∧ x ∈ liveBefore p position ∧ text = render (Int.ofNat x) := by
  cases hx : lookupIdent p.operandIdentities name with
  | none => simp [renderOperand, hx] at h
  | some x =>
    by_cases hlive : x ∈ liveBefore p position
    · refine ⟨x, rfl, hlive, ?_⟩
      simp only [renderOperand, hx, ite_eq_left hlive, Option.some.injEq] at h
      exact h.symm
    · simp [renderOperand, hx, hlive] at h

/-- **A borrowed render leaves the ledger unchanged**, so every obligation the
entry owed before it, it still owes after it. A specialisation of the general
clause, stated in its own right because it is clause (b) of the card's custody
argument. -/
theorem a_borrowed_render_leaves_the_ledger_unchanged (p : Program) (live : Ledger) (i : Nat) :
    stepAt p live (.io i) = live := a_rendering_leaves_the_ledger_unchanged p live i

/-- **A program of nothing but renderings moves no custody at all.** The
strongest form of "a rendering is not a publication" this model can state: the
identities reachable after any number of renderings are exactly the ones that
were reachable before, so no rendering created a second path to a resource and
none discharged one. -/
theorem a_program_of_renderings_leaves_the_ledger_unchanged (p : Program) (live : Ledger)
    (indices : List Nat) : (indices.map Step.io).foldl (stepAt p) live = live := by
  induction indices generalizing live with
  | nil => rfl
  | cons i rest ih =>
    simp only [List.map_cons, List.foldl_cons, a_rendering_leaves_the_ledger_unchanged]
    exact ih live

/-- The same, as a statement about a whole run. -/
theorem a_program_of_renderings_runs_to_its_starting_ledger (p : Program) (live : Ledger)
    (indices : List Nat) (h : p.order = indices.map Step.io) : run p live = live := by
  rw [run, h]
  exact a_program_of_renderings_leaves_the_ledger_unchanged p live indices

/-- **A borrowed render reads no value.** Whatever the value namespace holds,
the borrowed clause's answer is the same: the wall is which field a clause
consults, not a rule a checker applies. -/
theorem a_borrowed_render_reads_no_value (p : Program) (values : List (Name × Value))
    (position : Nat) (name : Name) :
    renderOperand { p with operandValues := values } position (.borrowed name)
      = renderOperand p position (.borrowed name) := rfl

/-- Changing the custody namespace changes no binding position: `boundAtFrom`
reads the extern events and the order, and neither is that field. -/
private theorem boundAtFrom_ignores_the_custody_namespace (p : Program)
    (identities : List (Name × Ident)) (v : Value) (position : Nat) (steps : List Step) :
    boundAtFrom { p with operandIdentities := identities } v position steps
      = boundAtFrom p v position steps := by
  induction steps generalizing position with
  | nil => rfl
  | cons s rest ih =>
    cases s <;> simp only [boundAtFrom, bindsHere, ih] <;> rfl

/-- And the wall in the other direction: a scalar render reads no identity, so
supplying identities can never make a bare operand denote a resource. This is
what keeps the standard-IO route's own resource-free rule for a bare operand
unweakened by this card. -/
theorem a_scalar_render_reads_no_identity (p : Program) (identities : List (Name × Ident))
    (position : Nat) (name : Name) :
    renderOperand { p with operandIdentities := identities } position (.scalar name)
      = renderOperand p position (.scalar name) := by
  simp only [renderOperand, boundAt, boundAtFrom_ignores_the_custody_namespace]

/-- A borrowed render binds nothing in the value namespace either: no `ValueId`
is introduced for it, which is the model's form of "the rendered identity is
data at the render site only". -/
theorem a_borrowed_render_binds_no_value (p : Program) (v : Value) (i : Nat) :
    bindsHere p v (.io i) = false := by
  simp [bindsHere]

/-! ## The two streams, counted separately -/

inductive Stream where
  | out
  | err
  deriving DecidableEq, Repr

/-- One published line. `text = none` is a line naming **no number**: the
extern route's synthesized call trace for a call whose declaration states no
outcome, whose other fields this model does not describe — only its stream. -/
structure Line where
  stream : Stream
  text : Option Text
  deriving DecidableEq, Repr

def linesOn (s : Stream) (ls : List Line) : Nat := (ls.filter (fun l => l.stream == s)).length

/-- The number an extern call's own trace line names, if any (card
`EXTERN-SCALAR-OUTCOME`, trace half).

A call whose declaration states an outcome names that number; a call that binds
nothing, or binds without a declared outcome, names none. This is deliberately
the *same* `outcome` field `determinedBy` reads, so the trace line and a
rendering of the same value cannot name two different numbers — proved below.

Why naming the declared number is an observation rather than a claim is a
property of the Rust backend, not of this model: the emitted program compares
the real result against the declaration and exits before the write, so a run
that publishes the line has already established the equality. What this model
owns is the weaker and still load-bearing half — that the number published is
the declared one, and that an undeclared binding publishes none. -/
def externLineText (c : Call) : Option Text :=
  match c.binding with
  | some s => s.outcome.map render
  | none => none

/-- The lines one composed step publishes, or `none` when it has no admitted
step. -/
def stepLines (p : Program) (position : Nat) : Step → Option (List Line)
  | .ext i => match p.externs[i]? with
      | some (.call c) => some [⟨Stream.out, externLineText c⟩]
      | some _ => some []
      | none => some []
  | .io i => match p.io[i]? with
      | some (.print o) => (renderOperand p position o).map (fun t => [⟨Stream.out, some t⟩])
      | some (.log o) => (renderOperand p position o).map (fun t => [⟨Stream.err, some t⟩])
      | some _ => some []
      | none => some []
  | .ret => some []

def linesFrom (p : Program) (position : Nat) : List Step → Option (List Line)
  | [] => some []
  | s :: rest =>
      match stepLines p position s, linesFrom p (position + 1) rest with
      | some a, some b => some (a ++ b)
      | _, _ => none

/-- Every line a composed program publishes, in composed order across both
streams. `none` is a refusal of the whole module. -/
def lines (p : Program) : Option (List Line) := linesFrom p 0 p.order

/-- A step with no admitted step refuses the program that contains it. -/
theorem a_refused_step_refuses_the_program (p : Program) (position : Nat) (s : Step)
    (rest : List Step) (h : stepLines p position s = none) :
    linesFrom p position (s :: rest) = none := by
  cases hr : linesFrom p (position + 1) rest <;> simp [linesFrom, h, hr]

/-- **A determined rendering publishes exactly one line, carrying exactly that
number.** -/
theorem a_determined_rendering_publishes_exactly_one_line (p : Program) (position i : Nat)
    (name : Name) (v : Value) (b : Nat) (n : Int)
    (hio : p.io[i]? = some (.print (.scalar name)))
    (hv : lookupValue p.operandValues name = some v)
    (hb : boundAt p v = some b) (hlt : b < position)
    (hn : numberOf p.externs v = some n) :
    stepLines p position (.io i) = some [⟨Stream.out, some (render n)⟩] := by
  simp [stepLines, hio, renderOperand, hv, hb, hlt, hn]

/-- **An operand whose number the program does not determine has no admitted
step.** Rendering it would publish a line no model computed. -/
theorem an_undetermined_operand_has_no_admitted_step (p : Program) (position i : Nat)
    (name : Name) (v : Value)
    (hio : p.io[i]? = some (.print (.scalar name)))
    (hv : lookupValue p.operandValues name = some v)
    (hn : numberOf p.externs v = none) :
    stepLines p position (.io i) = none := by
  cases hb : boundAt p v with
  | none => simp [stepLines, hio, renderOperand, hv, hb]
  | some b =>
    by_cases hlt : b < position <;>
      simp [stepLines, hio, renderOperand, hv, hb, hlt, hn]

/-- A name the value namespace does not resolve refuses too, rather than falling
back on anything. -/
theorem an_unresolved_name_has_no_admitted_step (p : Program) (position i : Nat) (name : Name)
    (hio : p.io[i]? = some (.print (.scalar name)))
    (hv : lookupValue p.operandValues name = none) :
    stepLines p position (.io i) = none := by
  simp [stepLines, hio, renderOperand, hv]

/-- `print` never writes the logger's stream. -/
theorem print_never_writes_stderr (p : Program) (position i : Nat) (o : Operand)
    (ls : List Line) (hio : p.io[i]? = some (.print o))
    (h : stepLines p position (.io i) = some ls) : linesOn Stream.err ls = 0 := by
  cases hr : renderOperand p position o with
  | none => simp [stepLines, hio, hr] at h
  | some t =>
    have hls : ls = [⟨Stream.out, some t⟩] := by simpa [stepLines, hio, hr] using h.symm
    subst hls
    rfl

/-- `log` never writes standard output. -/
theorem log_never_writes_stdout (p : Program) (position i : Nat) (o : Operand)
    (ls : List Line) (hio : p.io[i]? = some (.log o))
    (h : stepLines p position (.io i) = some ls) : linesOn Stream.out ls = 0 := by
  cases hr : renderOperand p position o with
  | none => simp [stepLines, hio, hr] at h
  | some t =>
    have hls : ls = [⟨Stream.err, some t⟩] := by simpa [stepLines, hio, hr] using h.symm
    subst hls
    rfl

/-- **The extern trace shares standard output**, and only standard output. Its
relative order with this module's `print` lines on that one stream is the
composed order; the order between the two *streams* is not modeled here and is
not claimed anywhere. -/
theorem the_extern_trace_shares_standard_output (p : Program) (position i : Nat) (c : Call)
    (h : p.externs[i]? = some (.call c)) :
    ∃ ls, stepLines p position (.ext i) = some ls
      ∧ linesOn Stream.out ls = 1 ∧ linesOn Stream.err ls = 0 := by
  refine ⟨[⟨Stream.out, externLineText c⟩], ?_, rfl, rfl⟩
  simp [stepLines, h]

/-! ## The trace half: what an extern call's own line names -/

/-- **A call whose declaration states an outcome names exactly that number on
its trace line** (card `EXTERN-SCALAR-OUTCOME`). One line, on standard output,
carrying the declared number and nothing else. -/
theorem a_declared_outcome_is_named_on_the_calls_trace_line (p : Program) (position i : Nat)
    (c : Call) (v : Value) (n : Int)
    (h : p.externs[i]? = some (.call c)) (hb : c.binding = some ⟨v, some n⟩) :
    stepLines p position (.ext i) = some [⟨Stream.out, some (render n)⟩] := by
  simp [stepLines, h, externLineText, hb]

/-- **A call with no declared outcome names no number**, whether it binds a
value or binds nothing at all. This is the boundary the trace half did not
cross: nothing determines a number for such a call, so its line publishes none —
the Lean statement of the byte-identical undeclared observation the Rust tests
pin. -/
theorem an_undeclared_outcome_names_no_number (p : Program) (position i : Nat) (c : Call)
    (h : p.externs[i]? = some (.call c)) (hnone : externLineText c = none) :
    stepLines p position (.ext i) = some [⟨Stream.out, none⟩] := by
  simp [stepLines, h, hnone]

/-- The two undeclared shapes, spelled out: a call binding nothing, and a call
binding a value its declaration states no outcome for. -/
theorem the_undeclared_shapes_name_no_number (c : Call) (v : Value) :
    (c.binding = none → externLineText c = none)
      ∧ (c.binding = some ⟨v, none⟩ → externLineText c = none) := by
  constructor
  · intro h; simp [externLineText, h]
  · intro h; simp [externLineText, h]

/-- **The trace line and a rendering cannot name two different numbers.** The
number an extern call publishes on its own line is the number that call
*determines* for the value it binds — the same relation `renderOperand` reads —
so a composed program whose `print` renders that value publishes the same
number twice rather than two numbers. -/
theorem the_trace_line_names_the_number_the_call_determines (c : Call) (v : Value) (n : Int)
    (hb : ∃ o, c.binding = some ⟨v, o⟩) :
    externLineText c = some (render n) ↔ determinedBy (.call c) v = some n := by
  obtain ⟨o, hb⟩ := hb
  cases o with
  | none => simp [externLineText, determinedBy, hb]
  | some m =>
    simp only [externLineText, determinedBy, hb, Option.map_some]
    constructor
    · intro h; simp [render] at h; simp [h]
    · intro h; simp at h; simp [render, h]

/-- A barrier publishes no line, on either stream. -/
theorem a_barrier_publishes_no_line (p : Program) (position i : Nat)
    (h : p.io[i]? = some IoEvent.flush) :
    stepLines p position (.io i) = some [] := by
  simp [stepLines, h]

/-! ## The borrowed render, at a step (card `STD-FMT-BORROW`)

The three rules of the borrowed render, stated where a step publishes lines:
what it publishes when the identity is live, and the two ways it refuses. They
are here rather than beside the definitions above because they are about
`stepLines`, which the stream section defines. -/

/-- **An identity the ledger does not hold live has no admitted step.** This is
the released, consumed, stored, reallocated and never-acquired cases at once:
each of them is exactly "not a member of the ledger here". -/
theorem a_borrowed_render_of_a_dead_identity_has_no_admitted_step (p : Program)
    (position i : Nat) (name : Name) (x : Ident)
    (hio : p.io[i]? = some (.print (.borrowed name)))
    (hx : lookupIdent p.operandIdentities name = some x)
    (hlive : x ∉ liveBefore p position) : stepLines p position (.io i) = none := by
  simp [stepLines, hio, renderOperand, hx, hlive]

/-- A name the custody namespace does not resolve refuses too, rather than
falling back on the value namespace or on anything else. -/
theorem an_unresolved_borrow_has_no_admitted_step (p : Program) (position i : Nat) (name : Name)
    (hio : p.io[i]? = some (.print (.borrowed name)))
    (hx : lookupIdent p.operandIdentities name = none) :
    stepLines p position (.io i) = none := by
  simp [stepLines, hio, renderOperand, hx]

/-- **A live borrowed render publishes exactly one line, carrying exactly the
identity.** Not the address, not an extent, not a byte of the object: the model
has no other datum about a resource, and this theorem is where that is pinned.
-/
theorem a_borrowed_render_publishes_exactly_one_line (p : Program) (position i : Nat)
    (name : Name) (x : Ident)
    (hio : p.io[i]? = some (.print (.borrowed name)))
    (hx : lookupIdent p.operandIdentities name = some x)
    (hlive : x ∈ liveBefore p position) :
    stepLines p position (.io i) = some [⟨Stream.out, some (render (Int.ofNat x))⟩] := by
  simp [stepLines, hio, renderOperand, hx, hlive]

/-! ## Effects: the union, and what admitting it admits -/

inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

def externEventEffects : ExtEvent → List Effect
  | .call c => if c.acquired.isSome then [Effect.io, Effect.alloc] else [Effect.io]
  | .bind _ => []
  | .ret => []

def ioEventEffects : IoEvent → List Effect
  | .print _ => [Effect.io]
  | .log _ => [Effect.io]
  | .flush => [Effect.io]
  | .ret => []

/-- A row in canonical form: the fixed vocabulary, filtered by membership. Two
rows with the same members are therefore the same list, which is what makes the
union below a set union rather than a concatenation. -/
def row (es : List Effect) : List Effect :=
  [Effect.io, Effect.alloc, Effect.mut].filter (fun e => es.contains e)

def externEffects (evs : List ExtEvent) : List Effect := row (evs.flatMap externEventEffects)

def ioEffects (evs : List IoEvent) : List Effect := row (evs.flatMap ioEventEffects)

def union (a b : List Effect) : List Effect := row (a ++ b)

/-- The composed derived row. Neither projection sees the other's effects, so
this is the one place a composed row exists at all. -/
def composedEffects (p : Program) : List Effect :=
  union (externEffects p.externs) (ioEffects p.io)

theorem mem_row (es : List Effect) (e : Effect) : e ∈ row es ↔ e ∈ es := by
  cases e <;> simp [row, List.mem_filter]

/-- **The composed effect row is the union of the two projections' rows.** -/
theorem the_composed_effect_row_is_the_union (p : Program) :
    composedEffects p = union (externEffects p.externs) (ioEffects p.io) := rfl

/-- Nothing is dropped at the join, and nothing is invented there either: the
composed row's members are exactly the two projections' members together. -/
theorem no_projection_effect_is_dropped (p : Program) (e : Effect) :
    e ∈ composedEffects p ↔ (e ∈ externEffects p.externs ∨ e ∈ ioEffects p.io) := by
  simp [composedEffects, union, mem_row, List.mem_append]

/-- The caller's declared row must admit the derived one. -/
def admits (declared derived : List Effect) : Bool := derived.all (fun e => declared.contains e)

/-- **Admitting the union admits each projection.** A composed program the
declared row admits is one whose declared row admits *each* projection's row —
so no assumption either route derived was dropped at the join. -/
theorem admitting_the_union_admits_each_projection (p : Program) (declared : List Effect)
    (h : admits declared (composedEffects p) = true) :
    admits declared (externEffects p.externs) = true
      ∧ admits declared (ioEffects p.io) = true := by
  simp only [admits, List.all_eq_true] at h ⊢
  constructor
  · intro e he
    exact h e ((no_projection_effect_is_dropped p e).mpr (Or.inl he))
  · intro e he
    exact h e ((no_projection_effect_is_dropped p e).mpr (Or.inr he))

/-! ## Premises: composing adds none -/

/-- The premises the standard-IO projection derives: none. A rendering reads a
value; there is nothing about a C implementation for it to assume. -/
def ioPremises (_evs : List IoEvent) : List Premise := []

def composedPremises (p : Program) : List Premise := p.externPremises ++ ioPremises p.io

theorem a_rendering_carries_no_premise (evs : List IoEvent) : ioPremises evs = [] := rfl

/-- **Composing adds no premise.** The composed premise list is exactly the
extern projection's: every premise a composed module carries is one that
projection derived, unchanged. -/
theorem composing_adds_no_premise (p : Program) : composedPremises p = p.externPremises := by
  simp [composedPremises, ioPremises]

/-! ## The whole join, and the fixture it admits -/

/-- A composed module is admitted when the order interleaves, every rendering
has an admitted step, and the declared row admits the composed union. -/
def admitted (p : Program) (declared : List Effect) : Bool :=
  admitsOrder p && (lines p).isSome && admits declared (composedEffects p)

/-- The card's payoff fixture: a literal binding, a call with a declared
outcome, a `log` of its result, and the return both models share. -/
def payoff : Program where
  externs := [.bind ⟨0, 7⟩, .call ⟨some ⟨1, some 42⟩, none, []⟩, .ret]
  io := [.log (.scalar 5), .ret]
  order := [.ext 0, .ext 1, .io 0, .ret]
  operandValues := [(5, 1)]
  operandIdentities := []
  externPremises := [.declaredContract 9]

/-- **Non-vacuity.** The fixture is admitted, publishes the extern trace on
standard output and the declared outcome on standard error, and ends with an
empty ledger — the composition held no custody at any point.

Re-pinned by card `EXTERN-SCALAR-OUTCOME` (trace half): the standard-output
line was `⟨Stream.out, none⟩` and is now `⟨Stream.out, some (render 42)⟩`,
because the call this fixture makes declares its outcome. The two lines this
fixture publishes now carry the same number, on their two streams, which is the
whole content of the trace half at this level. -/
theorem the_payoff_fixture_is_admitted :
    admitted payoff [Effect.io] = true
      ∧ lines payoff = some [⟨Stream.out, some (render 42)⟩, ⟨Stream.err, some (render 42)⟩]
      ∧ run payoff [] = []
      ∧ composedEffects payoff = [Effect.io] := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- And its premise list is its extern projection's, verbatim. -/
theorem the_payoff_fixture_carries_only_its_extern_premises :
    composedPremises payoff = [Premise.declaredContract 9] := by decide

/-- The same fixture with the rendering moved before the call that binds it is
refused: the interleaving is still an interleaving, and the ordering rule is
what refuses. -/
def readBeforeBinding : Program := { payoff with order := [.ext 0, .io 0, .ext 1, .ret] }

theorem a_rendering_before_its_binding_is_refused :
    admitsOrder readBeforeBinding = true
      ∧ lines readBeforeBinding = none
      ∧ admitted readBeforeBinding [Effect.io] = false := by
  refine ⟨by decide, by decide, by decide⟩

/-- The same fixture whose call declares no outcome is refused: the name is
still bound, and the determinacy rule is what refuses. -/
def undeterminedOutcome : Program :=
  { payoff with externs := [.bind ⟨0, 7⟩, .call ⟨some ⟨1, none⟩, none, []⟩, .ret] }

theorem an_undetermined_outcome_is_refused :
    boundAt undeterminedOutcome 1 = some 1
      ∧ numberOf undeterminedOutcome.externs 1 = none
      ∧ lines undeterminedOutcome = none
      ∧ admitted undeterminedOutcome [Effect.io] = false := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- The same undeclared call in a module that does **not** render its value: the
rendering names the literal instead, so the program is admitted and its trace
line names no number (card `EXTERN-SCALAR-OUTCOME`, trace half).

This is the reachability of the undeclared observation. Without it,
`an_undeclared_outcome_names_no_number` would be a statement about a shape no
admitted program has — and the boundary the trace half declines to cross would
be vacuous rather than kept. -/
def undeclaredTrace : Program where
  externs := [.bind ⟨0, 7⟩, .call ⟨some ⟨1, none⟩, none, []⟩, .ret]
  io := [.log (.scalar 5), .ret]
  order := [.ext 0, .ext 1, .io 0, .ret]
  operandValues := [(5, 0)]
  operandIdentities := []
  externPremises := [.declaredContract 9]

theorem the_undeclared_trace_line_is_reachable :
    admitted undeclaredTrace [Effect.io] = true
      ∧ lines undeclaredTrace = some [⟨Stream.out, none⟩, ⟨Stream.err, some (render 7)⟩]
      ∧ numberOf undeclaredTrace.externs 1 = none := by
  refine ⟨by decide, by decide, by decide⟩

/-- An acquisition inside a composed module still needs its `alloc` declared,
and the identity it establishes is still on the ledger at the end: the union
carries the extern projection's row and its custody, unchanged. -/
def acquiring : Program :=
  { payoff with externs := [.bind ⟨0, 7⟩, .call ⟨some ⟨1, some 42⟩, some 3, []⟩, .ret] }

theorem an_acquiring_composition_needs_its_alloc_declared :
    composedEffects acquiring = [Effect.io, Effect.alloc]
      ∧ admitted acquiring [Effect.io] = false
      ∧ admitted acquiring [Effect.io, Effect.alloc] = true
      ∧ run acquiring [] = [3] := by
  refine ⟨by decide, by decide, by decide, by decide⟩

end QCore.ComposedStdIo
