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

import QCore.AtomicPublication
import QCore.WeakMemory

/-!
# C atomics routed to the weak-memory model (card CG19-ATOMIC-SOURCE-ROUTE)

Gateway rule 11i. The Rust side is `crates/qcore-c-gateway/src/atomic_source_route.rs`,
bound to this module by the shared-vector tests in
`crates/qcore-c-gateway/tests/unit/atomic_source_route.rs`: `formTable` equals
`ATOMIC_FORM_TABLE` row for row, `decodeOrder`/`routeOrder` agree with the Rust
functions on every value, and `routeVectors` are run through both.

What is proved here:

* a routed access's order is one the model (`qcore_core::weak_memory`, whose
  order validity is `modelRepresents`) represents for that operation;
* since card CG28-WEAK-MEMORY-EVENTS (gateway rule 11t) read-modify-writes,
  compare-exchanges and fences are routed, not refused
  (`read_modify_write_routed`, `fence_routed`, `routeCas`): a
  compare-exchange's failure order is one `QCore.WeakMemory.casOrdersValid`
  admits (`routed_failure_admitted_by_model`), its strength is its name's
  or its evaluated weak flag's, and a consume fence is an acquire fence
  (C11 7.17.4.1p2, `consume_fence_is_acquire`);
* `memory_order_consume` on a load, read-modify-write or compare-exchange is
  refused, never read as acquire (`consume_load_refused`,
  `consume_order_refused`); the legacy `__sync` builtins, generic by-pointer
  forms and every unlisted builtin are refused by name;
* the routed store and load orders agree with the bounded publication model
  `QCore.AtomicPublication` on Clang's numbering, so the publication checker
  receives exactly the order the source spelled;
* a failed or undecided check is `Unknown` and carries the single-graph
  assumption; a `Safe` publication verdict needs, since card
  CG29-EXECUTION-COVERAGE-AND-RECLAMATION, the extracted shape race-free over
  every candidate execution too, and carries the all-executions assumption
  instead (`safe_publication_covers_every_execution`).

Faithful Clang extraction of the builtin name and order operand, and the
Rust-to-Lean correspondence, are stated review premises (trust boundary 28).
-/

namespace QCore.AtomicSourceRoute

inductive FormClass where
  | load | store | init | byPointer | readModifyWrite | compareExchange | fence
  | signalFence | legacySync
deriving DecidableEq, Repr

/-- Every builtin the route names, `ATOMIC_FORM_TABLE` row for row. -/
def formTable : List (String × FormClass) := [
  ("__c11_atomic_load", .load),
  ("__atomic_load_n", .load),
  ("__c11_atomic_store", .store),
  ("__atomic_store_n", .store),
  ("__c11_atomic_init", .init),
  ("__atomic_load", .byPointer),
  ("__atomic_store", .byPointer),
  ("__c11_atomic_exchange", .readModifyWrite),
  ("__atomic_exchange_n", .readModifyWrite),
  ("__atomic_exchange", .byPointer),
  ("__c11_atomic_fetch_add", .readModifyWrite),
  ("__c11_atomic_fetch_sub", .readModifyWrite),
  ("__c11_atomic_fetch_and", .readModifyWrite),
  ("__c11_atomic_fetch_or", .readModifyWrite),
  ("__c11_atomic_fetch_xor", .readModifyWrite),
  ("__c11_atomic_fetch_nand", .readModifyWrite),
  ("__c11_atomic_fetch_max", .readModifyWrite),
  ("__c11_atomic_fetch_min", .readModifyWrite),
  ("__atomic_fetch_add", .readModifyWrite),
  ("__atomic_fetch_sub", .readModifyWrite),
  ("__atomic_fetch_and", .readModifyWrite),
  ("__atomic_fetch_or", .readModifyWrite),
  ("__atomic_fetch_xor", .readModifyWrite),
  ("__atomic_fetch_nand", .readModifyWrite),
  ("__atomic_fetch_max", .readModifyWrite),
  ("__atomic_fetch_min", .readModifyWrite),
  ("__atomic_add_fetch", .readModifyWrite),
  ("__atomic_sub_fetch", .readModifyWrite),
  ("__atomic_and_fetch", .readModifyWrite),
  ("__atomic_or_fetch", .readModifyWrite),
  ("__atomic_xor_fetch", .readModifyWrite),
  ("__atomic_nand_fetch", .readModifyWrite),
  ("__atomic_max_fetch", .readModifyWrite),
  ("__atomic_min_fetch", .readModifyWrite),
  ("__atomic_test_and_set", .readModifyWrite),
  ("__sync_fetch_and_add", .legacySync),
  ("__sync_fetch_and_sub", .legacySync),
  ("__sync_add_and_fetch", .legacySync),
  ("__sync_sub_and_fetch", .legacySync),
  ("__sync_lock_test_and_set", .legacySync),
  ("__c11_atomic_compare_exchange_strong", .compareExchange),
  ("__c11_atomic_compare_exchange_weak", .compareExchange),
  ("__atomic_compare_exchange_n", .compareExchange),
  ("__atomic_compare_exchange", .compareExchange),
  ("__sync_bool_compare_and_swap", .legacySync),
  ("__sync_val_compare_and_swap", .legacySync),
  ("__c11_atomic_thread_fence", .fence),
  ("__c11_atomic_signal_fence", .signalFence),
  ("__atomic_thread_fence", .fence),
  ("__atomic_signal_fence", .signalFence),
  ("__sync_synchronize", .legacySync)
]

