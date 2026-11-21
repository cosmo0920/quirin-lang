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
import QCore.CjsonRepresentationPremises

/-!
# The librdkafka premises as formulas (card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES)

Contract specification C04.18.9. The 220 `resource_premise` clauses of
`qcore-provider-librdkafka` state sixteen formulas under twelve names; the
terms they use beyond SP1's are the SP2 block of `vocab` and `Model`
(`QCore.PremiseFormula`).

* The sixteen statements are well-formed (`librdkafka_formulas_wf`); the Rust
  test `crates/qcore-frontend/tests/premise_formula_librdkafka.rs` binds each
  definition below to every clause that states it, as read from the modules.
* `#false` is the falsum, so `P -> #false` is the negation of `P`
  (`false_negates`) and `quant all #x in S: #false` says `S` is empty
  (`quant_false_empty`).
* Where the prose says "so" or "because", the stated consequence follows from
  the stated prerequisites in every model where the terms mean what C04.18.9
  says they mean (`AssertCoherent`, `ListCoherent`, `RanCoherent`):
  `asserted_calloc_prerequisites_suffice` (and the malloc and toppar
  statements), `share_ack_prerequisites_suffice`,
  `log_no_event_prerequisites_suffice`. The conclusion conjunct is therefore
  redundant in those models, kept so the formula is never narrower than the
  prose.
* The print statement of `cjson_allocator_hooks` entails its cJSON_New_Item
  statement (`hooks_print_entails_new_item`).
* **Faithfulness of `detach_list_representation`** to the existing formal
  decomposition: in the model a `QCore.CjsonRepresentationPremises`
  environment induces (`detachModel`), the formula holds exactly when the nine
  propositions its decomposition's atoms are bound to hold
  (`detach_formula_is_the_nine_atoms`).

As in `QCore.PremiseFormula`, nothing here establishes that a model describes
librdkafka or cJSON: a premise stays an undischarged assumption.
-/

namespace QCore.PremiseFormula

/-! ## The sixteen statements (the Rust reader's `lean` rendering) -/

def closureFormula : Raw := (.bin .conj (.quant "f" (.bin .diff (.app1 "ran" (.app1 "slots" (.app0 "rk_conf"))) (.app0 "loaded_registered")) (.bin .conj (.bin .mem (.app0 "f") (.bin .union (.app0 "registered") (.app0 "defaults"))) (.bin .mem (.app0 "f") (.app0 "candidates")))) (.quant "f" (.app0 "loaded_registered") (.bin .mem (.app0 "f") (.app0 "enveloped"))))

def confWriterFormula : Raw := (.bin .conj (.quant "f" (.app0 "punned_stores") (.bin .mem (.app0 "f") (.bin .union (.bin .union (.bin .union (.app0 "registered") (.app0 "defaults")) (.app0 "conf_copies")) (.app0 "hidden_string")))) (.quant "f" (.app0 "hidden_string") (.bin .mem (.app0 "f") (.app0 "ut_handle_produce_response"))))

def assertedCallocFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .mapsTo (.app0 "call") (.app1 "returns_result_of" (.app0 "rd_calloc"))) (.bin .mapsTo (.app0 "rd_calloc") (.app0 "asserts_result"))) (.app0 "audited_assertions")) (.bin .mapsTo (.app0 "call") (.app0 "returns_nonnull")))

def assertedTopparFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mapsTo (.app0 "call") (.app1 "returns_result_of" (.app0 "rd_kafka_toppar_new0"))) (.bin .mapsTo (.app0 "rd_kafka_toppar_new0") (.app1 "returns_result_of" (.app0 "rd_calloc")))) (.bin .mapsTo (.app0 "rd_calloc") (.app0 "asserts_result"))) (.app0 "audited_assertions")) (.bin .mapsTo (.app0 "call") (.app0 "returns_nonnull")))

def assertedMallocFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .mapsTo (.app0 "call") (.app1 "returns_result_of" (.app0 "rd_malloc"))) (.bin .mapsTo (.app0 "rd_malloc") (.app0 "asserts_result"))) (.app0 "audited_assertions")) (.bin .mapsTo (.app0 "call") (.app0 "returns_nonnull")))

