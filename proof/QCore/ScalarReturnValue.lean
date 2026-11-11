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

import QCore.VerdictSemigroup

/-!
# A scalar `return` value read as the value of a statement (card W48-BOTTOM-UP-CALLEE-BODIES)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_scalar_return_value.rs` and its hook, asked
last in the `return` lowering of `legacy_c.rs`, gateway rule 12f.

C11 6.8.6.4p3: `return E;` evaluates `E` and hands its value to the caller,
converted as if by assignment to an object of the result type. In a function
whose result is a scalar value, `return E;` therefore evaluates what the
statement `T tmp = E; return tmp;` evaluates, and hands out a scalar. The
statement position decides that value with card W43's typed value walker; the
rule lowers the return with the walker's reading: every base's dereference
obligation, then an exit that escapes nothing.

## What is proved

* **The walker.** `walk` admits literals, scalar locals, a member `S.F` of an
  automatic aggregate (no base), a member `B->F` through a pointer (base `B`)
  and arithmetic over them, with at least one access; it refuses a call, an
  address `&X`, a pointer-valued name and a pointer member. A member of an
  automatic aggregate carries no obligation even when the aggregate's address
  was handed to a call (`an_automatic_member_carries_no_base`): reading a
  member of a live automatic object dereferences no pointer.
* **The reading is defined where the obligations hold.** For an admitted value
  whose every base is live, evaluation is defined
  (`admitted_value_is_defined`); evaluation writes nothing (it is a function of
  the state), and a released base fails its obligation
  (`a_released_base_fails_its_obligation`), which the flow reports.
* **Return and statement agree.** The return's lowering and the statement
  form's lowering check the same obligations, hand out the same value and
  leave every resource in the same state (`return_matches_statement`); the
  return escapes nothing (`a_scalar_return_escapes_nothing`), so an owned
  object dropped at the return is still owed (`an_owned_object_stays_owed`).
* **The decision.** The hook answers only for a return every route refused,
  in a function with a scalar result, with no descriptor or opaque callee,
  and a value the walker admits (`decide_iff`); the Rust table
  `DECISION_VECTORS` is `decisionVectors` (`decision_vectors_hold`).
* **Never `Safe` by declining.** A declined hook keeps the return's `Unknown`
  leaf, so its function is never `Safe` (`a_declined_return_is_never_safe`).

## What this is not

A proof that the Rust walker reads C faithfully: the walker's lexical reading
(`typed_field_access::typed_scalar_value`) and its declaration facts are card
W43's premises, and the correspondence with the compiled Rust is a review
premise (trust boundary 28), pinned by the shared-vector test in
`crates/qcore-c-gateway/tests/unit/legacy_scalar_return_value.rs` and the
fixtures in `crates/qcore-c-gateway/tests/w48_bottom_up_callee_bodies.rs`.
-/

namespace QCore.ScalarReturnValue

open QCore.VerdictSemigroup (V fold)

/-! ## Values the walker reads -/

/-- A scalar value expression, as far as the walker distinguishes it. -/
inductive Expr where
  /-- A numeric or character literal, an enumeration constant. -/
  | lit (n : Int)
  /-- A declared scalar local or formal. -/
  | scalar (x : Nat)
  /-- `S.F`: a scalar member of an automatic aggregate `S`. -/
  | member (s f : Nat)
  /-- `B->F`: a scalar member through the pointer `B`. -/
  | deref (b f : Nat)
  /-- An arithmetic or comparison operator. -/
  | bin (l r : Expr)
  /-- A call. -/
  | call (name : Nat)
  /-- `&X`. -/
  | addr (x : Nat)
  /-- A pointer-valued name or member. -/
  | pointer (x : Nat)
  deriving DecidableEq, Repr

/-- The walker: the bases whose dereference obligation the value carries and
its number of accesses, or `none` for a value it refuses. -/
def walkRaw : Expr → Option (List Nat × Nat)
  | .lit _ => some ([], 0)
  | .scalar _ => some ([], 0)
  | .member _ _ => some ([], 1)
  | .deref b _ => some ([b], 1)
  | .bin l r => do
      let (bl, al) ← walkRaw l
      let (br, ar) ← walkRaw r
      pure (bl ++ br, al + ar)
  | .call _ => none
  | .addr _ => none
  | .pointer _ => none

