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
# Forwarded callback formals (card SERVE-FORWARDED-CALLBACK-FORMALS)

The model of gateway rule 15w (`crates/qcore-c-gateway/src/legacy_callback_forward.rs`
and its hook in `legacy_callback_route.rs`'s `settle`).

A declaration `resource_callback_invoked(F, A…)` says: during the call, `F`
is called zero or more times, each time with actuals that are, position by
position, the caller's own formal `k` (a parameter word) or a value the
callee supplies (`_`, `supplied_owned`). Rule 13y validated it only on a body
that calls `F` directly. Rule 15w also validates it on a body whose every
mention of `F` is an *exact forward*: the whole actual, at position `p`, of a
call to a callee whose declaration in force states exactly one invocation of
its parameter `p`, `A'…`, such that `A'…` read through the call's actuals is
`A…` (`corresponds`).

* **Soundness of the reading.** Every invocation the callee makes under its
  clause, read in the forwarder's frame, meets the forwarder's clause
  (`forwarded_invocations_meet_the_caller_clause`): a parameter word of the
  callee names the call's actual there, which is the forwarder's own formal
  unchanged; a supplied value stays a supplied value.
* **Exactly when in force.** An exact forward validates the clause exactly
  when the callee's clause is in force and corresponds
  (`an_exact_forward_stands_iff_the_callee_clause_is_in_force`); a callee
  with no clause in force refuses (`a_forward_to_a_callee_not_in_force_refuses`).
* **Any deviation keeps the refusal.** A changed formal, a local use beside a
  forward (an invocation, a store, an operand), or no forward at all refuses
  (`a_changed_formal_refuses`, `any_other_mention_refuses`,
  `no_mention_refuses`); a forward whose actuals do not correspond refuses
  (`a_forward_that_does_not_correspond_refuses`).
* **The standing set is a least fixed point.** A callee defined in the unit
  is in force only once it stands; so a declaration whose only support is a
  forward to itself never stands at any round
  (`a_recursive_forward_alone_never_stands`).

`forwardVectors` is `FORWARD_VECTORS` of `legacy_callback_forward.rs`, bound
by its unit test `forward_vectors_hold`.
-/

namespace QCore.CallbackForward

/-- One actual of an invocation clause. -/
inductive Actual where
  /-- The caller's formal at this index, unchanged. -/
  | param (k : Nat)
  /-- `_`: a value the callee supplies. -/
  | supplied
  /-- `supplied_owned`: an object the callee supplies and hands over. -/
  | suppliedOwned
  /-- `element` (a registry invocation only). -/
  | element
  deriving DecidableEq, Repr

/-- `args`: the call's actuals, entry `k'` `some k` exactly when the actual at
the callee's parameter `k'` is the caller's formal `k`, unchanged. -/
abbrev Args := List (Option Nat)

/-- One position of `corresponds`. -/
def corr (args : Args) : Actual → Actual → Bool
  | .param k, .param k' => args.getD k' none == some k
  | .supplied, .supplied => true
  | .suppliedOwned, .suppliedOwned => true
  | _, _ => false

/-- The caller's clause actuals against the callee's, through `args`. -/
def corresponds (args : Args) : List Actual → List Actual → Bool
  | [], [] => true
  | a :: as, b :: bs => corr args a b && corresponds args as bs
  | _, _ => false

/-! ## The invocations -/

/-- A value an invocation passes, as the forwarder's frame sees it. -/
inductive Val where
  /-- The forwarder's formal `k`. -/
  | formal (k : Nat)
  /-- A value the callee supplies. -/
  | sup
  /-- An object the callee supplies and hands over. -/
  | supOwned
  /-- Anything else. -/
  | other
  deriving DecidableEq, Repr

/-- One actual of a clause met by one value, in a frame (`frame k'` the value
of the declaring function's parameter `k'`). -/
def meets1 (frame : Nat → Val) : Actual → Val → Prop
  | .param k, v => v = frame k
  | .supplied, v => v = .sup
  | .suppliedOwned, v => v = .supOwned
  | .element, _ => False

/-- An invocation's values meet a clause's actuals, position by position. -/
def meets (frame : Nat → Val) : List Actual → List Val → Prop
  | [], [] => True
  | a :: as, v :: vs => meets1 frame a v ∧ meets frame as vs
  | _, _ => False

/-- **The forward is sound.** If the callee's clause corresponds to the
caller's through the call's actuals, every invocation meeting the callee's
clause in the callee's frame (whose parameter `k'` holds the caller's formal
`k` wherever the actual there is that formal) meets the caller's clause in
the caller's frame. -/
theorem forwarded_invocations_meet_the_caller_clause (args : Args)
    (calleeFrame : Nat → Val)
    (hframe : ∀ k' k, args.getD k' none = some k → calleeFrame k' = .formal k) :
    ∀ (caller theirs : List Actual) (inv : List Val),
      corresponds args caller theirs = true →
      meets calleeFrame theirs inv → meets Val.formal caller inv := by
  intro caller
  induction caller with
  | nil =>
    intro theirs inv hc hm
    cases theirs with
    | nil =>
      cases inv with
      | nil => trivial
      | cons v vs => exact absurd hm (by simp [meets])
    | cons b bs => simp [corresponds] at hc
  | cons a as ih =>
    intro theirs inv hc hm
    cases theirs with
    | nil => simp [corresponds] at hc
    | cons b bs =>
      cases inv with
      | nil => exact absurd hm (by simp [meets])
      | cons v vs =>
        simp only [corresponds, Bool.and_eq_true] at hc
        obtain ⟨hab, hrest⟩ := hc
        obtain ⟨hv, hvs⟩ := hm
        refine ⟨?_, ih bs vs hrest hvs⟩
        cases a with
        | param k =>
          cases b with
          | param k' =>
            simp only [corr, beq_iff_eq] at hab
            simp only [meets1] at hv ⊢
            rw [hv, hframe k' k hab]
          | supplied => simp [corr] at hab
          | suppliedOwned => simp [corr] at hab
          | element => simp [corr] at hab
        | supplied =>
          cases b with
          | supplied => simpa [meets1] using hv
          | param _ => simp [corr] at hab
          | suppliedOwned => simp [corr] at hab
          | element => simp [corr] at hab
        | suppliedOwned =>
          cases b with
          | suppliedOwned => simpa [meets1] using hv
          | param _ => simp [corr] at hab
          | supplied => simp [corr] at hab
          | element => simp [corr] at hab
        | element => simp [corr] at hab

/-! ## The body check -/

/-- One mention of the callback formal in the body. -/
inductive Mention where
  /-- The whole actual at `position` of a direct call to `callee`. -/
  | forward (callee : String) (position : Nat) (args : Args)
  /-- A direct invocation of the formal. -/
  | invoke
  /-- Anything else: a store, an operand, a cast, a parenthesised use. -/
  | other
  deriving DecidableEq, Repr

/-- What the environment states: `stated g p` the one invocation `g`'s
declaration in force states of its parameter `p` (`none`: no declaration in
force, or the position named otherwise or more than once). -/
abbrev Stated := String → Nat → Option (List Actual)

/-- One mention validates the caller's clause `caller`. -/
def mentionValidates (stated : Stated) (caller : List Actual) : Mention → Bool
  | .forward g p args =>
    match stated g p with
    | some theirs => corresponds args caller theirs
    | none => false
  | _ => false

/-- Rule 15w's check of one clause: the formal unchanged, at least one
mention, and every mention an exact forward to a clause in force. -/
def forwardValidates (stated : Stated) (caller : List Actual) (unchanged : Bool)
    (mentions : List Mention) : Bool :=
  unchanged && !mentions.isEmpty && mentions.all (mentionValidates stated caller)

/-- **Exactly when in force.** A single exact forward validates the clause
exactly when the callee's declaration in force states a corresponding
invocation of that position. -/
theorem an_exact_forward_stands_iff_the_callee_clause_is_in_force (stated : Stated)
    (caller : List Actual) (g : String) (p : Nat) (args : Args) :
    forwardValidates stated caller true [.forward g p args] = true ↔
      ∃ theirs, stated g p = some theirs ∧ corresponds args caller theirs = true := by
  simp only [forwardValidates, Bool.true_and, List.isEmpty_cons, Bool.not_false,
    List.all_cons, List.all_nil, Bool.and_true, mentionValidates]
  cases h : stated g p with
  | none => simp
  | some theirs => simp

/-- A forward to a callee with no clause in force at that position refuses. -/
theorem a_forward_to_a_callee_not_in_force_refuses (stated : Stated) (caller : List Actual)
    (u : Bool) (g : String) (p : Nat) (args : Args) (rest : List Mention)
    (h : stated g p = none) :
    forwardValidates stated caller u (.forward g p args :: rest) = false := by
  simp [forwardValidates, mentionValidates, h]

/-- A forward whose actuals do not correspond refuses. -/
theorem a_forward_that_does_not_correspond_refuses (stated : Stated) (caller : List Actual)
    (u : Bool) (g : String) (p : Nat) (args : Args) (theirs : List Actual)
    (rest : List Mention) (h : stated g p = some theirs)
    (hc : corresponds args caller theirs = false) :
    forwardValidates stated caller u (.forward g p args :: rest) = false := by
  simp [forwardValidates, mentionValidates, h, hc]

/-- A formal reassigned, address-taken or otherwise changed refuses. -/
theorem a_changed_formal_refuses (stated : Stated) (caller : List Actual)
    (mentions : List Mention) : forwardValidates stated caller false mentions = false := by
  simp [forwardValidates]

/-- Any mention that is not a forward (a local invocation beside a forward,
a store, an operand) refuses. -/
theorem any_other_mention_refuses (stated : Stated) (caller : List Actual) (u : Bool)
    (mentions : List Mention) (m : Mention) (hm : m ∈ mentions)
    (hk : m = .invoke ∨ m = .other) :
    forwardValidates stated caller u mentions = false := by
  have hf : mentionValidates stated caller m = false := by
    rcases hk with rfl | rfl <;> rfl
  simp only [forwardValidates, Bool.and_eq_false_iff]
  right
  exact List.all_eq_false.mpr ⟨m, hm, by simp [hf]⟩

/-- A body that never mentions the formal validates nothing by forwarding. -/
theorem no_mention_refuses (stated : Stated) (caller : List Actual) (u : Bool) :
    forwardValidates stated caller u [] = false := by
  simp [forwardValidates]

/-! ## The least fixed point -/

/-- The environment of one round: `f`, defined in the unit, is in force only
once it stands (`S`); `outside` answers for every other callee. -/
def statedUnder (f : String) (own : List Actual) (outside : Stated) (S : Bool) : Stated :=
  fun g p => if g = f then (if S then some own else none) else outside g p

/-- One round for one declaration `f` whose clause's mentions are
`mentions`: it stands once its check passes against the round's
environment. -/
def round (f : String) (own : List Actual) (outside : Stated) (u : Bool)
    (mentions : List Mention) (S : Bool) : Bool :=
  S || forwardValidates (statedUnder f own outside S) own u mentions

/-- `n` rounds from nothing standing. -/
def rounds (f : String) (own : List Actual) (outside : Stated) (u : Bool)
    (mentions : List Mention) : Nat → Bool
  | 0 => false
  | n + 1 => round f own outside u mentions (rounds f own outside u mentions n)

/-- **A recursive forward alone never stands.** A declaration whose every
mention is a forward to itself is never in force at any round: its only
support would be itself. -/
theorem a_recursive_forward_alone_never_stands (f : String) (own : List Actual)
    (outside : Stated) (u : Bool) (mentions : List Mention)
    (hself : ∀ m ∈ mentions, ∃ p args, m = .forward f p args) :
    ∀ n, rounds f own outside u mentions n = false := by
  intro n
  induction n with
  | zero => rfl
  | succ n ih =>
    simp only [rounds, round, ih, Bool.false_or]
    cases hm : mentions with
    | nil => simp [forwardValidates]
    | cons m rest =>
      obtain ⟨p, args, rfl⟩ := hself m (by simp [hm])
      simp [forwardValidates, mentionValidates, statedUnder]

/-! ## Vectors bound to the Rust -/

/-- The callee `g` of the vectors: invokes its parameter 1 with
`(param 0, _, param 2)`; `keep` registers its parameter 1 (so states no
invocation of it). `gInForce` decides whether `g` is in force. -/
def vectorStated (gInForce : Bool) : Stated := fun g p =>
  if g = "g" ∧ p = 1 ∧ gInForce then some [.param 0, .supplied, .param 2] else none

/-- The caller's clause of every vector: `resource_callback_invoked(cb, r, _, c)`
over `(r, cb, c)`. -/
def vectorCaller : List Actual := [.param 0, .supplied, .param 2]

/-- `FORWARD_VECTORS` of `legacy_callback_forward.rs`: (g in force, formal
unchanged, mentions, stands). -/
def forwardVectors : List (Bool × Bool × List Mention × Bool) :=
  [ -- `return g ( r , cb , c ) ;`, g in force
    (true, true, [.forward "g" 1 [some 0, some 1, some 2]], true),
    -- the same, g not in force
    (false, true, [.forward "g" 1 [some 0, some 1, some 2]], false),
    -- `cb = 0 ; return g ( r , cb , c ) ;`
    (true, false, [.forward "g" 1 [some 0, some 1, some 2]], false),
    -- `return g ( r , c , cb ) ;`: another position
    (true, true, [.forward "g" 2 [some 0, some 2, some 1]], false),
    -- `return h ( r , cb , c ) ;`: an uncontracted callee
    (true, true, [.forward "h" 1 [some 0, some 1, some 2]], false),
    -- `cb ( r , 0 , c ) ; return g ( r , cb , c ) ;`
    (true, true, [.invoke, .forward "g" 1 [some 0, some 1, some 2]], false),
    -- `return g ( c , cb , r ) ;`: the actuals swapped
    (true, true, [.forward "g" 1 [some 2, some 1, some 0]], false),
    -- `keep ( r , cb ) ; return 0 ;`: a registration states no invocation
    (true, true, [.forward "keep" 1 [some 0, some 1]], false) ]

/-- Every vector holds. -/
theorem forward_vectors_hold :
    forwardVectors.all (fun (inForce, u, mentions, stands) =>
      forwardValidates (vectorStated inForce) vectorCaller u mentions == stands) = true := by
  decide

end QCore.CallbackForward
