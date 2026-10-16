# OUTER-CONTRACT-DESIGN: code loaded at runtime, stated as an outer contract

Card **OUTER-CONTRACT-DESIGN** (wave 48, round 31; card-high, design first).
Base: local `main` `3f8f1a617`. This is the census and the proposal. The card
**pauses here for the user's decision**: no product code, contract, rule or
Lean module is changed. Rule id `31` is reserved for the implementation.
Prediction: [PREDICTION.md](PREDICTION.md), committed with this README.

**Decided 2026-10-17** (D1 "B: loader envelope", D2 "Yes, as proposed", D3
"Both from one run", D4 "Later"); stage 1 is implemented by card
OUTER-CONTRACT ([report](../outer-contract/README.md)). This report is
frozen as the design evidence.

## 1. Authority

**User direction (2026-10-16, verbatim, as recorded in the brief).** Asked
whether Fluent Bit's nonnull_member facts should carry a premise for
out-of-tree plugins (shared objects through `flb_plugin_load`, Go plugins
through the proxy), which can store into `ins->p` from code Quirin never
sees, the user answered:

- "We need to provide outer_contract_v1 like contract set which provide Pending contract but it's not providing the reason."
- On what the reason should be: "Reason = quant #outer [[contract]] would be better to have?"
- On whether `rk_no_plugin_library` should become such a contract: "Still seated as Safe without the loading plugin but we also needed to provide a candidates of representing syntax like quant #outer where #<pointer> in #loaded [[runtime]] like situation."

Also in force: the sigils (`#` binders and value keywords, `@NAME` C names,
`alias`), `|->`, the merged `nonnull_member(TAG, FIELD) where quant #scope
[[...]]` form (round 30, decision 3), hard invariants 1 and 2, "handle as
safe as possible", and C04.18 (named premises change no verdict).

## 2. Census

### 2.1 Every way the build can run code from outside it

Bodies read at `66910c10` and `046accf6`. Every loader below is
byte-identical at the two revisions; `src/flb_config.c` differs by one line
outside the loader call. All units are in all three compile databases
([loader_units.py](scripts/loader_units.py),
[loader-units.json](artifacts/loader-units.json)); none is in the system or
slice selection except `src/flb_lib.c` (slice). The production binary is
linked with `ENABLE_EXPORTS ON` (`src/CMakeLists.txt:620`), so loaded code can
call every non-static symbol of Fluent Bit and its bundled libraries.

