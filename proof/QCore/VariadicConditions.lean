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

import QCore.Variadics

/-!
# A `va_arg` in a controlling expression, and the never-owned pointer value

Card **VA-LOOP-AND-POINTER-ARG** (gateway rule 12c), a sibling of
`QCore.Variadics` (card CG26-VARIADICS, rule 11w).

## The condition lowering preserves the protocol run

Rule 11w decides a function's `va_list` protocol over a graph whose head node
of `while ((x = va_arg(L, T)))` carries the condition's one event (`arg`) and
whose back edge re-enters the head. The custody gateway lowers the same
condition into two nodes: the event's statement, then the test of `x`, the
test inheriting the head's successors (the body and the exit). `lower g h`
is that transformation of a protocol graph: node `h` keeps its operations and
now leads only to a fresh node `g.length`, which performs no operation and
leads where `h` led.

* `lower_sound` — every concrete state some path of the lowered graph brings
  to an original node, some path of the protocol graph brings there too; one
  brought to the test node is the head's state after its event;
* `lower_complete` — and conversely: the lowering removes no path;
* `lowered_no_violation`, `lowered_exit` — so a protocol graph `certify`
  accepts has, after the lowering, no reachable violating node and only
  accepted states at the exit: the lowering preserves the protocol run on
  every path, the event happening once per evaluation, before the test.

## The never-owned binding

A pointer read by `va_arg` is bound as the derived pointer over no base (the
Rust `derive_pointer(None, …)`): no target, unknown, possibly null, derived.
`NeverOwned` models the custody positions the transfer function decides for
it: a release, consumption, store, escape or return through a derived binding
is never applied (`derived_custody_refusal`), and with no target it is never
accused. So it can be neither released nor counted as custody:
`never_owned_is_never_released`, `never_owned_changes_no_custody`,
`never_owned_is_never_accused`, and a null test narrows only the binding
(`a_null_test_refines_only_the_binding`).

## What this is not

The AST reading (which node is the condition, which declaration is an
automatic local) is trust boundary 28's premise; the Rust graph-lowering is
bound to `lower` by review and by the tests that pin it.
-/

namespace QCore.VariadicConditions

open QCore.Variadics

/-! ## The lowering -/

/-- Split node `h`: it keeps its operations and leads to the fresh test node
    `g.length`, which performs nothing and leads where `h` led. -/
def lower (g : List Node) (h : Nat) : List Node :=
  (g.set h ⟨(nodeAt g h).ops, [g.length]⟩) ++ [⟨[], (nodeAt g h).succ⟩]

/-- Every successor of a well-formed graph is one of its nodes. -/
def WellFormed (g : List Node) : Prop :=
  ∀ n m, m ∈ (nodeAt g n).succ → m < g.length

theorem length_lower (g : List Node) (h : Nat) : (lower g h).length = g.length + 1 := by
  simp [lower]

theorem nodeAt_lower_head (g : List Node) (h : Nat) (hh : h < g.length) :
    nodeAt (lower g h) h = ⟨(nodeAt g h).ops, [g.length]⟩ := by
  simp [lower, nodeAt, List.getD_eq_getElem?_getD, List.getElem?_append_left,
    List.length_set, hh]

theorem nodeAt_lower_test (g : List Node) (h : Nat) :
    nodeAt (lower g h) g.length = ⟨[], (nodeAt g h).succ⟩ := by
  simp [lower, nodeAt, List.getD_eq_getElem?_getD, List.length_set]

theorem nodeAt_lower_other (g : List Node) (h n : Nat) (hn : n < g.length) (hne : n ≠ h) :
    nodeAt (lower g h) n = nodeAt g n := by
  simp only [lower, nodeAt, List.getD_eq_getElem?_getD]
  rw [List.getElem?_append_left (by simpa [List.length_set] using hn)]
  rw [List.getElem?_set_ne (Ne.symm hne)]

