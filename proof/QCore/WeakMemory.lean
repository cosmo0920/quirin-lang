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
# The C11 weak-memory execution checker, atomic fragment (card CG28-WEAK-MEMORY-EVENTS)

The Rust side is `crates/qcore-core/src/weak_memory.rs` (`check_c11_execution`).
This module is the Lean model of its atomic fragment: plain loads and stores,
read-modify-writes, compare-exchange, thread and signal fences, release
sequences, the seq_cst order S and non-atomic races. Mutex histories and
thread joins (Cycle 17, constraint 27) are outside this module; a spawn
(card CG29-EXECUTION-COVERAGE-AND-RECLAMATION, additive) happens before the
first event of its child thread. The shared litmus vectors use neither.

**One standard.** C11 as published (N1570), not C++20's weakened release
sequences, not RC11, and not C++14's added seq_cst-fence/modification-order
rule. `memory_order_consume` is out of scope: there is no consume order here
(the source route refuses it, `QCore.AtomicSourceRoute.consume_order_refused`).

What is proved:

* `check` accepts only executions whose read-modify-writes read their
  immediate modification-order predecessor (`accepted_rmw_is_atomic`);
* a compare-exchange's failure order is neither release nor acq_rel and no
  stronger than its success order (`cas_failure_no_stronger`), a strong one
  fails only on a differing value and a weak one may fail spuriously
  (`strong_failure_observes_another_value`, `weak_failure_may_be_spurious`);
* the release sequence is N1570 5.1.2.4p10's definition
  (`release_sequence_definition`): an RMW of any thread continues it, a
  store of another thread ends it;
* the four synchronises-with rules (5.1.2.4p11 and 7.17.4p2–p4) each produce
  their edge (`release_acquire_synchronises`, `fence_fence_synchronises`,
  `fence_operation_synchronises`, `operation_fence_synchronises`), an
  accepted graph's supplied edges are exactly these
  (`accepted_sw_is_expected`, `accepted_expected_is_supplied`), and a signal
  fence is no thread fence (`signal_fence_is_not_a_thread_fence`);
* the shared litmus vectors (message passing, store buffering, with and
  without fences, RMW atomicity, release sequences, the compare-exchange
  forms) have the categories listed in `litmusVectors`
  (`litmus_vectors_hold`), which the Rust test
  `crates/qcore-core/tests/unit/weak_memory_cg28.rs` parses from this file
  and runs through `check_c11_execution`.

The Rust-to-Lean correspondence of the checker is a stated review premise;
the vectors bind the two on every listed graph.
-/

namespace QCore.WeakMemory

inductive Order where
  | relaxed | acquire | release | acqRel | seqCst
deriving DecidableEq, Repr

def Order.hasAcquire : Order → Bool
  | .acquire | .acqRel | .seqCst => true
  | _ => false

def Order.hasRelease : Order → Bool
  | .release | .acqRel | .seqCst => true
  | _ => false

/-- Acquire strength of a load or failure order (`none`: not a load order). -/
def Order.loadRank : Order → Option Nat
  | .relaxed => some 0
  | .acquire => some 1
  | .seqCst => some 2
  | _ => none

/-- Acquire strength of a success order's load part: release reads as
relaxed, acq_rel as acquire. -/
def Order.successRank : Order → Nat
  | .relaxed | .release => 0
  | .acquire | .acqRel => 1
  | .seqCst => 2

/-- `compare_exchange_failure_admitted`: C11 7.17.7.4p2. -/
def casOrdersValid (success failure : Order) : Bool :=
  match failure.loadRank with
  | none => false
  | some rank => decide (rank ≤ success.successRank)

inductive Kind where
  | init (loc v : Nat)
  | store (loc v : Nat) (o : Order)
  | load (loc v : Nat) (o : Order)
  | rmw (loc read written : Nat) (o : Order)
  | cas (loc expected observed desired : Nat) (success failure : Order) (weak succeeded : Bool)
  | fence (o : Order) (signal : Bool)
  | naRead (loc : Nat)
  | naWrite (loc : Nat)
  /-- Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (additive): the Rust
  `MemoryEventKind::Spawn`. It happens before the first event of its child
  thread, so the initialisations of a thread-0 prelude happen before every
  thread, as in `spawn_join_edges`. -/
  | spawn (child : Nat)
deriving DecidableEq, Repr

structure Event where
  id : Nat
  thread : Nat
  kind : Kind
deriving DecidableEq, Repr

def ev (id thread : Nat) (kind : Kind) : Event := ⟨id, thread, kind⟩

structure Execution where
  events : List Event
  rf : List (Nat × Nat)
  mo : List (Nat × Nat)
  sw : List (Nat × Nat)
  sc : List Nat
deriving Repr

/-! ## Event projections (`atomic_write`, `atomic_load`, `operation_order`) -/

def Kind.orderValid : Kind → Bool
  | .store _ _ o => o == .relaxed || o == .release || o == .seqCst
  | .load _ _ o => o == .relaxed || o == .acquire || o == .seqCst
  | .cas _ _ _ _ s f _ _ => casOrdersValid s f
  | _ => true