def assertedDeployedFormula : Raw := (.bin .conj (.bin .conj (.app0 "deployed_assertion") (.app0 "deployed_failure_call")) (.bin .mapsTo (.app0 "call") (.app0 "failure_no_return")))

def servedQueueFormula : Raw := (.bin .mapsTo (.app0 "op_callbacks") (.bin .eqv (.app1 "takes" (.app1 "held_on" (.app0 "served"))) .empty))

def brokerMonitorFormula : Raw := (.bin .conj (.quant "f" (.bin .union (.app0 "broker_monitor_cb") (.app0 "monitor_add_registered")) (.bin .mem (.app0 "f") (.bin .union (.app0 "coord_rkb_monitor_cb") (.app0 "txn_coord_monitor_cb")))) (.bin .mapsTo (.bin .union (.app0 "coord_rkb_monitor_cb") (.app0 "txn_coord_monitor_cb")) (.bin .conj (.bin .conj (.bin .conj (.app1 "handed_only" (.app0 "broker")) (.bin .eqv (.app1 "takes" (.app1 "refs" (.app0 "args"))) .empty)) (.bin .eqv (.app1 "releases" (.app1 "refs" (.app0 "args"))) .empty)) (.bin .eqv (.app1 "retains" (.app1 "refs" (.app0 "args"))) .empty))))

def assignorFormula : Raw := (.bin .conj (.quant "f" (.bin .union (.app0 "assign_cb") (.app0 "assignor_add_registered")) (.bin .mem (.app0 "f") (.bin .union (.bin .union (.app0 "range_assign") (.app0 "roundrobin_assign")) (.app0 "cooperative_sticky_assign")))) (.bin .mapsTo (.bin .union (.bin .union (.app0 "range_assign") (.app0 "roundrobin_assign")) (.app0 "cooperative_sticky_assign")) (.bin .conj (.bin .conj (.bin .conj (.app1 "reads_only" (.bin .union (.app0 "metadata") (.app0 "members"))) (.bin .eqv (.app1 "takes" (.app1 "refs" (.app0 "op"))) .empty)) (.bin .eqv (.app1 "releases" (.app1 "refs" (.app0 "op"))) .empty)) (.bin .eqv (.app1 "retains" (.app1 "refs" (.app0 "op"))) .empty))))

def shareAckFormula : Raw := (.bin .conj (.bin .conj (.quant "l" (.app0 "ack_batches") (.bin .mem (.app0 "l") (.app0 "ack_details"))) (.quant "l" (.app0 "ack_details") (.bin .mem (.app0 "l") (.app0 "first_add_lists")))) (.quant "l" (.app0 "ack_batches") (.bin .mem (.app0 "l") (.app0 "nonempty"))))

def logNoEventFormula : Raw := (.bin .conj (.quant "f" (.app1 "event_cb" (.app1 "forwards" (.app0 "logq"))) (.app0 "false")) (.quant "f" (.app1 "ran" (.app1 "event_cb" (.app1 "forwards" (.app0 "logq")))) (.app0 "false")))

def logForwardFormula : Raw := (.quant "q" (.app0 "fwd_queue") (.bin .conj (.bin .imp (.bin .mem (.app0 "q") (.app0 "freed")) (.app0 "false")) (.bin .imp (.bin .conj (.bin .mem (.app0 "q") (.app0 "unforwarded")) (.bin .mem (.app0 "q") (.app0 "last_released"))) (.app0 "false"))))

def hooksNewItemFormula : Raw := (.bin .conj (.bin .conj (.quant "f" (.bin .union (.app0 "cjson_allocate") (.app0 "cjson_deallocate")) (.bin .mem (.app0 "f") (.bin .union (.app0 "libc_hooks") (.app0 "installed_hooks")))) (.bin .mapsTo (.app0 "cjson_allocate") (.app0 "fresh_or_null"))) (.quant "o" (.app0 "hook_allocated") (.bin .mem (.app0 "o") (.app0 "deallocate_releases"))))

def hooksPrintFormula : Raw := (.bin .conj (.bin .conj (.quant "f" (.bin .union (.bin .union (.app0 "cjson_allocate") (.app0 "cjson_reallocate")) (.app0 "cjson_deallocate")) (.bin .mem (.app0 "f") (.bin .union (.app0 "libc_hooks") (.app0 "installed_hooks")))) (.bin .mapsTo (.bin .union (.app0 "cjson_allocate") (.app0 "cjson_reallocate")) (.app0 "fresh_or_null"))) (.quant "o" (.app0 "hook_allocated") (.bin .mem (.app0 "o") (.app0 "deallocate_releases"))))

