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

import QCore.PremiseFormula

/-!
# The libevent scheduler premises as formulas (card SP4-SYMBOLIC-LIBEVENT-PREMISES)

Contract specification C04.18.9, SP4 block. `qcore-provider-libevent` states
the seven premises of `mk_http_thread_destroy`
(`contracts/monkey/intrusive_list.qv`) as formulas over the SP4 terms, which
`QCore.PremiseFormula` registers (`vocab`) and interprets (`ext0`, `ext1El`,
`ext1St`, and `#singleton` as the singleton set).

Theorems:

* the seven statements are well-formed (`libevent_formulas_wf`); the Rust
  test `the_libevent_formulas_are_the_lean_definitions` binds each definition
  to its clause as read from the module;
* what each states, over the model (`eval_*`): the singleton quantifiers are
  the let-bindings they read as (`singleton_quant`);
* the premises compose: under the callee and membership premises, every
  concurrent run leaves the coroutine `th->callee` unwritten, unresumed and
  unreleased (`concurrent_runs_leave_the_callee`);
* they are jointly satisfiable with an argument present
  (`libevent_formulas_consistent`), and each assumes something: one model
  falsifies all seven (`libevent_formulas_assume_something`).

## What this is not

A premise is an undischarged assumption (C04.18): nothing here establishes
that a model describes Monkey, and no consumer that decides a verdict reads
these formulas.
-/

namespace QCore.PremiseFormula.Libevent

open QCore.PremiseFormula

/-! ## The seven statements (`intrusive_list.qv`, `mk_http_thread_destroy`) -/

def parentEmbeddingFormula : Raw := (.quant "m" (.app0 "args") (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app1 "mk_parent" (.app0 "m")) (.app0 "live")) (.bin .mem (.app0 "m") (.app1 "thread_data" (.app1 "mk_parent" (.app0 "m"))))) (.bin .mem (.app0 "m") (.app1 "inside" (.app1 "mk_parent" (.app0 "m"))))) (.bin .mem (.app1 "mk_parent" (.app0 "m")) (.app0 "released_only_by_call"))))

def tlsParamsFormula : Raw := (.bin .conj (.bin .conj (.bin .mem (.app0 "libco_params_key") (.app0 "getspecific_ok")) (.bin .mem (.app0 "libco_params_key") (.app0 "setspecific_ok"))) (.quant "p" (.app1 "holds" (.app0 "libco_params_key")) (.bin .conj (.bin .conj (.bin .mem (.app0 "p") (.bin .diff (.app0 "live") (.app1 "overlaps" (.app0 "tracked")))) (.bin .mem (.app0 "p") (.app1 "sole_owned" (.app0 "libco_params_key")))) (.bin .mem (.app0 "p") (.bin .diff (.app0 "freed") (.app1 "held_at_return" (.app0 "libco_params_key")))))))

def calleeLiveFormula : Raw := (.quant "m" (.app0 "args") (.quant "c" (.app1 "singleton" (.app1 "mk_callee" (.app1 "mk_parent" (.app0 "m")))) (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app0 "c") (.app0 "freed")) (.bin .mem (.app0 "c") (.app0 "co_deletable"))) (.bin .mem (.app0 "c") (.app1 "created_for" (.app1 "mk_parent" (.app0 "m"))))) (.bin .mem (.app0 "c") (.bin .diff (.app0 "suspended") (.app0 "co_active")))) (.bin .mapsTo (.app0 "concurrent") (.bin .conj (.app1 "no_resumes" (.app1 "singleton" (.app0 "c"))) (.app1 "no_releases" (.app1 "singleton" (.app0 "c"))))))))

def borrowedChainFormula : Raw := (.quant "m" (.app0 "args") (.quant "s" (.app1 "singleton" (.app1 "mk_session" (.app0 "m"))) (.quant "ch" (.app1 "singleton" (.app1 "mk_channel" (.app0 "s"))) (.quant "ev" (.app1 "singleton" (.app1 "mk_event" (.app0 "ch"))) (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app0 "s") (.app0 "live")) (.bin .mem (.app0 "ch") (.bin .diff (.app0 "live") (.bin .union (.app0 "released") (.app0 "ended"))))) (.bin .mem (.app0 "ev") (.bin .diff (.app0 "live") (.bin .union (.app0 "released") (.app0 "ended"))))) (.bin .mem (.app1 "mk_type" (.app0 "ev")) (.app1 "stores" (.bin .union (.bin .union (.app1 "singleton" (.app0 "s")) (.app1 "singleton" (.app0 "ch"))) (.app1 "singleton" (.app0 "ev")))))) (.quant "w" (.app1 "stores" (.bin .union (.bin .union (.app1 "singleton" (.app0 "s")) (.app1 "singleton" (.app0 "ch"))) (.app1 "singleton" (.app0 "ev")))) (.bin .mem (.app0 "w") (.app1 "singleton" (.app1 "mk_type" (.app0 "ev"))))))))))