/-- The write an event performs: location, value, order (`none`: the
non-atomic initialisation). -/
def Kind.write? : Kind → Option (Nat × Nat × Option Order)
  | .init l v => some (l, v, none)
  | .store l v o => some (l, v, some o)
  | .rmw l _ w o => some (l, w, some o)
  | .cas l _ _ d s _ _ true => some (l, d, some s)
  | _ => none

/-- The read an event performs: location, value, order. -/
def Kind.read? : Kind → Option (Nat × Nat × Order)
  | .load l v o => some (l, v, o)
  | .rmw l r _ o => some (l, r, o)
  | .cas l _ obs _ s f _ succeeded => some (l, obs, if succeeded then s else f)
  | _ => none

def Kind.opOrder? : Kind → Option Order
  | .store _ _ o | .load _ _ o | .rmw _ _ _ o => some o
  | .cas _ _ _ _ s f _ succeeded => some (if succeeded then s else f)
  | _ => none

def Kind.isRmw : Kind → Bool
  | .rmw .. => true
  | .cas _ _ _ _ _ _ _ true => true
  | _ => false

/-- An atomic operation's modification (not the initialisation). -/
def Kind.opWriteLoc? : Kind → Option Nat
  | .init .. => none
  | k => k.write?.map (·.1)

/-- A thread fence's order; a signal fence is none (7.17.4.2). -/
def Kind.threadFence? : Kind → Option Order
  | .fence o false => some o
  | _ => none

def Kind.nonAtomic? : Kind → Option (Nat × Bool)
  | .naRead l => some (l, false)
  | .naWrite l => some (l, true)
  | _ => none

/-! ## Relations -/

def Execution.lookup (x : Execution) (id : Nat) : Option Event :=
  x.events.find? (·.id == id)

def Execution.kindOf (x : Execution) (id : Nat) : Option Kind :=
  (x.lookup id).map (·.kind)

def Execution.ids (x : Execution) : List Nat := x.events.map (·.id)

def Execution.pairs (x : Execution) : List (Nat × Nat) :=
  x.ids.flatMap fun a => x.ids.map fun b => (a, b)

def Execution.index (x : Execution) (id : Nat) : Option Nat :=
  x.events.findIdx? (·.id == id)

/-- Sequenced-before: the same thread, earlier in the list. -/
def Execution.sb (x : Execution) (a b : Nat) : Bool :=
  match x.lookup a, x.lookup b, x.index a, x.index b with
  | some ea, some eb, some i, some j => ea.thread == eb.thread && decide (i < j)
  | _, _, _, _ => false

def compose (left right : List (Nat × Nat)) : List (Nat × Nat) :=
  left.flatMap fun (a, b) => right.filterMap fun (c, d) => if b == c then some (a, d) else none

def dedup : List (Nat × Nat) → List (Nat × Nat)
  | [] => []
  | e :: rest => if rest.contains e then dedup rest else e :: dedup rest

def grow (base current : List (Nat × Nat)) : List (Nat × Nat) :=
  dedup (current ++ compose current base)

/-- Paths of at most `n + 1` edges. -/
def closureN (base : List (Nat × Nat)) : Nat → List (Nat × Nat)
  | 0 => base
  | n + 1 => grow base (closureN base n)

def Execution.closure (x : Execution) (base : List (Nat × Nat)) : List (Nat × Nat) :=
  closureN base x.events.length

def Execution.moPlus (x : Execution) : List (Nat × Nat) := x.closure x.mo

/-- Spawn edges (`spawn_join_edges`, card CG29): a spawn happens before the
first event, in list order, of its child thread. -/
def Execution.spawnEdges (x : Execution) : List (Nat × Nat) :=
  x.events.flatMap fun e =>
    match e.kind with
    | .spawn c =>
      match x.events.find? (·.thread == c) with
      | some f => [(e.id, f.id)]
      | none => []
    | _ => []

def Execution.hb (x : Execution) : List (Nat × Nat) :=
  x.closure (x.pairs.filter (fun p => x.sb p.1 p.2) ++ x.sw ++ x.spawnEdges)

/-! ## Release sequences (N1570 5.1.2.4p10) -/

/-- Whether `c` continues a release sequence headed by `head`: an atomic
read-modify-write of any thread, or an atomic modification by the head's
thread. -/
def continues (head c : Event) : Bool :=
  c.kind.isRmw || (c.thread == head.thread && c.kind.opWriteLoc?.isSome)

/-- `in_release_sequence`: `w` is the head, or follows it in the
modification order with it and every modification between them continuing
the sequence. Used for real and for 7.17.4's hypothetical sequences. -/
def inReleaseSequence (x : Execution) (head : Event) (w : Nat) : Bool :=
  w == head.id ||
    ((x.moPlus).contains (head.id, w) &&
      x.events.all fun c =>
        !(c.id == w || ((x.moPlus).contains (head.id, c.id) && (x.moPlus).contains (c.id, w)))
          || continues head c)

