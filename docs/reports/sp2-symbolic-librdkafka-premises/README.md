# SP2-SYMBOLIC-LIBRDKAFKA-PREMISES

Base `1cd7da6b8` (origin/main with SP1-SYMBOLIC-OPENSSL-PREMISES landed). User
decision 2026-11-20 (verbatim): "we don't want to write down the sentences on
premise clauses. We really dislike this kind of results so we must write down
as condition symbols like |-> or other types of logical symbols to represent
the prerequisites which compose for premise conditions." User 2026-11-21:
"Launch SP2–SP4".

[Prediction](PREDICTION.md) (committed first, `658e65d00`): no status move on
any route, content changes only where a librdkafka premise is rendered.

## 1. What changed

* **All 220 `resource_premise` clauses** of `qcore-provider-librdkafka` state
  formulas (C04.18.9): 16 statements under 12 names on 190 declarations in 9
  modules. The 180 `rk_plugin_callback_closure` clauses were byte-identical and
  state one formula.
* **Vocabulary**: 75 terms appended as the SP2 block (Rust `VOCABULARY` from
  `#false`, specification table, Lean `vocab` and `Model`), existing sorts,
  operators and typing rules only. `#false` (prop) is the falsum, so a
  negation is `P -> #false`.
* **Semantics unchanged.** A formula premise is metadata as a string one is.
  The one consumer that matches a premise's exact statement,
  `qcore-c-gateway::legacy_detach::REPRESENTATION_STATEMENT`, now holds the
  canonical formula, so it admits exactly the declarations it admitted.
* **Prose ban**: the crate has no checker plugin; its own test
  `every_premise_is_a_formula` (`providers/qcore-provider-librdkafka/src/main.rs`)
  refuses a string statement and pins the 220.
* **Bindings**: 167 content-addressed binding manifests that pin a premised
  declaration's graph were refreshed ([bindings](bindings/index.json)), the
  graph digest each one's only change, with a new tool
  (`crates/qcore-contract/examples/premise_formula_binding_refresh.rs`).
* **Frozen checkpoints**: a test that compares a converted module with a
  historical copy reads the module as it stood before the card through an exact,
  hash-checked restoration ([migration record](migration.json),
  `tests/support/sp2_premise_checkpoint.rs`).
* **The source operator gate** (`Q038`, `crates/qcore-frontend/src/pipe.rs`)
  reads a premise formula's `|` and `&&` as the formula's own. SP1's formulas
  never met it: the corpus compiler regression compiles no user crate.

## 2. Census

Counted by the contract format's own reader, line comments excluded (the brief's
224 counts four clauses quoted in comments: `cjson.qv:190`, `:248`,
`client.qv:1004`, `:1338`). Before: 220 prose, 0 formula. After: 0 prose,
220 formula.

| Name | Clauses | Statements | Modules |
|---|---|---|---|
| `rk_plugin_callback_closure` | 180 | 1 | client 89, null_deref 71, header_inlines 12, logging 3, log_caller_frame 2, kafka_mem 1, memory 1, queue_destructor 1 |
| `conf_property_writer` | 18 | 1 | client 18 |
| `asserted_allocation` | 9 | 4 | client 7 (calloc 5, toppar 1, malloc 1), memory 2 (rule 37) |
| `served_queue` | 2 | 1 | client 2 |
| `cjson_allocator_hooks` | 2 | 2 | cjson 2 |
| `rk_log_enqueue_no_event_callback` | 2 | 1 | log_caller_frame 2 |
| `rk_log_enqueue_forward_not_last` | 2 | 1 | log_caller_frame 2 |
| `cjson_list_invariant` | 1 | 1 | cjson 1 |
| `detach_list_representation` | 1 | 1 | cjson 1 |
| `broker_monitor_callback` | 1 | 1 | client 1 |
| `assignor_callback` | 1 | 1 | client 1 |
| `share_ack_batches_nonempty` | 1 | 1 | client 1 |
| **total** | **220** | **16** | 190 declarations (163 with one premise, 24 with two, 3 with three) |

The nine `resource_hypothesis(detach_*, "…")` atoms of
`cJSON_DetachItemViaPointer` are unchanged (§7).

## 3. Vocabulary (the SP2 block)

The full table, with each term's sort and meaning, is C04.18.9's "librdkafka
block" in the [contract specification](../../QCORE_CONTRACT_SPEC_r0_3.md#c04189-symbolic-premise-statements-formula-version-1);
the Rust table is its source. Reused from SP1 with their meaning:
`#slots`, `#args`, `#freed`, `#takes`, `#releases`, `#retains`, `#inert` (in a
decision vector only). By group:

* logic: `#false` (prop);
* callback closure: `#rk_conf`, `#ran(F)`, `#registered`, `#defaults`,
  `#candidates`, `#loaded_registered`, `#enveloped`, `#punned_stores`,
  `#conf_copies`, `#hidden_string`, `#ut_handle_produce_response`;