| # | Mechanism (entry, how it is configured) | What the loaded code receives | Closed-world facts it could falsify |
| --- | --- | --- | --- |
| M1 | **Fluent Bit C shared-object plugin**: `flb_plugin_load` (`src/flb_plugin.c:200`; `dlopen`, `dlsym` of `<type>_<name>_plugin`), reached from `flb_plugin_load_router` by `-e PATH` (`fluent-bit.c:1155`), `plugins_file` (`flb_config.c:904`), a `[PLUGINS]`/`plugins:` section (`flb_config.c:1603`) and hot reload (`flb_reload.c:376`) | its descriptor is `memcpy`d into an `flb_malloc`ed `struct flb_{input,processor,filter,output}_plugin` linked into `config->*_plugins`; every callback then receives `struct flb_*_instance *`, `struct flb_config *`, event chunks and buffers; `cb_destroy` receives the plugin; plus every exported symbol | K1 stable members `flb_*_instance.p` (and any other member of anything reachable from `config`); K2 run sets of members it registers into through the setters (`flb_input_collector.cb_collect` via `flb_input_set_collector_*`, rule 12n); unchecked callers of exported functions |
| M2 | **Go plugin through the proxy**: `flb_plugin_proxy_create` (`src/flb_plugin_proxy.c:598`; `dlopen`, `dlsym` of `FLBPluginRegister`/`Init`/`Flush`/`InputCallback`…, `src/proxy/go/go.c`), same configuration paths (a file name without the `flb-` prefix) | in-tree proxy callbacks (`flb_proxy_*_cb_*`) stand in the descriptor, so `ins->p` is a Fluent Bit-built plugin; the Go side receives `struct flbgo_*_plugin` with `o_ins` / `i_ins` (the instance), `api` (`struct flb_api`: property getters, `*_get_cmt_instance`, `log_print`, log checks) and its own context; an input's collect hands back a buffer Fluent Bit `free`s unless the plugin's cleanup callback does (`go.c:236-275`) | through `unsafe`/cgo, the same as M1; through the Fluent Bit Go API only cmetrics state and the handed-back buffer's custody |
| M3 | **librdkafka plugin library**: `plugin.library.paths` → `rd_kafka_plugins_conf_set` → `rd_kafka_plugin_new` (`rdkafka_plugin.c:63`; `rd_dl_open`, `rd_dl_sym "conf_init"`); Fluent Bit passes every `rdkafka.*` property through (`src/flb_kafka.c:82-87`), so `rdkafka.plugin.library.paths` in an in_kafka / out_kafka section loads one; `WITH_PLUGINS 1`, `WITH_LIBDL 1` (generated `config.h`) | `conf_init(conf, &opaque, …)` receives the `rd_kafka_conf_t *`; it registers rk_conf callbacks and interceptors (`rd_kafka_conf_interceptor_add_on_*`, `rd_kafka_interceptor_add_on_*`), which then receive `rd_kafka_t *`, messages, partition lists, broker names | K2 the rk_conf callback run sets (closed over the run's registrations, C04.19.3) and the interceptor lists; K1 `rd_kafka_broker_s.rkb_rk` (a store is reachable from a function a callback may call, rule 14x); K3 premises stating an internal closed world (`assignor_callback`, `broker_monitor_callback`, `conf_property_writer`, `cjson_allocator_hooks`) through non-public symbols |
| M4 | **LuaJIT** (`filter_lua`, Lua processors): `require` of C modules (`lib_package.c:46`, `dlopen`), and the FFI (`lj_clib.c:118`, `ffi.load`, `ffi.C`; the FFI is not disabled) | records converted to Lua values (copies); no Fluent Bit object pointer; the FFI can call any exported symbol and forge pointers | none that any row rests on today (no closed-world fact names a Lua-reachable member); in principle anything, through forged pointers |
| M5 | **WASM** (`filter_wasm`, `in_exec_wasi`; `src/wasm/flb_wasm.c`, WAMR) | copies in the module's linear memory; `wasi_nn.c` (the WAMR `dlopen` of NN backends) is not compiled | none (sandboxed) |
| M6 | **NVML** (`in_gpu_metrics`, `nvml_gpu.c:553`, `dlopen("libnvidia-ml.so.1")`) | a vendor library called through `dlsym`ed pointers with the plugin's own buffers | none: every call is an indirect call through a context member, already uncontracted (`Unknown`) |
| M7 | **Monkey dynamic plugins** (`mk_plugin_load(MK_PLUGIN_DYNAMIC)`, `mk_server/mk_plugin.c:66`, from `plugins.load`) | `struct plugin_api *`, the server, sessions and requests | reachable only in the standalone `monkey` binary (`mk_bin/monkey.c:396`); in Fluent Bit's library mode `conf_plugin_load` is never set, so nothing loads |
| M8 | **Library mode host** (`src/flb_lib.c`, `out_lib`, `in_lib`): not loaded code but an outer *caller* | `flb_ctx_t` (whose `config` is public), the callbacks it registers (`out_lib`'s `cb`, `flb_output_set_test`) | unchecked callers of exported functions; the `out_lib` callback set (no consumer today) |
| — | Not reachable or not application extensions: SQLite extensions (`load_extension` never enabled: no `sqlite3_enable_load_extension` or `SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION` in the tree), c-ares' macOS `dlopen`, every Windows `LoadLibrary`/`GetProcAddress`, jemalloc's `dlsym(RTLD_NEXT, "pthread_create")` (symbol interposition), OpenSSL engines and providers (system library configuration; the OpenSSL plugin crates' business) | | |

Two facts reduce what needs a contract:

* **Plugin callbacks are already open.** M1's `memcpy` into `struct
  flb_*_plugin` is a whole-record write, which opens every member of the
  record in the run's member pre-pass (rule 12n), and no Fluent Bit
  declaration decides a call over `flb_*_plugin.cb_*`. Calls into loaded
  callbacks are therefore `Unknown` in every case, and no row rests on them.
* **An unchecked caller is not a new question.** A scope entry (rule 14z) or
  `quant null-deref` fact makes the callee's row `Safe` *conditional* on its
  callers; a loaded caller is one more caller bound by the published
  condition, like an indirect one. No caller-side row is decided by the
  absence of loaded callers. (`fb-scope` below is sized for completeness.)

What remains are the **closed-world facts**: conclusions the analysis draws
because it saw every store or every registration in the run. Loaded code is
exactly the code it did not see.

* **K1, stable members** (rules 14x/14y): "every store to an existing object
  is checked".
* **K2, run sets** (rule 12n, C04.19.3, premise P-CG22-MEMBER-CLOSED-WORLD):
  "every unit that can store into the member is in the run".
* **K3, named premises that state a closed world** (C04.18), today written as
  free text: `rk_no_plugin_library` (163 clauses), and, through non-public
  symbols only, `assignor_callback`, `broker_monitor_callback`,
  `conf_property_writer`, `cjson_allocator_hooks`. The other premises
  (`asserted_allocation`, `nested_call_admission`, `literal_argument`,
  `mk_event_descriptors`, `served_queue`, `untracked_handback`,
  `share_ack_batches_nonempty`, `ingestion_active`, P-RESULT-HOLDER-LAST-DROP,
  P-RETAINED-PREDEFINED) state build facts or tracking limits, not the
  absence of other code. `synchronous_output_reachability` is the converse:
  it says the function runs only when a proxy plugin is loaded.

Everything else a verdict assumes of all code (no data race, no wild write,
the ABI) applies to loaded code as it does to the run's own threads and is
not a closed-world fact.

### 2.2 Sizes

Main's binary `2d42ddb4…` on every route with the null property
([census-run.sh](scripts/census-run.sh), [measure.sh](scripts/measure.sh),
[outer_rows.py](scripts/outer_rows.py), [outer-rows.json](artifacts/outer-rows.json)).
The corpus reproduces round 31: custody `f2eb590a…` 939 / 9 / 3,886, null
1,114 / 1,435 / 2,354 / 18 (2,549 Safe). System `e6167449…` 95 / 4 / 349, null 86 / 125 / 243; slice `32ea1665…` 52 / 1 / 347 / 4 Pending, null 69 / 121 / 216 (the system and slice reports embed the worktree's paths, so their digests are this worktree's). Every total equals round 31's.

**Null property** (rows, by null verdict; obligations = the row's checks whose
proof or subject names the fact):

| Route | Class | Rows | Safe | Unknown | Unsafe | Combined Safe | Obligations |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| corpus | `fb-stable` | 1 | 1 | 0 | 0 | 1 | 2 |
| corpus | `fb-scope` | 6 | 2 | 4 | 0 | 0 | 3 |
| corpus | `premise:rk_no_plugin_library` | 339 | 75 | 262 | 2 | 3 | 716 |
| system | `fb-stable` | 16 | 2 | 14 | 0 | 2 | 16 |
| system | `fb-scope` | 12 | 4 | 8 | 0 | 0 | 6 |
| slice | `fb-stable` | 46 | 16 | 30 | 0 | 0 | 72 |
| slice | `fb-scope` | 38 | 22 | 16 | 0 | 0 | 78 |

**Custody** (rows resting on a declaration that carries the premise or states
a run-set clause; obligations = row × declaration pairs):

| Route | Class | Rows | Safe | Unknown | Unsafe | Obligations |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| corpus | `closure` | 341 | 5 | 336 | 0 | 497 |
| corpus | `premise:assignor_callback` | 92 | 2 | 90 | 0 | 92 |
| corpus | `premise:broker_monitor_callback` | 92 | 2 | 90 | 0 | 92 |
| corpus | `premise:cjson_allocator_hooks` | 32 | 9 | 23 | 0 | 32 |
| corpus | `premise:conf_property_writer` | 30 | 0 | 30 | 0 | 31 |
| corpus | `premise:rk_no_plugin_library` | 483 | 21 | 462 | 0 | 3,654 |
| system | `closure` | 3 | 0 | 3 | 0 | 3 |
| slice | `closure` | 11 | 0 | 11 | 0 | 13 |

Reading the classes:

* **`fb-stable` (K1, M1/M2):** the rows whose dependencies name
  `stable:flb_{input,output,filter,processor}_instance.p`. Their `Safe` rows
  are the ones the outer contract decides: `flb_chunk_route_plugin_matches`
  (combined `Safe`: corpus 1, system 2) and, on the slice, 16 null `Safe`
  conditional rows (custody `Unknown`): the seven scope functions whose entry
  fact crosses a call (`flb_input_pause`, `_resume`, `flb_input_instance_pre_run`,
  `_exit`, `flb_input_thread_instance_pause`, `_resume`, `_exit`) and the
  caller `flb_input_rate_gate_maybe_resume`, in both configurations.
* **`fb-scope`:** the ten scope functions' own rows and the callers that
  discharged their checks. A row here that is not also `fb-stable`
  (`flb_input_plugin_pause` / `_resume`, `flb_input_thread_instance_pre_run`,
  and the callers `flb_input_chunk_append_raw{,_local}`) rests on no
  closed-world fact (see 2.1): it is not affected.
* **`premise:rk_no_plugin_library` (K1/K2/K3, M3):** corpus only (neither
  other route analyses a librdkafka unit). Custody 21 `Safe` rows (the 17 of
  LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED, `rd_kafka_crash`, `rd_kafka_dump`,
  `rd_kafka_event_destroy`, `ut_op_destroy_free`); null 75 `Safe` rows, 3 of
  them combined `Safe`.
* **`closure` (K2):** every corpus row resting on a run-set declaration; its 5
  `Safe` rows all also carry `rk_no_plugin_library`. No Fluent Bit row
  rests on one (the collector setters' registrations have no consumer yet,
  C04.19.3).
* **Other K3 premises:** `assignor_callback`, `broker_monitor_callback`
  2 `Safe` each (the same two rows as the closure class), `cjson_allocator_hooks`
  9 `Safe` (`cJSON_Create*`, `cJSON_Print*`), `conf_property_writer` none.

## 3. Reading of the user's direction

The integrator's reading is **confirmed, with three precisions**:

1. **Confirmed:** an outer contract (`outer_contract_v1`) states what code
   outside the audited program, loaded at runtime, receives and may do.
2. **Confirmed:** with nothing loaded, verdicts stay `Safe`, and the
   assumption "nothing is loaded" stays visible in the dependencies.
3. **Confirmed, made precise:** with loaded code, a verdict that rests on an
   outer contract is `Pending`. "Provide Pending … but it's not providing the
   reason" reads as: unlike `resource_review_pending("reason", …)` (C06.1),
   a row resting on an outer contract carries **no free-text reason of its
   own**; the reason *is* the contract (`quant #outer [[contract]] NAME`,
   whose statement says what loaded code must do), resolved by a person who
   reviews the loaded code against it. This is a fourth `PendingReason`
   (`outer-contract`), not a reuse of `awaiting-human-review`.
4. **Precision 1 (fail-closed):** in the runtime case a closed-world fact
   that **no** outer contract covers cannot stay `Safe` or become `Pending`:
   the row is `Unknown`. Only a stated outer contract buys `Pending`.
5. **Precision 2 (what is affected):** only rows resting on a closed-world
   fact (K1–K3) move. A row that rests on none is the same in both cases.
   Calls into loaded callbacks are already `Unknown` and stay so.
6. **Precision 3 (where the runtime case comes from):** whether anything is
   loaded is a deployment fact (a configuration file, `-e`, an `rdkafka.*`
   property), not a source fact. The analysis cannot decide it. Both cases
   are therefore published from one run: the static verdict as today, and
   the runtime verdict as a projection over the dependency record. No
   second analysis is needed (rule 13f: the record never omits a
   consultation).
7. **Scope:** Fluent Bit (M1, M2) and librdkafka (M3) now; M8 can use the
   same vocabulary for outer callers later. `rk_no_plugin_library` is not
   converted here.

## 4. Syntax and semantics proposals

All three share the header and the verdicts of section 5; they differ in
where the runtime condition lives and how a fact comes to depend on it.

### Alternative A: named outer contract on the loader, cited by each fact

An outer contract is declared on the **loader's** declaration (so it is
bound to code, pinned to that body and digested), in a new module of the
crate that owns the loader:

```qcore
module outer;   // providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv

extern fn flb_plugin_load(path: ptr<mut, u8>, ctx: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] flb_external_plugin, "a C shared-object plugin or a Go plugin stores into no member of a struct flb_*_instance or struct flb_config the core owns, registers callbacks only through the core's setters, and calls core functions only under their published conditions";
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
};

extern fn flb_plugin_proxy_create(dso_path: ptr<const, u8>, type: i32, config: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] flb_external_plugin;
    quant #outer where #config in #loaded [[runtime]];
};
```

* `quant #outer [[contract]] NAME, "STATEMENT";` **defines** the contract
  (one definition per name, run-wide); `quant #outer [[contract]] NAME;`
  **cites** it.
* `quant #outer where #P in #loaded [[runtime]] when …;` is the runtime
  condition: after a call that meets the guard, the object `P` names (and
  what is reachable from it) is handed to loaded code. `#loaded` is a value
  keyword: the set of objects loaded code holds. The guard uses round 30's
  scalar `when` (an integer result against a constant); without a guard
  (`flb_plugin_proxy_create` returns a pointer) the hand-off is read on every
  return, which only over-approximates.
* A fact depends on the contract only where it cites it, beside the
  qualifier, replacing today's free-text premise:

```qcore
// fluent_bit/null_deref.qv
extern fn flb_output_new(config: ptr<mut, u8>, output: ptr<const, u8>, data: ptr<mut, u8>, public_only: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member(flb_output_instance, p) where quant #scope [[stable]];
    quant #outer [[contract]] flb_external_plugin;
};
```

**FB worked example.** `flb_chunk_route_plugin_matches` (`flb_router.c`)
tests `!o_ins || !o_ins->p || !o_ins->p->name`, calls `strlen`, and reads
`o_ins->p->name` again; the second read is proved by
`stable:flb_output_instance.p`. Static: `Safe` (combined `Safe`), row
dependency `stable:flb_output_instance.p` and `outer:flb_external_plugin
(not loaded)`. Runtime: `Pending`, reason `outer-contract:
flb_external_plugin`. The same holds for `flb_input_pause` (slice, null
`Safe` conditional): its entry fact survives `flb_input_buf_paused(ins)`
only by `stable:flb_input_instance.p`, which cites the contract through
`flb_input_new`'s declaration; runtime `Pending` (its custody stays
`Unknown`, so its combined row is `Unknown` in both cases).