theorem release_sequence_definition (x : Execution) (head : Event) (w : Nat) :
    inReleaseSequence x head w = true ↔
      (w = head.id ∨
        ((head.id, w) ∈ x.moPlus ∧
          ∀ c ∈ x.events,
            (c.id = w ∨ ((head.id, c.id) ∈ x.moPlus ∧ (c.id, w) ∈ x.moPlus)) →
              continues head c = true)) := by
  simp only [inReleaseSequence, Bool.or_eq_true, beq_iff_eq, Bool.and_eq_true,
    List.contains_iff_mem, List.all_eq_true, Bool.not_eq_true']
  constructor
  · rintro (h | ⟨hmo, hall⟩)
    · exact .inl h
    · refine .inr ⟨hmo, fun c hc hbetween => ?_⟩
      rcases hall c hc with hnot | hcont
      · exfalso
        rcases hbetween with heq | ⟨h1, h2⟩
        · simp [heq] at hnot
        · simp [h1, h2] at hnot
      · exact hcont
  · rintro (h | ⟨hmo, hall⟩)
    · exact .inl h
    · refine .inr ⟨hmo, fun c hc => ?_⟩
      by_cases hb : (c.id = w ∨ ((head.id, c.id) ∈ x.moPlus ∧ (c.id, w) ∈ x.moPlus))
      · exact .inr (hall c hc hb)
      · left
        simp only [not_or, not_and] at hb
        simp only [Bool.or_eq_false_iff, beq_eq_false_iff_ne, Bool.and_eq_false_iff]
        refine ⟨hb.1, ?_⟩
        by_cases h1 : (head.id, c.id) ∈ x.moPlus
        · exact .inr (by simpa using hb.2 h1)
        · exact .inl (by simpa using h1)

theorem rmw_continues_release_sequence (head c : Event) (h : c.kind.isRmw = true) :
    continues head c = true := by
  simp [continues, h]

theorem foreign_store_ends_release_sequence (head c : Event) (l v : Nat) (o : Order)
    (hk : c.kind = .store l v o) (ht : c.thread ≠ head.thread) :
    continues head c = false := by
  simp [continues, hk, Kind.isRmw, ht]

/-! ## Synchronises-with (5.1.2.4p11, 7.17.4p2–p4) -/

def Execution.releaseFence (x : Execution) (id : Nat) : Bool :=
  match x.kindOf id with
  | some k => (k.threadFence?.map Order.hasRelease).getD false
  | none => false

def Execution.acquireFence (x : Execution) (id : Nat) : Bool :=
  match x.kindOf id with
  | some k => (k.threadFence?.map Order.hasAcquire).getD false
  | none => false

/-- Whether the read `r`, of order `ro`, ends at `b`: `b` is `r` itself and
`r` acquires, or `b` is an acquire fence `r` is sequenced before. -/
def Execution.acquiresAt (x : Execution) (r : Nat) (ro : Order) (b : Nat) : Bool :=
  (b == r && ro.hasAcquire) || (x.acquireFence b && x.sb r b)

/-- `a` sw `b` through the read `(w, r)` of the modification `head`
(`expected_synchronizes_with`). -/
def Execution.swVia (x : Execution) (a b w r : Nat) (head : Event) : Bool :=
  match x.kindOf r >>= Kind.read? with
  | none => false
  | some (loc, _, ro) =>
    head.kind.opWriteLoc? == some loc && inReleaseSequence x head w &&
      x.acquiresAt r ro b &&
      ((head.id == a && (head.kind.opOrder?.map Order.hasRelease).getD false) ||
        (x.releaseFence a && x.sb a head.id))

def Execution.swEdge (x : Execution) (a b : Nat) : Bool :=
  a != b && x.rf.any fun (w, r) => x.events.any fun head => x.swVia a b w r head

theorem release_acquire_synchronises (x : Execution) (head : Event) (w r loc v : Nat)
    (ro : Order) (hhead : head ∈ x.events) (hrf : (w, r) ∈ x.rf)
    (hread : x.kindOf r >>= Kind.read? = some (loc, v, ro)) (hloc : head.kind.opWriteLoc? = some loc)
    (hrs : inReleaseSequence x head w = true)
    (hrel : (head.kind.opOrder?.map Order.hasRelease).getD false = true)
    (hacq : ro.hasAcquire = true) (hne : head.id ≠ r) :
    x.swEdge head.id r = true := by
  simp only [Execution.swEdge, Bool.and_eq_true, bne_iff_ne, ne_eq, List.any_eq_true]
  refine ⟨hne, (w, r), hrf, head, hhead, ?_⟩
  simp [Execution.swVia, hread, hloc, hrs, Execution.acquiresAt, hacq, hrel]

/-- 7.17.4p2: a release fence `f` sequenced before the modification `head`,
whose hypothetical release sequence holds `w`, read by `r` sequenced before
the acquire fence `b`. -/
theorem fence_fence_synchronises (x : Execution) (head : Event) (f b w r loc v : Nat)
    (ro : Order) (hhead : head ∈ x.events) (hrf : (w, r) ∈ x.rf)
    (hread : x.kindOf r >>= Kind.read? = some (loc, v, ro)) (hloc : head.kind.opWriteLoc? = some loc)
    (hrs : inReleaseSequence x head w = true)
    (hf : x.releaseFence f = true) (hfx : x.sb f head.id = true)
    (hb : x.acquireFence b = true) (hrb : x.sb r b = true) (hne : f ≠ b) :
    x.swEdge f b = true := by
  simp only [Execution.swEdge, Bool.and_eq_true, bne_iff_ne, ne_eq, List.any_eq_true]
  refine ⟨hne, (w, r), hrf, head, hhead, ?_⟩
  simp [Execution.swVia, hread, hloc, hrs, Execution.acquiresAt, hb, hrb, hf, hfx]

/-- 7.17.4p3: a release fence before the modification, an acquire read. -/
theorem fence_operation_synchronises (x : Execution) (head : Event) (f w r loc v : Nat)
    (ro : Order) (hhead : head ∈ x.events) (hrf : (w, r) ∈ x.rf)
    (hread : x.kindOf r >>= Kind.read? = some (loc, v, ro)) (hloc : head.kind.opWriteLoc? = some loc)
    (hrs : inReleaseSequence x head w = true)
    (hf : x.releaseFence f = true) (hfx : x.sb f head.id = true)
    (hacq : ro.hasAcquire = true) (hne : f ≠ r) :
    x.swEdge f r = true := by
  simp only [Execution.swEdge, Bool.and_eq_true, bne_iff_ne, ne_eq, List.any_eq_true]
  refine ⟨hne, (w, r), hrf, head, hhead, ?_⟩
  simp [Execution.swVia, hread, hloc, hrs, Execution.acquiresAt, hacq, hf, hfx]

/-- 7.17.4p4: a release modification, a read sequenced before an acquire
fence. -/
theorem operation_fence_synchronises (x : Execution) (head : Event) (b w r loc v : Nat)
    (ro : Order) (hhead : head ∈ x.events) (hrf : (w, r) ∈ x.rf)
    (hread : x.kindOf r >>= Kind.read? = some (loc, v, ro)) (hloc : head.kind.opWriteLoc? = some loc)
    (hrs : inReleaseSequence x head w = true)
    (hrel : (head.kind.opOrder?.map Order.hasRelease).getD false = true)
    (hb : x.acquireFence b = true) (hrb : x.sb r b = true) (hne : head.id ≠ b) :
    x.swEdge head.id b = true := by
  simp only [Execution.swEdge, Bool.and_eq_true, bne_iff_ne, ne_eq, List.any_eq_true]
  refine ⟨hne, (w, r), hrf, head, hhead, ?_⟩
  simp [Execution.swVia, hread, hloc, hrs, Execution.acquiresAt, hb, hrb, hrel]

theorem signal_fence_is_not_a_thread_fence (o : Order) :
    (Kind.fence o true).threadFence? = none := rfl

theorem signal_fence_neither_releases_nor_acquires (x : Execution) (id : Nat) (o : Order)
    (h : x.kindOf id = some (.fence o true)) :
    x.releaseFence id = false ∧ x.acquireFence id = false := by
  simp [Execution.releaseFence, Execution.acquireFence, h, Kind.threadFence?]

/-! ## The component checks -/

def Execution.ordersValid (x : Execution) : Bool := x.events.all (·.kind.orderValid)

def Execution.rfValid (x : Execution) : Bool :=
  (x.rf.all fun (w, r) =>
    match x.kindOf w >>= Kind.write?, x.kindOf r >>= Kind.read? with
    | some (wl, wv, _), some (rl, rv, _) => wl == rl && wv == rv
    | _, _ => false) &&
  (x.events.all fun e => e.kind.read?.isNone || (x.rf.filter (·.2 == e.id)).length == 1)

def Execution.writesAt (x : Execution) (loc : Nat) : List Nat :=
  (x.events.filter fun e => (e.kind.write?.map (·.1)) == some loc).map (·.id)

def Execution.moValid (x : Execution) : Bool :=
  (x.mo.all fun (a, b) =>
    match x.kindOf a >>= Kind.write?, x.kindOf b >>= Kind.write? with
    | some (la, _, _), some (lb, _, _) => la == lb
    | _, _ => false) &&
  (x.events.all fun e =>
    match e.kind.write? with
    | none => true
    | some (loc, _, _) =>
      (x.writesAt loc).all fun other =>
        other == e.id || (x.moPlus).contains (e.id, other) || (x.moPlus).contains (other, e.id))

/-- A strong compare-exchange fails only on a differing value; a weak one may
fail spuriously; a success observed the expected value. -/
def Kind.casConsistent : Kind → Bool
  | .cas _ e obs _ _ _ weak succeeded => if succeeded then obs == e else weak || obs != e
  | _ => true

def Execution.casValid (x : Execution) : Bool := x.events.all (·.kind.casConsistent)

/-- The write `w` an RMW `r` reads immediately precedes `r` in the
modification order. -/
def Execution.adjacent (x : Execution) (w r : Nat) : Bool :=
  (x.moPlus).contains (w, r) &&
    !(x.events.any fun c => (x.moPlus).contains (w, c.id) && (x.moPlus).contains (c.id, r))

def Execution.atomicityHolds (x : Execution) : Bool :=
  x.rf.all fun (w, r) => !((x.kindOf r).map Kind.isRmw).getD false || x.adjacent w r

def Execution.swValid (x : Execution) : Bool :=
  (x.sw.all fun (a, b) => x.swEdge a b) &&
    (x.pairs.all fun (a, b) => !x.swEdge a b || x.sw.contains (a, b))

def Execution.acyclic (x : Execution) : Bool :=
  !((x.moPlus).any fun (a, b) => a == b) && !((x.hb).any fun (a, b) => a == b)

def Execution.sameWriteLoc (x : Execution) (a b : Nat) : Bool :=
  match x.kindOf a >>= Kind.write?, x.kindOf b >>= Kind.write? with
  | some (la, _, _), some (lb, _, _) => la == lb
  | _, _ => false

/-- `validate_coherence`: write/write, read/write, write/read and read/read
coherence. -/
def Execution.coherent (x : Execution) : Bool :=
  let hb := x.hb
  let mo := x.moPlus
  (hb.all fun (a, b) => !x.sameWriteLoc a b || mo.contains (a, b)) &&
  (x.rf.all fun (w, r) =>
    !hb.contains (r, w) &&
    (x.ids.all fun e =>
      !(x.sameWriteLoc w e && mo.contains (w, e) && hb.contains (e, r)) &&
      !(x.sameWriteLoc w e && hb.contains (r, e) && !mo.contains (w, e))) &&
    (x.rf.all fun (w', r') => !(hb.contains (r, r') && mo.contains (w', w))))

def Event.isScMember (e : Event) : Bool :=
  ((e.kind.opOrder? <|> e.kind.threadFence?) == some .seqCst)

def Execution.scIndex (x : Execution) (id : Nat) : Option Nat := x.sc.findIdx? (· == id)

def Execution.lastScWriteBefore (x : Execution) (loc bound : Nat) : Option Nat :=
  ((x.sc.take bound).filter fun id => ((x.kindOf id >>= Kind.write?).map (·.1)) == some loc).getLast?

def Execution.atOrAfter (x : Execution) (source bound : Nat) : Bool :=
  source == bound || (x.moPlus).contains (bound, source)

/-- `validate_sc`: S holds exactly the seq_cst operations and thread fences,
agrees with happens-before and the modification order, an SC read observes
the last SC modification before it (or a non-SC one not happening before
it), and the three seq_cst fence rules of N1570 7.17.3 hold. -/
def Execution.scValid (x : Execution) : Bool :=
  let expected := (x.events.filter Event.isScMember).map (·.id)
  let hb := x.hb
  let fences := x.sc.filter fun id => ((x.kindOf id).bind Kind.threadFence?) == some .seqCst
  (expected.all (x.sc.contains ·)) && (x.sc.all (expected.contains ·)) &&
  (x.sc.length == expected.length) &&
  ((hb ++ x.moPlus).all fun (a, b) =>
    match x.scIndex a, x.scIndex b with
    | some i, some j => decide (i < j)
    | _, _ => true) &&
  (x.rf.all fun (w, r) =>
    match x.scIndex r, x.kindOf r >>= Kind.read? with
    | some at_, some (loc, _, _) =>
      let last := x.lastScWriteBefore loc at_
      if (x.scIndex w).isSome then last == some w
      else match last with
        | none => true
        | some l => !hb.contains (w, l)
    | _, _ => true) &&
  (x.rf.all fun (w, r) =>
    match x.kindOf r >>= Kind.read? with
    | none => true
    | some (loc, _, _) =>
      let mods := (x.events.filter fun e => e.kind.opWriteLoc? == some loc).map (·.id)
      fences.all fun f =>
        let fAt := (x.scIndex f).getD 0
        (!x.sb f r || match x.lastScWriteBefore loc fAt with
          | none => true
          | some l => x.atOrAfter w l) &&
        (mods.all fun a =>
          !x.sb a f ||
            ((!((x.scIndex r).map (fun i => decide (fAt < i))).getD false || x.atOrAfter w a) &&
              (fences.all fun g =>
                !(decide (fAt < (x.scIndex g).getD 0) && x.sb g r) || x.atOrAfter w a))))

def Execution.raceFree (x : Execution) : Bool :=
  let hb := x.hb
  x.events.all fun a => x.events.all fun b =>
    match a.kind.nonAtomic?, b.kind.nonAtomic? with
    | some (la, wa), some (lb, wb) =>
      a.thread == b.thread || la != lb || !(wa || wb) || a.id == b.id ||
        hb.contains (a.id, b.id) || hb.contains (b.id, a.id)
    | _, _ => true

/-! ## The checker -/

/-- The failing categories, the Rust `WeakMemoryError`s grouped
(`category` in the binding test). -/
inductive Category where
  | invalidOrder | readsFrom | modificationOrder | compareExchange | atomicity
  | synchronizesWith | cyclic | coherence | scOrder | race
deriving DecidableEq, Repr

def Execution.components (x : Execution) : List (Category × Bool) := [
  (.invalidOrder, x.ordersValid),
  (.readsFrom, x.rfValid),
  (.modificationOrder, x.moValid),
  (.compareExchange, x.casValid),
  (.atomicity, x.atomicityHolds),
  (.synchronizesWith, x.swValid),
  (.cyclic, x.acyclic),
  (.coherence, x.coherent),
  (.scOrder, x.scValid),
  (.race, x.raceFree)
]

def failures (l : List (Category × Bool)) : List Category :=
  l.filterMap fun p => if p.2 then none else some p.1

def check (x : Execution) : List Category := failures x.components

theorem failures_nil_iff (l : List (Category × Bool)) :
    failures l = [] ↔ ∀ p ∈ l, p.2 = true := by
  induction l with
  | nil => simp [failures]
  | cons p rest ih =>
    obtain ⟨c, ok⟩ := p
    cases ok <;> simp_all [failures]

theorem check_accepts_iff (x : Execution) :
    check x = [] ↔
      (x.ordersValid = true ∧ x.rfValid = true ∧ x.moValid = true ∧ x.casValid = true ∧
        x.atomicityHolds = true ∧ x.swValid = true ∧ x.acyclic = true ∧ x.coherent = true ∧
        x.scValid = true ∧ x.raceFree = true) := by
  rw [check, failures_nil_iff]
  simp [Execution.components]

/-- RMW atomicity: in an accepted graph every read-modify-write reads the
immediate modification-order predecessor of its own write. -/
theorem accepted_rmw_is_atomic (x : Execution) (h : check x = []) (w r : Nat)
    (hrf : (w, r) ∈ x.rf) (k : Kind) (hk : x.kindOf r = some k) (hrmw : k.isRmw = true) :
    x.adjacent w r = true := by
  have hall := ((check_accepts_iff x).mp h).2.2.2.2.1
  simp only [Execution.atomicityHolds, List.all_eq_true] at hall
  have := hall (w, r) hrf
  simp [hk, hrmw] at this
  exact this

theorem accepted_sw_is_expected (x : Execution) (h : check x = []) (a b : Nat)
    (hsw : (a, b) ∈ x.sw) : x.swEdge a b = true := by
  have hall := ((check_accepts_iff x).mp h).2.2.2.2.2.1
  simp only [Execution.swValid, Bool.and_eq_true, List.all_eq_true] at hall
  exact hall.1 (a, b) hsw

theorem accepted_expected_is_supplied (x : Execution) (h : check x = []) (a b : Nat)
    (hp : (a, b) ∈ x.pairs) (hedge : x.swEdge a b = true) : (a, b) ∈ x.sw := by
  have hall := ((check_accepts_iff x).mp h).2.2.2.2.2.1
  simp only [Execution.swValid, Bool.and_eq_true, List.all_eq_true] at hall
  have := hall.2 (a, b) hp
  simpa [hedge] using this

theorem accepted_is_race_free (x : Execution) (h : check x = []) : x.raceFree = true :=
  ((check_accepts_iff x).mp h).2.2.2.2.2.2.2.2.2

/-! ## Compare-exchange orders and outcomes -/

theorem cas_failure_no_stronger (s f : Order) (h : casOrdersValid s f = true) :
    f ≠ .release ∧ f ≠ .acqRel ∧ ∃ rank, f.loadRank = some rank ∧ rank ≤ s.successRank := by
  cases f <;> cases s <;> simp_all [casOrdersValid, Order.loadRank, Order.successRank]

theorem strong_failure_observes_another_value (l e obs d : Nat) (s f : Order)
    (h : (Kind.cas l e obs d s f false false).casConsistent = true) : obs ≠ e := by
  simpa [Kind.casConsistent] using h

theorem weak_failure_may_be_spurious (l e d : Nat) (s f : Order) :
    (Kind.cas l e e d s f true false).casConsistent = true := by
  simp [Kind.casConsistent]

theorem success_observes_expected (l e obs d : Nat) (s f : Order) (weak : Bool)
    (h : (Kind.cas l e obs d s f weak true).casConsistent = true) : obs = e := by
  simpa [Kind.casConsistent] using h

theorem failed_cas_writes_nothing (l e obs d : Nat) (s f : Order) (weak : Bool) :
    (Kind.cas l e obs d s f weak false).write? = none := rfl

theorem failed_cas_reads_with_failure_order (l e obs d : Nat) (s f : Order) (weak : Bool) :
    (Kind.cas l e obs d s f weak false).read? = some (l, obs, f) := rfl

/-! ## Shared litmus vectors

Locations: 1 and 2 are atomic flags, 9 is non-atomic data. Each `def` below
is parsed by the Rust binding test (`ev ID THREAD (KIND ARGS)`, relations as
`(from, to)` lists), checked by `check_c11_execution`, and its error
categories compared with `litmusVectors`. -/

/-- MP, release/acquire: the payload read happens after the payload write. -/
def mpReleaseAcquire : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 2 (.load 1 1 .acquire), ev 4 2 (.naRead 9)]
    rf := [(2, 3)], mo := [(0, 2)], sw := [(2, 3)], sc := [] }