* allocation: `#call`, `#rd_calloc`, `#rd_malloc`, `#rd_kafka_toppar_new0`
  (fn), `#returns_result_of(f)`, `#asserts_result`, `#returns_nonnull`,
  `#failure_no_return` (eff), `#audited_assertions`, `#deployed_assertion`,
  `#deployed_failure_call` (prop);
* queues and ops: `#op_callbacks`, `#served`, `#held_on(S)`, `#refs(S)`,
  `#broker_monitor_cb`, `#monitor_add_registered`, `#coord_rkb_monitor_cb`,
  `#txn_coord_monitor_cb`, `#broker`, `#handed_only(S)`, `#assign_cb`,
  `#assignor_add_registered`, `#range_assign`, `#roundrobin_assign`,
  `#cooperative_sticky_assign`, `#metadata`, `#members`, `#reads_only(S)`,
  `#op`, `#ack_batches`, `#ack_details`, `#first_add_lists`, `#nonempty`,
  `#logq`, `#forwards(S)`, `#event_cb(S)`, `#fwd_queue`, `#unforwarded`,
  `#last_released`;
* cJSON: `#cjson_allocate`, `#cjson_reallocate`, `#cjson_deallocate`,
  `#libc_hooks`, `#installed_hooks`, `#fresh_or_null`, `#hook_allocated`,
  `#deallocate_releases`, `#cjson_arrays`, `#children(o)`, `#first(o)`,
  `#last(o)`, `#pred(o, c)`, `#child(o)`, `#next(o)`, `#prev(o)`,
  `#next_reach(o)`, `#owned(o)`, `#nodup(o)` (prop), `#live`, `#parent` (obj),
  `#concurrent`, `#keeps_links(o)` (eff).

Conventions: a pointer member's term is the set of objects it designates
(empty for NULL), so "NULL or X" is `quant all #x in M: #x in X`, and "never
NULL" is `(quant all #p in M: #false) -> #false`. `#call` is the declared
function itself, so `#call |-> E` says the call has `E`. A name list in a term
meaning ("rd_calloc (rd.h:139-143)") is the term's identity, never an example.

The term first called `#nonnull_result` was renamed `#returns_nonnull`: the
bounds vocabulary reserves the spelling `nonnull_result` in every non-bounds
module (`bounds_contract::RESERVED_SPELLINGS`), and the test enforcing it
refused `client.qv`.

## 4. The formulas

As the modules state them (one line each; the canonical printing):

