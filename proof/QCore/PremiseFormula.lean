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
# Symbolic premise statements (card SP1-SYMBOLIC-OPENSSL-PREMISES)

Contract specification C04.18.9, grammar G05. A named premise may state
`requires resource_premise(NAME, FORMULA);`, a formula over a closed
vocabulary of sorted terms. This module is the formula language's model:

* **Syntax** (`Raw`): the tree `crates/qcore-frontend/src/premise_formula.rs`
  reads (`parse_tokens`), rendered by its `lean` function.
* **Sorts and well-formedness** (`check`, `wf`): the judgment the loader
  applies (`check` there): every term registered with its signature
  (`vocab`), every constant a class, every variable bound and
  never shadowing, the whole a proposition.
* **Denotation** (`eval`) over an abstract model (`Model`) of the call's
  objects, callback sets and the runs of each callback.

Theorems:

* well-formed formulas denote (`wf_denotes`, from `check_denotes`), and so
  does a well-formed quantifier's body at every element of its domain
  (`quant_body_denotes`);
* the loader's acceptance agrees with the judgment on the shared decision
  vectors (`decision_vectors_agree`; bound to the Rust table
  `DECISION_VECTORS` by `crates/qcore-frontend/tests/premise_formula.rs`,
  which also binds `vocab` and the four formulas below to the Rust reader);
* the six OpenSSL statements are well-formed (`openssl_formulas_wf`);
* `#inert` is its definition (`inert_definition`);
* `openssl_unstated_callback_slots` entails `openssl_ex_data_callbacks` in
  every coherent model (`unstated_entails_ex_data`), in particular for the
  SSL object's ex_data class (`unstated_entails_ex_data_for_ssl`).

Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES appends one block of Fluent Bit
terms (`vocab`, `FluentBitTerms`, `prim0`, `prim1`, `decisionVectors`); its
formulas and theorems are in `QCore.PremiseFormula.FluentBit`.

## What this is not

A premise is an undischarged assumption (C04.18): nothing here, and no
consumer that decides a verdict, establishes that a model describes OpenSSL.
That the Rust reader computes `check` is pinned by the shared vectors, not
proved; syntax rejections (a malformed token sequence) are the Rust reader's
and are tested there.
-/

namespace QCore.PremiseFormula

/-! ## Syntax -/

inductive Srt where
  | obj | cls | fn | meth | res | sobj | sfn | smeth | sres | prop | eff
  deriving DecidableEq, Repr

inductive Op where
  | imp | conj | mapsTo | mem | eqv | union | diff
  deriving DecidableEq, Repr

inductive Raw where
  | quant (v : String) (dom body : Raw)
  | bin (op : Op) (l r : Raw)
  | app0 (f : String)
  | app1 (f : String) (a : Raw)
  | app2 (f : String) (a b : Raw)
  | const (c : String)
  | empty
  deriving DecidableEq, Repr

/-! ## The closed vocabulary (the Rust `VOCABULARY`, `CLASSES`) -/

def vocab : List (String × List Srt × Srt) := [
  ("args", [], .sobj),
  ("created", [], .sobj),
  ("copied", [], .sobj),
  ("freed", [], .sobj),
  ("class", [.obj], .cls),
  ("of", [.cls, .sobj], .sobj),
  ("ctx", [.sobj], .sobj),
  ("bios", [.sobj], .sobj),
  ("chain", [.sobj], .sobj),
  ("ssl_of", [.obj], .sobj),
  ("slots", [.sobj], .sfn),
  ("stated", [], .sfn),
  ("ex_data", [.cls], .sfn),
  ("ex_data_free", [.cls], .sfn),
  ("method", [.obj], .meth),
  ("builtin", [], .smeth),
  ("fns", [.obj], .sfn),
  ("store", [.sobj], .sobj),
  ("lookups", [.sobj], .sobj),
  ("teardown", [.obj], .sfn),
  ("settable", [], .sfn),
  ("library", [], .sfn),
  ("loaded", [], .sfn),
  ("in_turn", [.fn, .sobj], .sfn),
  ("caller_held", [], .sres),
  ("takes", [.sres], .sres),
  ("releases", [.sres], .sres),
  ("retains", [.sres], .sres),
  ("inert", [], .eff),
  -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the qcore-provider-librdkafka block.
  ("false", [], .prop),
  ("rk_conf", [], .sobj),
  ("ran", [.sfn], .sfn),
  ("registered", [], .sfn),
  ("defaults", [], .sfn),
  ("candidates", [], .sfn),
  ("loaded_registered", [], .sfn),
  ("enveloped", [], .sfn),
  ("punned_stores", [], .sfn),
  ("conf_copies", [], .sfn),
  ("hidden_string", [], .sfn),
  ("ut_handle_produce_response", [], .sfn),
  ("call", [], .fn),
  ("rd_calloc", [], .fn),
  ("rd_malloc", [], .fn),
  ("rd_kafka_toppar_new0", [], .fn),
  ("returns_result_of", [.fn], .eff),
  ("asserts_result", [], .eff),
  ("returns_nonnull", [], .eff),
  ("audited_assertions", [], .prop),
  ("deployed_assertion", [], .prop),
  ("deployed_failure_call", [], .prop),
  ("failure_no_return", [], .eff),
  ("op_callbacks", [], .sfn),
  ("served", [], .sobj),
  ("held_on", [.sobj], .sres),
  ("refs", [.sobj], .sres),
  ("broker_monitor_cb", [], .sfn),
  ("monitor_add_registered", [], .sfn),
  ("coord_rkb_monitor_cb", [], .sfn),
  ("txn_coord_monitor_cb", [], .sfn),
  ("broker", [], .sobj),
  ("handed_only", [.sobj], .eff),
  ("assign_cb", [], .sfn),
  ("assignor_add_registered", [], .sfn),
  ("range_assign", [], .sfn),
  ("roundrobin_assign", [], .sfn),
  ("cooperative_sticky_assign", [], .sfn),
  ("metadata", [], .sobj),
  ("members", [], .sobj),
  ("reads_only", [.sobj], .eff),
  ("op", [], .sobj),
  ("ack_batches", [], .sobj),
  ("ack_details", [], .sobj),
  ("first_add_lists", [], .sobj),
  ("nonempty", [], .sobj),
  ("logq", [], .sobj),
  ("forwards", [.sobj], .sobj),
  ("event_cb", [.sobj], .sfn),
  ("fwd_queue", [], .sobj),
  ("unforwarded", [], .sobj),
  ("last_released", [], .sobj),
  ("cjson_allocate", [], .sfn),
  ("cjson_reallocate", [], .sfn),
  ("cjson_deallocate", [], .sfn),
  ("libc_hooks", [], .sfn),
  ("installed_hooks", [], .sfn),
  ("fresh_or_null", [], .eff),
  ("hook_allocated", [], .sobj),
  ("deallocate_releases", [], .sobj),
  ("cjson_arrays", [], .sobj),
  ("children", [.obj], .sobj),
  ("first", [.obj], .sobj),
  ("last", [.obj], .sobj),
  ("pred", [.obj, .obj], .sobj),
  ("child", [.obj], .sobj),
  ("next", [.obj], .sobj),
  ("prev", [.obj], .sobj),
  ("next_reach", [.obj], .sobj),
  ("owned", [.obj], .sobj),
  ("nodup", [.obj], .prop),
  ("live", [], .sobj),
  ("parent", [], .obj),
  ("concurrent", [], .sfn),
  ("keeps_links", [.obj], .eff),
  -- End of the SP2 block.
  -- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: the Fluent Bit terms.
  ("result", [], .sobj),
  ("interior", [.sobj], .sobj),
  ("chunk_name", [.sobj], .sobj),
  ("tracked", [], .sobj),
  ("claimed", [], .sobj),
  ("bound", [], .sobj),
  ("stored", [], .sobj),
  ("faithful", [], .prop),
  ("admitted", [], .prop),
  ("config_of", [.sobj], .sobj),
  ("ingesting", [], .sobj),
  ("collector", [], .sobj),
  ("event_registered", [], .sobj),
  ("set_running", [], .sobj),
  ("returns_zero", [], .prop),
  ("in_tree", [], .sobj),
  ("synchronous", [], .sobj),
  ("proxy", [], .sobj),
  ("production", [], .prop),
  ("out_plugin", [.sobj], .sobj)
]