theorem names_unique : (formTable.map (·.1)).Nodup := by decide

def lookup (name : String) : Option FormClass :=
  (formTable.find? (·.1 == name)).map (·.2)

inductive SourceOrder where
  | relaxed | consume | acquire | release | acqRel | seqCst
deriving DecidableEq, Repr

/-- Clang's builtin numbering, which `<stdatomic.h>`'s `memory_order` equals. -/
def decodeOrder : Int → Option SourceOrder
  | 0 => some .relaxed
  | 1 => some .consume
  | 2 => some .acquire
  | 3 => some .release
  | 4 => some .acqRel
  | 5 => some .seqCst
  | _ => none

inductive Operation where
  | load | store | init | readModifyWrite | compareExchange | threadFence | signalFence
deriving DecidableEq, Repr

/-- `qcore_core::MemoryOrder`: the model has no consume order. -/
inductive ModelOrder where | relaxed | acquire | release | acqRel | seqCst
deriving DecidableEq, Repr

inductive Refusal where
  | consumeLoad | consumeOrder | legacySync | invalidOrder | unevaluatedOrder
  | unevaluatedStrength | byPointer | nonScalarValue | unsupportedShape | unlisted
deriving DecidableEq, Repr

/-- `route_order`: C11 7.17.7.1p2 and 7.17.7.2p2; an initialisation is not an
atomic operation and takes no order; a read-modify-write, a compare-exchange's
success and a fence take any order (CG28). Consume is refused by name on a
load, read-modify-write or compare-exchange; a consume fence is an acquire
fence (7.17.4.1p2). -/
def routeOrder : Operation → SourceOrder → Except Refusal ModelOrder
  | .init, _ => .error .invalidOrder
  | .load, .consume => .error .consumeLoad
  | .readModifyWrite, .consume | .compareExchange, .consume => .error .consumeOrder
  | .threadFence, .consume | .signalFence, .consume => .ok .acquire
  | _, .relaxed => .ok .relaxed
  | .load, .acquire => .ok .acquire
  | .store, .release => .ok .release
  | .load, .seqCst => .ok .seqCst
  | .store, .seqCst => .ok .seqCst
  | .load, _ | .store, _ => .error .invalidOrder
  | _, .acquire => .ok .acquire
  | _, .release => .ok .release
  | _, .acqRel => .ok .acqRel
  | _, .seqCst => .ok .seqCst