1. `rk_plugin_callback_closure`: `(quant all #f in #ran(#slots(#rk_conf)) \ #loaded_registered: #f in #registered | #defaults && #f in #candidates) && quant all #f in #loaded_registered: #f in #enveloped`
2. `conf_property_writer`: `(quant all #f in #punned_stores: #f in #registered | #defaults | #conf_copies | #hidden_string) && quant all #f in #hidden_string: #f in #ut_handle_produce_response`
3. `asserted_allocation`, rd_calloc (5): `#call |-> #returns_result_of(#rd_calloc) && #rd_calloc |-> #asserts_result && #audited_assertions && #call |-> #returns_nonnull`
4. `asserted_allocation`, toppar (1, `rd_kafka_toppar_desired_add`): `#call |-> #returns_result_of(#rd_kafka_toppar_new0) && #rd_kafka_toppar_new0 |-> #returns_result_of(#rd_calloc) && #rd_calloc |-> #asserts_result && #audited_assertions && #call |-> #returns_nonnull`
5. `asserted_allocation`, rd_malloc (1): `#call |-> #returns_result_of(#rd_malloc) && #rd_malloc |-> #asserts_result && #audited_assertions && #call |-> #returns_nonnull`
6. `asserted_allocation`, rule 37 (`rd_calloc`, `rd_strdup` in `memory.qv`): `#deployed_assertion && #deployed_failure_call && #call |-> #failure_no_return`
7. `served_queue`: `#op_callbacks |-> #takes(#held_on(#served)) == {}`
8. `broker_monitor_callback`: `(quant all #f in #broker_monitor_cb | #monitor_add_registered: #f in #coord_rkb_monitor_cb | #txn_coord_monitor_cb) && #coord_rkb_monitor_cb | #txn_coord_monitor_cb |-> (#handed_only(#broker) && #takes(#refs(#args)) == {} && #releases(#refs(#args)) == {} && #retains(#refs(#args)) == {})`
9. `assignor_callback`: `(quant all #f in #assign_cb | #assignor_add_registered: #f in #range_assign | #roundrobin_assign | #cooperative_sticky_assign) && #range_assign | #roundrobin_assign | #cooperative_sticky_assign |-> (#reads_only(#metadata | #members) && #takes(#refs(#op)) == {} && #releases(#refs(#op)) == {} && #retains(#refs(#op)) == {})`
10. `share_ack_batches_nonempty`: `(quant all #l in #ack_batches: #l in #ack_details) && (quant all #l in #ack_details: #l in #first_add_lists) && quant all #l in #ack_batches: #l in #nonempty`
11. `rk_log_enqueue_no_event_callback`: `(quant all #f in #event_cb(#forwards(#logq)): #false) && quant all #f in #ran(#event_cb(#forwards(#logq))): #false`
12. `rk_log_enqueue_forward_not_last`: `quant all #q in #fwd_queue: (#q in #freed -> #false) && (#q in #unforwarded && #q in #last_released -> #false)`
13. `cjson_allocator_hooks`, `cJSON_New_Item`: `(quant all #f in #cjson_allocate | #cjson_deallocate: #f in #libc_hooks | #installed_hooks) && #cjson_allocate |-> #fresh_or_null && quant all #o in #hook_allocated: #o in #deallocate_releases`
14. `cjson_allocator_hooks`, `print`: `(quant all #f in #cjson_allocate | #cjson_reallocate | #cjson_deallocate: #f in #libc_hooks | #installed_hooks) && #cjson_allocate | #cjson_reallocate |-> #fresh_or_null && quant all #o in #hook_allocated: #o in #deallocate_releases`
15. `cjson_list_invariant`: `quant all #a in #cjson_arrays: quant all #c in #child(#a): (quant all #l in #last(#a): #l in #prev(#c)) && ((quant all #p in #prev(#c): #false) -> #false)`
16. `detach_list_representation`: `#parent in #live && (quant all #c in #owned(#parent): #c in #children(#parent)) && (quant all #c in #children(#parent): #c in #owned(#parent) && #c in #live) && (quant all #c in #child(#parent): #c in #first(#parent)) && (quant all #c in #first(#parent): #c in #child(#parent)) && (quant all #b in #children(#parent): quant all #a in #pred(#parent, #b): #b in #next(#a)) && (quant all #c in #children(#parent): #c in #next_reach(#c) -> #false) && ((quant all #c in #child(#parent): #false) -> quant all #c in #children(#parent): #false) && ((quant all #c in #children(#parent): #false) -> quant all #c in #child(#parent): #false) && (quant all #b in #children(#parent): quant all #a in #pred(#parent, #b): #a in #prev(#b)) && (quant all #f in #first(#parent): quant all #l in #last(#parent): #l in #prev(#f)) && #nodup(#parent) && (quant all #c in #children(#parent): quant all #y in #next(#c) | #prev(#c): #y in #children(#parent) && #y in #live) && (quant all #l in #last(#parent): quant all #y in #next(#l): #false) && (#parent in #children(#parent) -> #false) && #concurrent |-> #keeps_links(#parent)`