/-- MP, relaxed: no synchronisation, the payload accesses race. -/
def mpRelaxed : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .relaxed),
      ev 3 2 (.load 1 1 .relaxed), ev 4 2 (.naRead 9)]
    rf := [(2, 3)], mo := [(0, 2)], sw := [], sc := [] }

/-- MP, relaxed accesses between a release fence and an acquire fence
(7.17.4p2). -/
def mpFences : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.fence .release false),
      ev 3 1 (.store 1 1 .relaxed), ev 4 2 (.load 1 1 .relaxed), ev 5 2 (.fence .acquire false),
      ev 6 2 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 3)], sw := [(2, 5)], sc := [] }

/-- MP, a release fence and an acquire load (7.17.4p3). -/
def mpFenceAcquire : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.fence .release false),
      ev 3 1 (.store 1 1 .relaxed), ev 4 2 (.load 1 1 .acquire), ev 5 2 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 3)], sw := [(2, 4)], sc := [] }

/-- MP, a release store and an acquire fence (7.17.4p4). -/
def mpReleaseFence : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 2 (.load 1 1 .relaxed), ev 4 2 (.fence .acquire false), ev 5 2 (.naRead 9)]
    rf := [(2, 3)], mo := [(0, 2)], sw := [(2, 4)], sc := [] }

