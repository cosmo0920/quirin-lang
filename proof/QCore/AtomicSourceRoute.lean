import QCore.AtomicPublication

/-!
# C atomics routed to the weak-memory model (card CG19-ATOMIC-SOURCE-ROUTE)

Gateway rule 11i. The Rust side is `crates/qcore-c-gateway/src/atomic_source_route.rs`,
bound to this module by the shared-vector tests in
`crates/qcore-c-gateway/tests/unit/atomic_source_route.rs`: `formTable` equals
`ATOMIC_FORM_TABLE` row for row, `decodeOrder`/`routeOrder` agree with the Rust
functions on every value, and `routeVectors` are run through both.

What is proved here:

* a routed access is a **plain** load, store or initialisation, and its order
  is one the model (`qcore_core::weak_memory`, whose order validity is
  `modelRepresents`) represents for that operation;
* read-modify-writes, compare-exchange, fences and a consume load are refused,
  each by its own name; so is every unlisted builtin;
* the routed store and load orders agree with the bounded publication model
  `QCore.AtomicPublication` on Clang's numbering, so the publication checker
  receives exactly the order the source spelled;
* a publication verdict carries the single-graph assumption, and a failed or
  undecided check is `Unknown`.

Faithful Clang extraction of the builtin name and order operand, and the
Rust-to-Lean correspondence, are stated review premises (trust boundary 28).
-/

namespace QCore.AtomicSourceRoute

inductive FormClass where
  | load | store | init | byPointer | readModifyWrite | compareExchange | fence
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
  ("__atomic_exchange", .readModifyWrite),
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
  ("__sync_fetch_and_add", .readModifyWrite),
  ("__sync_fetch_and_sub", .readModifyWrite),
  ("__sync_add_and_fetch", .readModifyWrite),
  ("__sync_sub_and_fetch", .readModifyWrite),
  ("__sync_lock_test_and_set", .readModifyWrite),
  ("__c11_atomic_compare_exchange_strong", .compareExchange),
  ("__c11_atomic_compare_exchange_weak", .compareExchange),
  ("__atomic_compare_exchange_n", .compareExchange),
  ("__atomic_compare_exchange", .compareExchange),
  ("__sync_bool_compare_and_swap", .compareExchange),
  ("__sync_val_compare_and_swap", .compareExchange),
  ("__c11_atomic_thread_fence", .fence),
  ("__c11_atomic_signal_fence", .fence),
  ("__atomic_thread_fence", .fence),
  ("__atomic_signal_fence", .fence),
  ("__sync_synchronize", .fence)
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

inductive Operation where | load | store | init
deriving DecidableEq, Repr

/-- `qcore_core::MemoryOrder`: the model has no consume order. -/
inductive ModelOrder where | relaxed | acquire | release | acqRel | seqCst
deriving DecidableEq, Repr

inductive Refusal where
  | readModifyWrite | compareExchange | fence | consumeLoad | invalidOrder
  | unevaluatedOrder | byPointer | nonScalarValue | unsupportedShape | unlisted
deriving DecidableEq, Repr

/-- `route_order`: C11 7.17.7.1p2 and 7.17.7.2p2; consume refused by name; an
initialisation is not an atomic operation and takes no order. -/
def routeOrder : Operation → SourceOrder → Except Refusal ModelOrder
  | .init, _ => .error .invalidOrder
  | .load, .consume => .error .consumeLoad
  | _, .relaxed => .ok .relaxed
  | .load, .acquire => .ok .acquire
  | .store, .release => .ok .release
  | .load, .seqCst => .ok .seqCst
  | .store, .seqCst => .ok .seqCst
  | _, _ => .error .invalidOrder

/-- The model's own order validity: `weak_memory::event_order_represented`. -/
def modelRepresents : Operation → ModelOrder → Bool
  | .load, .relaxed | .load, .acquire | .load, .seqCst => true
  | .store, .relaxed | .store, .release | .store, .seqCst => true
  | _, _ => false

def classOperation : FormClass → Except Refusal Operation
  | .load => .ok .load
  | .store => .ok .store
  | .init => .ok .init
  | .byPointer => .error .byPointer
  | .readModifyWrite => .error .readModifyWrite
  | .compareExchange => .error .compareExchange
  | .fence => .error .fence

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

theorem route_class_is_plain (name : String) (order : Option Int)
    (r : Operation × Option ModelOrder) (h : route name order = .ok r) :
    lookup name = some .load ∨ lookup name = some .store ∨ lookup name = some .init := by
  unfold route at h
  split at h
  · cases h
  · rename_i c hc
    cases c <;> simp_all [classOperation]