Where the prose gives a reason and a consequence ("non-null because …, so it
never returns NULL"; "NULL or non-empty: its producer …"; "no … is registered,
so … runs no callback"), the formula states every prerequisite and the stated
consequence; §6 proves the consequence follows. Keeping the consequence makes
the formula never narrower than the prose.

## 5. Faithfulness: prose to formula

Every conjunct and parenthetical has a row. "Evidence" marks a phrase that
gives the reason a prerequisite holds, or what would follow if it failed: it
assumes nothing beyond the rows above it, and its content is in the named
term's meaning where it identifies something.

**`rk_plugin_callback_closure`** (180)

| Prose | Formula |
|---|---|
| "P-RK-PLUGIN-CALLBACK-CLOSURE:" | the premise's name (a label, not a condition) |
| "in the static scope" | `\ #loaded_registered`: the loaded registrations are excluded from the static claim; in the static scope that set is empty, so the claim covers every run callback |
| "every rk_conf callback" | `#slots(#rk_conf)`: every callback slot (SP1's term) of every rd_kafka_conf_t |
| "a call may run" | `#ran(…)`: the members the call runs, directly or in turn |
| "including rd_kafka_log_buf's log callback" | a member of `#ran(#slots(#rk_conf))` (`log_cb` is an rk_conf slot, run in turn by `rd_kafka_log_buf`; `#ran`'s meaning names it) |
| "is a run registration" | `#f in #registered` |
| "or a stated default" | `\| #defaults` |
| "with a declared candidate" | `&& #f in #candidates`, on both (reading R1) |
| "with loaded external code, extra registrations" | `#loaded_registered` |
| "depend on the applicable pinned rk_plugin_library and flb_external_plugin outer envelopes" | `quant all #f in #loaded_registered: #f in #enveloped` |
| "(OP-APP-RK-CALLBACKS-VIA-POLL-CB, C04.20)" | citations, carried by the terms' meanings (C04.19.3, C04.20) |

**`conf_property_writer`** (18)

| Prose | Formula |
|---|---|
| "the setter stores through rd_kafka_anyconf_set_prop0's _RK_C_PTR case (rdkafka_conf.c:2087-2088), a type-punned store through the property table" | `#punned_stores` (its meaning: the stores of that case, the setter's own among them) |
| "which writes a callback member only from" | `quant all #f in #punned_stores: #f in …` |
| "a dedicated rd_kafka_conf_set_*_cb setter (each declared a registration)" | `#registered` |
| "a pdef default (stated on rd_kafka_conf_new)" | `\| #defaults` |
| "rd_kafka_anyconf_copy of the same property of another conf" | `\| #conf_copies` |
| "or the string API for an _RK_HIDDEN property" | `\| #hidden_string` |
| "(only ut_handle_ProduceResponse)" | `&& quant all #f in #hidden_string: #f in #ut_handle_produce_response` |
| "Clang's member facts see none of these stores" | evidence (why the premise is needed), in `#punned_stores`' meaning |

**`asserted_allocation`**, rd_calloc (5), toppar (1), rd_malloc (1)

| Prose | Formula |
|---|---|
| "the result is non-null" / "so it never returns NULL in the audited build" | `#call \|-> #returns_nonnull` |
| "because it is allocated with rd_calloc (rd.h:139-143)" | `#call \|-> #returns_result_of(#rd_calloc)` (`#rd_calloc` names the lines) |
| toppar: "because its last source is rd_kafka_toppar_new0's reference" | `#call \|-> #returns_result_of(#rd_kafka_toppar_new0)` ("last source" is `#returns_result_of`'s meaning) |
| toppar: "allocated with rd_calloc (rd.h:139-143)" | `&& #rd_kafka_toppar_new0 \|-> #returns_result_of(#rd_calloc)` |
| malloc: "allocated with rd_malloc (rd.h:145-149)" | `#call \|-> #returns_result_of(#rd_malloc)`, `#rd_malloc \|-> #asserts_result` |
| "which asserts its result" | `&& #rd_calloc \|-> #asserts_result` |
| "and the audited compile commands define no NDEBUG" | `&& #audited_assertions` |

**`asserted_allocation`**, rule 37 (2)

| Prose | Formula |
|---|---|
| "the deployed build preserves the analyzed assertion" | `#deployed_assertion` |
| "and its declared terminating failure call" | `&& #deployed_failure_call` (its meaning: preserved, and a declaration in force states it terminating) |
| "allocation failure has no normal return" | `&& #call \|-> #failure_no_return` |

**`served_queue`** (2)

| Prose | Formula |
|---|---|
| "an op callback the call may run" | `#op_callbacks \|-> …` (`\|->`: every run during the call) |
| "(rd_kafka_op_handle, rd_kafka_op_call, rd_kafka_op_destroy)" | `#op_callbacks`' meaning: the run sites, naming the class |
| "takes over no reference" | `#takes(…) == {}` |
| "the caller holds on the served queue" | `#held_on(#served)` |

**`broker_monitor_callback`** (1)

| Prose | Formula |
|---|---|
| "the monitor callback in rko_u.broker_monitor.cb" | `#broker_monitor_cb` |
| "is rd_kafka_coord_rkb_monitor_cb or rd_kafka_txn_coord_monitor_cb" | `quant all #f in #broker_monitor_cb \| …: #f in #coord_rkb_monitor_cb \| #txn_coord_monitor_cb` |
| "the only two rd_kafka_broker_monitor_add registers" | the same quantifier over `\| #monitor_add_registered` |
| "which receive only the broker" | `#coord_rkb_monitor_cb \| #txn_coord_monitor_cb \|-> (#handed_only(#broker) && …)` |
| "and take no custody of the op, its handle or its queue" | `#takes(#refs(#args)) == {} && #releases(#refs(#args)) == {} && #retains(#refs(#args)) == {}` (the call's arguments are `rk`, `rkq`, `rko`) |

**`assignor_callback`** (1)

| Prose | Formula |
|---|---|
| "the assign callback the group assignor runs" | `#assign_cb` |
| "is range, roundrobin or cooperative-sticky" | `#f in #range_assign \| #roundrobin_assign \| #cooperative_sticky_assign` |
| "the only ones rd_kafka_assignor_add registers" | the quantifier's domain also `\| #assignor_add_registered` |
| "which read the metadata and members" | `#reads_only(#metadata \| #members)` (reading R2) |
| "and take no custody of the op" | `#takes(#refs(#op)) == {} && #releases(#refs(#op)) == {} && #retains(#refs(#op)) == {}` |

**`share_ack_batches_nonempty`** (1)

| Prose | Formula |
|---|---|
| "P-SHARE-ACK-BATCHES-NONEMPTY:" | the name |
| "the op's rko_u.share_fetch_fanout.ack_batches is NULL or non-empty" | `quant all #l in #ack_batches: #l in #nonempty` (NULL designates no list) |
| "its producer rd_kafka_share_build_ack_details (rdkafka_share_acknowledgement.c:686-842)" | `#ack_details` |
| "creates the list only at its first rd_list_add (:822-826)" | `quant all #l in #ack_details: #l in #first_add_lists` |
| "and rd_kafka_share_fetch_fanout (rdkafka.c:3404) stores only that value" | `quant all #l in #ack_batches: #l in #ack_details` |
| "a non-NULL empty list with a selected broker would be cleared without its release (has_fanout_acks ? ack_batches : NULL, rdkafka.c:3801-3803)" | evidence: what would follow if the premise failed; assumes nothing |

**`rk_log_enqueue_no_event_callback`** (2)

| Prose | Formula |
|---|---|
| "no queue event callback is registered" | `quant all #f in #event_cb(…): #false` |
| "on rk->rk_logq" | `#logq` |
| "or on any queue it is forwarded to" | `#forwards(#logq)` |
| "while rd_kafka_log_buf enqueues on it" | `#event_cb`'s and `#forwards`' meanings ("while the call enqueues on it") |
| "so rd_kafka_q_io_event runs no callback" | `&& quant all #f in #ran(#event_cb(#forwards(#logq))): #false` |
| "(rd_kafka_q_cb_event_enable is reached only from the public rd_kafka_queue_cb_event_enable, which no analysed unit calls; loaded code may)" | evidence, in `#event_cb`'s meaning; "loaded code may" is why this stays a premise |
| "(W48-RK-LOG-CALLER-FRAME, rule 32)" | citation |

**`rk_log_enqueue_forward_not_last`** (2)

| Prose | Formula |
|---|---|
| "the forward-queue reference rd_kafka_q_enq1 takes with rd_kafka_q_fwd_get" | `quant all #q in #fwd_queue` |
| "is not the last when it releases it with rd_kafka_q_destroy" | `#q in #freed -> #false` (`#freed`: the objects whose last reference the call drops) |
| "so the enqueue destroys no queue" | the same conjunct: `#fwd_queue`'s meaning records it is the only queue reference the enqueue releases |
| "no other thread unforwards that queue" | `#q in #unforwarded` |
| "and releases its last other reference during the call" | `&& #q in #last_released -> #false` (reading R3) |
| "(in sequential execution it never is)" | evidence: with no other thread both sets are empty |
| "(W48-RK-LOG-CALLER-FRAME, rule 32)" | citation |

**`cjson_allocator_hooks`**, `cJSON_New_Item` (1) and `print` (1)

| Prose | Formula |
|---|---|
| "hooks->allocate returns a fresh allocation or NULL" | `#cjson_allocate \|-> #fresh_or_null` |
| print: "hooks->allocate and hooks->reallocate return a fresh allocation or NULL" | `#cjson_allocate \| #cjson_reallocate \|-> #fresh_or_null` |
| New_Item: "which cJSON_Delete releases through the matching deallocate hook" | `quant all #o in #hook_allocated: #o in #deallocate_releases` (its meaning names cJSON_Delete) |
| print: "and hooks->deallocate releases one" | the same conjunct |
| "malloc and free in global_hooks (cJSON.c:178, :186)" / "malloc, realloc and free in global_hooks (cJSON.c:178-186)" | `#libc_hooks` |
| "unless cJSON_InitHooks installed a user pair" / "unless a user set was installed" | `quant all #f in #cjson_allocate \| #cjson_deallocate (\| #cjson_reallocate): #f in #libc_hooks \| #installed_hooks` |
| "a premise of whoever installed it" | attribution: an installed hook meets the same conditions, which its installer assumes; no further condition |
| "nothing is stated about the hooks changing between the allocation and its release" | an explicit non-assumption: no term reads the hooks at another time |

**`cjson_list_invariant`** (1)

| Prose | Formula |
|---|---|
| "cJSON's list invariant, kept by every cJSON list operation" | `quant all #a in #cjson_arrays` (it holds of every array; "kept by every operation" is evidence) |
| "a non-empty array's child->prev" | `quant all #c in #child(#a)`: the array's child, when non-NULL |
| "is its last element" | `quant all #l in #last(#a): #l in #prev(#c)` |
| "never NULL" | `(quant all #p in #prev(#c): #false) -> #false` |
| "if a caller broke it, the child->prev == NULL arm returns true without linking item, which this declaration would read as a transfer" | evidence: what would follow if the premise failed |

**`detach_list_representation`** (1)

| Prose | Formula |
|---|---|
| "For this removed-result declaration" | the premise's declaration; `#parent` is its `parent` argument |
| "the live parent" | `#parent in #live` |
| "represents exactly its owned children" | `(quant all #c in #owned(#parent): #c in #children(#parent)) && (quant all #c in #children(#parent): #c in #owned(#parent) && …)` |
| "by a finite" | `#children` is the finite child sequence (its meaning; Lean `List`) |
| "acyclic" | `quant all #c in #children(#parent): #c in #next_reach(#c) -> #false` |
| "child and next chain" | `#c in #live` for every child, `#child(#parent)` is `#first(#parent)` (two inclusions), and each child after another is that one's `#next` (`#pred`) |
| "child is NULL precisely when the chain is empty" | the two implications between `quant all #c in #child(#parent): #false` and `quant all #c in #children(#parent): #false` |
| "each nonfirst child's prev is its predecessor" | `quant all #b in #children(#parent): quant all #a in #pred(#parent, #b): #a in #prev(#b)` |
| "the first child's prev is the last child" | `quant all #f in #first(#parent): quant all #l in #last(#parent): #l in #prev(#f)` |
| "chain child identities are pairwise distinct" | `#nodup(#parent)` |
| "every non-NULL next or prev target is a live child of that same parent" | `quant all #c in #children(#parent): quant all #y in #next(#c) \| #prev(#c): #y in #children(#parent) && #y in #live` |
| "the last child's next is NULL" | `quant all #l in #last(#parent): quant all #y in #next(#l): #false` |
| "the parent is not a child" | `#parent in #children(#parent) -> #false` |
| "and no concurrent writer mutates these links during the call" | `#concurrent \|-> #keeps_links(#parent)` |

`detach_formula_is_the_nine_atoms` (§6) checks this table mechanically against
the nine propositions the declaration's decomposition is already bound to.

**Readings recorded**

* **R1** (`rk_plugin_callback_closure`). "a run registration or a stated
  default with a declared candidate" is read with "with a declared candidate"
  qualifying both, the stronger reading: C04.19.3 states that every candidate of
  the closure is declared ("each candidate declared the identity"), and a member
  invocation is composed only over declared candidates (C04.19.7), so the
  narrower reading would assume no less of a run the analysis accepts.
* **R2** (`assignor_callback`). "which read the metadata and members" is read
  as access by reading only (`#reads_only`: no write), as the declaration's
  comment says ("which it only reads"); the reading assumes more than "reads"
  alone, never less.
* **R3** (`rk_log_enqueue_forward_not_last`). "no other thread unforwards that
  queue and releases its last other reference" is read per queue: no queue is
  both unforwarded and last-released by other threads during the call, whoever
  does each. This also excludes the two steps by two different threads, which
  destroys the queue the same way, so it is never narrower than the one-thread
  reading.

None of the three is a choice between readings that differ on a run the
analysis accepts, so none was handed back.

## 6. Lean

`proof/QCore/PremiseFormula.lean` (the base module) gains the SP2 block:
75 `vocab` rows, 74 `Model` components with defaults (`#false` denotes `False`
and needs none), the `prim0`/`prim1`/`prim2` arms, and 20 decision vectors
(the first 22 are SP1's). Its three `prim*_kind` lemmas now enumerate the
vocabulary by `repeat' rcases` instead of a 27-way pattern, so they hold for
any vocabulary length (an SP3 or SP4 block merges without touching them).
`check_denotes`, `wf_denotes`, `decision_vectors_agree` (42 vectors) and every
SP1 theorem, including the counterexample model, build unchanged.

New module `proof/QCore/PremiseFormula/Librdkafka.lean`, audited by
`proof/AuditPremiseFormulaLibrdkafka.lean` (18 theorems, standard axioms only):

* `librdkafka_formulas_wf`: the 16 statements are well-formed; the Rust test
  `the_librdkafka_formulas_are_the_lean_definitions`
  (`crates/qcore-frontend/tests/premise_formula_librdkafka.rs`) binds each
  definition to every clause stating it, 220 in all;
* `false_negates`, `quant_false_empty`: `#false` negates and empties;
* `asserted_calloc_prerequisites_suffice`, `asserted_malloc_…`,
  `asserted_toppar_prerequisites_suffice` (with `eval_asserted_*` and
  `returned_from_asserting_nonnull`): in every model where an asserting
  allocator's assertion is compiled in (`Model.AssertCoherent`), the
  prerequisites entail `#call |-> #returns_nonnull`, through two returns for
  the toppar statement;
* `share_ack_prerequisites_suffice` (`Model.ListCoherent`: a list created only
  at its first add holds an element);
* `log_no_event_prerequisites_suffice` (`Model.RanCoherent`: the call runs only
  members of a set it runs from);
* `hooks_print_entails_new_item`: the print statement entails the
  cJSON_New_Item one;
* `detach_formula_is_the_nine_atoms`: in the model an entry environment of
  `QCore.CjsonRepresentationPremises` induces (`detachModel`), the detach
  formula holds exactly when `chain ∧ distinct ∧ empty ∧ firstPrev ∧ lastNext ∧
  linkTargets ∧ noWriter ∧ parentOutside ∧ predecessor` holds: the propositions
  the nine `detach_*` atoms are bound to by the declaration's binding manifest.

No entailment between two premises is stated by the prose (each names its own
assumption); the theorems above are the consequences the prose states within a
premise, plus the one inclusion between the two hooks statements.

## 7. The hypotheses: a named follow-up

The nine `resource_hypothesis(detach_*, "…")` atoms keep their strings.
`hypothesis::validate` admits a C04.18.9 formula only as the statement of a
generated `legacy_atom_` (the erasure of a named premise); a
`resource_hypothesis` atom has the opaque-string form or the C04.18.7 typed
form (`quant proposition #ID(…) = FORMULA;`, G01p), which `plugin_*` atoms
already use. Converting the detach atoms consistently means the typed form:
it rebinds each atom's digest in the cJSON binding manifest, the nine
`resource_guarded_symbol_v1` clauses that repeat each atom's text, the guarded
adoption plan and the native local binding checker
(`quirin-source-analysis::cjson_local_binding`), a migration of its own
(`quant_binding_migration` does it for typed atoms). Registered as
`SP2-DETACH-HYPOTHESES-TYPED-ATOMS`. Each atom is already bound to a Lean
proposition, and §6's theorem ties the premise's formula to exactly those
propositions.

## 8. Consumers and migrations

* **`legacy_detach::REPRESENTATION_STATEMENT`** and the three gateway tests
  that state the premise (`detach_same_item.rs`, `detach_lookup_membership.rs`,
  `detach_wrapper_composition.rs`) hold the canonical formula. An in-unit
  removed-result declaration stating the old sentence is now withdrawn (the
  check was and is exact identity: fail-closed).
* **Binding manifests.** `premise_formula_binding_refresh` admits exactly a
  prose-to-formula rewrite: same atom and premise IDs, refinements,
  requirements and version; every non-legacy atom byte-identical (each bound
  atom's `atom_sha256` unchanged); each changed legacy statement prose before
  and a canonical formula after, the same in atom and erasure. It refreshed
  167 manifests (166 `rk_plugin_callback_closure` in 6 modules, 1
  `detach_list_representation`), rewrote 193 legacy statements, patched the 167
  markers and resolved every new reference from the explicit inputs
  (`private/logs/refresh-4.log`, exit 0; deterministic across runs). The
  manifests and `index.json` (previous and new manifest and graph digests) are
  in [bindings/](bindings/); the cJSON detach frozen graph is
  `bindings/cjson-expected-graph.json`. Consumers repointed:
  `production_symbolic_adoption.rs`, `hypothesis_bindings.rs`, `hypothesis.rs`,
  `qcore-audit`'s `contract_dependencies_report.rs`,
  `quirin-source-analysis::cjson_local_binding` and its example, and
  `scripts/gate_scope_walkers.json`. The historical manifests stay in their
  reports.
* **Frozen checkpoints.** `production_symbolic_adoption`,
  `guarded_contract_adoption`, `guarded_decomposition` and
  `contract_inherit_qcore_provider` compare converted modules with frozen
  copies (a migration record's `after_sha256`, the guarded adoption plan's
  whole-source hashes, the hand-written token fixture). Each now reads the
  module's base bytes through `sp2_premise_checkpoint::restored_file`, which
  checks the current bytes against the record's `after_sha256`, writes the 16
  formulas back as their prose and the 167 markers back as their previous
  digests, and checks the result against `before_sha256`; an unrecorded change
  still fails. Bindings, lifecycle symbols and flows are still checked on the
  current module.
* **The operator gate** admits `|` and `&&` inside a `resource_premise(…)`
  clause (`premise_formula_spans`); `compiler/tests/premise_formula_operators.rs`
  shows a formula compiles and a `|`, `&&` or `&` elsewhere still refuses.

## 9. Lock diff, reviewed

Order: `quirin-audit pin --source-root <66910c10 tree> --crate
qcore-provider-librdkafka`; `pin --source-root <046accf6 tree> … --function`
the 116 changed entries reviewed there before; `pin --source-root <34155955
tree> … --function rd_calloc --function rd_strdup` (reviewed there before); the
same for `--bounds` at `66910c10` and `046accf6`; then `contracts.py sync`.
A first unrestricted run at `046accf6` would have added that revision to
`rd_kafka_log0` and `rd_kafka_log_buf`, which were reviewed only at
`66910c10`; it was discarded so no review is widened.

* `contracts.lock`: 118 of 231 entries change exactly `contract_sha256` and
  `pinned_at`; identities (sites and every `reviewed_at` revision) are
  identical to base; nothing added or removed; top-level keys unchanged.
* `bounds-contracts.lock`: 71 of 86 entries change exactly `contract_sha256`
  and `pinned_at`; identities identical.
* `outer-contracts.lock`: unchanged (`outer.qv` names premises only).
* `contract-inventory.lock` and `contracts.json`: regenerated by `sync` for the
  9 modules' digests and premise manifests.
* `providers/contract-load-path.json`: unchanged (the crate publishes no module).

The 190 premised declarations are the 118 + 71 pinned ones and the model
contract `rd_kafka_q_destroy_final` (`queue_destructor.qv`, unpinned).

## 10. Gate

`python3 scripts/gate_scope.py --base 1cd7da6b8 --out private/gate-1` on
`d602bb6a2` (the card merged with origin/main `9b107caf1`, r45, a clean merge)
([record](gate/gate-1.json); mode full: the diff reaches `qcore-frontend`,
`qcore-contract`, `qcore-c-gateway` and a proof root). Exit 1:

| Step | Exit | Counts |
|---|---|---|
| fmt | 0 | |
| clippy | 101 | `collapsible_if` in the new refresh tool (fixed, below) |
| rust-suite | 101 | 672 units, 9817 passed, **1 failed**, 21 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 470 audits |
| python | 1 | 779 tests, **1 failed** (fixed, below) |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 167 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

* **Rust.** The one failure is the brief's known host failure,
  `qcore-audit::test::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
  (the system `curlver.h`); not fixed here.
* **Clippy.** `premise_formula_binding_refresh.rs` nested two `if let`s;
  collapsed in `a023b4739`. `cargo clippy --all-targets -- -D warnings` on
  that tree: exit 0 (`private/logs/clippy-2.log`).
* **Python.** `test_gate_scope_census`'s observed-read check: the recorded read
  census still had `production_symbolic_adoption` reading the old manifest
  paths. Re-traced with `gate_read_census.py record --packages qcore-contract`
  and `reduce` (56 units; `private/census-1`), and only that unit replaced in
  `docs/reports/gate-scope-precise-selection/read-census.json` (`1c4ba08fe`;
  every other qcore-contract unit read the same files). Rule 13h's amendment
  also needed `gen_rule_docs.py` (`docs/agents/gateway.md`, `d99b5a1cf`), which
  the gate's run did not catch only because its python step started after it.
  Rerun at `1c4ba08fe` with `QUIRIN_CARD_GATE_BASE=1cd7da6b8`: 779 tests,
  exit 0 (`private/gate-2/python.log`).

Targeted runs before the gate, each exit 0 unless noted: `qcore-contract`,
`qcore-c-gateway`, `quirin-source-analysis`, `qcore-compiler`, `qcore-frontend`
whole (`private/logs/probe-2.log`); `qcore-audit`'s
`contract_dependencies_report`, `plugin_null_hypotheses`, `outer_projection`
(`probe-4.log`, `probe-5.log`); `qcore-compiler --test premise_formula_operators`
(`probe-8.log`); `quirin-audit-plugin-measure` (`probe-6.log`: every target
passed but `slot_wait`, which waits on a gate slot and failed only because the
run itself held one; it passed in the gate's suite); the librdkafka provider
crate (`provider-test-1.log`, 3 passed); `contracts.py check` (exit 0).

## 11. Follow-ups

* `SP2-DETACH-HYPOTHESES-TYPED-ATOMS` (§7).
* `SP2-SHIPPED-PROVIDER-PROSE-BAN`: a shipped (`qcore-provider-*`) crate has no
  checker plugin, so the prose ban is a crate test; a load-time refusal (the
  contract loader refusing a string statement for crates that opt in, as the
  OpenSSL checker does) needs a hook of its own.

## 12. Measurement

Every pinned route loads the crate (it is on `providers/contract-load-path.json`).
Predicted: no status move on corpus, twelve-home, system or slice; content
changes only where a librdkafka premise statement is rendered (a contract
dependency record's premise list, a reading-site environment text, a
hypothesis graph's legacy atom). The `measure-prediction` block is in
[PREDICTION.md](PREDICTION.md), for the integrator's `quirin-audit measure
--declared`.

## Integrator measurement (2026-11-21)

Landing `17b0e77b2` on base `70e10930f`, measured with `--declared PREDICTION.md`:
corpus, twelve-home, system and slice all match the declared prediction with
0 moves. Comparison: `integration/native-comparison.json.gz` (uncompressed
sha256 `9aca98a1406876290b94e37462f549b04072f6322ff3d1aadb0c021455fed54f`). Landing gate: `integration/gate.json` (Rust 9817
passed, 1 failed: the known host libcurl failure; Lean 471 audits; Python
779).

Landing race: main moved to `2aefebbc1` (r47, r48) during the gate; merged in
`36439854b` (records-only conflicts) and checked with the targeted suite named
in `integration/gate.json`, all exit 0.