def classes : List String :=
  ["BIO", "DH", "DSA", "EC_KEY", "ENGINE", "EVP_PKEY", "RSA", "SSL", "SSL_CTX",
    "SSL_SESSION", "UI", "UI_METHOD", "X509", "X509_STORE", "X509_STORE_CTX"]

/-- A context or environment lookup: the most recent binding first. -/
def lk {α : Type} : List (String × α) → String → Option α
  | [], _ => none
  | (k, v) :: rest, x => if k = x then some v else lk rest x

def term (f : String) : Option (List Srt × Srt) := lk vocab f

def Srt.setOf : Srt → Option Srt
  | .obj => some .sobj
  | .fn => some .sfn
  | .meth => some .smeth
  | _ => none

def Srt.elem : Srt → Option Srt
  | .sobj => some .obj
  | .sfn => some .fn
  | .smeth => some .meth
  | _ => none

/-- The sort of a binary operation (the Rust `sort_of`'s `Raw::Bin` arm). -/
def binSort : Op → Srt → Srt → Option Srt
  | .imp, .prop, .prop => some .prop
  | .conj, .prop, .prop => some .prop
  | .conj, .eff, .eff => some .eff
  | .mapsTo, .fn, .eff => some .prop
  | .mapsTo, .sfn, .eff => some .prop
  | .mem, a, b => if a.setOf = some b then some .prop else none
  | .eqv, .sres, .sres => some .eff
  | .union, .sobj, .sobj => some .sobj
  | .union, .sfn, .sfn => some .sfn
  | .union, .smeth, .smeth => some .smeth
  | .diff, .sobj, .sobj => some .sobj
  | .diff, .sfn, .sfn => some .sfn
  | .diff, .smeth, .smeth => some .smeth
  | _, _, _ => none

def bound (Γ : List (String × Srt)) (v : String) : Bool :=
  (lk Γ v).isSome

/-- The sort judgment (the Rust `sort_of`). -/
def check (Γ : List (String × Srt)) : Raw → Option Srt
  | .quant v d b =>
      if (term v).isSome || bound Γ v then none
      else
        match (check Γ d).bind Srt.elem with
        | some e => if check ((v, e) :: Γ) b = some .prop then some .prop else none
        | none => none
  | .bin op l r =>
      match check Γ l, check Γ r with
      | some a, some c => binSort op a c
      | _, _ => none
  | .app0 f =>
      match lk Γ f with
      | some s => some s
      | none =>
          match term f with
          | some ([], s) => some s
          | _ => none
  | .app1 f a =>
      match term f, check Γ a with
      | some ([x], s), some y => if x = y then some s else none
      | _, _ => none
  | .app2 f a b =>
      match term f, check Γ a, check Γ b with
      | some ([x, y], s), some x', some y' => if x = x' ∧ y = y' then some s else none
      | _, _, _ => none
  | .const c => if classes.contains c then some .cls else none
  | .empty => some .sres

/-- The well-formedness judgment: a closed proposition (the Rust `check`). -/
def wf (t : Raw) : Bool := check [] t == some .prop

/-! ## Denotation -/

/-- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: one component per Fluent Bit
term, each its own interpretation (C04.18.9's Fluent Bit block), so a model
says what these terms mean of the call and nothing more. Sets of objects are
predicates on the model's elements; the three propositions are `Prop`s. -/
structure FluentBitTerms (U : Type) where
  result : U → Prop
  interior : (U → Prop) → U → Prop
  chunkName : (U → Prop) → U → Prop
  tracked : U → Prop
  claimed : U → Prop
  bound : U → Prop
  stored : U → Prop
  faithful : Prop
  admitted : Prop
  configOf : (U → Prop) → U → Prop
  ingesting : U → Prop
  collector : U → Prop
  eventRegistered : U → Prop
  setRunning : U → Prop
  returnsZero : Prop
  inTree : U → Prop
  synchronous : U → Prop
  proxy : U → Prop
  production : Prop
  outPlugin : (U → Prop) → U → Prop

/-- The Fluent Bit components that hold nothing: every set empty, every
proposition false. -/
def FluentBitTerms.none (U : Type) : FluentBitTerms U where
  result := fun _ => False
  interior := fun _ _ => False
  chunkName := fun _ _ => False
  tracked := fun _ => False
  claimed := fun _ => False
  bound := fun _ => False
  stored := fun _ => False
  faithful := False
  admitted := False
  configOf := fun _ _ => False
  ingesting := fun _ => False
  collector := fun _ => False
  eventRegistered := fun _ => False
  setRunning := fun _ => False
  returnsZero := False
  inTree := fun _ => False
  synchronous := fun _ => False
  proxy := fun _ => False
  production := False
  outPlugin := fun _ _ => False

/-- An abstract model of one call: its objects, classes, methods, callbacks
are elements of `U`; sets are predicates; `runs f` are the runs
of callback `f` during the call; the resource terms are read per run. -/
structure Model where
  U : Type
  Run : Type
  args : U → Prop
  created : U → Prop
  copied : U → Prop
  freed : U → Prop
  cls : U → U
  ctx : (U → Prop) → U → Prop
  bios : (U → Prop) → U → Prop
  chain : (U → Prop) → U → Prop
  sslOf : U → U → Prop
  slots : (U → Prop) → U → Prop
  stated : U → Prop
  exData : U → U → Prop
  exDataFree : U → U → Prop
  method : U → U
  builtin : U → Prop
  fns : U → U → Prop
  store : (U → Prop) → U → Prop
  lookups : (U → Prop) → U → Prop
  teardown : U → U → Prop
  settable : U → Prop
  library : U → Prop
  loaded : U → Prop
  inTurn : U → (U → Prop) → U → Prop
  callerHeld : Run → U → Prop
  takes : Run → (U → Prop) → U → Prop
  releases : Run → (U → Prop) → U → Prop
  retains : Run → (U → Prop) → U → Prop
  runs : U → Run → Prop
  const : String → U
  -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the librdkafka block, each with a
  -- default so a model written before it still elaborates. `#false` needs no
  -- component (it denotes `False`); the named functions and the parent are
  -- elements, by default the constant of their name.
  rkConf : U → Prop := fun _ => False
  ran : (U → Prop) → U → Prop := fun _ _ => False
  registered : U → Prop := fun _ => False
  defaults : U → Prop := fun _ => False
  candidates : U → Prop := fun _ => False
  loadedRegistered : U → Prop := fun _ => False
  enveloped : U → Prop := fun _ => False
  punnedStores : U → Prop := fun _ => False
  confCopies : U → Prop := fun _ => False
  hiddenString : U → Prop := fun _ => False
  utHandleProduceResponse : U → Prop := fun _ => False
  call : U := const "call"
  rdCalloc : U := const "rd_calloc"
  rdMalloc : U := const "rd_malloc"
  rdKafkaTopparNew0 : U := const "rd_kafka_toppar_new0"
  returnsResultOf : U → Run → Prop := fun _ _ => False
  assertsResult : Run → Prop := fun _ => False
  returnsNonnull : Run → Prop := fun _ => False
  auditedAssertions : Prop := False
  deployedAssertion : Prop := False
  deployedFailureCall : Prop := False
  failureNoReturn : Run → Prop := fun _ => False
  opCallbacks : U → Prop := fun _ => False
  served : U → Prop := fun _ => False
  heldOn : Run → (U → Prop) → U → Prop := fun _ _ _ => False
  refs : (U → Prop) → U → Prop := fun _ _ => False
  brokerMonitorCb : U → Prop := fun _ => False
  monitorAddRegistered : U → Prop := fun _ => False
  coordRkbMonitorCb : U → Prop := fun _ => False
  txnCoordMonitorCb : U → Prop := fun _ => False
  broker : U → Prop := fun _ => False
  handedOnly : (U → Prop) → Run → Prop := fun _ _ => False
  assignCb : U → Prop := fun _ => False
  assignorAddRegistered : U → Prop := fun _ => False
  rangeAssign : U → Prop := fun _ => False
  roundrobinAssign : U → Prop := fun _ => False
  cooperativeStickyAssign : U → Prop := fun _ => False
  metadata : U → Prop := fun _ => False
  members : U → Prop := fun _ => False
  readsOnly : (U → Prop) → Run → Prop := fun _ _ => False
  op : U → Prop := fun _ => False
  ackBatches : U → Prop := fun _ => False
  ackDetails : U → Prop := fun _ => False
  firstAddLists : U → Prop := fun _ => False
  nonempty : U → Prop := fun _ => False
  logq : U → Prop := fun _ => False
  forwards : (U → Prop) → U → Prop := fun _ _ => False
  eventCb : (U → Prop) → U → Prop := fun _ _ => False
  fwdQueue : U → Prop := fun _ => False
  unforwarded : U → Prop := fun _ => False
  lastReleased : U → Prop := fun _ => False
  cjsonAllocate : U → Prop := fun _ => False
  cjsonReallocate : U → Prop := fun _ => False
  cjsonDeallocate : U → Prop := fun _ => False
  libcHooks : U → Prop := fun _ => False
  installedHooks : U → Prop := fun _ => False
  freshOrNull : Run → Prop := fun _ => False
  hookAllocated : U → Prop := fun _ => False
  deallocateReleases : U → Prop := fun _ => False
  cjsonArrays : U → Prop := fun _ => False
  children : U → U → Prop := fun _ _ => False
  first : U → U → Prop := fun _ _ => False
  last : U → U → Prop := fun _ _ => False
  pred : U → U → U → Prop := fun _ _ _ => False
  child : U → U → Prop := fun _ _ => False
  next : U → U → Prop := fun _ _ => False
  prev : U → U → Prop := fun _ _ => False
  nextReach : U → U → Prop := fun _ _ => False
  owned : U → U → Prop := fun _ _ => False
  nodup : U → Prop := fun _ => False
  live : U → Prop := fun _ => False
  parent : U := const "parent"
  concurrent : U → Prop := fun _ => False
  keepsLinks : U → Run → Prop := fun _ _ => False
  /-- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: the Fluent Bit terms. -/
  fb : FluentBitTerms U := FluentBitTerms.none U

/-- A value: an element, a set, a per-run resource set, a proposition, a
property of a run, or nothing (an ill-sorted application). -/
inductive Val (M : Model) where
  | el (u : M.U)
  | st (s : M.U → Prop)
  | rs (s : M.Run → M.U → Prop)
  | pr (p : Prop)
  | ef (e : M.Run → Prop)
  | bad

def Val.holds {M : Model} : Val M → Prop
  | .pr p => p
  | _ => False

def noRes {M : Model} : M.U → Prop := fun _ => False

/-- `#inert`, spelled out exactly as `eval` reads its definition. -/
def inertEff (M : Model) : M.Run → Prop := fun r =>
  ((M.takes r (M.callerHeld r) = noRes ∧ M.releases r (M.callerHeld r) = noRes) ∧
    M.retains r (M.callerHeld r) = noRes)

def prim0 (M : Model) (f : String) : Val M :=
  if f = "args" then .st M.args
  else if f = "created" then .st M.created
  else if f = "copied" then .st M.copied
  else if f = "freed" then .st M.freed
  else if f = "stated" then .st M.stated
  else if f = "builtin" then .st M.builtin
  else if f = "settable" then .st M.settable
  else if f = "library" then .st M.library
  else if f = "loaded" then .st M.loaded
  else if f = "caller_held" then .rs M.callerHeld
  else if f = "inert" then .ef (inertEff M)
  -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES.
  else if f = "false" then .pr False
  else if f = "rk_conf" then .st M.rkConf
  else if f = "registered" then .st M.registered
  else if f = "defaults" then .st M.defaults
  else if f = "candidates" then .st M.candidates
  else if f = "loaded_registered" then .st M.loadedRegistered
  else if f = "enveloped" then .st M.enveloped
  else if f = "punned_stores" then .st M.punnedStores
  else if f = "conf_copies" then .st M.confCopies
  else if f = "hidden_string" then .st M.hiddenString
  else if f = "ut_handle_produce_response" then .st M.utHandleProduceResponse
  else if f = "call" then .el M.call
  else if f = "rd_calloc" then .el M.rdCalloc
  else if f = "rd_malloc" then .el M.rdMalloc
  else if f = "rd_kafka_toppar_new0" then .el M.rdKafkaTopparNew0
  else if f = "asserts_result" then .ef M.assertsResult
  else if f = "returns_nonnull" then .ef M.returnsNonnull
  else if f = "audited_assertions" then .pr M.auditedAssertions
  else if f = "deployed_assertion" then .pr M.deployedAssertion
  else if f = "deployed_failure_call" then .pr M.deployedFailureCall
  else if f = "failure_no_return" then .ef M.failureNoReturn
  else if f = "op_callbacks" then .st M.opCallbacks
  else if f = "served" then .st M.served
  else if f = "broker_monitor_cb" then .st M.brokerMonitorCb
  else if f = "monitor_add_registered" then .st M.monitorAddRegistered
  else if f = "coord_rkb_monitor_cb" then .st M.coordRkbMonitorCb
  else if f = "txn_coord_monitor_cb" then .st M.txnCoordMonitorCb
  else if f = "broker" then .st M.broker
  else if f = "assign_cb" then .st M.assignCb
  else if f = "assignor_add_registered" then .st M.assignorAddRegistered
  else if f = "range_assign" then .st M.rangeAssign
  else if f = "roundrobin_assign" then .st M.roundrobinAssign
  else if f = "cooperative_sticky_assign" then .st M.cooperativeStickyAssign
  else if f = "metadata" then .st M.metadata
  else if f = "members" then .st M.members
  else if f = "op" then .st M.op
  else if f = "ack_batches" then .st M.ackBatches
  else if f = "ack_details" then .st M.ackDetails
  else if f = "first_add_lists" then .st M.firstAddLists
  else if f = "nonempty" then .st M.nonempty
  else if f = "logq" then .st M.logq
  else if f = "fwd_queue" then .st M.fwdQueue
  else if f = "unforwarded" then .st M.unforwarded
  else if f = "last_released" then .st M.lastReleased
  else if f = "cjson_allocate" then .st M.cjsonAllocate
  else if f = "cjson_reallocate" then .st M.cjsonReallocate
  else if f = "cjson_deallocate" then .st M.cjsonDeallocate
  else if f = "libc_hooks" then .st M.libcHooks
  else if f = "installed_hooks" then .st M.installedHooks
  else if f = "fresh_or_null" then .ef M.freshOrNull
  else if f = "hook_allocated" then .st M.hookAllocated
  else if f = "deallocate_releases" then .st M.deallocateReleases
  else if f = "cjson_arrays" then .st M.cjsonArrays
  else if f = "live" then .st M.live
  else if f = "parent" then .el M.parent
  else if f = "concurrent" then .st M.concurrent
  -- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES.
  else if f = "result" then .st M.fb.result
  else if f = "tracked" then .st M.fb.tracked
  else if f = "claimed" then .st M.fb.claimed
  else if f = "bound" then .st M.fb.bound
  else if f = "stored" then .st M.fb.stored
  else if f = "faithful" then .pr M.fb.faithful
  else if f = "admitted" then .pr M.fb.admitted
  else if f = "ingesting" then .st M.fb.ingesting
  else if f = "collector" then .st M.fb.collector
  else if f = "event_registered" then .st M.fb.eventRegistered
  else if f = "set_running" then .st M.fb.setRunning
  else if f = "returns_zero" then .pr M.fb.returnsZero
  else if f = "in_tree" then .st M.fb.inTree
  else if f = "synchronous" then .st M.fb.synchronous
  else if f = "proxy" then .st M.fb.proxy
  else if f = "production" then .pr M.fb.production
  else .bad

def prim1 (M : Model) (f : String) : Val M → Val M
  | .el o =>
      if f = "class" then .el (M.cls o)
      else if f = "ssl_of" then .st (M.sslOf o)
      else if f = "ex_data" then .st (M.exData o)
      else if f = "ex_data_free" then .st (M.exDataFree o)
      else if f = "method" then .el (M.method o)
      else if f = "fns" then .st (M.fns o)
      else if f = "teardown" then .st (M.teardown o)
      -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES.
      else if f = "returns_result_of" then .ef (M.returnsResultOf o)
      else if f = "children" then .st (M.children o)
      else if f = "first" then .st (M.first o)
      else if f = "last" then .st (M.last o)
      else if f = "child" then .st (M.child o)
      else if f = "next" then .st (M.next o)
      else if f = "prev" then .st (M.prev o)
      else if f = "next_reach" then .st (M.nextReach o)
      else if f = "owned" then .st (M.owned o)
      else if f = "nodup" then .pr (M.nodup o)
      else if f = "keeps_links" then .ef (M.keepsLinks o)
      else .bad
  | .st s =>
      if f = "ctx" then .st (M.ctx s)
      else if f = "bios" then .st (M.bios s)
      else if f = "chain" then .st (M.chain s)
      else if f = "slots" then .st (M.slots s)
      else if f = "store" then .st (M.store s)
      else if f = "lookups" then .st (M.lookups s)
      -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES.
      else if f = "ran" then .st (M.ran s)
      else if f = "held_on" then .rs (fun r => M.heldOn r s)
      else if f = "refs" then .rs (fun _ => M.refs s)
      else if f = "handed_only" then .ef (M.handedOnly s)
      else if f = "reads_only" then .ef (M.readsOnly s)
      else if f = "forwards" then .st (M.forwards s)
      else if f = "event_cb" then .st (M.eventCb s)
      -- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES.
      else if f = "interior" then .st (M.fb.interior s)
      else if f = "chunk_name" then .st (M.fb.chunkName s)
      else if f = "config_of" then .st (M.fb.configOf s)
      else if f = "out_plugin" then .st (M.fb.outPlugin s)
      else .bad
  | .rs s =>
      if f = "takes" then .rs (fun r => M.takes r (s r))
      else if f = "releases" then .rs (fun r => M.releases r (s r))
      else if f = "retains" then .rs (fun r => M.retains r (s r))
      else .bad
  | _ => .bad

def prim2 (M : Model) (f : String) : Val M → Val M → Val M
  | .el k, .st s =>
      if f = "of" then .st (fun o => s o ∧ M.cls o = k)
      else if f = "in_turn" then .st (M.inTurn k s)
      else .bad
  -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES.
  | .el a, .el b => if f = "pred" then .st (M.pred a b) else .bad
  | _, _ => .bad

/-- `#of(k, S)` is the members of `S` of class `k`; `F |-> E` is every run of
callback `F` (or of every member of set `F`) during the call having `E`. -/
def binVal (M : Model) : Op → Val M → Val M → Val M
  | .imp, .pr p, .pr q => .pr (p → q)
  | .conj, .pr p, .pr q => .pr (p ∧ q)
  | .conj, .ef e, .ef g => .ef (fun r => e r ∧ g r)
  | .mapsTo, .el f, .ef e => .pr (∀ r, M.runs f r → e r)
  | .mapsTo, .st F, .ef e => .pr (∀ f, F f → ∀ r, M.runs f r → e r)
  | .mem, .el u, .st s => .pr (s u)
  | .eqv, .rs a, .rs b => .ef (fun r => a r = b r)
  | .union, .st a, .st b => .st (fun u => a u ∨ b u)
  | .diff, .st a, .st b => .st (fun u => a u ∧ ¬ b u)
  | _, _, _ => .bad

def eval (M : Model) : List (String × Val M) → Raw → Val M
  | ρ, .quant v d b =>
      match eval M ρ d with
      | .st s => .pr (∀ u, s u → (eval M ((v, .el u) :: ρ) b).holds)
      | _ => .bad
  | ρ, .bin op l r => binVal M op (eval M ρ l) (eval M ρ r)
  | ρ, .app0 f =>
      match lk ρ f with
      | some x => x
      | none => prim0 M f
  | ρ, .app1 f a => prim1 M f (eval M ρ a)
  | ρ, .app2 f a b => prim2 M f (eval M ρ a) (eval M ρ b)
  | _, .const c => .el (M.const c)
  | _, .empty => .rs (fun _ => noRes)

/-! ## Well-formed formulas denote -/

inductive Kind where
  | el | st | rs | pr | ef
  deriving DecidableEq, Repr

def Srt.kind : Srt → Kind
  | .obj | .cls | .fn | .meth | .res => .el
  | .sobj | .sfn | .smeth => .st
  | .sres => .rs
  | .prop => .pr
  | .eff => .ef

def Val.kind {M : Model} : Val M → Option Kind
  | .el _ => some .el
  | .st _ => some .st
  | .rs _ => some .rs
  | .pr _ => some .pr
  | .ef _ => some .ef
  | .bad => none

/-- An environment agrees with a context: the same names, in order, each
value of its sort's kind. -/
inductive Agree {M : Model} : List (String × Srt) → List (String × Val M) → Prop
  | nil : Agree [] []
  | cons {Γ ρ} (x : String) (s : Srt) (v : Val M) :
      v.kind = some s.kind → Agree Γ ρ → Agree ((x, s) :: Γ) ((x, v) :: ρ)

theorem agree_lookup {M : Model} {Γ : List (String × Srt)} {ρ : List (String × Val M)}
    (h : Agree Γ ρ) (x : String) :
    (lk Γ x = none ∧ lk ρ x = none) ∨
      ∃ s v, lk Γ x = some s ∧ lk ρ x = some v ∧ v.kind = some s.kind := by
  induction h with
  | nil => exact Or.inl ⟨rfl, rfl⟩
  | cons y s v hv _ ih =>
      by_cases hy : y = x
      · exact Or.inr ⟨s, v, by simp [lk, hy], by simp [lk, hy], hv⟩
      · simpa [lk, hy] using ih

theorem term_mem {f : String} {sig : List Srt × Srt} (h : term f = some sig) :
    (f, sig) ∈ vocab := by
  unfold term at h
  generalize vocab = l at h
  induction l with
  | nil => simp [lk] at h
  | cons p rest ih =>
      obtain ⟨k, w⟩ := p
      by_cases hk : k = f
      · subst hk; simp [lk] at h; subst h; exact List.mem_cons_self ..
      · simp only [lk, hk, ↓reduceIte] at h; exact List.mem_cons_of_mem _ (ih h)

theorem prim0_kind (M : Model) {f : String} {s : Srt} (h : term f = some ([], s)) :
    (prim0 M f).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  repeat' (rcases hm with ⟨rfl, hs⟩ | hm)
  all_goals
    simp_all [prim0, Val.kind, Srt.kind]

theorem prim1_kind (M : Model) {f : String} {x s : Srt} {v : Val M}
    (h : term f = some ([x], s)) (hv : v.kind = some x.kind) :
    (prim1 M f v).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  repeat' (rcases hm with ⟨rfl, hs⟩ | hm)
  all_goals
    (try simp at hs) <;>
    (try obtain ⟨rfl, rfl⟩ := hs) <;>
    (cases v <;> simp_all [prim1, Val.kind, Srt.kind])

theorem prim2_kind (M : Model) {f : String} {x y s : Srt} {v w : Val M}
    (h : term f = some ([x, y], s)) (hv : v.kind = some x.kind)
    (hw : w.kind = some y.kind) :
    (prim2 M f v w).kind = some s.kind := by
  have hm := term_mem h
  simp only [vocab, List.mem_cons, Prod.mk.injEq, List.mem_nil_iff, or_false] at hm
  repeat' (rcases hm with ⟨rfl, hs⟩ | hm)
  all_goals
    (try simp at hs) <;>
    (try obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hs) <;>
    (cases v <;> cases w <;> simp_all [prim2, Val.kind, Srt.kind])

theorem binVal_kind (M : Model) {op : Op} {a b s : Srt} {v w : Val M}
    (h : binSort op a b = some s) (hv : v.kind = some a.kind)
    (hw : w.kind = some b.kind) : (binVal M op v w).kind = some s.kind := by
  cases op <;> cases a <;> cases b <;> simp [binSort, Srt.setOf] at h <;> subst h <;>
    (cases v <;> cases w <;> simp_all [binVal, Val.kind, Srt.kind])

theorem elem_kind {d e : Srt} (h : d.elem = some e) : d.kind = .st ∧ e.kind = .el := by
  cases d <;> simp [Srt.elem] at h <;> subst h <;> exact ⟨rfl, rfl⟩

/-- Every well-sorted term denotes a value of its sort's kind. -/
theorem check_denotes (M : Model) :
    ∀ (t : Raw) (Γ : List (String × Srt)) (ρ : List (String × Val M)) (s : Srt),
      Agree Γ ρ → check Γ t = some s → (eval M ρ t).kind = some s.kind := by
  intro t
  induction t with
  | quant v d b ihd _ =>
      intro Γ ρ s hA h
      simp only [check] at h
      split at h
      · exact absurd h (by simp)
      · cases hd : check Γ d with
        | none => simp [hd] at h
        | some ds =>
            cases he : ds.elem with
            | none => simp [hd, he] at h
            | some e =>
                simp only [hd, he, Option.bind_some] at h
                split at h
                · have hs : s = .prop := by simpa using h.symm
                  subst hs
                  have hk := ihd Γ ρ ds hA hd
                  rw [(elem_kind he).1] at hk
                  simp only [eval]
                  cases hev : eval M ρ d <;> simp_all [Val.kind, Srt.kind]
                · simp at h
  | bin op l r ihl ihr =>
      intro Γ ρ s hA h
      simp only [check] at h
      cases hl : check Γ l <;> cases hr : check Γ r <;> simp [hl, hr] at h
      exact binVal_kind M h (ihl Γ ρ _ hA hl) (ihr Γ ρ _ hA hr)
  | app0 f =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      rcases agree_lookup hA f with ⟨hΓ, hρ⟩ | ⟨s', v, hΓ, hρ, hk⟩
      · rw [hΓ] at h
        rw [hρ]
        cases ht : term f with
        | none => simp [ht] at h
        | some sig =>
            obtain ⟨args, out⟩ := sig
            cases args with
            | nil =>
                simp [ht] at h
                subst h
                exact prim0_kind M ht
            | cons _ _ => simp [ht] at h
      · rw [hΓ] at h
        simp at h
        subst h
        rw [hρ]
        exact hk
  | app1 f a iha =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      cases ht : term f with
      | none => simp [ht] at h
      | some sig =>
          obtain ⟨args, out⟩ := sig
          cases ha : check Γ a with
          | none => simp [ht, ha] at h
          | some y =>
              rcases args with _ | ⟨x, _ | ⟨_, _⟩⟩
              · simp [ht, ha] at h
              · by_cases hxy : x = y
                · subst hxy
                  simp [ht, ha] at h
                  subst h
                  exact prim1_kind M ht (iha Γ ρ _ hA ha)
                · simp [ht, ha, hxy] at h
              · simp [ht, ha] at h
  | app2 f a b iha ihb =>
      intro Γ ρ s hA h
      simp only [check] at h
      simp only [eval]
      cases ht : term f with
      | none => simp [ht] at h
      | some sig =>
          obtain ⟨args, out⟩ := sig
          cases ha : check Γ a with
          | none => simp [ht, ha] at h
          | some x' =>
              cases hb : check Γ b with
              | none => simp [ht, ha, hb] at h
              | some y' =>
                  rcases args with _ | ⟨x, _ | ⟨y, _ | ⟨_, _⟩⟩⟩
                  · simp [ht, ha, hb] at h
                  · simp [ht, ha, hb] at h
                  · by_cases hxy : x = x' ∧ y = y'
                    · obtain ⟨rfl, rfl⟩ := hxy
                      simp [ht, ha, hb] at h
                      subst h
                      exact prim2_kind M ht (iha Γ ρ _ hA ha) (ihb Γ ρ _ hA hb)
                    · simp [ht, ha, hb, hxy] at h
                  · simp [ht, ha, hb] at h
  | const c =>
      intro Γ ρ s _ h
      simp only [check] at h
      split at h
      · simp at h; subst h; rfl
      · simp at h
  | empty =>
      intro Γ ρ s _ h
      simp only [check] at h
      simp at h
      subst h
      rfl

/-- A well-formed formula denotes a proposition. -/
theorem wf_denotes (M : Model) {t : Raw} (h : wf t = true) :
    ∃ p, eval M [] t = .pr p := by
  have hc : check [] t = some .prop := by simpa [wf] using h
  have hk := check_denotes M t [] [] .prop Agree.nil hc
  cases he : eval M [] t with
  | pr p => exact ⟨p, rfl⟩
  | _ => simp_all [Val.kind, Srt.kind]

/-- A well-formed quantifier's body denotes a proposition at every element
of its domain. -/
theorem quant_body_denotes (M : Model) {Γ : List (String × Srt)}
    {ρ : List (String × Val M)} {v : String} {d b : Raw} (hA : Agree Γ ρ)
    (h : check Γ (.quant v d b) = some .prop) (u : M.U) :
    ∃ p, eval M ((v, .el u) :: ρ) b = .pr p := by
  simp only [check] at h
  split at h
  · simp at h
  · cases hd : check Γ d with
    | none => simp [hd] at h
    | some ds =>
        cases he : ds.elem with
        | none => simp [hd, he] at h
        | some e =>
            simp only [hd, he, Option.bind_some] at h
            split at h
            · rename_i hb
              have hk := check_denotes M b ((v, e) :: Γ) ((v, .el u) :: ρ) .prop
                (Agree.cons v e (.el u) (by simp [Val.kind, (elem_kind he).2]) hA) hb
              cases hev : eval M ((v, .el u) :: ρ) b with
              | pr p => exact ⟨p, rfl⟩
              | _ => simp_all [Val.kind, Srt.kind]
            · simp at h

/-! ## The defined term -/

/-- `#takes(#caller_held) == {} && #releases(#caller_held) == {} &&
#retains(#caller_held) == {}` (the Rust `INERT_DEFINITION`). -/
def inertDefinition : Raw := (.bin .conj (.bin .conj (.bin .eqv (.app1 "takes" (.app0 "caller_held")) .empty) (.bin .eqv (.app1 "releases" (.app0 "caller_held")) .empty)) (.bin .eqv (.app1 "retains" (.app0 "caller_held")) .empty))

theorem inert_definition (M : Model) :
    eval M [] inertDefinition = eval M [] (.app0 "inert") := by
  simp [eval, inertDefinition, lk, prim0, prim1, binVal]
  rfl

theorem inert_definition_sort : check [] inertDefinition = some .eff := by decide

/-! ## Decision vectors (the Rust `DECISION_VECTORS`) -/

def decisionVectors : List (Raw × Bool) := [
  ((.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "inert"))), true),
  ((.bin .mapsTo (.app0 "stated") (.bin .conj (.bin .conj (.bin .eqv (.app1 "takes" (.app0 "caller_held")) .empty) (.bin .eqv (.app1 "releases" (.app0 "caller_held")) .empty)) (.bin .eqv (.app1 "retains" (.app0 "caller_held")) .empty))), true),
  ((.quant "f" (.app0 "stated") (.bin .mapsTo (.app0 "f") (.app0 "inert"))), true),
  ((.quant "b" (.app2 "of" (.const "BIO") (.app0 "args")) (.bin .imp (.bin .mem (.app0 "b") (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data_free" (.app1 "class" (.app0 "b"))) (.app0 "inert")))), true),
  ((.quant "b" (.app0 "args") (.bin .imp (.bin .mem (.app1 "method" (.app0 "b")) (.app0 "builtin")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert")))), true),
  ((.bin .conj (.bin .mapsTo (.bin .diff (.app1 "slots" (.app0 "args")) (.app0 "stated")) (.app0 "inert")) (.bin .mapsTo (.app1 "slots" (.app1 "ctx" (.app0 "args"))) (.app0 "inert"))), true),
  ((.bin .mapsTo (.app0 "unknown") (.app0 "inert")), false),
  ((.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "args"))) (.app0 "inert")), false),
  ((.quant "o" (.app0 "created") (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "x"))) (.app0 "inert"))), false),
  ((.quant "args" (.app0 "created") (.bin .mapsTo (.app0 "stated") (.app0 "inert"))), false),
  ((.quant "o" (.app0 "created") (.quant "o" (.app0 "freed") (.bin .mapsTo (.app0 "stated") (.app0 "inert")))), false),
  ((.app0 "stated"), false),
  ((.app0 "inert"), false),
  ((.bin .mapsTo (.bin .union (.app0 "created") (.app0 "stated")) (.app0 "inert")), false),
  ((.bin .mem (.const "SSL") (.app0 "args")), false),
  ((.bin .mapsTo (.app0 "freed") (.app0 "inert")), false),
  ((.quant "b" (.app2 "of" (.const "NOT_A_CLASS") (.app0 "args")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert"))), false),
  ((.quant "b" (.app2 "of" (.app0 "args") (.const "BIO")) (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert"))), false),
  ((.bin .mapsTo (.app0 "slots") (.app0 "inert")), false),
  ((.quant "o" (.app0 "created") (.app0 "o")), false),
  ((.bin .mapsTo .empty (.app0 "inert")), false),
  ((.bin .conj (.bin .mapsTo (.app0 "stated") (.app0 "inert")) (.app0 "inert")), false),
  -- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the qcore-provider-librdkafka block.
  ((.app0 "false"), true),
  ((.bin .mapsTo (.app0 "concurrent") (.app1 "keeps_links" (.app0 "parent"))), true),
  ((.quant "c" (.app1 "children" (.app0 "parent")) (.bin .imp (.bin .mem (.app0 "c") (.app1 "next_reach" (.app0 "c"))) (.app0 "false"))), true),
  ((.quant "b" (.app1 "children" (.app0 "parent")) (.quant "a" (.app2 "pred" (.app0 "parent") (.app0 "b")) (.bin .mem (.app0 "a") (.app1 "prev" (.app0 "b"))))), true),
  ((.bin .mapsTo (.app0 "op_callbacks") (.bin .eqv (.app1 "takes" (.app1 "held_on" (.app0 "served"))) .empty)), true),
  ((.bin .conj (.bin .mapsTo (.app0 "call") (.app1 "returns_result_of" (.app0 "rd_calloc"))) (.app0 "audited_assertions")), true),
  ((.quant "f" (.app1 "ran" (.app1 "event_cb" (.app1 "forwards" (.app0 "logq")))) (.app0 "false")), true),
  ((.app1 "nodup" (.app0 "parent")), true),
  ((.bin .mapsTo (.app0 "coord_rkb_monitor_cb") (.bin .conj (.app1 "handed_only" (.app0 "broker")) (.app1 "reads_only" (.app0 "op")))), true),
  ((.bin .conj (.bin .mapsTo (.app0 "coord_rkb_monitor_cb") (.app1 "handed_only" (.app0 "broker"))) (.app1 "reads_only" (.app0 "op"))), false),
  ((.bin .mem (.app0 "call") (.app0 "registered")), true),
  ((.bin .mapsTo (.app0 "false") (.app0 "inert")), false),
  ((.app1 "nodup" (.app1 "children" (.app0 "parent"))), false),
  ((.bin .mem (.app1 "pred" (.app0 "parent")) (.app0 "live")), false),
  ((.bin .mapsTo (.app0 "parent") (.app0 "inert")), false),
  ((.bin .mem (.app0 "parent") (.app0 "registered")), false),
  ((.bin .mapsTo (.app0 "call") (.app1 "returns_result_of" (.app0 "parent"))), false),
  ((.bin .eqv (.app1 "held_on" (.app0 "served")) (.app1 "refs" (.app0 "op"))), false),
  ((.app1 "keeps_links" (.app0 "parent")), false),
  ((.quant "x" (.app0 "parent") (.app0 "false")), false),
  -- End of the SP2 block.
  -- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: every Fluent Bit term.
  ((.quant "r" (.app0 "result") (.bin .conj (.bin .mem (.app0 "r") (.app1 "interior" (.app0 "args"))) (.bin .mem (.app0 "r") (.app1 "chunk_name" (.app0 "args"))))), true),
  ((.quant "r" (.app0 "result") (.bin .mem (.app0 "r") (.bin .diff (.app0 "result") (.bin .union (.bin .union (.bin .union (.app0 "tracked") (.app0 "claimed")) (.app0 "bound")) (.app0 "stored"))))), true),
  ((.bin .imp (.app0 "admitted") (.app0 "faithful")), true),
  ((.quant "g" (.app1 "config_of" (.app0 "args")) (.bin .imp (.bin .mem (.app0 "g") (.app0 "ingesting")) (.app0 "returns_zero"))), true),
  ((.quant "c" (.app0 "collector") (.bin .conj (.bin .mem (.app0 "c") (.app0 "event_registered")) (.bin .mem (.app0 "c") (.app0 "set_running")))), true),
  ((.bin .imp (.app0 "production") (.quant "p" (.app1 "out_plugin" (.app0 "args")) (.bin .conj (.bin .mem (.app0 "p") (.bin .diff (.app0 "synchronous") (.app0 "in_tree"))) (.bin .mem (.app0 "p") (.app0 "proxy"))))), true),
  ((.bin .mapsTo (.app0 "faithful") (.app0 "inert")), false),
  ((.app0 "result"), false),
  ((.bin .eqv (.app1 "interior" (.app0 "admitted")) .empty), false),
  ((.quant "r" (.app0 "result") (.bin .mem (.app0 "r") (.app0 "chunk_name"))), false),
  ((.bin .mem (.app0 "returns_zero") (.app0 "collector")), false),
  ((.bin .mapsTo (.app1 "out_plugin" (.app0 "args")) (.app0 "inert")), false)
]

-- The merged SP2 and SP3 tables exceed the default recursion depth of the
-- kernel's `decide` evaluation; the proof is unchanged.
set_option maxRecDepth 8192 in
theorem decision_vectors_agree :
    decisionVectors.all (fun v => wf v.1 == v.2) = true := by decide

/-! ## The OpenSSL premises (`quirin-provider-openssl`'s `documented.rs`): seven names,
nine statements (the unstated-slots premise has a handshake and a free statement) -/

def exDataFormula : Raw := (.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "inert")))

def unstatedFormula : Raw := (.bin .conj (.bin .conj (.bin .conj (.bin .mapsTo (.bin .diff (.app1 "slots" (.bin .union (.bin .union (.app2 "of" (.const "SSL") (.app0 "args")) (.app1 "ctx" (.app2 "of" (.const "SSL") (.app0 "args")))) (.app2 "of" (.const "SSL_CTX") (.app0 "args")))) (.app0 "stated")) (.app0 "inert")) (.quant "l" (.app1 "lookups" (.app1 "store" (.bin .union (.app1 "ctx" (.app2 "of" (.const "SSL") (.app0 "args"))) (.app2 "of" (.const "SSL_CTX") (.app0 "args"))))) (.bin .mapsTo (.bin .diff (.app1 "fns" (.app0 "l")) (.app0 "stated")) (.app0 "inert")))) (.quant "o" (.bin .union (.bin .union (.app0 "created") (.app0 "copied")) (.app0 "freed")) (.bin .mapsTo (.bin .diff (.app1 "ex_data" (.app1 "class" (.app0 "o"))) (.app0 "stated")) (.app0 "inert")))) (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.bin .union (.app0 "stated") (.app0 "settable"))) (.app0 "inert"))))

def builtinFormula : Raw := (.quant "b" (.app1 "chain" (.bin .union (.app2 "of" (.const "BIO") (.app0 "args")) (.app1 "bios" (.app2 "of" (.const "SSL") (.app0 "args"))))) (.bin .imp (.bin .mem (.app1 "method" (.app0 "b")) (.app0 "builtin")) (.bin .conj (.bin .mapsTo (.app1 "fns" (.app0 "b")) (.app0 "inert")) (.quant "f" (.app1 "fns" (.app0 "b")) (.bin .mapsTo (.app2 "in_turn" (.app0 "f") (.bin .diff (.app1 "ssl_of" (.app0 "b")) (.app0 "args"))) (.app0 "inert"))))))

def bioFreeFormula : Raw := (.quant "b" (.bin .union (.app2 "of" (.const "BIO") (.app0 "args")) (.app1 "chain" (.app1 "bios" (.app2 "of" (.const "SSL") (.app0 "args"))))) (.bin .imp (.bin .mem (.app0 "b") (.app0 "freed")) (.bin .mapsTo (.app1 "ex_data_free" (.const "BIO")) (.app0 "inert"))))

def unstatedFreeFormula : Raw := (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.bin .union (.app0 "stated") (.app0 "settable"))) (.app0 "inert")))

def builtinLookupFormula : Raw := (.quant "l" (.app1 "lookups" (.app2 "of" (.const "X509_STORE") (.app0 "freed"))) (.bin .imp (.bin .mem (.app1 "method" (.app0 "l")) (.app0 "builtin")) (.bin .mapsTo (.app1 "fns" (.app0 "l")) (.app0 "inert"))))

/-- Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: the library-value premise
of the frees, the one premise for the modules the library loads, and the
unstated-slots statement of a free through a released argument. -/
def builtinTeardownFormula : Raw := (.quant "o" (.app0 "freed") (.quant "f" (.app1 "teardown" (.app0 "o")) (.bin .imp (.bin .mem (.app0 "f") (.app0 "library")) (.bin .mapsTo (.app0 "f") (.app0 "inert")))))

def unstatedReleasedFreeFormula : Raw := (.quant "o" (.app0 "freed") (.bin .mapsTo (.bin .diff (.app1 "teardown" (.app0 "o")) (.app0 "stated")) (.app0 "inert")))

def loadedTeardownFormula : Raw := (.quant "o" (.app0 "freed") (.quant "f" (.app1 "teardown" (.app0 "o")) (.bin .imp (.bin .mem (.app0 "f") (.app0 "loaded")) (.bin .conj (.bin .mapsTo (.app0 "f") (.app0 "inert")) (.quant "g" (.app2 "in_turn" (.app0 "f") (.bin .union (.bin .union (.app0 "freed") (.app0 "args")) (.app0 "created"))) (.bin .mem (.app0 "g") (.bin .union (.app0 "library") (.app0 "loaded"))))))))

theorem teardown_formulas_wf :
    wf builtinTeardownFormula = true ∧ wf loadedTeardownFormula = true ∧
      wf unstatedReleasedFreeFormula = true := by decide

theorem openssl_formulas_wf :
    wf exDataFormula = true ∧ wf unstatedFormula = true ∧ wf unstatedFreeFormula = true ∧
      wf builtinFormula = true ∧ wf builtinLookupFormula = true ∧
      wf bioFreeFormula = true := by decide

/-! ## The unstated-slots premise entails the ex_data premise -/

/-- No clause states an ex_data callback (C04.18: the registry is not a slot,
and no clause of the vocabulary states one), so `#stated` holds none. -/
def Model.Coherent (M : Model) : Prop := ∀ k f, M.exData k f → ¬ M.stated f

/-- What `openssl_ex_data_callbacks` states. -/
theorem eval_exData (M : Model) :
    (eval M [] exDataFormula).holds ↔
      ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) →
        ∀ f, M.exData (M.cls o) f → ∀ r, M.runs f r → inertEff M r := by
  simp [eval, exDataFormula, lk, prim0, prim1, binVal, Val.holds]