**librdkafka worked example.** The loader is `rd_kafka_plugin_new`
(`static` in `rdkafka_plugin.c`; declarations bind by name, and its
caller `rd_kafka_plugins_conf_set` could host the clause instead):

```qcore
module outer;   // providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv

alias #NO_ERROR = @RD_KAFKA_RESP_ERR_NO_ERROR;

extern fn rd_kafka_plugin_new(conf: ptr<mut, u8>, path: ptr<const, u8>, errstr: ptr<mut, u8>, errstr_size: u64) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] rk_plugin_library, "a library named by plugin.library.paths (in Fluent Bit, rdkafka.plugin.library.paths) registers rk_conf callbacks and interceptors only through the public setters, each callback it registers meets the candidate contract of its member, and it calls no non-public symbol";
    quant #outer where #conf in #loaded [[runtime]] when #result == #NO_ERROR;
};
```

Each of the 163 `requires resource_premise(rk_no_plugin_library, "…")`
becomes `quant #outer [[contract]] rk_plugin_library;`. Example:
`rd_kafka_topic_partition_destroy` (`rdkafka_partition.c`) rests on
`rd_kafka_topic_partition_destroy0`, whose release cascade reaches
`rd_kafka_log0` and so `rk_conf.log_cb`. Static: `Safe` (custody and null).
Runtime: `Pending`, reason `outer-contract: rk_plugin_library`.