theorem nodeAt_lower_beyond (g : List Node) (h n : Nat) (hn : g.length < n) :
    nodeAt (lower g h) n = ⟨[], []⟩ := by
  apply node_beyond
  rw [length_lower]
  omega

/-- **Soundness**: a state the lowered graph brings to an original node, the
    protocol graph brings there too; a state it brings to the test node is the
    head's state after the head's operations. -/
theorem lower_sound (g : List Node) (o : Origin) (entry h : Nat)
    (wf : WellFormed g) (he : entry < g.length) (hh : h < g.length) :
    ∀ {m : Nat} {x : ListState}, Reach (lower g h) o entry m x →
      (m < g.length → Reach g o entry m x) ∧
      (m = g.length → ∃ x₀, Reach g o entry h x₀ ∧ runOne o (nodeAt g h).ops x₀ = some x) := by
  intro m x r
  induction r with
  | init => exact ⟨fun _ => Reach.init, fun heq => absurd heq (by omega)⟩
  | @next n m x y r hrun hm ih =>
    by_cases hnh : n = h
    · subst hnh
      rw [nodeAt_lower_head g n hh] at hrun hm
      simp only [List.mem_singleton] at hm
      subst hm
      have rn := ih.1 hh
      exact ⟨fun hlt => absurd hlt (by omega), fun _ => ⟨x, rn, hrun⟩⟩
    · by_cases hnk : n = g.length
      · subst hnk
        rw [nodeAt_lower_test g h] at hrun hm
        simp only [runOne, Option.some.injEq] at hrun
        subst hrun
        obtain ⟨x₀, r₀, hrun₀⟩ := ih.2 rfl
        have hml : m < g.length := wf h m hm
        exact ⟨fun _ => Reach.next r₀ hrun₀ hm, fun heq => absurd heq (by omega)⟩
      · by_cases hnl : n < g.length
        · rw [nodeAt_lower_other g h n hnl hnh] at hrun hm
          have hml : m < g.length := wf n m hm
          exact ⟨fun _ => Reach.next (ih.1 hnl) hrun hm, fun heq => absurd heq (by omega)⟩
        · rw [nodeAt_lower_beyond g h n (by omega)] at hm
          simp at hm

/-- **Completeness**: the lowering removes no path to an original node. -/
theorem lower_complete (g : List Node) (o : Origin) (entry h : Nat)
    (hh : h < g.length) :
    ∀ {m : Nat} {x : ListState}, Reach g o entry m x → Reach (lower g h) o entry m x := by
  intro m x r
  induction r with
  | init => exact Reach.init
  | @next n m x y _ hrun hm ih =>
    by_cases hnh : n = h
    · subst hnh
      have step₁ : Reach (lower g n) o entry g.length y :=
        Reach.next ih (by rw [nodeAt_lower_head g n hh]; exact hrun)
          (by rw [nodeAt_lower_head g n hh]; simp)
      exact Reach.next step₁ (by rw [nodeAt_lower_test]; rfl)
        (by rw [nodeAt_lower_test]; exact hm)
    · by_cases hnl : n < g.length
      · exact Reach.next ih (by rw [nodeAt_lower_other g h n hnl hnh]; exact hrun)
          (by rw [nodeAt_lower_other g h n hnl hnh]; exact hm)
      · rw [node_beyond g n hnl] at hm
        simp at hm

/-- **No violation after the lowering**: a protocol graph `certify` accepts
    has, lowered, no reachable node whose operations fail. -/
theorem lowered_no_violation (o : Origin) (g : List Node) (a : List States)
    (entry exit h : Nat) (wf : WellFormed g) (he : entry < g.length) (hh : h < g.length)
    (ok : certify o g a entry exit = true) {m : Nat} {x : ListState}
    (r : Reach (lower g h) o entry m x) :
    ∃ y, runOne o (nodeAt (lower g h) m).ops x = some y := by
  have s := lower_sound g o entry h wf he hh r
  by_cases hml : m < g.length
  · by_cases hmh : m = h
    · subst hmh
      rw [nodeAt_lower_head g m hh]
      exact certified_no_violation o g a entry exit ok (s.1 hml)
    · rw [nodeAt_lower_other g h m hml hmh]
      exact certified_no_violation o g a entry exit ok (s.1 hml)
  · by_cases hmk : m = g.length
    · subst hmk
      rw [nodeAt_lower_test]
      exact ⟨x, rfl⟩
    · rw [nodeAt_lower_beyond g h m (by omega)]
      exact ⟨x, rfl⟩

