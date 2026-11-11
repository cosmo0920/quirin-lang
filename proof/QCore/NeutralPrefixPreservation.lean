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

import QCore.NeutralFrameRecursion

/-!
The N-prefix preservation induction (card W4-RECURSIVE-SUMMARIES, wave 30):
the soundness argument `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` §4.3
writes for the neutral-element fixed-point start, stated compositionally over
a call tree rather than claim-locally over one run.

`QCore.NeutralFrameRecursion` already proves the *claim-local* half: the
neutral claim is prefix-closed, is independent of returning, and refuses an
effectful run, while an exit claim is satisfied vacuously by divergence. What
it does not state is the induction itself — that **per-frame** validation
under the all-neutral assumption map propagates to **every finite prefix of
the composed execution**, across completed and still-running inner calls
alike. That is the argument by which the optimistic start at `N(m)` installs
no entry assumption: a violating custody event at step `k` would be executed
by *some* frame's own straight-line instruction, every frame's body was
validated with no formal bound, and the assumption is used only about an
inner call that already completed — a strictly shorter sub-execution.

The model is a finite execution prefix of a call stack:

* `Exec.prim e rest` — one straight-line instruction of the current frame,
  performing `e` on an identity the caller holds, then the rest of the frame;
* `Exec.callCompleted sub rest` — an inner call that ran to completion,
  executing `sub`, then the rest of the frame;
* `Exec.callRunning sub` — the inner call still on the stack: the prefix is
  cut somewhere inside `sub`, and nothing of the current frame follows;
* `Exec.stop` — nothing (further) of this frame is in the prefix.

Every finite prefix of any concrete execution — returning, diverging or cut
mid-flight — is a value of this type, so a theorem quantifying over `Exec` is
a theorem about every prefix; no completion flag appears in the neutral
direction, which is exactly `neutral_claim_is_independent_of_returning`
composed over frames.

