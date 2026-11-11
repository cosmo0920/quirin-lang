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

import QCore.Concurrency

/-!
# F01 — one checker/model correspondence boundary

The Lean side of the correspondence between the generation-indexed lifetime
relation of the Rust checker `qcore_core::concurrency::check_concurrent` and its
model twin `QCore.Concurrency.Lifetime.step` / `QCore.Concurrency.Lifetime.run`.

`Lifetime` itself is **unchanged**: this module only declares a finite domain
over it and pins, by kernel evaluation, exactly what the Rust harness
`crates/qcore-core/tests/lifetime_correspondence.rs` recomputes on its side.

What this file establishes:

* `domain_size`, `domain_trace_digest` — the enumerated domain is the one the
  Rust harness enumerates (agreement is *checked*, not assumed).
* `domain_outcome_digest`, `domain_accepted_trace_count` — `Lifetime.run`'s
  verdict *and* final state on every trace of that domain, so the harness's Rust
  transcription of `step`/`run` is pinned against kernel evaluation rather than
  trusted.
* `domain_refuses_every_terminal_participant` — the selected model lemma, over
  the whole domain: no alphabet event with a terminal participant is admitted in
  any reachable state.
* `domain_terminal_reaching_trace_count`,
  `domain_terminal_participant_probe_count` — non-vacuity of that lemma.

What this file does **not** establish: that a Lean theorem proves the Rust
implementation correct. `Lifetime` is a projection: it carries no vector clock,
no mutex identity, no permission share and no footprint, so it is strictly
coarser than `check_concurrent` and the harness measures where.
-/

namespace QCore.Concurrency.LifetimeCorrespondence

open QCore.Concurrency.Lifetime

-- Every theorem below is closed by kernel evaluation over 1885 traces, which
-- needs more elaborator recursion and heartbeats than the defaults allow.
-- Neither option changes what is checked: `decide` still produces
-- `of_decide_eq_true rfl` and the kernel still replays the whole computation.
set_option maxRecDepth 100000
set_option maxHeartbeats 4000000

/-! ## The declared finite domain -/

/-- The declared event alphabet. Mirrored by `model_alphabet` in the Rust
harness, in this order. -/
def alphabet : List Event :=
  [ .spawn 0 1, .spawn 1 2, .join 0 1, .join 1 2,
    .lock 0, .lock 1, .unlock 0, .unlock 1,
    .access 1 7, .transfer 0 1 7 true,
    .destroy 0 7 true, .destroy 0 7 false ]

/-- The abstraction of the checker's own initial configuration: task `0` alone
is known, resource `7` starts in task `0`'s custody, and tasks `0` and `1` each
hold a positive share of it. Mirrored by `model_initial` in the Rust harness. -/
def domainInitial : State :=
  { knownTasks := [0]
    terminalTasks := []
    destroyedResources := []
    parents := []
    custody := [(7, 0)]
    permissionHolders := [(7, 0), (7, 1)]
    guardHolders := [] }

def extend (traces : List (List Event)) : List (List Event) :=
  traces.foldr (fun trace rest => alphabet.map (fun event => trace ++ [event]) ++ rest) []

def levels : Nat → List (List Event)
  | 0 => [[]]
  | n + 1 => extend (levels n)

/-- Every trace of length at most three over `alphabet`, shortest first. -/
def domainTraces : List (List Event) :=
  levels 0 ++ levels 1 ++ levels 2 ++ levels 3

/-! ## Digests

Plain FNV-style folds over `Nat`, reduced modulo `2 ^ 64` after every step so
that the Rust harness reproduces them exactly with wrapping `u64` arithmetic.
A digest is a comparison device, not a proof device: two different domains or
two different transcriptions agreeing on a digest is a collision, and the report
records that as the one residual encoding premise. -/

def modulus : Nat := 18446744073709551616
def traceSeed : Nat := 1469598103934665603
def traceMul : Nat := 1000003
def domainSeed : Nat := 14695981039346656037
def domainMul : Nat := 1099511628211

def mix (accumulator value : Nat) : Nat :=
  (accumulator * traceMul + value + 1) % modulus

/-- Injective for task identifiers below `8` and resource identifiers below
`64`, which the declared alphabet respects. -/
def encodeEvent : Event → Nat
  | .access task resource => 1 + 16 * (task + 8 * resource)
  | .transfer sender to resource synchronized =>
      2 + 16 * (sender + 8 * (to + 8 * (resource + 64 * (if synchronized then 1 else 0))))
  | .destroy task resource ordered =>
      3 + 16 * (task + 8 * (resource + 64 * (if ordered then 1 else 0)))
  | .spawn parent child => 4 + 16 * (parent + 8 * child)
  | .join parent child => 5 + 16 * (parent + 8 * child)
  | .lock task => 6 + 16 * task
  | .unlock task => 7 + 16 * task
  | .taskAction task => 8 + 16 * task