/-- **The exit after the lowering**: every concrete state reaching an
    original exit node is accepted, so the lowering never hides a missing
    `va_end`. -/
theorem lowered_exit (o : Origin) (g : List Node) (a : List States)
    (entry exit h : Nat) (wf : WellFormed g) (he : entry < g.length) (hh : h < g.length)
    (hx : exit < g.length) (ok : certify o g a entry exit = true) {x : ListState}
    (r : Reach (lower g h) o entry exit x) : exitOkOne o x = true :=
  certified_exit o g a entry exit ok ((lower_sound g o entry h wf he hh r).1 hx)

/-! ### The loop shape, concretely

`while ((x = va_arg(L, T))) { … }` between `va_start` and `va_end`:
entry 1 → start 2 → head 3 (`arg`) → body 4 → head, and head → end 5 → exit 0.
The protocol graph is certified, and so is every lowered path. -/

def whileLoop : List Node :=
  [⟨[], []⟩, ⟨[], [2]⟩, ⟨[.start], [3]⟩, ⟨[.arg], [4, 5]⟩, ⟨[], [3]⟩, ⟨[.end_], [0]⟩]

def whileStates : List States :=
  [States.single .ended, States.single .uninitialised, States.single .uninitialised,
   States.single .active, States.single .active, States.single .active]

theorem the_while_protocol_is_certified :
    certify .own whileLoop whileStates 1 0 = true := by decide

theorem the_while_loop_is_well_formed : WellFormed whileLoop := by
  intro n m hm
  show m < 6
  rcases n with _ | _ | _ | _ | _ | _ | n
  · simp [whileLoop, nodeAt] at hm
  · simp [whileLoop, nodeAt] at hm; omega
  · simp [whileLoop, nodeAt] at hm; omega
  · simp [whileLoop, nodeAt] at hm; omega
  · simp [whileLoop, nodeAt] at hm; omega
  · simp [whileLoop, nodeAt] at hm; omega
  · rw [node_beyond whileLoop (n + 6) (by simp [whileLoop])] at hm
    simp at hm

/-- The lowered loop (event node 3, test node 6) has no violating path. -/
theorem the_lowered_while_loop_has_no_violation {m : Nat} {x : ListState}
    (r : Reach (lower whileLoop 3) .own 1 m x) :
    ∃ y, runOne .own (nodeAt (lower whileLoop 3) m).ops x = some y :=
  lowered_no_violation .own whileLoop whileStates 1 0 3 the_while_loop_is_well_formed
    (by decide) (by decide) the_while_protocol_is_certified r

/-- The defective body (a `return` inside the loop without `va_end`, Monkey's
    master `mk_vhost_set`): the body node also leads to the exit, and no
    assignment certifies it. -/
def leakingLoop : List Node :=
  [⟨[], []⟩, ⟨[], [2]⟩, ⟨[.start], [3]⟩, ⟨[.arg], [4, 5]⟩, ⟨[], [3, 0]⟩, ⟨[.end_], [0]⟩]

theorem the_leaking_loop_reaches_the_exit_active :
    Reach leakingLoop .own 1 0 .active :=
  Reach.next (n := 4)
    (Reach.next (n := 3)
      (Reach.next (n := 2) (Reach.next (n := 1) Reach.init rfl (by decide)) rfl (by decide))
      rfl (by decide))
    rfl (by decide)

