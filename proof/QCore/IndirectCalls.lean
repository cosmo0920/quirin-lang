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

import QCore.TypedefProjection

/-!
# Indirect calls and callbacks (card CG22-INDIRECT-CALLS-AND-CALLBACKS)

The abstract rule implemented by `crates/qcore-c-gateway/src/indirect_calls.rs`
(`join_over`, `callback_trace`) and the route-G rewrite
`legacy_indirect_calls.rs`, gateway rule 11p.

## Candidate sets (K01)

A call through a function pointer calls whatever function the pointer holds.
The gateway resolves a *candidate set* `S` from facts the unit establishes (a
designator, a local's initializer and assignments, a constant table) and
decides the call as the join of its candidates' verdicts. The model:

* `Verdict` is the four-valued verdict, ordered `safe < pending < unknown <
  unsafe` by `rank`; `join` is the greater.
* `joinOver covers vs` is `unknown` for an open set (`covers = false`) or an
  empty one, else the join of `vs` (`join_over`).
* `Covers vals S`: every value the pointer may hold at the call is in `S`.

`join_over_covers_sound`: when `S` covers the pointer's values, every value's
verdict is at most the decision, so a `safe` decision means every function
the call may make is `safe`. `an_open_set_can_miss_a_violation` shows the
cover premise is necessary, which is why an open set is `unknown`.
`a_local_holds_one_of_its_definitions` is the reaching-definition premise of a
local whose address is never taken: whatever path runs, its value is its
initializer or one of its assignments' values, so the union of them covers it.

## Callbacks (K02)

`callbackTrace` decides one registered callback and its context: registration
needs a live context and no earlier teardown, an invocation needs the
registration in force and the context live, the context may be released only
after unregistration, and a trace that ends registered is `unknown` (the
capture escapes). `invocation_after_release_is_never_safe` and
`registration_after_teardown_is_never_safe` are the two never-`safe` shapes.

## What this is not

A model of the decision. Faithful Clang extraction (callee shapes,
declaration ids, `const` qualification, initializer order), the token
placement of each rewrite, and the correspondence with the compiled Rust are
review premises (trust boundary 28), pinned by `joinVectors` and
`callbackVectors` (shared with the Rust tables by
`crates/qcore-c-gateway/tests/unit/indirect_calls.rs`) and by
`crates/qcore-c-gateway/tests/cg22_indirect_calls.rs`.
-/

namespace QCore.IndirectCalls

/-! ## The verdict and the join -/

inductive Verdict where
  | safe
  | pending
  | unknown
  | unsafe_
  deriving DecidableEq, Repr

def Verdict.rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .unsafe_ => 3

theorem rank_injective {a b : Verdict} (h : a.rank = b.rank) : a = b := by
  cases a <;> cases b <;> simp [Verdict.rank] at h ⊢

def join (a b : Verdict) : Verdict := if a.rank ≤ b.rank then b else a

theorem join_rank (a b : Verdict) : (join a b).rank = max a.rank b.rank := by
  unfold join
  split <;> omega

/-- `join_over` of `indirect_calls.rs`. -/
def joinOver (covers : Bool) : List Verdict → Verdict
  | [] => .unknown
  | v :: vs => if covers then vs.foldl join v else .unknown

theorem an_open_set_is_unknown (vs : List Verdict) : joinOver false vs = .unknown := by
  cases vs <;> rfl

theorem an_empty_set_is_unknown (c : Bool) : joinOver c [] = .unknown := rfl

theorem foldl_ge_acc (acc : Verdict) (vs : List Verdict) :
    acc.rank ≤ (vs.foldl join acc).rank := by
  induction vs generalizing acc with
  | nil => simp
  | cons v vs ih =>
    simp only [List.foldl]
    have h := ih (join acc v)
    rw [join_rank] at h
    omega

theorem foldl_ge_each (acc : Verdict) (vs : List Verdict) :
    ∀ v ∈ vs, v.rank ≤ (vs.foldl join acc).rank := by
  induction vs generalizing acc with
  | nil => simp
  | cons w vs ih =>
    intro v hv
    simp only [List.foldl]
    rcases List.mem_cons.mp hv with h | h
    · subst h
      have := foldl_ge_acc (join acc v) vs
      rw [join_rank] at this
      omega
    · exact ih (join acc w) v h

/-- Every candidate's verdict is at most a covering decision. -/
theorem join_over_ge_each (vs : List Verdict) :
    ∀ v ∈ vs, v.rank ≤ (joinOver true vs).rank := by
  intro v hv
  cases vs with
  | nil => simp at hv
  | cons w ws =>
    change v.rank ≤ (ws.foldl join w).rank
    rcases List.mem_cons.mp hv with h | h
    · subst h; exact foldl_ge_acc v ws
    · exact foldl_ge_each w ws v h

/-- A `safe` decision needs a covering set. -/
theorem safe_needs_a_cover (c : Bool) (vs : List Verdict) (h : joinOver c vs = .safe) :
    c = true := by
  cases vs with
  | nil => simp [joinOver] at h
  | cons v vs =>
    cases c with
    | false => simp [joinOver] at h
    | true => rfl

/-- A `safe` decision means every candidate is `safe`. -/
theorem safe_means_every_candidate_safe (vs : List Verdict)
    (h : joinOver true vs = .safe) : ∀ v ∈ vs, v = .safe := by
  intro v hv
  have := join_over_ge_each vs v hv
  rw [h] at this
  cases v <;> simp_all [Verdict.rank]

/-! ## Covering is what makes the join sound -/

/-- The pointer's possible values at the call are all candidates. -/
def Covers (vals S : List Nat) : Prop := ∀ x ∈ vals, x ∈ S

/-- **Soundness.** When the candidate set covers the pointer's values, the
verdict of every function the call may make is at most the decision. -/
theorem join_over_covers_sound (verdict : Nat → Verdict) (vals S : List Nat)
    (h : Covers vals S) : ∀ x ∈ vals, (verdict x).rank ≤ (joinOver true (S.map verdict)).rank := by
  intro x hx
  exact join_over_ge_each (S.map verdict) (verdict x) (List.mem_map.mpr ⟨x, h x hx, rfl⟩)

/-- In particular a `safe` decision over a covering set is `safe` for every
value the pointer may hold. -/
theorem a_covering_safe_decision_is_safe (verdict : Nat → Verdict) (vals S : List Nat)
    (h : Covers vals S) (hs : joinOver true (S.map verdict) = .safe) :
    ∀ x ∈ vals, verdict x = .safe := by
  intro x hx
  exact safe_means_every_candidate_safe _ hs (verdict x)
    (List.mem_map.mpr ⟨x, h x hx, rfl⟩)

/-- **The cover premise is necessary.** A set that misses one of the pointer's
values can decide `safe` while the missed function violates: so an open set
is `unknown`, never the join of what it names. -/
theorem an_open_set_can_miss_a_violation :
    ∃ (verdict : Nat → Verdict) (vals S : List Nat),
      ¬ Covers vals S ∧ joinOver true (S.map verdict) = .safe ∧
        ∃ x ∈ vals, verdict x = .unsafe_ := by
  refine ⟨fun n => if n = 0 then .safe else .unsafe_, [0, 1], [0], ?_, rfl, 1, by simp, rfl⟩
  intro h
  have := h 1 (by simp)
  simp at this

/-! ## A local's reaching definitions -/

/-- The value a local holds after a path runs its assignments in order. -/
def holds (init : Nat) (path : List Nat) : Nat := path.foldl (fun _ x => x) init

/-- Whatever path runs (any order, any repetition, any subset of the
assignments), the local holds its initializer or an assigned value: the
union of the definitions covers it. Needs the address never taken (no other
write) and an initializer (no indeterminate value), both checked by
`resolve`. -/
theorem a_local_holds_one_of_its_definitions (init : Nat) (assigns path : List Nat)
    (h : ∀ x ∈ path, x ∈ assigns) : holds init path ∈ init :: assigns := by
  induction path generalizing init with
  | nil => simp [holds]
  | cons x xs ih =>
    simp only [holds, List.foldl]
    have hx : x ∈ assigns := h x (by simp)
    have := ih x (fun y hy => h y (by simp [hy]))
    simp only [holds] at this
    rcases List.mem_cons.mp this with e | e
    · rw [e]; exact List.mem_cons_of_mem _ hx
    · exact List.mem_cons_of_mem _ e

/-! ## Callbacks -/

inductive Ev where
  | register
  | invoke
  | unregister
  | release
  deriving DecidableEq, Repr

structure St where
  reg : Bool
  live : Bool
  torn : Bool
  deriving DecidableEq, Repr

def step (s : St) : Ev → Option St
  | .register => if s.reg || !s.live || s.torn then none else some { s with reg := true }
  | .invoke => if !s.reg || !s.live then none else some s
  | .unregister => if !s.reg then none else some { s with reg := false, torn := true }
  | .release => if s.reg || !s.live then none else some { s with live := false }

def go (s : St) : List Ev → Verdict
  | [] => if s.reg then .unknown else .safe
  | e :: es =>
    match step s e with
    | none => .unsafe_
    | some s' => go s' es

/-- `callback_trace` of `indirect_calls.rs`. -/
def callbackTrace (t : List Ev) : Verdict := go ⟨false, true, false⟩ t

theorem step_keeps_released {s s' : St} {e : Ev} (hl : s.live = false)
    (h : step s e = some s') : s'.live = false := by
  obtain ⟨r, l, t⟩ := s
  simp only at hl
  subst hl
  cases e <;> cases r <;> cases t <;> simp [step] at h <;> subst h <;> rfl

theorem invoke_once_released (s : St) (hl : s.live = false) (es rest : List Ev) :
    go s (es ++ .invoke :: rest) ≠ .safe := by
  induction es generalizing s with
  | nil => simp [go, step, hl]
  | cons e es ih =>
    simp only [List.cons_append, go]
    cases h : step s e with
    | none => simp
    | some s' => exact ih s' (step_keeps_released hl h)

theorem release_then_invoke (s : St) (es mid rest : List Ev) :
    go s (es ++ .release :: (mid ++ .invoke :: rest)) ≠ .safe := by
  induction es generalizing s with
  | nil =>
    simp only [List.nil_append, go]
    cases h : step s .release with
    | none => simp
    | some s' =>
      have hl : s'.live = false := by
        obtain ⟨r, l, t⟩ := s
        cases r <;> cases l <;> cases t <;> simp [step] at h <;> subst h <;> rfl
      exact invoke_once_released s' hl mid rest
  | cons e es ih =>
    simp only [List.cons_append, go]
    cases h : step s e with
    | none => simp
    | some s' => exact ih s'

/-- **An invocation after the context's release is never `safe`.** -/
theorem invocation_after_release_is_never_safe (pre mid post : List Ev) :
    callbackTrace (pre ++ .release :: (mid ++ .invoke :: post)) ≠ .safe :=
  release_then_invoke _ pre mid post

theorem step_keeps_torn {s s' : St} {e : Ev} (ht : s.torn = true)
    (h : step s e = some s') : s'.torn = true := by
  obtain ⟨r, l, t⟩ := s
  simp only at ht
  subst ht
  cases e <;> cases r <;> cases l <;> simp [step] at h <;> subst h <;> rfl

theorem register_once_torn (s : St) (ht : s.torn = true) (es rest : List Ev) :
    go s (es ++ .register :: rest) ≠ .safe := by
  induction es generalizing s with
  | nil => simp [go, step, ht]
  | cons e es ih =>
    simp only [List.cons_append, go]
    cases h : step s e with
    | none => simp
    | some s' => exact ih s' (step_keeps_torn ht h)

theorem unregister_then_register (s : St) (es mid rest : List Ev) :
    go s (es ++ .unregister :: (mid ++ .register :: rest)) ≠ .safe := by
  induction es generalizing s with
  | nil =>
    simp only [List.nil_append, go]
    cases h : step s .unregister with
    | none => simp
    | some s' =>
      have ht : s'.torn = true := by
        obtain ⟨r, l, t⟩ := s
        cases r <;> cases l <;> cases t <;> simp [step] at h <;> subst h <;> rfl
      exact register_once_torn s' ht mid rest
  | cons e es ih =>
    simp only [List.cons_append, go]
    cases h : step s e with
    | none => simp
    | some s' => exact ih s'

/-- **A registration after teardown is never `safe`.** -/
theorem registration_after_teardown_is_never_safe (pre mid post : List Ev) :
    callbackTrace (pre ++ .unregister :: (mid ++ .register :: post)) ≠ .safe :=
  unregister_then_register _ pre mid post

/-- A registration with no teardown is an escaped capture: `unknown`. -/
theorem an_escaped_registration_is_unknown :
    callbackTrace [.register, .invoke] = .unknown := rfl

/-- The full protocol is `safe`. -/
theorem the_protocol_is_safe :
    callbackTrace [.register, .invoke, .invoke, .unregister, .release] = .safe := rfl

/-! ## The inline function-pointer parameter declarator

Absorbed LOADER-ABI-INLINE-FNPTR-DECLARATOR: the declared-ABI reader reads a
parameter `RET (*NAME)(PARAMETERS)` (`inline_function_pointer_parameter` of
`legacy_declared_abi.rs`) as exactly the target a function-pointer typedef
reads (`QCore.TypedefProjection`), so it projects to `fnptr` and nothing
else; a deeper or qualified inline declarator stays unread. -/

/-- An inline parameter declarator: `stars` pointer levels inside the
parentheses, `qualified` when a qualifier stands there too. -/
inductive InlineDeclarator where
  | fnptr (stars : Nat) (qualified : Bool)
  | other
  deriving DecidableEq, Repr

/-- The target the reader commits to: one unqualified level only. -/
def readInline : InlineDeclarator → Option QCore.TypedefProjection.Target
  | .fnptr 1 false => some .functionPointer
  | _ => none

/-- The parameter's projection. -/
def projectInline (d : InlineDeclarator) : QCore.DeclaredAbiAgreement.Projection :=
  match readInline d with
  | some t => QCore.TypedefProjection.project t 0 false
  | none => none

theorem an_inline_declarator_reads_as_the_typedef :
    projectInline (.fnptr 1 false) =
      QCore.TypedefProjection.project .functionPointer 0 false := rfl

theorem an_inline_declarator_projects_only_fnptr (d : InlineDeclarator)
    (x : List QCore.DeclaredAbiAgreement.AbiType) (h : projectInline d = some x) :
    x = [.fnptr] := by
  cases d with
  | other => simp [projectInline, readInline] at h
  | fnptr k q =>
    match k, q with
    | 1, false =>
      simp [projectInline, readInline, QCore.TypedefProjection.project] at h
      exact h.symm
    | 0, _ => simp [projectInline, readInline] at h
    | n + 2, _ => simp [projectInline, readInline] at h
    | 1, true => simp [projectInline, readInline] at h

theorem a_deeper_inline_declarator_stays_unread (k : Nat) (q : Bool) :
    projectInline (.fnptr (k + 2) q) = none := by
  simp [projectInline, readInline]

theorem a_qualified_inline_declarator_stays_unread (k : Nat) :
    projectInline (.fnptr k true) = none := by
  cases k <;> simp [projectInline, readInline]

/-! ## Shared vectors -/

/-- `JOIN_VECTORS` of `indirect_calls.rs`: (covers, candidates, the join). -/
def joinVectors : List (Bool × List Verdict × Verdict) := [
  (true, [.safe, .safe], .safe),
  (true, [.safe, .unknown], .unknown),
  (true, [.safe, .pending], .pending),
  (true, [.unsafe_, .safe], .unsafe_),
  (true, [.pending, .unknown], .unknown),
  (true, [.safe], .safe),
  (true, [], .unknown),
  (false, [.safe], .unknown),
  (false, [.safe, .safe], .unknown),
  (false, [], .unknown)
]

theorem join_vectors_hold :
    joinVectors.all (fun (c, vs, r) => joinOver c vs == r) = true := by
  decide

theorem join_vectors_count : joinVectors.length = 10 := by decide

/-- `CALLBACK_VECTORS` of `indirect_calls.rs`: a trace and its decision. -/
def callbackVectors : List (List Ev × Verdict) := [
  ([.register, .invoke, .invoke, .unregister, .release], .safe),
  ([.register, .release, .invoke], .unsafe_),
  ([.register, .invoke, .release], .unsafe_),
  ([.release, .register], .unsafe_),
  ([.register, .unregister, .register], .unsafe_),
  ([.register, .unregister, .invoke], .unsafe_),
  ([.register, .invoke], .unknown),
  ([.invoke], .unsafe_),
  ([.register, .register], .unsafe_),
  ([], .safe)
]

theorem callback_vectors_hold :
    callbackVectors.all (fun (t, r) => callbackTrace t == r) = true := by
  decide

theorem callback_vectors_count : callbackVectors.length = 10 := by decide

end QCore.IndirectCalls