def listInvariantFormula : Raw := (.quant "a" (.app0 "cjson_arrays") (.quant "c" (.app1 "child" (.app0 "a")) (.bin .conj (.quant "l" (.app1 "last" (.app0 "a")) (.bin .mem (.app0 "l") (.app1 "prev" (.app0 "c")))) (.bin .imp (.quant "p" (.app1 "prev" (.app0 "c")) (.app0 "false")) (.app0 "false")))))

def detachFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .conj (.bin .mem (.app0 "parent") (.app0 "live")) (.quant "c" (.app1 "owned" (.app0 "parent")) (.bin .mem (.app0 "c") (.app1 "children" (.app0 "parent"))))) (.quant "c" (.app1 "children" (.app0 "parent")) (.bin .conj (.bin .mem (.app0 "c") (.app1 "owned" (.app0 "parent"))) (.bin .mem (.app0 "c") (.app0 "live"))))) (.quant "c" (.app1 "child" (.app0 "parent")) (.bin .mem (.app0 "c") (.app1 "first" (.app0 "parent"))))) (.quant "c" (.app1 "first" (.app0 "parent")) (.bin .mem (.app0 "c") (.app1 "child" (.app0 "parent"))))) (.quant "b" (.app1 "children" (.app0 "parent")) (.quant "a" (.app2 "pred" (.app0 "parent") (.app0 "b")) (.bin .mem (.app0 "b") (.app1 "next" (.app0 "a")))))) (.quant "c" (.app1 "children" (.app0 "parent")) (.bin .imp (.bin .mem (.app0 "c") (.app1 "next_reach" (.app0 "c"))) (.app0 "false")))) (.bin .imp (.quant "c" (.app1 "child" (.app0 "parent")) (.app0 "false")) (.quant "c" (.app1 "children" (.app0 "parent")) (.app0 "false")))) (.bin .imp (.quant "c" (.app1 "children" (.app0 "parent")) (.app0 "false")) (.quant "c" (.app1 "child" (.app0 "parent")) (.app0 "false")))) (.quant "b" (.app1 "children" (.app0 "parent")) (.quant "a" (.app2 "pred" (.app0 "parent") (.app0 "b")) (.bin .mem (.app0 "a") (.app1 "prev" (.app0 "b")))))) (.quant "f" (.app1 "first" (.app0 "parent")) (.quant "l" (.app1 "last" (.app0 "parent")) (.bin .mem (.app0 "l") (.app1 "prev" (.app0 "f")))))) (.app1 "nodup" (.app0 "parent"))) (.quant "c" (.app1 "children" (.app0 "parent")) (.quant "y" (.bin .union (.app1 "next" (.app0 "c")) (.app1 "prev" (.app0 "c"))) (.bin .conj (.bin .mem (.app0 "y") (.app1 "children" (.app0 "parent"))) (.bin .mem (.app0 "y") (.app0 "live")))))) (.quant "l" (.app1 "last" (.app0 "parent")) (.quant "y" (.app1 "next" (.app0 "l")) (.app0 "false")))) (.bin .imp (.bin .mem (.app0 "parent") (.app1 "children" (.app0 "parent"))) (.app0 "false"))) (.bin .mapsTo (.app0 "concurrent") (.app1 "keeps_links" (.app0 "parent"))))

theorem librdkafka_formulas_wf :
    wf closureFormula = true ∧ wf confWriterFormula = true ∧
      wf assertedCallocFormula = true ∧ wf assertedTopparFormula = true ∧
      wf assertedMallocFormula = true ∧ wf assertedDeployedFormula = true ∧
      wf servedQueueFormula = true ∧ wf brokerMonitorFormula = true ∧
      wf assignorFormula = true ∧ wf shareAckFormula = true ∧
      wf logNoEventFormula = true ∧ wf logForwardFormula = true ∧
      wf hooksNewItemFormula = true ∧ wf hooksPrintFormula = true ∧
      wf listInvariantFormula = true ∧ wf detachFormula = true := by decide