*Cost and risk:* smallest change (the premise machinery already lists
premises per row); but a fact that should cite and does not stays `Safe` in
the runtime case, the omission the FB card's premise reading describes.

### Alternative B: the loader states what loaded code receives and may do; dependence is derived

Same header and runtime condition as A, plus an **envelope**: one `|->`
line per closed-world fact the contract vouches for, using the member-path
value keywords:

```qcore
extern fn flb_plugin_load(path: ptr<mut, u8>, ctx: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] flb_external_plugin, "…as in A…";
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
    quant #outer #loaded: flb_input_instance.p |-> #kept;
    quant #outer #loaded: flb_output_instance.p |-> #kept;
    quant #outer #loaded: flb_filter_instance.p |-> #kept;
    quant #outer #loaded: flb_processor_instance.p |-> #kept;
    quant #outer #loaded: flb_input_collector.cb_collect |-> #registered;
};
```

* `TAG.FIELD |-> #kept`: loaded code never stores into the member (K1).
* `TAG.FIELD |-> #registered`: loaded code writes the member only through
  a declared registration, each callback meeting the member's candidate
  contract (K2).
* `quant #outer #loaded: #premise NAME;` vouches for a K3 premise by name.
* **Facts cite nothing.** The analysis derives the dependence: a row resting
  on a closed-world fact whose key is a record reachable from a `#loaded`
  binder (Clang's record graph, which the member pre-pass already reads,
  plus every exported global) depends on the contract. If an envelope line
  covers the key, the runtime verdict is `Pending`; if none does, it is
  `Unknown` (`outer-uncovered: stable:TAG.FIELD`).