def validMembershipFormula : Raw := (.quant "m" (.app0 "args") (.quant "n" (.app1 "singleton" (.app1 "mk_link" (.app0 "m"))) (.quant "l" (.app1 "singleton" (.app1 "caller_list" (.app0 "n"))) (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app0 "l") (.app1 "lists" (.app0 "n"))) (.quant "k" (.app1 "lists" (.app0 "n")) (.bin .mem (.app0 "k") (.app1 "singleton" (.app0 "l"))))) (.bin .mem (.app0 "l") (.app0 "circular_lists"))) (.quant "k" (.app1 "nodes" (.app0 "l")) (.bin .mem (.app0 "k") (.app0 "reciprocal")))) (.bin .mapsTo (.app0 "concurrent") (.app1 "no_writes" (.bin .union (.bin .union (.bin .union (.bin .union (.bin .union (.app1 "nodes" (.app0 "l")) (.app1 "singleton" (.app1 "mk_callee" (.app1 "mk_parent" (.app0 "m"))))) (.app1 "singleton" (.app1 "mk_session" (.app0 "m")))) (.app1 "singleton" (.app1 "mk_channel" (.app1 "mk_session" (.app0 "m"))))) (.app1 "singleton" (.app1 "mk_event" (.app1 "mk_channel" (.app1 "mk_session" (.app0 "m")))))) (.app1 "singleton" (.app0 "libco_params_key")))))))))

def noLaterAliasFormula : Raw := (.quant "m" (.app0 "args") (.quant "a" (.app1 "aliases" (.app1 "mk_parent" (.app0 "m"))) (.bin .conj (.bin .mem (.app0 "a") (.app0 "unread_after")) (.bin .mem (.app0 "a") (.app0 "unresumed_after")))))

def valgrindNeutralFormula : Raw := (.quant "m" (.app0 "args") (.bin .mapsTo (.app1 "stack_deregister" (.app1 "mk_parent" (.app0 "m"))) (.bin .conj (.app1 "no_writes" (.app0 "tracked")) (.app1 "no_releases" (.app0 "tracked")))))

theorem libevent_formulas_wf :
    wf parentEmbeddingFormula = true ∧ wf tlsParamsFormula = true ∧
      wf calleeLiveFormula = true ∧ wf borrowedChainFormula = true ∧
      wf validMembershipFormula = true ∧ wf noLaterAliasFormula = true ∧
      wf valgrindNeutralFormula = true := by decide

/-! ## What each states -/

/-- `#mk_parent(#m)`: the mk_thread `th = mth->parent`. -/
def parent (M : Model) (m : M.U) : M.U := M.xEl1 "mk_parent" m

/-- `#mk_callee(#mk_parent(#m))`: the coroutine `th->callee`. -/
def callee (M : Model) (m : M.U) : M.U := M.xEl1 "mk_callee" (parent M m)

/-- `#mk_session(#m)`: `mth->session`. -/
def session (M : Model) (m : M.U) : M.U := M.xEl1 "mk_session" m

/-- `#mk_channel(#mk_session(#m))`: the session's channel. -/
def channel (M : Model) (m : M.U) : M.U := M.xEl1 "mk_channel" (session M m)

/-- `#mk_event(…)`: that channel's event. -/
def event (M : Model) (m : M.U) : M.U := M.xEl1 "mk_event" (channel M m)

/-- `#libco_params_key`. -/
def paramsKey (M : Model) : M.U := M.xEl0 "libco_params_key"

/-- `#concurrent |-> E`: every run of a concurrent function has `E`. -/
def ConcurrentRuns (M : Model) (E : M.Run → Prop) : Prop :=
  ∀ f, M.xSt0 "concurrent" f → ∀ r, M.runs f r → E r