/-- MP with signal fences: a signal fence orders nothing between threads. -/
def mpSignalFences : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.fence .seqCst true),
      ev 3 1 (.store 1 1 .relaxed), ev 4 2 (.load 1 1 .relaxed), ev 5 2 (.fence .seqCst true),
      ev 6 2 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 3)], sw := [], sc := [] }

/-- The same graph claiming the signal fences synchronise: refused. -/
def mpSignalFencesClaimed : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.fence .seqCst true),
      ev 3 1 (.store 1 1 .relaxed), ev 4 2 (.load 1 1 .relaxed), ev 5 2 (.fence .seqCst true),
      ev 6 2 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 3)], sw := [(2, 5)], sc := [] }

/-- SB, relaxed: both loads read the initial values, which C11 allows. Each
location is initialised by the thread that stores to it, so the
initialisation happens before that store. -/
def sbRelaxed : Execution :=
  { events := [ev 0 1 (.init 1 0), ev 1 2 (.init 2 0), ev 2 1 (.store 1 1 .relaxed),
      ev 3 1 (.load 2 0 .relaxed), ev 4 2 (.store 2 1 .relaxed), ev 5 2 (.load 1 0 .relaxed)]
    rf := [(1, 3), (0, 5)], mo := [(0, 2), (1, 4)], sw := [], sc := [] }