/-- An admitted value carries at least one access (a value with none is the
lexical predicate's, not the walker's). -/
def walk (e : Expr) : Option (List Nat) :=
  match walkRaw e with
  | some (bases, accesses) => if 0 < accesses then some bases else none
  | none => none

/-! ## A state and the reading -/

/-- What a value reads: the automatic aggregates' members, the scalar locals,
and the heap objects behind pointers with their liveness. -/
structure State where
  scalars : Nat → Int
  automatic : Nat → Nat → Int
  /-- `B`'s object, `none` once released. -/
  heap : Nat → Option (Nat → Int)

/-- Evaluation: `none` only where a base's object is gone (undefined
behaviour the flow must report). It is a function of the state: a value
writes nothing. -/
def eval (σ : State) : Expr → Option Int
  | .lit n => some n
  | .scalar x => some (σ.scalars x)
  | .member s f => some (σ.automatic s f)
  | .deref b f => (σ.heap b).map (fun object => object f)
  | .bin l r => do
      let a ← eval σ l
      let c ← eval σ r
      pure (a + c)
  | .call _ => none
  | .addr _ => none
  | .pointer _ => none

/-- Every base's object is live. -/
def obligations (σ : State) (bases : List Nat) : Prop := ∀ b ∈ bases, (σ.heap b).isSome