def ParentEmbedding (M : Model) : Prop :=
  ∀ m, M.args m →
    M.xSt0 "live" (parent M m) ∧ M.xSt1 "thread_data" (parent M m) m ∧
      M.xSt1 "inside" (parent M m) m ∧ M.xSt0 "released_only_by_call" (parent M m)

def TlsParams (M : Model) : Prop :=
  M.xSt0 "getspecific_ok" (paramsKey M) ∧ M.xSt0 "setspecific_ok" (paramsKey M) ∧
    ∀ p, M.xSt1 "holds" (paramsKey M) p →
      (M.xSt0 "live" p ∧ ¬ M.xStSt "overlaps" (M.xSt0 "tracked") p) ∧
        M.xSt1 "sole_owned" (paramsKey M) p ∧
        (M.freed p ∧ ¬ M.xSt1 "held_at_return" (paramsKey M) p)

def CalleeLive (M : Model) : Prop :=
  ∀ m, M.args m →
    M.freed (callee M m) ∧ M.xSt0 "co_deletable" (callee M m) ∧
      M.xSt1 "created_for" (parent M m) (callee M m) ∧
      (M.xSt0 "suspended" (callee M m) ∧ ¬ M.xSt0 "co_active" (callee M m)) ∧
      ConcurrentRuns M (fun r =>
        M.xEfSt "no_resumes" (fun u => u = callee M m) r ∧
          M.xEfSt "no_releases" (fun u => u = callee M m) r)

/-- The borrowed chain `mth->session`, its channel and that channel's event. -/
def chain (M : Model) (m : M.U) : M.U → Prop :=
  fun u => (u = session M m ∨ u = channel M m) ∨ u = event M m

def BorrowedChain (M : Model) : Prop :=
  ∀ m, M.args m →
    M.xSt0 "live" (session M m) ∧
      (M.xSt0 "live" (channel M m) ∧
        ¬ (M.xSt0 "released" (channel M m) ∨ M.xSt0 "ended" (channel M m))) ∧
      (M.xSt0 "live" (event M m) ∧
        ¬ (M.xSt0 "released" (event M m) ∨ M.xSt0 "ended" (event M m))) ∧
      M.xStSt "stores" (chain M m) (M.xEl1 "mk_type" (event M m)) ∧
      ∀ w, M.xStSt "stores" (chain M m) w → w = M.xEl1 "mk_type" (event M m)

/-- The caller's list of `mth->_head`. -/
def callerList (M : Model) (m : M.U) : M.U :=
  M.xEl1 "caller_list" (M.xEl1 "mk_link" m)

/-- What no concurrent run writes: the list, the coroutine, the chain, the key. -/
def unwritten (M : Model) (m : M.U) : M.U → Prop :=
  fun u => ((((M.xSt1 "nodes" (callerList M m) u ∨ u = callee M m) ∨ u = session M m) ∨
    u = channel M m) ∨ u = event M m) ∨ u = paramsKey M

def ValidMembership (M : Model) : Prop :=
  ∀ m, M.args m →
    M.xSt1 "lists" (M.xEl1 "mk_link" m) (callerList M m) ∧
      (∀ k, M.xSt1 "lists" (M.xEl1 "mk_link" m) k → k = callerList M m) ∧
      M.xSt0 "circular_lists" (callerList M m) ∧
      (∀ k, M.xSt1 "nodes" (callerList M m) k → M.xSt0 "reciprocal" k) ∧
      ConcurrentRuns M (M.xEfSt "no_writes" (unwritten M m))

def NoLaterAlias (M : Model) : Prop :=
  ∀ m, M.args m → ∀ a, M.xSt1 "aliases" (parent M m) a →
    M.xSt0 "unread_after" a ∧ M.xSt0 "unresumed_after" a

def ValgrindNeutral (M : Model) : Prop :=
  ∀ m, M.args m → ∀ f, M.xSt1 "stack_deregister" (parent M m) f → ∀ r, M.runs f r →
    M.xEfSt "no_writes" (M.xSt0 "tracked") r ∧ M.xEfSt "no_releases" (M.xSt0 "tracked") r

theorem eval_parent_embedding (M : Model) :
    (eval M [] parentEmbeddingFormula).holds ↔ ParentEmbedding M := by
  simp [eval, parentEmbeddingFormula, lk, prim0, prim1, ext0, ext1El, term, vocab, binVal,
    Val.holds, and_assoc, ParentEmbedding, parent]