/-- SB, seq_cst: the weak outcome is refused for this S (the second load
reads an initialisation that happens before the last SC store before it). -/
def sbSeqCst : Execution :=
  { events := [ev 0 1 (.init 1 0), ev 1 2 (.init 2 0), ev 2 1 (.store 1 1 .seqCst),
      ev 3 1 (.load 2 0 .seqCst), ev 4 2 (.store 2 1 .seqCst), ev 5 2 (.load 1 0 .seqCst)]
    rf := [(1, 3), (0, 5)], mo := [(0, 2), (1, 4)], sw := [], sc := [2, 3, 4, 5] }

/-- SB, relaxed accesses separated by seq_cst fences: the weak outcome is
refused by the third fence rule. -/
def sbScFences : Execution :=
  { events := [ev 0 1 (.init 1 0), ev 1 2 (.init 2 0), ev 2 1 (.store 1 1 .relaxed),
      ev 3 1 (.fence .seqCst false), ev 4 1 (.load 2 0 .relaxed), ev 5 2 (.store 2 1 .relaxed),
      ev 6 2 (.fence .seqCst false), ev 7 2 (.load 1 0 .relaxed)]
    rf := [(1, 4), (0, 7)], mo := [(0, 2), (1, 5)], sw := [], sc := [3, 6] }