theorem walkRaw_some_eval {σ : State} :
    ∀ {e : Expr} {bases : List Nat} {n : Nat},
      walkRaw e = some (bases, n) → obligations σ bases → (eval σ e).isSome
  | .lit _, _, _, _, _ => by simp [eval]
  | .scalar _, _, _, _, _ => by simp [eval]
  | .member _ _, _, _, _, _ => by simp [eval]
  | .deref b f, bases, n, h, live => by
      simp only [walkRaw, Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, -⟩ := h
      have := live b (by simp)
      simp [eval, Option.isSome_map, this]
  | .bin l r, bases, n, h, live => by
      simp only [walkRaw, Option.bind_eq_bind] at h
      cases hl : walkRaw l with
      | none => simp [hl] at h
      | some pl =>
        cases hr : walkRaw r with
        | none => simp [hl, hr] at h
        | some pr =>
          simp [hl, hr] at h
          obtain ⟨rfl, -⟩ := h
          have el := walkRaw_some_eval (σ := σ) hl (fun b hb => live b (by simp [hb]))
          have er := walkRaw_some_eval (σ := σ) hr (fun b hb => live b (by simp [hb]))
          obtain ⟨a, ha⟩ := Option.isSome_iff_exists.mp el
          obtain ⟨c, hc⟩ := Option.isSome_iff_exists.mp er
          simp [eval, ha, hc]
  | .call _, _, _, h, _ => by simp [walkRaw] at h
  | .addr _, _, _, h, _ => by simp [walkRaw] at h
  | .pointer _, _, _, h, _ => by simp [walkRaw] at h

/-- The reading is defined wherever the admitted value's obligations hold. -/
theorem admitted_value_is_defined {σ : State} {e : Expr} {bases : List Nat}
    (h : walk e = some bases) (live : obligations σ bases) : (eval σ e).isSome := by
  unfold walk at h
  cases hw : walkRaw e with
  | none => simp [hw] at h
  | some p =>
    obtain ⟨bs, n⟩ := p
    simp [hw] at h
    obtain ⟨-, rfl⟩ := h
    exact walkRaw_some_eval hw live

/-- `S.F` carries no base, whatever was done with `&S`: an automatic object
is live for its whole block. -/
theorem an_automatic_member_carries_no_base (s f : Nat) : walk (.member s f) = some [] := by
  simp [walk, walkRaw]

/-- `rd_clock`'s return, `(T)ts.tv_sec * 1000000 + (T)ts.tv_nsec / 1000`,
in the walker's terms: two automatic members and literals, no base. -/
def rdClock : Expr := .bin (.bin (.member 0 0) (.lit 1000000)) (.bin (.member 0 1) (.lit 1000))

theorem rd_clock_is_admitted_with_no_base : walk rdClock = some [] := by decide

/-- A released base fails its obligation, so the flow's dereference check
reports the read instead of admitting it. -/
theorem a_released_base_fails_its_obligation {σ : State} {b : Nat} {bases : List Nat}
    (hb : b ∈ bases) (released : σ.heap b = none) : ¬ obligations σ bases := by
  intro live
  have := live b hb
  simp [released] at this

theorem a_call_is_refused (n : Nat) (e : Expr) : walk (.bin e (.call n)) = none := by
  simp [walk, walkRaw]

theorem an_address_is_refused (x : Nat) (e : Expr) : walk (.bin (.addr x) e) = none := by
  simp [walk, walkRaw]

theorem a_pointer_is_refused (x : Nat) (e : Expr) : walk (.bin e (.pointer x)) = none := by
  simp [walk, walkRaw]

/-! ## Return and statement -/

/-- The custody state the flow tracks: which resource sites are still owed. -/
abbrev Owed := List Nat

/-- What a lowering does to the flow: the obligations it checks, the value it
hands out, the sites the exit escapes and the owed set after it. -/
structure Effect where
  checks : List Nat
  value : Option Int
  escaped : List Nat
  owed : Owed
  deriving DecidableEq, Repr

/-- Rule 12f's lowering of `return E;`: the walker's read, then an exit that
escapes nothing. -/
def lowerReturn (σ : State) (owed : Owed) (e : Expr) (bases : List Nat) : Effect :=
  { checks := bases, value := eval σ e, escaped := [], owed := owed }

/-- The statement form `T tmp = E; return tmp;`: the same read into a scalar
local (which binds no identity), then the return of that scalar. -/
def lowerStatement (σ : State) (owed : Owed) (e : Expr) (bases : List Nat) : Effect :=
  let stored := eval σ e
  { checks := bases, value := stored, escaped := [], owed := owed }

theorem return_matches_statement (σ : State) (owed : Owed) (e : Expr) (bases : List Nat) :
    lowerReturn σ owed e bases = lowerStatement σ owed e bases := rfl

theorem a_scalar_return_escapes_nothing (σ : State) (owed : Owed) (e : Expr) (bases : List Nat) :
    (lowerReturn σ owed e bases).escaped = [] := rfl

/-- An object owed before the return is owed after it: the leak stays. -/
theorem an_owned_object_stays_owed (σ : State) (owed : Owed) (e : Expr) (bases : List Nat)
    (site : Nat) (h : site ∈ owed) : site ∈ (lowerReturn σ owed e bases).owed := h

/-! ## The decision and its shared vectors -/

/-- The hook's answer. -/
inductive Decision where
  | admit
  | decline
  deriving DecidableEq, Repr

/-- `refused`: every existing route refused the return; `scalarResult`: the
function's declared result is a scalar value; `hidden`: the value names a
tracked descriptor or an opaque callee spelling; `walker`: the walker admits
the value. -/
def decideReturn (refused scalarResult hidden walker : Bool) : Decision :=
  if refused && scalarResult && !hidden && walker then .admit else .decline

theorem decide_iff (refused scalarResult hidden walker : Bool) :
    decideReturn refused scalarResult hidden walker = .admit ↔
      refused = true ∧ scalarResult = true ∧ hidden = false ∧ walker = true := by
  cases refused <;> cases scalarResult <;> cases hidden <;> cases walker <;> decide

abbrev DecisionVector := Bool × Bool × Bool × Bool × Decision

/-- The Rust table `DECISION_VECTORS`, row for row. -/
def decisionVectors : List DecisionVector := [
  (false, false, false, false, .decline),
  (false, false, false, true, .decline),
  (false, false, true, false, .decline),
  (false, false, true, true, .decline),
  (false, true, false, false, .decline),
  (false, true, false, true, .decline),
  (false, true, true, false, .decline),
  (false, true, true, true, .decline),
  (true, false, false, false, .decline),
  (true, false, false, true, .decline),
  (true, false, true, false, .decline),
  (true, false, true, true, .decline),
  (true, true, false, false, .decline),
  (true, true, false, true, .admit),
  (true, true, true, false, .decline),
  (true, true, true, true, .decline)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun v => decideReturn v.1 v.2.1 v.2.2.1 v.2.2.2.1 == v.2.2.2.2) = true := by
  decide

/-! ## Never `Safe` by declining -/

/-- A declined hook keeps the return's refusal, an `Unknown` leaf. -/
theorem a_declined_return_is_never_safe (xs : List V) : fold (.unknown :: xs) ≠ .safe := by
  intro h
  have := (QCore.VerdictSemigroup.fold_eq_safe_iff (.unknown :: xs)).mp h .unknown (by simp)
  exact absurd this (by decide)

end QCore.ScalarReturnValue