**FB worked example.** `flb_chunk_route_plugin_matches`: `stable:
flb_output_instance.p` is a K1 key on a record reachable from `#config`, so it
is covered by the second envelope line. Static `Safe`; runtime `Pending`
(`outer-contract: flb_external_plugin`). Without that line it would be
runtime `Unknown`.

**librdkafka worked example.**

```qcore
extern fn rd_kafka_plugin_new(conf: ptr<mut, u8>, path: ptr<const, u8>, errstr: ptr<mut, u8>, errstr_size: u64) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] rk_plugin_library, "…as in A…";
    quant #outer where #conf in #loaded [[runtime]] when #result == #NO_ERROR;
    quant #outer #loaded: rd_kafka_conf_s.log_cb |-> #registered;
    // … one line per rk_conf callback member and interceptor list …
    quant #outer #loaded: rd_kafka_broker_s.rkb_rk |-> #kept;
    quant #outer #loaded: #premise assignor_callback;
    quant #outer #loaded: #premise broker_monitor_callback;
};
```

The 163 premise clauses are retired (the envelope states the closed world
once). `rd_kafka_topic_partition_destroy`: static `Safe`; runtime `Pending`
through the `log_cb` line. `cJSON_CreateObject` (resting on
`cjson_allocator_hooks`, a global that any loaded code can reach through the
exported `cJSON_InitHooks`) is runtime `Unknown` unless a line vouches for it.