/-- SB with seq_cst fences, the second load seeing the first store: allowed. -/
def sbScFencesSeen : Execution :=
  { events := [ev 0 1 (.init 1 0), ev 1 2 (.init 2 0), ev 2 1 (.store 1 1 .relaxed),
      ev 3 1 (.fence .seqCst false), ev 4 1 (.load 2 0 .relaxed), ev 5 2 (.store 2 1 .relaxed),
      ev 6 2 (.fence .seqCst false), ev 7 2 (.load 1 1 .relaxed)]
    rf := [(1, 4), (2, 7)], mo := [(0, 2), (1, 5)], sw := [], sc := [3, 6] }

/-- Two fetch-and-adds both reading the initial value: not atomic. -/
def rmwLostUpdate : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.rmw 1 0 1 .relaxed), ev 2 2 (.rmw 1 0 1 .relaxed)]
    rf := [(0, 1), (0, 2)], mo := [(0, 1), (1, 2)], sw := [], sc := [] }

/-- Two fetch-and-adds in sequence: accepted. -/
def rmwChain : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.rmw 1 0 1 .relaxed), ev 2 2 (.rmw 1 1 2 .relaxed)]
    rf := [(0, 1), (1, 2)], mo := [(0, 1), (1, 2)], sw := [], sc := [] }