/-- What `openssl_unstated_callback_slots` states of the ex_data callbacks:
its third conjunct (card OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS added a fourth, the
statement of a free). -/
theorem unstated_ex_data_part (M : Model) (h : (eval M [] unstatedFormula).holds) :
    ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) →
      ∀ f, (M.exData (M.cls o) f ∧ ¬ M.stated f) → ∀ r, M.runs f r → inertEff M r := by
  simp only [eval, unstatedFormula, lk, prim0, prim1, prim2, binVal, Val.holds] at h
  simpa using h.1.2

/-- What it states of the SSL object's and its SSL_CTX's slots: its first
conjunct. -/
theorem unstated_slots_part (M : Model) (h : (eval M [] unstatedFormula).holds) :
    ∀ f, (M.slots (fun u =>
            ((M.args u ∧ M.cls u = M.const "SSL") ∨
              M.ctx (fun o => M.args o ∧ M.cls o = M.const "SSL") u) ∨
            (M.args u ∧ M.cls u = M.const "SSL_CTX")) f ∧ ¬ M.stated f) →
        ∀ r, M.runs f r → inertEff M r := by
  simp only [eval, unstatedFormula, lk, prim0, prim1, prim2, binVal, Val.holds] at h
  simpa using h.1.1.1

theorem unstated_entails_ex_data (M : Model) (hC : M.Coherent)
    (h : (eval M [] unstatedFormula).holds) : (eval M [] exDataFormula).holds := by
  have hx := unstated_ex_data_part M h
  rw [eval_exData]
  intro o ho f hf r hr
  exact hx o ho f ⟨hf, hC _ _ hf⟩ r hr