`frameNeutralLocally` is what V5/V6 validation established for one frame's
own body under the all-neutral assumption map: the frame's own instructions
are quiet, and an inner call is accepted **on the assumption alone** — its
executed sub-prefix is not inspected. The premise that a custody event on an
unresolved identity refuses rather than passes (design premise NFR-P1, the
gateway's standing fail-closed rule) is modelled by `prim` being checked
against `Event.quiet` with no exemption.

The effectful contrast is drawn at the same granularity. Under an assumed
`released` contract, per-frame exit validation constrains only a frame that
*returned* — `an_assumed_effect_appears_in_no_divergent_prefix` cuts the
circular `purge`/`mint` of the design's measured counterexample into a prefix
that every exit check accepts while the trace performs the opposite effect
and no release at all; the same prefix is refused by the neutral per-frame
check. `a_returned_prefix_discharges_the_exit_claim` is the flip side, probe
row 17 in the model: once the root frame actually returns, the exit
comparison does force the effect into the trace, so divergence is the *only*
gap — which is why an effectful candidate needs a termination rank and the
neutral candidate needs none.

Nothing here establishes termination of any C function, and no theorem below
claims an exit is concretely reachable. The trusted implementation premises
are the two existing recursive routes' own: Clang import, the complete call
inventory and CFG construction.
-/

namespace QCore.NeutralPrefixPreservation
open QCore.NeutralFrameRecursion (Event NeutralClaim)

/-- A finite prefix of one frame's execution, inner calls included. -/
inductive Exec where
  /-- Nothing (further) of this frame is in the prefix. -/
  | stop
  /-- One straight-line instruction of this frame, then the rest. -/
  | prim (e : Event) (rest : Exec)
  /-- An inner call that ran to completion, then the rest of this frame. -/
  | callCompleted (sub : Exec) (rest : Exec)
  /-- The inner call still on the stack: the prefix ends inside it. -/
  | callRunning (sub : Exec)
  deriving Repr

/-- The concrete event trace of the prefix, in execution order. -/
def events : Exec → List Event
  | .stop => []
  | .prim e rest => e :: events rest
  | .callCompleted sub rest => events sub ++ events rest
  | .callRunning sub => events sub

/-- What per-frame validation under the all-neutral assumption map inspected
    of **this** frame: its own instructions must be quiet (NFR-P1 — a custody
    event on an unresolved identity refuses, it is never ignored), while an
    inner call is accepted on the assumption `N(m)` alone, its executed
    sub-prefix deliberately not inspected. -/
def frameNeutralLocally : Exec → Bool
  | .stop => true
  | .prim e rest => (e == Event.quiet) && frameNeutralLocally rest
  | .callCompleted _ rest => frameNeutralLocally rest
  | .callRunning _ => true

/-- Every frame the prefix pushed below this one, at any depth. -/
def calledFrames : Exec → List Exec
  | .stop => []
  | .prim _ rest => calledFrames rest
  | .callCompleted sub rest => sub :: (calledFrames sub ++ calledFrames rest)
  | .callRunning sub => sub :: calledFrames sub

/-- The frames of the prefix: this one and every called one. -/
def frames (e : Exec) : List Exec := e :: calledFrames e

/-- The route's hypothesis, exactly as the descent's fixed point hands it
    over: **every** frame of the prefix — the member whose call is being
    judged and every component member it pushed — had its body validated
    locally under the all-neutral assumption map, with no formal bound. -/
def EveryFrameValidated (e : Exec) : Prop :=
  ∀ f ∈ frames e, frameNeutralLocally f = true

/-- The well-foundedness the induction runs on: a completed inner call is a
    strictly shorter sub-execution of the prefix, which is the design's
    "inner call that already completed within the first `k − 1` steps". -/
theorem a_completed_call_is_a_strictly_shorter_sub_execution (sub rest : Exec) :
    sizeOf sub < sizeOf (Exec.callCompleted sub rest) := by
  simp only [Exec.callCompleted.sizeOf_spec]
  omega

private theorem validated_root {e : Exec} (validated : EveryFrameValidated e) :
    frameNeutralLocally e = true :=
  validated e (List.Mem.head _)

private theorem validated_called {e f : Exec} (validated : EveryFrameValidated e)
    (called : f ∈ calledFrames e) : frameNeutralLocally f = true :=
  validated f (List.Mem.tail _ called)

/-- **N-prefix preservation.** If every frame of a finite execution prefix was
    validated locally under the all-neutral assumption map, the prefix's whole
    event trace is quiet: no frame performed any custody event on an identity
    its caller holds. The induction is structural on the prefix — equivalently,
    strong induction on its length — and the assumption is consumed only at
    `callCompleted`/`callRunning` nodes, each a strictly shorter sub-execution.
    No completion flag appears: the conclusion holds of a divergent or
    mid-flight prefix exactly as of a returned one, which is what lets the
    optimistic start assume nothing an execution could falsify. -/
theorem neutral_prefix_preservation :
    ∀ {e : Exec}, EveryFrameValidated e →
      (events e).all (· == Event.quiet) = true := by
  intro e
  induction e with
  | stop => intro _; rfl
  | prim ev rest ih =>
      intro validated
      have root := validated_root validated
      simp only [frameNeutralLocally, Bool.and_eq_true] at root
      have restValidated : EveryFrameValidated rest := by
        intro f mem
        cases List.mem_cons.mp mem with
        | inl eq => exact eq ▸ root.2
        | inr called => exact validated_called validated called
      simp only [events, List.all_cons, Bool.and_eq_true]
      exact ⟨root.1, ih restValidated⟩
  | callCompleted sub rest ihSub ihRest =>
      intro validated
      have root := validated_root validated
      have subValidated : EveryFrameValidated sub := by
        intro f mem
        cases List.mem_cons.mp mem with
        | inl eq =>
            exact eq ▸ validated_called validated (List.Mem.head _)
        | inr called =>
            exact validated_called validated
              (List.Mem.tail _ (List.mem_append.mpr (Or.inl called)))
      have restValidated : EveryFrameValidated rest := by
        intro f mem
        cases List.mem_cons.mp mem with
        | inl eq => exact eq ▸ root
        | inr called =>
            exact validated_called validated
              (List.Mem.tail _ (List.mem_append.mpr (Or.inr called)))
      simp only [events, List.all_append, Bool.and_eq_true]
      exact ⟨ihSub subValidated, ihRest restValidated⟩
  | callRunning sub ih =>
      intro validated
      have subValidated : EveryFrameValidated sub := by
        intro f mem
        cases List.mem_cons.mp mem with
        | inl eq => exact eq ▸ validated_called validated (List.Mem.head _)
        | inr called =>
            exact validated_called validated (List.Mem.tail _ called)
      simpa only [events] using ih subValidated

/-- The bridge to the claim the run-level model states: a prefix whose every
    frame validated satisfies `NeutralClaim` whether or not the call returns —
    the composed form of `neutral_claim_is_independent_of_returning`. -/
theorem validated_frames_satisfy_the_neutral_claim {e : Exec}
    (validated : EveryFrameValidated e) (returns : Bool) :
    NeutralClaim { events := events e, returns := returns } = true := by
  simpa only [NeutralClaim] using neutral_prefix_preservation validated

/-- Non-vacuity: a prefix with a non-empty trace whose every frame validates —
    a quiet instruction beside a completed quiet inner call. -/
theorem a_validated_prefix_exists :
    ∃ e : Exec, EveryFrameValidated e ∧ events e ≠ [] := by
  refine ⟨Exec.prim Event.quiet (Exec.callCompleted Exec.stop Exec.stop), ?_, by decide⟩
  intro f mem
  simp only [frames, calledFrames, List.mem_cons] at mem
  rcases mem with rfl | rfl | mem
  · rfl
  · rfl
  · cases mem

/-! ## The effectful contrast, at the same granularity

Under an assumed `released` contract for the component, per-frame exit
validation obliges only a frame that **returned**: its executed body must
contain the release itself or a completed inner call (discharged by the
assumption). A still-running frame is unconstrained, because an exit claim
says nothing about a call that has not returned — and that asymmetry, not the
exit comparison, is the hole the design's §4.3 counterexample measures. -/

/-- What discharges one returned frame's exit obligation locally: its own
    release instruction, or an inner call that completed — the assumption. -/
def frameDischargesLocally : Exec → Bool
  | .stop => false
  | .prim e rest => (e == Event.release) || frameDischargesLocally rest
  | .callCompleted _ _ => true
  | .callRunning _ => false

/-- Every **called** frame of the prefix passes its own exit check: a
    completed call's frame is obliged to discharge, a running call's is not. -/
def subsExitValidated : Exec → Bool
  | .stop => true
  | .prim _ rest => subsExitValidated rest
  | .callCompleted sub rest =>
      (frameDischargesLocally sub && subsExitValidated sub) && subsExitValidated rest
  | .callRunning sub => subsExitValidated sub

/-- Per-frame exit validation of the whole prefix, `returned` saying whether
    the root frame itself returned within it. -/
def exitValidated (returned : Bool) (e : Exec) : Bool :=
  (!returned || frameDischargesLocally e) && subsExitValidated e

/-- **The counterexample, compositionally.** The design's circular
    `purge`/`mint`, cut at a finite prefix: the root performs the *opposite*
    effect and defers the assumed release to an inner call that never returns.
    Every frame passes the effectful exit validation, yet no release occurs
    anywhere in the trace — and the same prefix is refused by the neutral
    per-frame check, which is the whole asymmetry: validation that assumes an
    effect constrains no divergent prefix, validation that assumes the neutral
    element constrains every prefix. -/
theorem an_assumed_effect_appears_in_no_divergent_prefix :
    ∃ e : Exec,
      exitValidated false e = true ∧
        Event.release ∉ events e ∧
        events e ≠ [] ∧
        ¬ EveryFrameValidated e := by
  refine ⟨Exec.prim Event.acquire (Exec.callRunning Exec.stop),
    rfl, by decide, by decide, ?_⟩
  intro validated
  have root := validated_root validated
  simp [frameNeutralLocally] at root

/-- **Probe row 17, in the model.** Once the root frame really returns, the
    per-frame exit checks do force a release into the trace: the deferral
    chain runs through completed calls only, each a strictly shorter
    sub-execution, and ends at a frame whose own instruction releases. The
    exit comparison is not the weak point; divergence is its only gap. -/
theorem a_returned_prefix_discharges_the_exit_claim :
    ∀ {e : Exec}, exitValidated true e = true → Event.release ∈ events e := by
  have core :
      ∀ {e : Exec}, frameDischargesLocally e = true → subsExitValidated e = true →
        Event.release ∈ events e := by
    intro e
    induction e with
    | stop => intro discharge _; cases discharge
    | prim ev rest ih =>
        intro discharge subs
        cases own : ev == Event.release with
        | true =>
            have : ev = Event.release := by simpa using own
            exact this ▸ List.Mem.head _
        | false =>
            have deferred : frameDischargesLocally rest = true := by
              simpa [frameDischargesLocally, own] using discharge
            exact List.Mem.tail _ (ih deferred subs)
    | callCompleted sub rest ihSub _ =>
        intro _ subs
        simp only [subsExitValidated, Bool.and_eq_true] at subs
        exact List.mem_append.mpr (Or.inl (ihSub subs.1.1 subs.1.2))
    | callRunning sub _ =>
        intro discharge _
        cases discharge
  intro e validated
  have split := (Bool.and_eq_true _ _).mp validated
  have discharge : frameDischargesLocally e = true := by
    simpa using split.1
  exact core discharge split.2

end QCore.NeutralPrefixPreservation