/-- A release store, a relaxed RMW of another thread, an acquire read of the
RMW: the RMW continues the release sequence, so the payload is ordered. -/
def releaseSequenceRmw : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 2 (.rmw 1 1 2 .relaxed), ev 4 3 (.load 1 2 .acquire), ev 5 3 (.naRead 9)]
    rf := [(2, 3), (3, 4)], mo := [(0, 2), (2, 3)], sw := [(2, 4)], sc := [] }

/-- The same with a plain relaxed store of another thread instead of the RMW:
it ends the release sequence, so the payload accesses race. -/
def releaseSequenceBroken : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 2 (.store 1 2 .relaxed), ev 4 3 (.load 1 2 .acquire), ev 5 3 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 2), (2, 3)], sw := [], sc := [] }

/-- A same-thread relaxed store continues the sequence (C11's wording, which
C++20 dropped). -/
def releaseSequenceSameThread : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 1 (.store 1 2 .relaxed), ev 4 2 (.load 1 2 .acquire), ev 5 2 (.naRead 9)]
    rf := [(3, 4)], mo := [(0, 2), (2, 3)], sw := [(2, 4)], sc := [] }

/-- A weak compare-exchange loop: the first attempt fails spuriously on the
expected value, the second succeeds and publishes with release. -/
def casWeakLoop : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9),
      ev 2 1 (.cas 1 0 0 1 .release .relaxed true false),
      ev 3 1 (.cas 1 0 0 1 .release .relaxed true true),
      ev 4 2 (.load 1 1 .acquire), ev 5 2 (.naRead 9)]
    rf := [(0, 2), (0, 3), (3, 4)], mo := [(0, 3)], sw := [(3, 4)], sc := [] }

/-- The same failure on the expected value by a strong exchange: refused. -/
def casStrongSpurious : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9),
      ev 2 1 (.cas 1 0 0 1 .release .relaxed false false),
      ev 3 1 (.cas 1 0 0 1 .release .relaxed false true),
      ev 4 2 (.load 1 1 .acquire), ev 5 2 (.naRead 9)]
    rf := [(0, 2), (0, 3), (3, 4)], mo := [(0, 3)], sw := [(3, 4)], sc := [] }

/-- A failure order stronger than the success order: refused. -/
def casFailureStronger : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.cas 1 1 0 2 .relaxed .acquire false false)]
    rf := [(0, 1)], mo := [], sw := [], sc := [] }

/-- A failed compare-exchange with acquire failure order reading a release
store synchronises (it is an acquire load). -/
def casFailureAcquires : Execution :=
  { events := [ev 0 0 (.init 1 0), ev 1 1 (.naWrite 9), ev 2 1 (.store 1 1 .release),
      ev 3 2 (.cas 1 0 1 2 .acquire .acquire false false), ev 4 2 (.naRead 9)]
    rf := [(2, 3)], mo := [(0, 2)], sw := [(2, 3)], sc := [] }

def litmusVectors : List (String × Execution × List Category) := [
  ("mpReleaseAcquire", mpReleaseAcquire, []),
  ("mpRelaxed", mpRelaxed, [.race]),
  ("mpFences", mpFences, []),
  ("mpFenceAcquire", mpFenceAcquire, []),
  ("mpReleaseFence", mpReleaseFence, []),
  ("mpSignalFences", mpSignalFences, [.race]),
  ("mpSignalFencesClaimed", mpSignalFencesClaimed, [.synchronizesWith]),
  ("sbRelaxed", sbRelaxed, []),
  ("sbSeqCst", sbSeqCst, [.scOrder]),
  ("sbScFences", sbScFences, [.scOrder]),
  ("sbScFencesSeen", sbScFencesSeen, []),
  ("rmwLostUpdate", rmwLostUpdate, [.atomicity]),
  ("rmwChain", rmwChain, []),
  ("releaseSequenceRmw", releaseSequenceRmw, []),
  ("releaseSequenceBroken", releaseSequenceBroken, [.race]),
  ("releaseSequenceSameThread", releaseSequenceSameThread, []),
  ("casWeakLoop", casWeakLoop, []),
  ("casStrongSpurious", casStrongSpurious, [.compareExchange]),
  ("casFailureStronger", casFailureStronger, [.invalidOrder]),
  ("casFailureAcquires", casFailureAcquires, [])
]

theorem mp_release_acquire_accepted : check mpReleaseAcquire = [] := by decide
theorem mp_relaxed_races : check mpRelaxed = [.race] := by decide
theorem mp_fences_accepted : check mpFences = [] := by decide
theorem sb_seq_cst_weak_outcome_refused : check sbSeqCst = [.scOrder] := by decide
theorem sb_sc_fences_weak_outcome_refused : check sbScFences = [.scOrder] := by decide
theorem rmw_lost_update_refused : check rmwLostUpdate = [.atomicity] := by decide
theorem release_sequence_continues_through_rmw : check releaseSequenceRmw = [] := by decide
theorem release_sequence_ends_at_foreign_store : check releaseSequenceBroken = [.race] := by
  decide
theorem cas_weak_loop_accepted : check casWeakLoop = [] := by decide

theorem litmus_vectors_hold :
    litmusVectors.all (fun v => check v.2.1 == v.2.2) = true := by
  decide

end QCore.WeakMemory
