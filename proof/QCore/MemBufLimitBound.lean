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
FB-LEDGER-MEM-BUF-LIMIT-BOUND (discharges the `mem_buf_limit` part of
FB-LEDGER-ADVISORY-LIMITS; user decision of 2026-11-28: certify limit + one
write). Fluent Bit `046accf6` pauses a memory-buffered input instance only
after a write has taken `mem_chunks_size` to `mem_buf_limit` or above
(`src/flb_input_chunk.c:3081-3132`, test at `:2961`), so the strict bound
`mem_chunks_size <= mem_buf_limit` does not hold. This module models the
counter and its transitions and proves

  `mem_chunks_size <= mem_buf_limit + W` and `resident <= mem_buf_limit + W`

at every state a trace of the pinned transitions reaches, where `resident`
is `cio_stream_size_chunks_up` of the instance's stream (the value
`mem_chunks_size` snapshots) and `W` bounds `final_data_size`, the bytes one
`input_chunk_append_raw` call writes (a named premise: no site bounds it).

Modelled transitions (the census, `docs/reports/fb-ledger-mem-buf-limit-bound/CENSUS.md`):

* `append`: `input_chunk_append_raw` (`:3251-3600`), with the `memrb`
  release (`:3280-3310`), the paused gate (`:3314-3320`), each exit after the
  write (`:3490-3600`), `flb_input_chunk_set_limits` (`:3004-3077`) and
  `flb_input_chunk_protect` (`:3081-3132`);
* `release`: a chunk destroyed outside an append (flush, task destroy,
  eviction): the resident bytes fall, the snapshot is not refreshed;
* `setLimits`: `flb_task_destroy` refreshes the snapshot and may resume
  (`src/flb_task.c:939-941`).

The pause reasons other than `mem_buf_status` (storage, rate gate) only add
refusals at the gate, so the model admits every append they would admit and
more: the bound is proved for a superset of the C's admitted appends.