theorem eval_tls_params (M : Model) :
    (eval M [] tlsParamsFormula).holds ↔ TlsParams M := by
  simp [eval, tlsParamsFormula, lk, prim0, prim1, ext0, ext1El, ext1St, term, vocab, binVal,
    Val.holds, and_assoc, TlsParams, paramsKey]

theorem eval_callee_live (M : Model) :
    (eval M [] calleeLiveFormula).holds ↔ CalleeLive M := by
  simp [eval, calleeLiveFormula, lk, prim0, prim1, ext0, ext1El, ext1St, term, vocab, binVal,
    Val.holds, and_assoc, CalleeLive, ConcurrentRuns, callee, parent]

theorem eval_borrowed_chain (M : Model) :
    (eval M [] borrowedChainFormula).holds ↔ BorrowedChain M := by
  simp [eval, borrowedChainFormula, lk, prim0, prim1, ext0, ext1El, ext1St, term, vocab, binVal,
    Val.holds, and_assoc, BorrowedChain, session, channel, event]
  rfl

theorem eval_valid_membership (M : Model) :
    (eval M [] validMembershipFormula).holds ↔ ValidMembership M := by
  simp [eval, validMembershipFormula, lk, prim0, prim1, ext0, ext1El, ext1St, term, vocab,
    binVal, Val.holds, ValidMembership, ConcurrentRuns, callerList]
  constructor
  · intro h m hm
    obtain ⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, h5⟩ := h m hm
    exact ⟨h1, h2, h3, h4, h5⟩
  · intro h m hm
    obtain ⟨h1, h2, h3, h4, h5⟩ := h m hm
    exact ⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, h5⟩

theorem eval_no_later_alias (M : Model) :
    (eval M [] noLaterAliasFormula).holds ↔ NoLaterAlias M := by
  simp [eval, noLaterAliasFormula, lk, prim0, prim1, ext0, ext1El, term, vocab, binVal,
    Val.holds, NoLaterAlias, parent]

theorem eval_valgrind_neutral (M : Model) :
    (eval M [] valgrindNeutralFormula).holds ↔ ValgrindNeutral M := by
  simp [eval, valgrindNeutralFormula, lk, prim0, prim1, ext0, ext1El, ext1St, term, vocab,
    binVal, Val.holds, ValgrindNeutral, parent]

/-! ## The premises compose -/

/-- `#no_writes(S)` reads as "the run writes no object of `S`", so a run that
writes no object of a set writes none of a part of it. -/
def NoWritesAntitone (M : Model) : Prop :=
  ∀ (S T : M.U → Prop) r, (∀ u, T u → S u) →
    M.xEfSt "no_writes" S r → M.xEfSt "no_writes" T r

/-- Under the callee and membership premises, every concurrent run leaves the
coroutine `th->callee` unwritten (membership: no concurrent writer of the
coroutine), unresumed and unreleased (callee: no concurrent resume or
deletion). -/
theorem concurrent_runs_leave_the_callee (M : Model) (hW : NoWritesAntitone M)
    (hC : (eval M [] calleeLiveFormula).holds)
    (hV : (eval M [] validMembershipFormula).holds) :
    ∀ m, M.args m → ConcurrentRuns M (fun r =>
      M.xEfSt "no_writes" (fun u => u = callee M m) r ∧
        M.xEfSt "no_resumes" (fun u => u = callee M m) r ∧
        M.xEfSt "no_releases" (fun u => u = callee M m) r) := by
  rw [eval_callee_live] at hC
  rw [eval_valid_membership] at hV
  intro m hm f hf r hr
  obtain ⟨_, _, _, _, hc⟩ := hC m hm
  obtain ⟨_, _, _, _, hv⟩ := hV m hm
  obtain ⟨h1, h2⟩ := hc f hf r hr
  refine ⟨hW _ _ r ?_ (hv f hf r hr), h1, h2⟩
  intro u hu
  subst hu
  simp [unwritten]

/-! ## Satisfiable, and not vacuous -/