theorem the_leaking_loop_is_never_certified (a : List States) :
    certify .own leakingLoop a 1 0 = false := by
  cases h : certify .own leakingLoop a 1 0 with
  | false => rfl
  | true =>
    have := certified_exit .own leakingLoop a 1 0 h the_leaking_loop_reaches_the_exit_active
    simp [exitOkOne] at this

/-! ## The never-owned binding -/

/-- A pointer binding as the custody positions read it: its targets, whether
    it is unknown, and whether it is a derived (non-owning) binding. -/
structure Binding where
  targets : List Nat
  unknown : Bool
  derived : Bool
  deriving DecidableEq, Repr

/-- The binding a `va_arg` pointer gets: no target, unknown, derived. -/
def vaArgBinding : Binding := ⟨[], true, true⟩

/-- The custody positions a call or a `return` hands a binding to. -/
inductive Position where
  | borrowed
  | released
  | consumed
  | stored
  | escaped
  deriving DecidableEq, Repr

/-- What a custody position does with a binding (the Rust transfer's
    decision): `apply s` changes the custody of target `s`; `refuse` records
    an unresolved alias and applies nothing; `accuse` is a validated
    violation candidate; `read` is a borrowed read changing nothing. -/
inductive Decision where
  | read
  | apply (site : Nat)
  | refuse
  | accuse
  deriving DecidableEq, Repr

/-- The transfer's decision (`apply_contract_call`, `derived_custody_refusal`,
    `derived_release_is_accusable`, the escape rule): a borrowed position only
    reads; a derived binding is never applied, and accused only for a release
    of exactly one known target; an identity binding is applied only to exactly
    one known target. -/
def decide' (p : Position) (b : Binding) : Decision :=
  match p with
  | .borrowed => .read
  | _ =>
    if b.derived then
      if p == .released && !b.unknown && b.targets.length == 1 then .accuse else .refuse
    else
      match b.targets, b.unknown with
      | [s], false => .apply s
      | _, _ => .refuse

/-- Custody held by the function: the sites whose custody it may change. A
    decision changes custody only by `apply`. -/
def custodyChange : Decision → Option Nat
  | .apply s => some s
  | _ => none

/-- **Never released**: a `va_arg` pointer at a release, consumption, store or
    escape is refused. -/
theorem never_owned_is_never_released (p : Position) (h : p ≠ .borrowed) :
    decide' p vaArgBinding = .refuse := by
  cases p <;> simp_all [decide', vaArgBinding]

/-- **Never counted as custody**: no position changes any custody through it. -/
theorem never_owned_changes_no_custody (p : Position) :
    custodyChange (decide' p vaArgBinding) = none := by
  cases p <;> simp [decide', vaArgBinding, custodyChange]

/-- **Never accused**: with no target it is never a validated violation, so
    a release of a caller's pointer is `Unknown`, never `Unsafe`. -/
theorem never_owned_is_never_accused (p : Position) :
    decide' p vaArgBinding ≠ .accuse := by
  cases p <;> simp [decide', vaArgBinding]

/-- A borrowed position reads it. -/
theorem never_owned_is_read_at_a_borrowed_position :
    decide' .borrowed vaArgBinding = .read := rfl

/-- A null test narrows the derived binding alone (the Rust
    `refine_pointer_null`'s derived arm): its targets, here none, are kept or
    cleared, so no site's custody or nullness changes. -/
def refineNull (b : Binding) (assumeNull : Bool) : Binding :=
  if b.derived then
    if assumeNull then ⟨[], false, true⟩ else b
  else b

theorem a_null_test_refines_only_the_binding (assumeNull : Bool) :
    (refineNull vaArgBinding assumeNull).targets = [] ∧
      (refineNull vaArgBinding assumeNull).derived = true := by
  cases assumeNull <;> simp [refineNull, vaArgBinding]

/-- Non-vacuity: an identity binding of one known target at a release is
    applied, so `never_owned_is_never_released` is about the binding, not the
    position. -/
theorem an_owned_identity_is_released :
    decide' .released ⟨[7], false, false⟩ = .apply 7 := rfl

end QCore.VariadicConditions