def mixTrace (events : List Event) : Nat :=
  events.foldl (fun accumulator event => mix accumulator (encodeEvent event)) traceSeed

def traceDigest (traces : List (List Event)) : Nat :=
  traces.foldl (fun accumulator trace =>
    (accumulator * domainMul + mixTrace trace + 1) % modulus) domainSeed

def mixNats (seed : Nat) (values : List Nat) : Nat :=
  values.foldl (fun accumulator value => mix accumulator value) (mix seed values.length)

def mixPairs (seed : Nat) (values : List (Nat × Nat)) : Nat :=
  values.foldl (fun accumulator pair => mix accumulator (pair.1 * 64 + pair.2))
    (mix seed values.length)

def encodeState (state : State) : Nat :=
  mixNats
    (mixPairs
      (mixPairs
        (mixPairs
          (mixNats (mixNats (mixNats traceSeed state.knownTasks) state.terminalTasks)
            state.destroyedResources)
          state.parents)
        state.custody)
      state.permissionHolders)
    state.guardHolders

def encodeOutcome : Option State → Nat
  | none => 0
  | some state => (encodeState state + 1) % modulus

def outcomeDigest (traces : List (List Event)) : Nat :=
  traces.foldl (fun accumulator trace =>
    (accumulator * domainMul + encodeOutcome (run domainInitial trace) + 1) % modulus) domainSeed

/-! ## Domain agreement -/

theorem domain_size : domainTraces.length = 1885 := by decide

theorem domain_trace_digest : traceDigest domainTraces = 16753757158032361952 := by decide

/-! ## Transcription agreement

`outcomeDigest` folds `run domainInitial` over the whole domain together with
every field of each reached state, in the model's own list order. The Rust
harness recomputes the identical number from its transcription of `step`/`run`,
so a transcription that differed anywhere in the domain would be caught here
rather than assumed away. -/

def accepted (trace : List Event) : Bool := (run domainInitial trace).isSome

theorem domain_outcome_digest : outcomeDigest domainTraces = 12252691954508661170 := by decide

theorem domain_accepted_trace_count : domainTraces.countP accepted = 53 := by decide

/-! ## The selected model lemma

Every task named by an alphabet event is a *participant*. In every state the
domain can reach, an event with a terminal participant is refused — for `spawn`
because a terminal child is still a known task, and for every other constructor
because `State.Active` fails. This is the finite-domain, whole-alphabet
instantiation of the per-constructor theorems in `QCore.Concurrency.Lifetime`
(`terminal_task_rejects_lock`, `terminal_child_rejects_join`, and the rest). -/

def participants : Event → List TaskId
  | .access task _ => [task]
  | .transfer sender to _ _ => [sender, to]
  | .destroy task _ _ => [task]
  | .spawn parent child => [parent, child]
  | .join parent child => [parent, child]
  | .lock task => [task]
  | .unlock task => [task]
  | .taskAction task => [task]

def hasTerminalParticipant (state : State) (event : Event) : Bool :=
  (participants event).any (fun task => state.terminalTasks.contains task)

def refusesTerminalParticipants (trace : List Event) : Bool :=
  match run domainInitial trace with
  | none => true
  | some state =>
      alphabet.all fun event =>
        !hasTerminalParticipant state event || (step state event).isNone

theorem domain_refuses_every_terminal_participant :
    domainTraces.all refusesTerminalParticipants = true := by decide

/-! ## Non-vacuity

The lemma above is trivially true on a domain that never reaches a terminal
task, so both counts are pinned and both are recomputed by the Rust harness. -/

def reachesTerminal (trace : List Event) : Bool :=
  match run domainInitial trace with
  | none => false
  | some state => !state.terminalTasks.isEmpty

def terminalParticipantProbes (trace : List Event) : Nat :=
  match run domainInitial trace with
  | none => 0
  | some state => (alphabet.filter (hasTerminalParticipant state)).length

theorem domain_terminal_reaching_trace_count :
    domainTraces.countP reachesTerminal = 6 := by decide

theorem domain_terminal_participant_probe_count :
    (domainTraces.map terminalParticipantProbes).foldl (· + ·) 0 = 42 := by decide

/-- The probe count above is positive, so
`domain_refuses_every_terminal_participant` discharges real refusals. -/
theorem domain_terminal_participant_probes_are_nonvacuous :
    0 < (domainTraces.map terminalParticipantProbes).foldl (· + ·) 0 := by
  rw [domain_terminal_participant_probe_count]
  decide

end QCore.Concurrency.LifetimeCorrespondence