*Cost and risk:* fail-closed by construction (an uncited fact cannot stay
`Safe`); one place states what a mechanism may do; it needs the fact keys of
K2 run sets in the row record (today a custody row names the declaration,
not the member key) and a reachability pass.

### Alternative C: the runtime condition on the fact, per pointer

The literal shape of the user's sketch. The loader only defines the contract
(`quant #outer [[contract]] NAME, "…";`, no receive clause). Each fact
carries its own guard, naming the pointer whose object loaded code might
hold:

```qcore
extern fn flb_output_new(config: ptr<mut, u8>, output: ptr<const, u8>, data: ptr<mut, u8>, public_only: i32) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    requires nonnull_member(flb_output_instance, p) where quant #scope [[stable]];
    quant #outer where #result in #loaded [[runtime]] [[contract]] flb_external_plugin;
};

// librdkafka client.qv, replacing the premise on an rk_conf-reaching declaration
extern fn rd_kafka_topic_partition_destroy0(rktpar: ptr<mut, u8>, do_free: i32) -> unit
contract {
    requires resource_contract_v1;
    quant #outer where #rktpar in #loaded [[runtime]] [[contract]] rk_plugin_library;
    …
};
```

The fact holds unconditionally for an object no loaded code can hold, and
rests on the contract for one that it can. Today no route tracks which
objects a loader's hand-off reaches, so every pointer may be in `#loaded`
and C behaves exactly as A; the binder leaves room for a later card to
separate, say, an `rd_kafka_t` built from a conf without
`plugin.library.paths`. Worked examples as in A, with the same verdicts.