/-- The model's own order validity: `weak_memory::event_order_represented`
(for a compare-exchange, of its success order; the failure order is
`routeFailureOrder`'s). -/
def modelRepresents : Operation → ModelOrder → Bool
  | .load, .relaxed | .load, .acquire | .load, .seqCst => true
  | .store, .relaxed | .store, .release | .store, .seqCst => true
  | .readModifyWrite, _ | .compareExchange, _ | .threadFence, _ | .signalFence, _ => true
  | _, _ => false

def classOperation : FormClass → Except Refusal Operation
  | .load => .ok .load
  | .store => .ok .store
  | .init => .ok .init
  | .byPointer => .error .byPointer
  | .readModifyWrite => .ok .readModifyWrite
  | .compareExchange => .ok .compareExchange
  | .fence => .ok .threadFence
  | .signalFence => .ok .signalFence
  | .legacySync => .error .legacySync

/-- `classify_builtin` on a builtin's name and its evaluated order operand
(`none`: not an integer constant the route evaluates). -/
def route (name : String) (order : Option Int) :
    Except Refusal (Operation × Option ModelOrder) :=
  match lookup name with
  | none => .error .unlisted
  | some c =>
    match classOperation c with
    | .error r => .error r
    | .ok .init => .ok (.init, none)
    | .ok op =>
      match order with
      | none => .error .unevaluatedOrder
      | some v =>
        match decodeOrder v with
        | none => .error .invalidOrder
        | some o =>
          match routeOrder op o with
          | .error r => .error r
          | .ok m => .ok (op, some m)

/-- A routed order is one the model represents for the operation. -/
theorem routed_order_is_represented (op : Operation) (o : SourceOrder) (m : ModelOrder)
    (h : routeOrder op o = .ok m) : modelRepresents op m = true := by
  cases op <;> cases o <;> simp_all [routeOrder, modelRepresents] <;> subst h <;> rfl

/-- Routed plain atomics agree with the model: every admission is a plain
operation whose order the model represents. -/
theorem routed_plain_agrees_with_model (name : String) (order : Option Int)
    (op : Operation) (m : ModelOrder)
    (h : route name order = .ok (op, some m)) :
    op ≠ .init ∧ modelRepresents op m = true := by
  unfold route at h
  split at h
  · cases h
  · rename_i c _
    split at h
    · cases h
    · cases h
    · rename_i op' _ _
      split at h
      · cases h
      · split at h
        · cases h
        · rename_i o _
          split at h
          · cases h
          · rename_i m' hm
            cases h
            refine ⟨?_, routed_order_is_represented _ _ _ hm⟩
            intro hinit
            subst hinit
            cases o <;> simp [routeOrder] at hm

/-- A routed builtin is never a by-pointer form or a legacy `__sync` one. -/
theorem route_class_is_routed (name : String) (order : Option Int)
    (r : Operation × Option ModelOrder) (h : route name order = .ok r) :
    lookup name ≠ some .byPointer ∧ lookup name ≠ some .legacySync := by
  unfold route at h
  split at h
  · cases h
  · rename_i c hc
    cases c <;> simp_all [classOperation]

/-- Card CG28: a read-modify-write is routed with every order but consume. -/
theorem read_modify_write_routed (name : String) (o : SourceOrder) (v : Int)
    (h : lookup name = some .readModifyWrite) (hv : decodeOrder v = some o)
    (hc : o ≠ .consume) :
    ∃ m, route name (some v) = .ok (.readModifyWrite, some m) := by
  cases o <;> simp_all [route, classOperation, routeOrder]

/-- Card CG28: a thread or signal fence is routed with every order. -/
theorem fence_routed (name : String) (o : SourceOrder) (v : Int)
    (h : lookup name = some .fence ∨ lookup name = some .signalFence)
    (hv : decodeOrder v = some o) :
    ∃ op m, route name (some v) = .ok (op, some m) ∧
      (op = .threadFence ∨ op = .signalFence) := by
  rcases h with h | h <;> cases o <;> simp_all [route, classOperation, routeOrder]

/-- C11 7.17.4.1p2: a consume fence is an acquire fence. -/
theorem consume_fence_is_acquire :
    routeOrder .threadFence .consume = .ok .acquire ∧
      routeOrder .signalFence .consume = .ok .acquire := ⟨rfl, rfl⟩

theorem legacy_sync_refused (name : String) (order : Option Int)
    (h : lookup name = some .legacySync) : route name order = .error .legacySync := by
  simp [route, h, classOperation]

theorem by_pointer_refused (name : String) (order : Option Int)
    (h : lookup name = some .byPointer) : route name order = .error .byPointer := by
  simp [route, h, classOperation]

/-- A consume load is refused, never read as acquire. -/
theorem consume_load_refused (name : String) (h : lookup name = some .load) :
    route name (some 1) = .error .consumeLoad := by
  simp [route, h, classOperation, decodeOrder, routeOrder]

/-- Card CG28: consume on a read-modify-write or a compare-exchange's
success is refused, never read as acquire. -/
theorem consume_order_refused (name : String)
    (h : lookup name = some .readModifyWrite ∨ lookup name = some .compareExchange) :
    route name (some 1) = .error .consumeOrder := by
  rcases h with h | h <;> simp [route, h, classOperation, decodeOrder, routeOrder]

/-- Order value 1 (consume) is admitted for no load, store, read-modify-write
or compare-exchange: a resolver that answers 1 for `memory_order_consume`
alone is fail-closed. -/
theorem order_one_never_routed (name : String)
    (h : lookup name = some .load ∨ lookup name = some .store ∨
      lookup name = some .readModifyWrite ∨ lookup name = some .compareExchange)
    (r : Operation × Option ModelOrder) : route name (some 1) ≠ .ok r := by
  rcases h with h | h | h | h <;> simp [route, h, classOperation, decodeOrder, routeOrder]

theorem consume_never_routed (op : Operation) (m : ModelOrder)
    (h : op ≠ .threadFence ∧ op ≠ .signalFence) :
    routeOrder op .consume ≠ .ok m := by
  cases op <;> simp_all [routeOrder]

theorem unlisted_refused (name : String) (order : Option Int) (h : lookup name = none) :
    route name order = .error .unlisted := by
  simp [route, h]

/-- An order the route cannot evaluate is refused for every ordered form. -/
theorem unevaluated_order_refused (name : String)
    (h : lookup name = some .load ∨ lookup name = some .store ∨
      lookup name = some .readModifyWrite ∨ lookup name = some .compareExchange ∨
      lookup name = some .fence ∨ lookup name = some .signalFence) :
    route name none = .error .unevaluatedOrder := by
  rcases h with h | h | h | h | h | h <;> simp [route, h, classOperation]

/-! ## Compare-exchange (card CG28) -/

/-- `qcore_core::MemoryOrder` as the weak-memory model's order. -/
def toWeak : ModelOrder → QCore.WeakMemory.Order
  | .relaxed => .relaxed
  | .acquire => .acquire
  | .release => .release
  | .acqRel => .acqRel
  | .seqCst => .seqCst

/-- `route_failure_order`: C11 7.17.7.4p2 — not release or acq_rel, no
stronger than the success order; consume refused by name. -/
def failureModel : SourceOrder → Option ModelOrder
  | .relaxed => some .relaxed
  | .acquire => some .acquire
  | .seqCst => some .seqCst
  | _ => none

def routeFailureOrder (success : ModelOrder) (f : SourceOrder) : Except Refusal ModelOrder :=
  if f = .consume then .error .consumeOrder
  else
    match failureModel f with
    | none => .error .invalidOrder
    | some m =>
      if QCore.WeakMemory.casOrdersValid (toWeak success) (toWeak m) = true then .ok m
      else .error .invalidOrder

/-- A routed failure order is one the weak-memory model admits. -/
theorem routed_failure_admitted_by_model (s : ModelOrder) (f : SourceOrder) (m : ModelOrder)
    (h : routeFailureOrder s f = .ok m) :
    QCore.WeakMemory.casOrdersValid (toWeak s) (toWeak m) = true := by
  unfold routeFailureOrder at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · cases h
        assumption
      · cases h

/-- The route's failure order is never stronger than its success order. -/
theorem routed_failure_no_stronger (s : ModelOrder) (f : SourceOrder) (m : ModelOrder)
    (h : routeFailureOrder s f = .ok m) :
    toWeak m ≠ .release ∧ toWeak m ≠ .acqRel ∧
      ∃ rank, (toWeak m).loadRank = some rank ∧ rank ≤ (toWeak s).successRank :=
  QCore.WeakMemory.cas_failure_no_stronger _ _ (routed_failure_admitted_by_model s f m h)

inductive Strength where | strong | weak
deriving DecidableEq, Repr

/-- The strength of a listed compare-exchange: the C11 builtins by name, the
GNU ones by their evaluated weak flag (`none`: not an integer constant). -/
def strengthOf (name : String) (weak : Option Int) : Except Refusal Strength :=
  if name = "__c11_atomic_compare_exchange_strong" then .ok .strong
  else if name = "__c11_atomic_compare_exchange_weak" then .ok .weak
  else match weak with
    | none => .error .unevaluatedStrength
    | some 0 => .ok .strong
    | some _ => .ok .weak

/-- `classify_builtin` on a compare-exchange: success order, failure order and
strength. -/
def routeCas (name : String) (success failure weak : Option Int) :
    Except Refusal (ModelOrder × ModelOrder × Strength) :=
  match route name success with
  | .error r => .error r
  | .ok (.compareExchange, some s) =>
    match failure with
    | none => .error .unevaluatedOrder
    | some v =>
      match decodeOrder v with
      | none => .error .invalidOrder
      | some f =>
        match routeFailureOrder s f with
        | .error r => .error r
        | .ok m =>
          match strengthOf name weak with
          | .error r => .error r
          | .ok k => .ok (s, m, k)
  | .ok _ => .error .unsupportedShape

theorem cas_strength_by_name :
    strengthOf "__c11_atomic_compare_exchange_weak" none = .ok .weak ∧
      strengthOf "__c11_atomic_compare_exchange_strong" none = .ok .strong := by
  simp [strengthOf]

theorem gnu_cas_unevaluated_weak_flag_refused :
    strengthOf "__atomic_compare_exchange_n" none = .error .unevaluatedStrength := by
  simp [strengthOf]

theorem gnu_cas_strength_by_weak_flag (name : String)
    (h1 : name ≠ "__c11_atomic_compare_exchange_strong")
    (h2 : name ≠ "__c11_atomic_compare_exchange_weak") (flag : Int) :
    strengthOf name (some flag) = .ok (if flag = 0 then .strong else .weak) := by
  by_cases hf : flag = 0 <;> simp [strengthOf, h1, h2, hf]

theorem routed_cas_failure_admitted (name : String) (s f w : Option Int)
    (sm fm : ModelOrder) (k : Strength) (h : routeCas name s f w = .ok (sm, fm, k)) :
    QCore.WeakMemory.casOrdersValid (toWeak sm) (toWeak fm) = true := by
  unfold routeCas at h
  split at h
  · cases h
  · split at h
    · cases h
    · split at h
      · cases h
      · rename_i fo _
        split at h
        · cases h
        · rename_i m hm
          split at h
          · cases h
          · cases h
            exact routed_failure_admitted_by_model _ _ _ hm
  · cases h

/-! ## Agreement with the bounded publication model -/

open QCore.AtomicPublication in
def storeOrderOf : StoreOrder → ModelOrder
  | .relaxed => .relaxed
  | .release => .release
  | .seqCst => .seqCst

open QCore.AtomicPublication in
def loadOrderOf : LoadOrder → ModelOrder
  | .relaxed => .relaxed
  | .acquire => .acquire
  | .seqCst => .seqCst

open QCore.AtomicPublication in
/-- On Clang's numbering, the route gives a store exactly the order the
publication model names for that value. -/
theorem routed_store_matches_publication (s : StoreOrder) :
    (decodeOrder (clangStoreValue s)).map (routeOrder .store) = some (.ok (storeOrderOf s)) := by
  cases s <;> rfl

open QCore.AtomicPublication in
theorem routed_load_matches_publication (l : LoadOrder) :
    (decodeOrder (clangLoadValue l)).map (routeOrder .load) = some (.ok (loadOrderOf l)) := by
  cases l <;> rfl

open QCore.AtomicPublication in
/-- Releasing in the publication model is having a release order in the
route's vocabulary. -/
theorem releases_iff (s : StoreOrder) :
    releases s ↔ (storeOrderOf s = .release ∨ storeOrderOf s = .seqCst) := by
  cases s <;> simp [releases, storeOrderOf]

open QCore.AtomicPublication in
theorem acquires_iff (l : LoadOrder) :
    acquires l ↔ (loadOrderOf l = .acquire ∨ loadOrderOf l = .seqCst) := by
  cases l <;> simp [acquires, loadOrderOf]

/-! ## The publication verdict -/

inductive Status where | safe | unknown
deriving DecidableEq, Repr

inductive Assumption where
  | singleGraph | callerDeclaredBounds | faithfulClangAst | reviewedCorrespondence
  /-- Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION: the extracted shape was
  checked over every candidate execution within its bounds
  (`QCore.ExecutionCoverage.verdict_race_free_sound`). -/
  | allExecutions
deriving DecidableEq, Repr

structure Verdict where
  status : Status
  assumptions : List Assumption
deriving DecidableEq, Repr

/-- The gateway's production gate: the composed check's outcome
(`some true`: checked; `some false`: refused; `none`: undecided) and, card
CG29, whether the extracted shape is race-free over every candidate
execution (`covered`). `Safe` needs both and carries the all-executions
assumption; `Unknown` carries the single-graph one. -/
def publicationVerdict (checked : Option Bool) (covered : Bool) : Verdict where
  status := if checked = some true ∧ covered = true then .safe else .unknown
  assumptions :=
    (if checked = some true ∧ covered = true then .allExecutions else .singleGraph) ::
      [.callerDeclaredBounds, .faithfulClangAst, .reviewedCorrespondence]

/-- An `Unknown` publication verdict carries the single-graph assumption. -/
theorem unknown_publication_carries_single_graph (checked : Option Bool) (covered : Bool)
    (h : (publicationVerdict checked covered).status = .unknown) :
    Assumption.singleGraph ∈ (publicationVerdict checked covered).assumptions := by
  unfold publicationVerdict at h ⊢
  by_cases hc : checked = some true ∧ covered = true
  · simp [hc] at h
  · simp [hc]

/-- A `Safe` publication verdict claims all executions, never one graph. -/
theorem safe_publication_covers_every_execution (checked : Option Bool) (covered : Bool)
    (h : (publicationVerdict checked covered).status = .safe) :
    Assumption.allExecutions ∈ (publicationVerdict checked covered).assumptions ∧
      Assumption.singleGraph ∉ (publicationVerdict checked covered).assumptions := by
  unfold publicationVerdict at h ⊢
  by_cases hc : checked = some true ∧ covered = true
  · simp [hc]
  · simp [hc] at h

theorem failed_publication_is_unknown (covered : Bool) :
    (publicationVerdict (some false) covered).status = .unknown := by
  simp [publicationVerdict]

theorem undecided_publication_is_unknown (covered : Bool) :
    (publicationVerdict none covered).status = .unknown := by
  simp [publicationVerdict]

/-- Card CG29: a checked single graph whose program is not race-free over
every execution (a racing candidate, or an exceeded bound) is `Unknown`. -/
theorem uncovered_publication_is_unknown (checked : Option Bool) :
    (publicationVerdict checked false).status = .unknown := by
  simp [publicationVerdict]

theorem safe_publication_only_when_checked (checked : Option Bool) (covered : Bool)
    (h : (publicationVerdict checked covered).status = .safe) :
    checked = some true ∧ covered = true := by
  unfold publicationVerdict at h
  by_cases hc : checked = some true ∧ covered = true
  · exact hc
  · simp [hc] at h

/-! ## Shared vectors -/

inductive Outcome where
  | routed (op : Operation) (order : Option ModelOrder)
  | routedCas (success failure : ModelOrder) (strength : Strength)
  | refused (r : Refusal)
deriving DecidableEq, Repr

def outcome (name : String) (order : Option Int) : Outcome :=
  match route name order with
  | .ok (op, m) => .routed op m
  | .error r => .refused r

/-- Run through `classify_builtin`'s table and order functions by the Rust test. -/
def routeVectors : List (String × Option Int × Outcome) := [
  ("__atomic_load_n", some 5, .routed .load (some .seqCst)),
  ("__atomic_store_n", some 5, .routed .store (some .seqCst)),
  ("__c11_atomic_load", some 2, .routed .load (some .acquire)),
  ("__c11_atomic_store", some 3, .routed .store (some .release)),
  ("__c11_atomic_load", some 0, .routed .load (some .relaxed)),
  ("__c11_atomic_init", none, .routed .init none),
  ("__c11_atomic_load", some 1, .refused .consumeLoad),
  ("__atomic_load_n", some 3, .refused .invalidOrder),
  ("__atomic_store_n", some 2, .refused .invalidOrder),
  ("__c11_atomic_store", some 4, .refused .invalidOrder),
  ("__c11_atomic_load", some 9, .refused .invalidOrder),
  ("__c11_atomic_load", none, .refused .unevaluatedOrder),
  ("__c11_atomic_fetch_add", some 5, .routed .readModifyWrite (some .seqCst)),
  ("__c11_atomic_fetch_add", some 4, .routed .readModifyWrite (some .acqRel)),
  ("__c11_atomic_fetch_add", some 1, .refused .consumeOrder),
  ("__atomic_exchange_n", some 5, .routed .readModifyWrite (some .seqCst)),
  ("__atomic_fetch_or", some 0, .routed .readModifyWrite (some .relaxed)),
  ("__c11_atomic_compare_exchange_strong", some 5, .routed .compareExchange (some .seqCst)),
  ("__c11_atomic_thread_fence", some 5, .routed .threadFence (some .seqCst)),
  ("__c11_atomic_thread_fence", some 1, .routed .threadFence (some .acquire)),
  ("__atomic_signal_fence", some 3, .routed .signalFence (some .release)),
  ("__sync_synchronize", none, .refused .legacySync),
  ("__sync_fetch_and_add", some 5, .refused .legacySync),
  ("__atomic_exchange", some 5, .refused .byPointer),
  ("__atomic_load", some 5, .refused .byPointer),
  ("__atomic_fetch_umax", some 5, .refused .unlisted)
]

theorem route_vectors_hold :
    routeVectors.all (fun v => outcome v.1 v.2.1 == v.2.2) = true := by
  decide

def casOutcome (name : String) (success failure weak : Option Int) : Outcome :=
  match routeCas name success failure weak with
  | .ok (s, f, k) => .routedCas s f k
  | .error r => .refused r

/-- Compare-exchange vectors (name, success, failure, weak flag), run through
`classify_builtin` on synthetic nodes by the Rust test. -/
def casVectors : List (String × Option Int × Option Int × Option Int × Outcome) := [
  ("__c11_atomic_compare_exchange_strong", some 5, some 5, none, .routedCas .seqCst .seqCst .strong),
  ("__c11_atomic_compare_exchange_weak", some 3, some 0, none, .routedCas .release .relaxed .weak),
  ("__atomic_compare_exchange_n", some 4, some 2, some 1, .routedCas .acqRel .acquire .weak),
  ("__atomic_compare_exchange", some 5, some 5, some 0, .routedCas .seqCst .seqCst .strong),
  ("__c11_atomic_compare_exchange_strong", some 0, some 2, none, .refused .invalidOrder),
  ("__c11_atomic_compare_exchange_strong", some 3, some 2, none, .refused .invalidOrder),
  ("__c11_atomic_compare_exchange_strong", some 5, some 3, none, .refused .invalidOrder),
  ("__c11_atomic_compare_exchange_strong", some 5, some 4, none, .refused .invalidOrder),
  ("__c11_atomic_compare_exchange_strong", some 5, some 1, none, .refused .consumeOrder),
  ("__c11_atomic_compare_exchange_strong", some 1, some 0, none, .refused .consumeOrder),
  ("__c11_atomic_compare_exchange_strong", some 2, none, none, .refused .unevaluatedOrder),
  ("__atomic_compare_exchange_n", some 5, some 5, none, .refused .unevaluatedStrength)
]

theorem cas_vectors_hold :
    casVectors.all (fun v => casOutcome v.1 v.2.1 v.2.2.1 v.2.2.2.1 == v.2.2.2.2) = true := by
  decide

/-- `decode_order` then `route_order` on one operation and value. -/
def orderOutcome (op : Operation) (v : Int) : Outcome :=
  match decodeOrder v with
  | none => .refused .invalidOrder
  | some o =>
    match routeOrder op o with
    | .ok m => .routed op (some m)
    | .error r => .refused r

/-- Every load, store, read-modify-write and thread-fence order value from -1
to 6, run through the Rust functions by the binding test. -/
def orderVectors : List (Operation × Int × Outcome) := [
  (.load, -1, .refused .invalidOrder),
  (.load, 0, .routed .load (some .relaxed)),
  (.load, 1, .refused .consumeLoad),
  (.load, 2, .routed .load (some .acquire)),
  (.load, 3, .refused .invalidOrder),
  (.load, 4, .refused .invalidOrder),
  (.load, 5, .routed .load (some .seqCst)),
  (.load, 6, .refused .invalidOrder),
  (.store, -1, .refused .invalidOrder),
  (.store, 0, .routed .store (some .relaxed)),
  (.store, 1, .refused .invalidOrder),
  (.store, 2, .refused .invalidOrder),
  (.store, 3, .routed .store (some .release)),
  (.store, 4, .refused .invalidOrder),
  (.store, 5, .routed .store (some .seqCst)),
  (.store, 6, .refused .invalidOrder),
  (.readModifyWrite, -1, .refused .invalidOrder),
  (.readModifyWrite, 0, .routed .readModifyWrite (some .relaxed)),
  (.readModifyWrite, 1, .refused .consumeOrder),
  (.readModifyWrite, 2, .routed .readModifyWrite (some .acquire)),
  (.readModifyWrite, 3, .routed .readModifyWrite (some .release)),
  (.readModifyWrite, 4, .routed .readModifyWrite (some .acqRel)),
  (.readModifyWrite, 5, .routed .readModifyWrite (some .seqCst)),
  (.readModifyWrite, 6, .refused .invalidOrder),
  (.threadFence, -1, .refused .invalidOrder),
  (.threadFence, 0, .routed .threadFence (some .relaxed)),
  (.threadFence, 1, .routed .threadFence (some .acquire)),
  (.threadFence, 2, .routed .threadFence (some .acquire)),
  (.threadFence, 3, .routed .threadFence (some .release)),
  (.threadFence, 4, .routed .threadFence (some .acqRel)),
  (.threadFence, 5, .routed .threadFence (some .seqCst)),
  (.threadFence, 6, .refused .invalidOrder)
]

theorem order_vectors_hold :
    orderVectors.all (fun v => orderOutcome v.1 v.2.1 == v.2.2) = true := by
  decide

end QCore.AtomicSourceRoute