theorem read_modify_write_refused (name : String) (order : Option Int)
    (h : lookup name = some .readModifyWrite) : route name order = .error .readModifyWrite := by
  simp [route, h, classOperation]

theorem compare_exchange_refused (name : String) (order : Option Int)
    (h : lookup name = some .compareExchange) : route name order = .error .compareExchange := by
  simp [route, h, classOperation]

theorem fence_refused (name : String) (order : Option Int)
    (h : lookup name = some .fence) : route name order = .error .fence := by
  simp [route, h, classOperation]

theorem by_pointer_refused (name : String) (order : Option Int)
    (h : lookup name = some .byPointer) : route name order = .error .byPointer := by
  simp [route, h, classOperation]

/-- A consume load is refused, never read as acquire. -/
theorem consume_load_refused (name : String) (h : lookup name = some .load) :
    route name (some 1) = .error .consumeLoad := by
  simp [route, h, classOperation, decodeOrder, routeOrder]

/-- Order value 1 (consume) is admitted for no load or store: a resolver
that answers 1 for `memory_order_consume` alone is fail-closed. -/
theorem order_one_never_routed (name : String)
    (h : lookup name = some .load ∨ lookup name = some .store)
    (r : Operation × Option ModelOrder) : route name (some 1) ≠ .ok r := by
  rcases h with h | h <;> simp [route, h, classOperation, decodeOrder, routeOrder]

theorem consume_never_routed (op : Operation) (m : ModelOrder) :
    routeOrder op .consume ≠ .ok m := by
  cases op <;> simp [routeOrder]

theorem unlisted_refused (name : String) (order : Option Int) (h : lookup name = none) :
    route name order = .error .unlisted := by
  simp [route, h]

/-- An order the route cannot evaluate is refused for a load or store. -/
theorem unevaluated_order_refused (name : String)
    (h : lookup name = some .load ∨ lookup name = some .store) :
    route name none = .error .unevaluatedOrder := by
  rcases h with h | h <;> simp [route, h, classOperation]

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
deriving DecidableEq, Repr

structure Verdict where
  status : Status
  assumptions : List Assumption
deriving DecidableEq, Repr

/-- The gateway's production gate: the composed check's outcome
(`some true`: checked; `some false`: refused; `none`: undecided). -/
def publicationVerdict (checked : Option Bool) : Verdict where
  status := if checked = some true then .safe else .unknown
  assumptions := [.singleGraph, .callerDeclaredBounds, .faithfulClangAst, .reviewedCorrespondence]

theorem publication_verdict_carries_single_graph (checked : Option Bool) :
    Assumption.singleGraph ∈ (publicationVerdict checked).assumptions := by
  simp [publicationVerdict]

theorem failed_publication_is_unknown :
    (publicationVerdict (some false)).status = .unknown := rfl

theorem undecided_publication_is_unknown :
    (publicationVerdict none).status = .unknown := rfl

theorem safe_publication_only_when_checked (checked : Option Bool)
    (h : (publicationVerdict checked).status = .safe) : checked = some true := by
  unfold publicationVerdict at h
  by_cases hc : checked = some true
  · exact hc
  · simp [hc] at h

/-! ## Shared vectors -/

inductive Outcome where
  | routed (op : Operation) (order : Option ModelOrder)
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
  ("__c11_atomic_fetch_add", some 5, .refused .readModifyWrite),
  ("__atomic_exchange_n", some 5, .refused .readModifyWrite),
  ("__c11_atomic_compare_exchange_strong", some 5, .refused .compareExchange),
  ("__c11_atomic_thread_fence", some 5, .refused .fence),
  ("__atomic_load", some 5, .refused .byPointer),
  ("__atomic_fetch_umax", some 5, .refused .unlisted)
]

theorem route_vectors_hold :
    routeVectors.all (fun v => outcome v.1 v.2.1 == v.2.2) = true := by
  decide

/-- `decode_order` then `route_order` on one operation and value. -/
def orderOutcome (op : Operation) (v : Int) : Outcome :=
  match decodeOrder v with
  | none => .refused .invalidOrder
  | some o =>
    match routeOrder op o with
    | .ok m => .routed op (some m)
    | .error r => .refused r

/-- Every load and store order value from -1 to 6, run through the Rust
functions by the binding test. -/
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
  (.store, 6, .refused .invalidOrder)
]

theorem order_vectors_hold :
    orderVectors.all (fun v => orderOutcome v.1 v.2.1 == v.2.2) = true := by
  decide

end QCore.AtomicSourceRoute