/-! ## The falsum -/

theorem false_negates (M : Model) (p : Raw) (q : Prop) (h : eval M [] p = .pr q) :
    eval M [] (.bin .imp p (.app0 "false")) = .pr (¬ q) := by
  simp [eval, h, lk, prim0, binVal]

theorem quant_false_empty (M : Model) (d : Raw) (s : M.U → Prop) (h : eval M [] d = .st s) :
    (eval M [] (.quant "x" d (.app0 "false"))).holds ↔ ∀ u, ¬ s u := by
  simp [eval, h, lk, prim0, Val.holds]

/-! ## asserted_allocation: "non-null because …, so it never returns NULL" -/

/-- What the allocation terms mean together: a run returning what a run of `g`
returned has a non-null result when that run's is, and, with the audited
build's assertions compiled in, a run that asserts its result non-null returns
non-null (a failed assertion does not return). -/
def Model.AssertCoherent (M : Model) : Prop :=
  (∀ g r, M.returnsResultOf g r → ∃ r', M.runs g r' ∧ (M.returnsNonnull r' → M.returnsNonnull r)) ∧
  (M.auditedAssertions → ∀ r, M.assertsResult r → M.returnsNonnull r)

theorem eval_asserted_calloc (M : Model) :
    (eval M [] assertedCallocFormula).holds ↔
      (((∀ r, M.runs M.call r → M.returnsResultOf M.rdCalloc r) ∧
          (∀ r, M.runs M.rdCalloc r → M.assertsResult r)) ∧ M.auditedAssertions) ∧
        (∀ r, M.runs M.call r → M.returnsNonnull r) := by
  simp [eval, assertedCallocFormula, lk, prim0, prim1, binVal, Val.holds]

/-- A result returned from an asserting allocator is non-null. -/
theorem returned_from_asserting_nonnull (M : Model) (hC : M.AssertCoherent) (g : M.U)
    (h1 : ∀ r, M.runs M.call r → M.returnsResultOf g r)
    (h2 : ∀ r, M.runs g r → M.assertsResult r) (h3 : M.auditedAssertions) :
    ∀ r, M.runs M.call r → M.returnsNonnull r := by
  intro r hr
  obtain ⟨r', hr', himp⟩ := hC.1 _ _ (h1 r hr)
  exact himp (hC.2 h3 r' (h2 r' hr'))

theorem asserted_calloc_prerequisites_suffice (M : Model) (hC : M.AssertCoherent)
    (h1 : ∀ r, M.runs M.call r → M.returnsResultOf M.rdCalloc r)
    (h2 : ∀ r, M.runs M.rdCalloc r → M.assertsResult r) (h3 : M.auditedAssertions) :
    (eval M [] assertedCallocFormula).holds :=
  (eval_asserted_calloc M).2
    ⟨⟨⟨h1, h2⟩, h3⟩, returned_from_asserting_nonnull M hC _ h1 h2 h3⟩

theorem eval_asserted_malloc (M : Model) :
    (eval M [] assertedMallocFormula).holds ↔
      (((∀ r, M.runs M.call r → M.returnsResultOf M.rdMalloc r) ∧
          (∀ r, M.runs M.rdMalloc r → M.assertsResult r)) ∧ M.auditedAssertions) ∧
        (∀ r, M.runs M.call r → M.returnsNonnull r) := by
  simp [eval, assertedMallocFormula, lk, prim0, prim1, binVal, Val.holds]

theorem asserted_malloc_prerequisites_suffice (M : Model) (hC : M.AssertCoherent)
    (h1 : ∀ r, M.runs M.call r → M.returnsResultOf M.rdMalloc r)
    (h2 : ∀ r, M.runs M.rdMalloc r → M.assertsResult r) (h3 : M.auditedAssertions) :
    (eval M [] assertedMallocFormula).holds :=
  (eval_asserted_malloc M).2
    ⟨⟨⟨h1, h2⟩, h3⟩, returned_from_asserting_nonnull M hC _ h1 h2 h3⟩

theorem eval_asserted_toppar (M : Model) :
    (eval M [] assertedTopparFormula).holds ↔
      ((((∀ r, M.runs M.call r → M.returnsResultOf M.rdKafkaTopparNew0 r) ∧
          (∀ r, M.runs M.rdKafkaTopparNew0 r → M.returnsResultOf M.rdCalloc r)) ∧
          (∀ r, M.runs M.rdCalloc r → M.assertsResult r)) ∧ M.auditedAssertions) ∧
        (∀ r, M.runs M.call r → M.returnsNonnull r) := by
  simp [eval, assertedTopparFormula, lk, prim0, prim1, binVal, Val.holds]

/-- The last source's own allocation is asserted, so the result is non-null
through two returns. -/
theorem asserted_toppar_prerequisites_suffice (M : Model) (hC : M.AssertCoherent)
    (h1 : ∀ r, M.runs M.call r → M.returnsResultOf M.rdKafkaTopparNew0 r)
    (h2 : ∀ r, M.runs M.rdKafkaTopparNew0 r → M.returnsResultOf M.rdCalloc r)
    (h3 : ∀ r, M.runs M.rdCalloc r → M.assertsResult r) (h4 : M.auditedAssertions) :
    (eval M [] assertedTopparFormula).holds := by
  refine (eval_asserted_toppar M).2 ⟨⟨⟨⟨h1, h2⟩, h3⟩, h4⟩, ?_⟩
  intro r hr
  obtain ⟨r', hr', himp⟩ := hC.1 _ _ (h1 r hr)
  obtain ⟨r'', hr'', himp'⟩ := hC.1 _ _ (h2 r' hr')
  exact himp (himp' (hC.2 h4 r'' (h3 r'' hr'')))

/-! ## share_ack_batches_nonempty: "NULL or non-empty: its producer …" -/

/-- A list created only at its first `rd_list_add` holds an element. -/
def Model.ListCoherent (M : Model) : Prop := ∀ l, M.firstAddLists l → M.nonempty l

theorem eval_share_ack (M : Model) :
    (eval M [] shareAckFormula).holds ↔
      ((∀ l, M.ackBatches l → M.ackDetails l) ∧ (∀ l, M.ackDetails l → M.firstAddLists l)) ∧
        ∀ l, M.ackBatches l → M.nonempty l := by
  simp [eval, shareAckFormula, lk, prim0, binVal, Val.holds]

theorem share_ack_prerequisites_suffice (M : Model) (hC : M.ListCoherent)
    (h1 : ∀ l, M.ackBatches l → M.ackDetails l)
    (h2 : ∀ l, M.ackDetails l → M.firstAddLists l) :
    (eval M [] shareAckFormula).holds :=
  (eval_share_ack M).2 ⟨⟨h1, h2⟩, fun l hl => hC l (h2 l (h1 l hl))⟩

/-! ## rk_log_enqueue_no_event_callback: "no … is registered …, so rd_kafka_q_io_event runs no callback" -/

/-- The call runs only members of a set it runs from. -/
def Model.RanCoherent (M : Model) : Prop := ∀ s f, M.ran s f → s f

theorem eval_log_no_event (M : Model) :
    (eval M [] logNoEventFormula).holds ↔
      (∀ f, ¬ M.eventCb (M.forwards M.logq) f) ∧
        ∀ f, ¬ M.ran (M.eventCb (M.forwards M.logq)) f := by
  simp [eval, logNoEventFormula, lk, prim0, prim1, binVal, Val.holds]

theorem log_no_event_prerequisites_suffice (M : Model) (hC : M.RanCoherent)
    (h : ∀ f, ¬ M.eventCb (M.forwards M.logq) f) :
    (eval M [] logNoEventFormula).holds :=
  (eval_log_no_event M).2 ⟨h, fun f hf => h f (hC _ f hf)⟩

/-! ## cjson_allocator_hooks: the print statement widens the cJSON_New_Item one -/

theorem hooks_print_entails_new_item (M : Model) :
    (eval M [] hooksPrintFormula).holds → (eval M [] hooksNewItemFormula).holds := by
  simp only [eval, hooksPrintFormula, hooksNewItemFormula, lk, prim0, binVal, Val.holds]
  rintro ⟨⟨h1, h2⟩, h3⟩
  refine ⟨⟨fun u hu => h1 u ?_, fun f hf => h2 f (Or.inl hf)⟩, h3⟩
  rcases hu with hu | hu
  · exact Or.inl (Or.inl hu)
  · exact Or.inr hu

/-! ## detach_list_representation is the nine Lean atoms -/

open QCore.CjsonRepresentationPremises in
/-- The model a cJSON entry environment (`QCore.CjsonRepresentationPremises`)
induces: objects are its `Nat` identities, the runs are its trace's steps, and
one callback, `0`, stands for every step another actor takes. Each list term is
read off the environment exactly as its C04.18.9 meaning says (the child
sequence is `children`, the links `child`, `next`, `prev`). -/
def detachModel (e : Environment) : Model where
  U := Nat
  Run := Step
  args := fun _ => False
  created := fun _ => False
  copied := fun _ => False
  freed := fun _ => False
  cls := id
  ctx := fun _ _ => False
  bios := fun _ _ => False
  chain := fun _ _ => False
  sslOf := fun _ _ => False
  slots := fun _ _ => False
  stated := fun _ => False
  exData := fun _ _ => False
  exDataFree := fun _ _ => False
  method := id
  builtin := fun _ => False
  fns := fun _ _ => False
  store := fun _ _ => False
  lookups := fun _ _ => False
  teardown := fun _ _ => False
  settable := fun _ => False
  inTurn := fun _ _ _ => False
  callerHeld := fun _ _ => False
  takes := fun _ _ _ => False
  releases := fun _ _ _ => False
  retains := fun _ _ _ => False
  runs := fun f s => f = 0 ∧ s ∈ e.trace ∧ s.actor ≠ e.callActor
  const := fun _ => 0
  parent := e.parent
  live := e.live
  owned := fun o x => o = e.parent ∧ e.ownedChild x
  children := fun o x => o = e.parent ∧ x ∈ e.children
  child := fun o x => o = e.parent ∧ e.child = some x
  first := fun o x => o = e.parent ∧ e.children.head? = some x
  last := fun o x => o = e.parent ∧ e.children.getLast? = some x
  pred := fun o b a => o = e.parent ∧ ∃ before suffix, e.children = before ++ a :: b :: suffix
  next := fun o x => e.next o = some x
  prev := fun o x => e.prev o = some x
  nextReach := fun o x => NextPath e.next o x
  nodup := fun o => o = e.parent ∧ e.children.Nodup
  concurrent := fun f => f = 0
  keepsLinks := fun _ s => s.writesLinks = false

open QCore.CjsonRepresentationPremises in
/-- What the detach formula states of an environment's model. -/
theorem eval_detach (e : Environment) :
    (eval (detachModel e) [] detachFormula).holds ↔
      ((((((((((((((e.live e.parent ∧ ∀ (u : Nat), e.ownedChild u → u ∈ e.children) ∧
          ∀ (u : Nat), u ∈ e.children → e.ownedChild u ∧ e.live u) ∧
          ∀ (u : Nat), e.child = some u → e.children.head? = some u) ∧
          ∀ (u : Nat), e.children.head? = some u → e.child = some u) ∧
          ∀ (u : Nat), u ∈ e.children →
            ∀ (u_1 : Nat) (x x_1 : List Nat), e.children = x ++ u_1 :: u :: x_1 →
              e.next u_1 = some u) ∧
          ∀ (u : Nat), u ∈ e.children → ¬NextPath e.next u u) ∧
          ((∀ (u : Nat), ¬e.child = some u) → ∀ (u : Nat), ¬u ∈ e.children)) ∧
          ((∀ (u : Nat), ¬u ∈ e.children) → ∀ (u : Nat), ¬e.child = some u)) ∧
          ∀ (u : Nat), u ∈ e.children →
            ∀ (u_1 : Nat) (x x_1 : List Nat), e.children = x ++ u_1 :: u :: x_1 →
              e.prev u = some u_1) ∧
          ∀ (u : Nat), e.children.head? = some u →
            ∀ (u_1 : Nat), e.children.getLast? = some u_1 → e.prev u = some u_1) ∧
          e.children.Nodup) ∧
          ∀ (u : Nat), u ∈ e.children → ∀ (u_1 : Nat),
            e.next u = some u_1 ∨ e.prev u = some u_1 → u_1 ∈ e.children ∧ e.live u_1) ∧
          ∀ (u : Nat), e.children.getLast? = some u → ∀ (u_1 : Nat), ¬e.next u = some u_1) ∧
          ¬e.parent ∈ e.children) ∧
          ∀ (r : Step), r ∈ e.trace → ¬r.actor = e.callActor → r.writesLinks = false := by
  simp [eval, detachFormula, lk, prim0, prim1, prim2, binVal, Val.holds, detachModel]

/-- Two options agree when each value of one is the other's. -/
theorem option_eq_of_values {α : Type} {a b : Option α}
    (h1 : ∀ u, a = some u → b = some u) (h2 : ∀ u, b = some u → a = some u) : a = b := by
  cases a with
  | none => cases b with
    | none => rfl
    | some v => exact absurd (h2 v rfl) (by simp)
  | some v => exact (h1 v rfl).symm

open QCore.CjsonRepresentationPremises in
/-- **Faithfulness of `detach_list_representation`.** In the model an entry
environment induces, the premise's formula holds exactly when the nine
propositions of `QCore.CjsonRepresentationPremises` its decomposition's atoms
are bound to hold (in the decomposition's order): the formula is the prose
statement's conjunction, conjunct for conjunct, neither narrower nor wider. -/
theorem detach_formula_is_the_nine_atoms (e : Environment) :
    (eval (detachModel e) [] detachFormula).holds ↔
      chain e ∧ distinct e ∧ empty e ∧ firstPrev e ∧ lastNext e ∧ linkTargets e ∧
        noWriter e ∧ parentOutside e ∧ predecessor e := by
  rw [eval_detach]
  constructor
  · rintro ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨hlive, hown1⟩, hown2⟩, hc1⟩, hc2⟩, hadj⟩, hacyc⟩, he1⟩, he2⟩,
      hpred⟩, hfp⟩, hnd⟩, hlt⟩, hln⟩, hpo⟩, hnw⟩
    refine ⟨⟨hlive, fun x => ⟨hown1 x, fun hx => (hown2 x hx).1⟩, fun x hx => (hown2 x hx).2,
      option_eq_of_values hc1 hc2, ?_, hacyc⟩, hnd, ?_, fun f l hf hl => hfp f hf l hl, ?_, hlt,
      hnw, hpo, ?_⟩
    · intro before a b suffix hs
      exact hadj b (by rw [hs]; simp) a before suffix hs
    · constructor
      · intro hc
        cases hcs : e.children with
        | nil => rfl
        | cons x xs =>
            exact absurd (hcs ▸ List.mem_cons_self ..) (he1 (fun u h => by simp [hc] at h) x)
      · intro hcs
        cases hc : e.child with
        | none => rfl
        | some x => exact absurd hc (he2 (fun u h => by simp [hcs] at h) x)
    · intro l hl
      cases hn : e.next l with
      | none => rfl
      | some y => exact absurd hn (hln l hl y)
    · intro before a b suffix hs
      exact hpred b (by rw [hs]; simp) a before suffix hs
  · rintro ⟨⟨hlive, hown, hlive', hhead, hadj, hacyc⟩, hnd, ⟨he1, he2⟩, hfp, hln, hlt, hnw,
      hpo, hpred⟩
    refine ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨hlive, fun u hu => (hown u).1 hu⟩,
      fun u hu => ⟨(hown u).2 hu, hlive' u hu⟩⟩, fun u hu => hhead ▸ hu⟩,
      fun u hu => hhead ▸ hu⟩, fun u _ a x y hs => hadj x a u y hs⟩, hacyc⟩, ?_⟩, ?_⟩,
      fun u _ a x y hs => hpred x a u y hs⟩, fun u hu l hl => hfp u l hu hl⟩, hnd⟩, hlt⟩, ?_⟩, hpo⟩, hnw⟩
    · intro hc u hu
      have : e.child = none := by
        cases h : e.child with
        | none => rfl
        | some v => exact absurd h (hc v)
      rw [he1 this] at hu
      exact absurd hu (by simp)
    · intro hcs u hu
      have : e.children = [] := by
        cases h : e.children with
        | nil => rfl
        | cons x xs => exact absurd (h ▸ List.mem_cons_self ..) (hcs x)
      rw [he2 this] at hu
      exact absurd hu (by simp)
    · intro u hu y hy
      rw [hln u hu] at hy
      exact absurd hy (by simp)

end QCore.PremiseFormula