*Cost and risk:* as A, plus a binder whose precision is not yet used; the
guard is repeated on every fact.

## 5. Semantics common to the three

* **Verdicts.** Static scope (nothing loaded): every verdict is today's; the
  row's dependency record lists `outer:NAME` for each contract its
  closed-world facts rest on (A, C: cited; B: derived). Runtime scope:
  `runtime(row) = static(row) ⊔ ⨆ c(fact)` over the row's closed-world facts,
  with `c = Pending(outer-contract: NAME)` for a fact the contract NAME
  covers and `c = Unknown(outer-uncovered)` for one no contract covers (in A
  and C, an uncited fact reachable from `#loaded` is not detected; that is
  their risk). The join is the existing `Safe < Pending < Unknown < Unsafe`
  (§L21.2): `Safe` becomes `Pending`, `Pending` stays `Pending` with both
  reasons, `Unknown` and `Unsafe` are unchanged. Nothing is promoted, and no
  static row moves.
* **Reason.** A new `PendingReason::OuterContract` (category
  `outer-contract`), carrying the contract name and the loader names; the
  row's text is `rests on outer contract flb_external_plugin (loaded by
  flb_plugin_load, flb_plugin_proxy_create) through
  stable:flb_output_instance.p`. Resolved by a person who reviews the loaded
  code against the contract, or by a deployment that loads nothing.
* **Publication.** One run publishes both scopes: the report's rows stay the
  static verdicts (the headline, unchanged), and a sidecar section
  `outer_scopes` gives, per outer contract and combined, the runtime tallies
  and every row that moves. The runtime tallies are never mixed into the
  static denominators. (Alternative publication, for the user to pick: a
  profile switch that publishes the runtime scope as the row verdict.)
* **Pinning and digests.** An outer module is a provider module: its
  declarations are pinned with `quirin-audit pin` to the loaders' bodies at
  both revisions, `qcore_contract_corpus.rs` pins its clause counts, and the
  contract digest covers the statement, the runtime condition and the
  envelope. A citation (A, C) is part of the citing declaration's text, so
  adding one re-pins that declaration. The row record names
  `outer:NAME@<digest>`; changing the contract takes it out of force, and
  then the runtime scope has no covering contract (`Unknown`), while the
  static scope is unchanged. A loader declaration can be checked
  structurally where its unit is analysed (its body calls a declared
  dynamic loader, `dlopen` / `rd_dl_open`, and the receive formal flows to
  the loaded entry point); otherwise it is a declared assumption.
* **Rule 13h.** It says no verdict reads the dependency record. The runtime
  projection does read it (and only lowers); rule 31 would amend 13h for that
  one reader. Static verdicts still read nothing of it.
* **Lean** (`QCore.OuterContract`, registered in an `Audit*.lean`):
  `static_projection_is_identity`; `runtime_never_promotes`
  (`static ≤ runtime`); `a_covered_fact_is_pending_with_its_contract`;
  `an_uncovered_fact_is_never_safe_or_pending`; `a_row_without_closed_world_facts_is_unchanged`;
  `runtime_is_the_join` (associative, commutative, monotone, so the order
  of facts does not matter); `every_outer_pending_names_a_contract`
  (invariant 2); for B also `reachability_over_approximates` (every record a
  write from a `#loaded` binder can reach is in the computed closure). No
  axioms.