/-- A model of one object in which every SP4 set holds it except the released,
ended, running, overlapping and returned-to ones. -/
def satisfyingModel : Model := {
  U := Unit, Run := Unit,
  args := fun _ => True, created := fun _ => False, copied := fun _ => False,
  freed := fun _ => True, cls := fun _ => (), ctx := fun _ _ => False,
  bios := fun _ _ => False, chain := fun _ _ => False, sslOf := fun _ _ => False,
  slots := fun _ _ => False, stated := fun _ => False, exData := fun _ _ => False,
  exDataFree := fun _ _ => False, method := id, builtin := fun _ => False,
  fns := fun _ _ => False, store := fun _ _ => False, lookups := fun _ _ => False,
  teardown := fun _ _ => False, settable := fun _ => False,
  inTurn := fun _ _ _ => False, callerHeld := fun _ _ => False,
  takes := fun _ _ _ => False, releases := fun _ _ _ => False,
  retains := fun _ _ _ => False, runs := fun _ _ => True, const := fun _ => (),
  xSt0 := fun f _ => f ≠ "released" ∧ f ≠ "ended" ∧ f ≠ "co_active",
  xSt1 := fun f _ _ => f ≠ "held_at_return",
  xStSt := fun f _ _ => f ≠ "overlaps",
  xEfSt := fun _ _ _ => True }

/-- The seven premises hold together, with the argument present. -/
theorem libevent_formulas_consistent :
    ∃ M : Model, (∃ m, M.args m) ∧
      (eval M [] parentEmbeddingFormula).holds ∧ (eval M [] tlsParamsFormula).holds ∧
      (eval M [] calleeLiveFormula).holds ∧ (eval M [] borrowedChainFormula).holds ∧
      (eval M [] validMembershipFormula).holds ∧ (eval M [] noLaterAliasFormula).holds ∧
      (eval M [] valgrindNeutralFormula).holds := by
  refine ⟨satisfyingModel, ⟨(), trivial⟩, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [eval_parent_embedding]; simp [ParentEmbedding, satisfyingModel]
  · rw [eval_tls_params]; simp [TlsParams, satisfyingModel]
  · rw [eval_callee_live]; simp [CalleeLive, ConcurrentRuns, satisfyingModel]
  · rw [eval_borrowed_chain]; simp [BorrowedChain, satisfyingModel]
  · rw [eval_valid_membership]; simp [ValidMembership, ConcurrentRuns, satisfyingModel]
  · rw [eval_no_later_alias]; simp [NoLaterAlias, satisfyingModel]
  · rw [eval_valgrind_neutral]; simp [ValgrindNeutral, satisfyingModel]

/-- A model of one object in which no SP4 set of objects holds it, every set
the argument names does, and no run has an SP4 effect. -/
def falsifyingModel : Model := {
  satisfyingModel with
  freed := fun _ => False,
  xSt0 := fun _ _ => False,
  xSt1 := fun _ _ _ => True,
  xStSt := fun _ _ _ => False,
  xEfSt := fun _ _ _ => False }

/-- Each premise assumes something: in one model with the argument present,
all seven fail. -/
theorem libevent_formulas_assume_something :
    ∃ M : Model, (∃ m, M.args m) ∧
      ¬ (eval M [] parentEmbeddingFormula).holds ∧ ¬ (eval M [] tlsParamsFormula).holds ∧
      ¬ (eval M [] calleeLiveFormula).holds ∧ ¬ (eval M [] borrowedChainFormula).holds ∧
      ¬ (eval M [] validMembershipFormula).holds ∧ ¬ (eval M [] noLaterAliasFormula).holds ∧
      ¬ (eval M [] valgrindNeutralFormula).holds := by
  refine ⟨falsifyingModel, ⟨(), trivial⟩, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [eval_parent_embedding]; simp [ParentEmbedding, falsifyingModel, satisfyingModel]
  · rw [eval_tls_params]; simp [TlsParams, falsifyingModel]
  · rw [eval_callee_live]; simp [CalleeLive, falsifyingModel, satisfyingModel]
  · rw [eval_borrowed_chain]; simp [BorrowedChain, falsifyingModel, satisfyingModel]
  · rw [eval_valid_membership]; simp [ValidMembership, falsifyingModel, satisfyingModel]
  · rw [eval_no_later_alias]; simp [NoLaterAlias, falsifyingModel, satisfyingModel]
  · rw [eval_valgrind_neutral]
    simp [ValgrindNeutral, falsifyingModel, satisfyingModel]

end QCore.PremiseFormula.Libevent