Premises (stated, not proved): P-MBL-ENGINE-THREAD (every append, release
and refresh of one instance runs on the engine thread, one at a time; there
is no lock), P-MBL-NO-REENTRY (nothing called inside an append writes to the
same instance's stream), P-MBL-WRITE-BOUND (`final_data_size <= W`),
P-MBL-OWN-STREAM (the stream holds only this instance's chunks),
P-MBL-PHYSICAL (the `size_t` sum does not wrap, so `Nat` models it),
P-MBL-LIMIT-SET (`mem_buf_limit > 0`; zero disables the test), for
`memory`: P-MBL-NO-SHUTDOWN-FLUSH and P-MBL-RATE-GATE-QUIET; for `memrb`:
P-MBL-FILTER-NONEXPANDING. Each excluded case has a counterexample below.
The Rust/model correspondence is maintained by review (constraint 28).
-/
namespace QCore.MemBufLimitBound

/-- `in->storage_type`. -/
inductive Store where
  | mem
  | memrb
  | fs
  deriving DecidableEq, Repr

/-- One input instance's memory state. -/
structure St where
  /-- `cio_stream_size_chunks_up(storage->stream)`: content bytes of the up chunks. -/
  resident : Nat
  /-- `in->mem_chunks_size`, assigned only from `resident`. -/
  snapshot : Nat
  /-- `in->mem_buf_status == FLB_INPUT_PAUSED`. -/
  paused : Bool
  deriving DecidableEq, Repr

/-- `flb_input.c:661-663`. -/
def init : St := ⟨0, 0, false⟩

/-- `flb_input_chunk_is_mem_overlimit` (`flb_input_chunk.c:2955-2966`). -/
def overlimit (limit snapshot : Nat) : Bool := limit != 0 && decide (limit ≤ snapshot)

/-- The memory half of `flb_input_chunk_set_limits` (`:3004-3077`):
`mem_chunks_size = total`, then resume only when not over the limit;
`resumeAllowed` stands for `is_running && is_ingestion_active`. -/
def setLimits (limit : Nat) (resumeAllowed : Bool) (s : St) : St :=
  { resident := s.resident
    snapshot := s.resident
    paused := if !overlimit limit s.resident && resumeAllowed then false else s.paused }

/-- `flb_input_chunk_protect` (`:3081-3132`). `early` is the storage-overlimit
or rate-gate return before the memory test (`:3085-3098`); a filesystem or
`memrb` instance returns before pausing. -/
def protect (store : Store) (limit : Nat) (early : Bool) (s : St) : St :=
  if early then s
  else
    match store with
    | .mem => if overlimit limit s.snapshot then { s with paused := true } else s
    | _ => s

/-- A chunk destroyed: its content leaves the stream. -/
def release (d : Nat) (s : St) : St := { s with resident := s.resident - d }

/-- How an admitted `input_chunk_append_raw` call ends. -/
inductive Exit where
  /-- `buf_size == 0`, no chunk, `cio_chunk_up_force` failure, or a failed
  write: `cio_memfs_write` leaves `buf_len` unchanged on failure. -/
  | refused
  /-- `content_size == 0` (`:3510-3514`): the empty chunk is destroyed and the
  limits refreshed. -/
  | emptied
  /-- `routable == FLB_FALSE` (`:3541-3544`): the chunk, holding the write and
  `rest` earlier bytes, is destroyed. -/
  | unroutable (rest : Nat)
  /-- The stored path (`:3547-3600`): `set_limits`, then possibly `evicted`
  bytes released by output accounting, then `protect`. -/
  | stored (evicted : Nat) (early : Bool)

/-- One call's inputs. -/
structure Append where
  /-- `buf_size`. -/
  bufSize : Nat
  /-- `final_data_size`: the bytes written after the filters. -/
  written : Nat
  /-- Content bytes `memrb_input_chunk_release_space` destroyed. -/
  freed : Nat
  /-- Whether it returned 0. -/
  freedOk : Bool
  exit : Exit
  resumeAllowed : Bool

/-- The gate (`:3314-3320`) restricted to `mem_buf_status`;
`shutdownFlush` is `is_shutting_down && (flags & FLB_INPUT_SHUTDOWN_FLUSH)`. -/
def admitted (shutdownFlush : Bool) (s : St) : Bool := !s.paused || shutdownFlush

/-- The call after the `memrb` checker. -/
def body (store : Store) (limit : Nat) (shutdownFlush : Bool) (a : Append) (s : St) : St :=
  if admitted shutdownFlush s then
    match a.exit with
    | .refused => s
    | .emptied => setLimits limit a.resumeAllowed s
    | .unroutable rest => release rest s
    | .stored evicted early =>
        protect store limit early
          (release evicted
            (setLimits limit a.resumeAllowed { s with resident := s.resident + a.written }))
  else s

/-- `input_chunk_append_raw`. -/
def append (store : Store) (limit : Nat) (shutdownFlush : Bool) (a : Append) (s : St) : St :=
  if store = .memrb ∧ overlimit limit s.snapshot = true then
    if a.freedOk then body store limit shutdownFlush a (release a.freed s)
    else release a.freed s
  else body store limit shutdownFlush a s

/-- One instance's configuration. -/
structure Cfg where
  store : Store
  /-- `mem_buf_limit`. -/
  limit : Nat
  /-- `W`: the largest `final_data_size` (premise P-MBL-WRITE-BOUND). -/
  maxWrite : Nat
  shutdownFlush : Bool

inductive Step where
  | append (a : Append)
  | release (d : Nat)
  | setLimits (resumeAllowed : Bool)

def step (c : Cfg) : Step → St → St
  | .append a, s => append c.store c.limit c.shutdownFlush a s
  | .release d, s => release d s
  | .setLimits r, s => setLimits c.limit r s

def run (c : Cfg) : List Step → St → St
  | [], s => s
  | t :: ts, s => run c ts (step c t s)

def earlyOf : Exit → Bool
  | .stored _ e => e
  | _ => false

/-- The per-step premises: P-MBL-WRITE-BOUND; for `memory`,
P-MBL-RATE-GATE-QUIET (protect reaches the memory test); for `memrb`,
P-MBL-FILTER-NONEXPANDING and the release's own success condition
(`released_space >= required_space`, with credit never above the bytes
destroyed). -/
def Admissible (c : Cfg) : Step → Prop
  | .append a =>
      a.written ≤ c.maxWrite ∧
      (c.store = .mem → earlyOf a.exit = false) ∧
      (c.store = .memrb → a.written ≤ a.bufSize ∧ (a.freedOk = true → a.bufSize ≤ a.freed))
  | _ => True

/-- The configuration premises: P-MBL-LIMIT-SET, the certified storage types,
and P-MBL-NO-SHUTDOWN-FLUSH for `memory`. -/
def Certified (c : Cfg) : Prop :=
  0 < c.limit ∧ (c.store = .mem ∧ c.shutdownFlush = false ∨ c.store = .memrb)

/-- The invariant. -/
def Inv (c : Cfg) (s : St) : Prop :=
  s.resident ≤ s.snapshot ∧ s.snapshot ≤ c.limit + c.maxWrite ∧
  (c.store = .mem → s.paused = false → s.snapshot < c.limit)

theorem overlimit_false_iff (limit snap : Nat) (h : 0 < limit) :
    overlimit limit snap = false ↔ snap < limit := by
  unfold overlimit
  have : limit ≠ 0 := by omega
  simp [this]

theorem overlimit_true_iff (limit snap : Nat) (h : 0 < limit) :
    overlimit limit snap = true ↔ limit ≤ snap := by
  unfold overlimit
  have : limit ≠ 0 := by omega
  simp [this]

theorem inv_init (c : Cfg) (hc : Certified c) : Inv c init := by
  unfold Inv init
  exact ⟨Nat.le_refl 0, Nat.zero_le _, fun _ _ => hc.1⟩

theorem release_preserves (c : Cfg) (d : Nat) (s : St) (h : Inv c s) :
    Inv c (release d s) := by
  obtain ⟨h1, h2, h3⟩ := h
  unfold Inv release
  exact ⟨by simp; omega, h2, h3⟩

theorem setLimits_preserves (c : Cfg) (r : Bool) (s : St) (hc : Certified c) (h : Inv c s) :
    Inv c (setLimits c.limit r s) := by
  obtain ⟨h1, h2, h3⟩ := h
  unfold Inv setLimits
  refine ⟨Nat.le_refl _, by simp; omega, ?_⟩
  intro hm hp
  simp only at hp ⊢
  cases ho : overlimit c.limit s.resident
  · exact (overlimit_false_iff _ _ hc.1).mp ho
  · rw [ho] at hp
    simp at hp
    have := h3 hm hp
    omega

/-- The stored path of a `memory` instance: `set_limits`, eviction and a
`protect` that reaches the memory test. -/
theorem stored_mem_preserves (c : Cfg) (a : Append) (ev : Nat) (s : St)
    (hc : Certified c) (hm : c.store = .mem) (h : Inv c s) (hp : s.paused = false)
    (hw : a.written ≤ c.maxWrite) :
    Inv c (protect c.store c.limit false
      (release ev (setLimits c.limit a.resumeAllowed { s with resident := s.resident + a.written }))) := by
  obtain ⟨h1, h2, h3⟩ := h
  have hs := h3 hm hp
  rw [hm]
  unfold protect release setLimits Inv
  simp only [hm, hp]
  cases ho : overlimit c.limit (s.resident + a.written)
  · have hlt := (overlimit_false_iff _ _ hc.1).mp ho
    simp
    omega
  · simp
    omega

/-- The stored path of a `memrb` instance (protect never pauses it). -/
theorem stored_memrb_bound (c : Cfg) (a : Append) (ev : Nat) (early : Bool) (s : St)
    (hm : c.store = .memrb) (_h1 : s.resident ≤ s.snapshot)
    (hb : s.resident + a.written ≤ c.limit + c.maxWrite) :
    Inv c (protect c.store c.limit early
      (release ev (setLimits c.limit a.resumeAllowed { s with resident := s.resident + a.written }))) := by
  rw [hm]
  unfold protect release setLimits Inv
  cases early <;> simp [hm] <;> omega

theorem append_mem_preserves (c : Cfg) (a : Append) (s : St)
    (hc : Certified c) (hm : c.store = .mem) (hf : c.shutdownFlush = false)
    (ha : Admissible c (.append a)) (h : Inv c s) :
    Inv c (append c.store c.limit c.shutdownFlush a s) := by
  obtain ⟨hw, hq, _⟩ := ha
  have hq := hq hm
  unfold append
  split
  · rename_i hh
    rw [hm] at hh
    simp at hh
  · unfold body
    split
    · rename_i had
      have hp : s.paused = false := by simpa [admitted, hf] using had
      cases hx : a.exit with
      | refused => simpa only [hx] using h
      | emptied => simpa only [hx] using setLimits_preserves c a.resumeAllowed s hc h
      | unroutable rest => simpa only [hx] using release_preserves c rest s h
      | stored ev early =>
          simp only [hx, earlyOf] at hq
          subst hq
          simpa only [hx] using stored_mem_preserves c a ev s hc hm h hp hw
    · exact h

theorem append_memrb_preserves (c : Cfg) (a : Append) (s : St)
    (hc : Certified c) (hm : c.store = .memrb)
    (ha : Admissible c (.append a)) (h : Inv c s) :
    Inv c (append c.store c.limit c.shutdownFlush a s) := by
  obtain ⟨hw, _, hr⟩ := ha
  obtain ⟨hnx, hfree⟩ := hr hm
  -- `body` from a state whose resident bytes plus the write stay in bound.
  have hbody : ∀ s', Inv c s' → s'.resident + a.written ≤ c.limit + c.maxWrite →
      Inv c (body c.store c.limit c.shutdownFlush a s') := by
    intro s' hs' hb
    unfold body
    split
    · cases hx : a.exit with
      | refused => simpa only [hx] using hs'
      | emptied => simpa only [hx] using setLimits_preserves c a.resumeAllowed s' hc hs'
      | unroutable rest => simpa only [hx] using release_preserves c rest s' hs'
      | stored ev early => simpa only [hx] using stored_memrb_bound c a ev early s' hm hs'.1 hb
    · exact hs'
  obtain ⟨h1, h2, h3⟩ := h
  unfold append
  split
  · rename_i hh
    split
    · rename_i hok
      have hfr := hfree hok
      apply hbody
      · exact release_preserves c a.freed s ⟨h1, h2, h3⟩
      · unfold release; simp; omega
    · exact release_preserves c a.freed s ⟨h1, h2, h3⟩
  · rename_i hh
    have ho : overlimit c.limit s.snapshot = false := by
      cases ho : overlimit c.limit s.snapshot
      · rfl
      · exact absurd ⟨hm, ho⟩ hh
    have hlt := (overlimit_false_iff _ _ hc.1).mp ho
    exact hbody s ⟨h1, h2, h3⟩ (by omega)

theorem step_preserves (c : Cfg) (t : Step) (s : St) (hc : Certified c)
    (ht : Admissible c t) (h : Inv c s) : Inv c (step c t s) := by
  cases t with
  | append a =>
      unfold step
      rcases hc.2 with ⟨hm, hf⟩ | hm
      · exact append_mem_preserves c a s hc hm hf ht h
      · exact append_memrb_preserves c a s hc hm ht h
  | release d => exact release_preserves c d s h
  | setLimits r => exact setLimits_preserves c r s hc h

theorem run_preserves (c : Cfg) (hc : Certified c) :
    ∀ (ts : List Step) (s : St), (∀ t ∈ ts, Admissible c t) → Inv c s → Inv c (run c ts s)
  | [], _, _, h => h
  | t :: ts, s, hts, h => by
      unfold run
      exact run_preserves c hc ts _ (fun u hu => hts u (List.mem_cons_of_mem _ hu))
        (step_preserves c t s hc (hts t List.mem_cons_self) h)

/-- **The certified bound.** Under the named premises, every state a trace
of the pinned transitions reaches from instance creation has
`mem_chunks_size <= mem_buf_limit + W` and resident bytes no larger. -/
theorem mem_buf_limit_plus_one_write (c : Cfg) (hc : Certified c) (ts : List Step)
    (hts : ∀ t ∈ ts, Admissible c t) :
    (run c ts init).snapshot ≤ c.limit + c.maxWrite ∧
    (run c ts init).resident ≤ c.limit + c.maxWrite := by
  obtain ⟨h1, h2, _⟩ := run_preserves c hc ts init hts (inv_init c hc)
  exact ⟨h2, by omega⟩

/-- A running `memory` instance is below its limit. -/
theorem running_mem_below_limit (c : Cfg) (hc : Certified c) (hm : c.store = .mem)
    (ts : List Step) (hts : ∀ t ∈ ts, Admissible c t)
    (hp : (run c ts init).paused = false) :
    (run c ts init).resident < c.limit := by
  obtain ⟨h1, _, h3⟩ := run_preserves c hc ts init hts (inv_init c hc)
  have := h3 hm hp
  omega

/-! ## Decision tables (the census rows and the Rust fixture paths) -/

/-- The gate as `input_chunk_append_raw` spells it (`:3314-3316`): refuse when
`flb_input_paused(in) && (is_shutting_down == FLB_FALSE || (flags &
FLB_INPUT_SHUTDOWN_FLUSH) == 0)`. -/
def gateC (paused shuttingDown flushFlag : Bool) : Bool :=
  !(paused && (!shuttingDown || !flushFlag))

theorem gateC_is_admitted (paused shuttingDown flushFlag : Bool) (r n : Nat) :
    gateC paused shuttingDown flushFlag = admitted (shuttingDown && flushFlag) ⟨r, n, paused⟩ := by
  cases paused <;> cases shuttingDown <;> cases flushFlag <;> rfl

/-- Rows `(paused, is_shutting_down, SHUTDOWN_FLUSH)` and whether the gate
admits: the census's gate table. The fixture's `append_entry` falls through
on exactly the admitted rows `(false, _, _)` and `(true, true, true)`
(`entry_rows`). -/
theorem gate_vectors :
    [(false, false, false), (false, false, true), (false, true, false), (false, true, true),
     (true, false, false), (true, false, true), (true, true, false), (true, true, true)].map
      (fun (p, sd, f) => gateC p sd f) =
    [true, true, true, true, false, false, false, true] := by decide

/-- Whether `flb_input_chunk_protect` sets `mem_buf_status = FLB_INPUT_PAUSED`,
in the C's order: storage overlimit (`:3085-3092`), rate gate (`:3095-3098`),
filesystem (`:3100-3102`), memory overlimit (`:3105`), `memrb` (`:3111-3113`). -/
def protectPausesMem (storageOver rateGate : Bool) (store : Store) (over : Bool) : Bool :=
  if storageOver then false
  else if rateGate then false
  else if store = .fs then false
  else if over then (if store = .memrb then false else true)
  else false

theorem protect_matches (storageOver rateGate : Bool) (store : Store) (limit : Nat) (s : St)
    (hs : s.paused = false) :
    (protect store limit (storageOver || rateGate) s).paused =
      protectPausesMem storageOver rateGate store (overlimit limit s.snapshot) := by
  unfold protect protectPausesMem
  cases storageOver <;> cases rateGate <;> cases store <;>
    cases overlimit limit s.snapshot <;> simp [hs]

/-- One row per path of the `flb_input_chunk_protect` fixture, in route T's
path order (`PROTECT_VECTORS` in `tests/unit/ledger_transition.rs`):
`(storage overlimit, rate gate fired, store, memory overlimit)` and whether
it pauses; a test the path does not reach is filled in. -/
theorem protect_vectors :
    [(true, false, Store.mem, true), (false, true, Store.mem, true),
     (false, false, Store.fs, true), (false, false, Store.memrb, true),
     (false, false, Store.mem, true), (false, false, Store.mem, false)].map
      (fun (so, rg, st, ov) => protectPausesMem so rg st ov) =
    [false, false, false, false, true, false] := by decide

/-- Whether `flb_input_chunk_set_limits` clears `mem_buf_status`
(`:3029-3035`). -/
def resumes (over running ingestionActive paused : Bool) : Bool :=
  !over && running && ingestionActive && paused

theorem setLimits_resumes (limit : Nat) (running active : Bool) (s : St) :
    (setLimits limit (running && active) s).paused =
      (s.paused && !resumes (overlimit limit s.resident) running active s.paused) := by
  unfold setLimits resumes
  cases running <;> cases active <;> cases s.paused <;>
    cases overlimit limit s.resident <;> simp

/-- The rows of the `flb_input_chunk_set_limits` fixture's paths
(`RESUME_VECTORS` in `tests/unit/ledger_transition.rs`): `(memory overlimit,
is_running, is_ingestion_active, paused)` and whether it resumes. -/
theorem resume_vectors :
    [(false, true, true, true), (true, true, true, true), (false, false, true, true),
     (false, true, false, true), (false, true, true, false)].map
      (fun (o, r, a, p) => resumes o r a p) =
    [true, false, false, false, false] := by decide

/-! ## Tightness and the excluded cases -/

/-- A stored write of `n` bytes with no filter, eviction or early return. -/
def plain (n : Nat) : Append := ⟨n, n, 0, true, .stored 0 false, true⟩

/-- The same write when `protect` returns early (the rate gate fired). -/
def gated (n : Nat) : Append := ⟨n, n, 0, true, .stored 0 true, true⟩

def memCfg : Cfg := ⟨.mem, 10, 4, false⟩

/-- The bound is reached: resident bytes `limit - 1 + W`. -/
theorem bound_reached :
    run memCfg [.append (plain 3), .append (plain 3), .append (plain 3), .append (plain 4)] init =
      ⟨13, 13, true⟩ := by decide

/-- While paused a `memory` instance refuses appends. -/
theorem paused_refuses :
    run memCfg [.append (plain 3), .append (plain 3), .append (plain 3), .append (plain 4),
      .append (plain 4)] init = ⟨13, 13, true⟩ := by decide

/-- P-MBL-RATE-GATE-QUIET is needed: when the rate gate returns before the
memory test, the instance stays running above its limit and the next write
passes `limit + W`. -/
theorem rate_gate_breaks_bound :
    (run memCfg [.append (plain 3), .append (plain 3), .append (plain 3), .append (gated 4),
      .append (gated 4)] init).resident = 17 ∧ 17 > memCfg.limit + memCfg.maxWrite := by decide

/-- P-MBL-NO-SHUTDOWN-FLUSH is needed: a paused instance flushing at shutdown
still appends. -/
theorem shutdown_flush_breaks_bound :
    (run { memCfg with shutdownFlush := true }
      [.append (plain 3), .append (plain 3), .append (plain 3), .append (plain 4),
       .append (plain 4)] init).resident = 17 ∧ 17 > memCfg.limit + memCfg.maxWrite := by decide

/-- Each further shutdown-flush append adds its bytes: no bound holds there. -/
theorem shutdown_flush_append_grows (s : St) (n : Nat) (hp : s.paused = true) :
    (append .mem 10 true (plain n) s).resident = s.resident + n := by
  unfold append body plain admitted protect release setLimits
  simp [hp]
  split <;> simp

/-- P-MBL-FILTER-NONEXPANDING is needed for `memrb`: the release frees
`buf_size` bytes, and a filter that writes more than it was given grows the
instance past `limit + W` with every write within `W`. -/
theorem memrb_expanding_filter_breaks_bound :
    (run ⟨.memrb, 10, 4, false⟩
      [.append (plain 3), .append (plain 3), .append (plain 3), .append (plain 4),
       .append ⟨1, 4, 1, true, .stored 0 false, true⟩,
       .append ⟨1, 4, 1, true, .stored 0 false, true⟩] init).resident = 19 ∧
      19 > 10 + 4 := by decide

/-- A filesystem instance is never paused by `mem_buf_limit` (`protect`
returns at `:3100-3102`): its resident bytes are governed by chunk up/down
and `max_chunks_up`, which this card does not certify. -/
theorem fs_not_paused_by_mem_buf_limit :
    (run ⟨.fs, 10, 4, false⟩
      [.append (plain 4), .append (plain 4), .append (plain 4), .append (plain 4),
       .append (plain 4)] init).resident = 20 ∧ 20 > 10 + 4 := by decide

/-- `mem_buf_limit 0` disables the test (P-MBL-LIMIT-SET). -/
theorem zero_limit_unbounded :
    (run ⟨.mem, 0, 4, false⟩
      [.append (plain 4), .append (plain 4), .append (plain 4)] init).resident = 12 ∧
      12 > 0 + 4 := by decide

end QCore.MemBufLimitBound