/-- In particular for the SSL object's ex_data class. -/
theorem unstated_entails_ex_data_for_ssl (M : Model) (hC : M.Coherent)
    (h : (eval M [] unstatedFormula).holds) :
    ∀ o, ((M.created o ∨ M.copied o) ∨ M.freed o) → M.cls o = M.const "SSL" →
      ∀ f, M.exData (M.const "SSL") f → ∀ r, M.runs f r → inertEff M r := by
  intro o ho hcls f hf r hr
  rw [← hcls] at hf
  exact (eval_exData M).1 (unstated_entails_ex_data M hC h) o ho f hf r hr

/-- The converse fails: the ex_data premise says nothing of the SSL object's
and its SSL_CTX's unstated slots. -/
theorem ex_data_does_not_entail_unstated :
    ¬ ∀ M : Model, M.Coherent → (eval M [] exDataFormula).holds →
      (eval M [] unstatedFormula).holds := by
  intro hall
  let M : Model := {
    U := Bool, Run := Unit,
    args := fun _ => True, created := fun _ => False, copied := fun _ => False,
    freed := fun _ => False, cls := fun _ => true, ctx := fun _ _ => False,
    bios := fun _ _ => False, chain := fun _ _ => False, sslOf := fun _ _ => False,
    slots := fun _ _ => True, stated := fun _ => False, exData := fun _ _ => False,
    exDataFree := fun _ _ => False, method := id, builtin := fun _ => False,
    fns := fun _ _ => False, store := fun _ _ => False, lookups := fun _ _ => False,
    teardown := fun _ _ => False, settable := fun _ => False,
    library := fun _ => False, loaded := fun _ => False,
    inTurn := fun _ _ _ => False, callerHeld := fun _ _ => True,
    takes := fun _ s => s, releases := fun _ _ _ => False,
    retains := fun _ _ _ => False, runs := fun _ _ => True, const := fun _ => true,
    fb := FluentBitTerms.none Bool }
  have hC : M.Coherent := fun _ _ h => h.elim
  have hE : (eval M [] exDataFormula).holds := by
    rw [eval_exData]; intro o ho; simp [M] at ho
  have hU := unstated_slots_part M (hall M hC hE)
  have := hU true ⟨by simp [M], by simp [M]⟩ () trivial
  have h1 := congrFun this.1.1 true
  simp [M, noRes] at h1

end QCore.PremiseFormula