## 6. Recommendation

**Alternative B**, with A's spelling for the header and citation-free facts:

1. It is fail-closed by construction (hard invariant 1): in the runtime
   case an uncovered closed-world fact is `Unknown`, so forgetting a
   citation can never leave a wrong `Safe`. A and C rely on every author
   citing every fact.
2. It puts the user's `quant #outer where #P in #loaded [[runtime]]` where
   the hand-off happens, on the loader, pinned to that body.
3. It states a mechanism's closed world once (one envelope instead of 163
   premise clauses) with the user's `|->` and value keywords.
4. The static case is identity, as the user asked ("Still seated as Safe
   without the loading plugin").

Staging for the implementation card (rule 31): first the vocabulary, the
`outer-contract` reason, the two-scope publication and K1 coverage (Fluent
Bit's four `p` members); then `rk_plugin_library` replacing the premise
(K1 `rkb_rk`, K2 rk_conf members, K3 premises). K2 keys need to be named in
the row record first.

**Decisions for the user:**

* D1: A, B or C (recommended: B).
* D2: the spellings `quant #outer [[contract]] NAME, "…";` /
  `quant #outer where #P in #loaded [[runtime]] when …;` /
  `quant #outer #loaded: TAG.FIELD |-> #kept | #registered;` /
  `quant #outer #loaded: #premise NAME;` (new value keywords `#loaded`,
  `#registered`; `#kept` exists).
* D3: publication: both scopes from one run (recommended), or a profile
  switch.
* D4: whether M8 (the library-mode host) is in the first implementation
  (recommended: later; no row rests on it today).

## Files

Scripts: [build.sh](scripts/build.sh), [measure.sh](scripts/measure.sh),
[census-run.sh](scripts/census-run.sh), [loader_units.py](scripts/loader_units.py),
[outer_rows.py](scripts/outer_rows.py), [project.py](scripts/project.py),
[tables.py](scripts/tables.py). Artifacts:
[loader-units.json](artifacts/loader-units.json),
[outer-rows.json](artifacts/outer-rows.json),
[projection.json](artifacts/projection.json), [run.json](artifacts/run.json)
(binary, head, report digests, exits and wall times: corpus 962 s, system
228 s, slice 169 s inside the lock).
Private outputs: `private/ocd/` (binary, corpus and system runs, null
streams) and `/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/ocd-main/`
(slice).
