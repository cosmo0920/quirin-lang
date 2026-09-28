# W47-LOOP-OWNER-TYPED-DISPATCH: type-keyed field dispatch and a plugin copy's instance flag

Card W47-LOOP-OWNER-TYPED-DISPATCH (wave 47, card-high), amending gateway
rule **10p**. Base: local `main` at `27854776` (this worktree's HEAD was
already there). Frozen prediction: [PREDICTION.md](PREDICTION.md) (commit
`91727a6d`, before any census, gateway, kernel or Lean change).

Nothing here is a verified race, a verified fix, an `Unsafe` claim or an
upstream defect claim. Every old-vs-fixed statement reads *the owner
obligation is discharged / not discharged / unresolved at this path under
these derived premises*.

## 1. What held the fixed test after the coroutine hop

W47-LOOP-OWNER-COROUTINE-HOP left `cb_tail_pause_test`'s three sites
(`tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) unresolved on two
over-approximations: field-keyed V (a call through `x->F` reached every
registrant of `F` in any struct, so the Go proxy's
`plugin->cb_collect(&data, len)`, `src/proxy/go/go.c:248`, a
`struct flbgo_input_plugin` field, dispatched every collector) and the
instance flow (the test's callback is registered on a runtime copy of the
tail plugin, so its coroutine flag was not derived).

Reading the source before changing the census corrected the brief's model of
Step B: the route switch reads the **instance's** flag
(`collector->instance->runs_in_coroutine`, `src/flb_input.c:3313`,
`src/flb_input_thread.c:162`), which only `flb_input_new` writes, from the
plugin the instance was **created** from (`:501`, `:625-626`). The test copies
the plugin after the instance exists (`test.plugin = *ins->p`, `:2554`;
`ins->p = &test.plugin`, `:2556`), so the copy's `.flags` alone cannot decide
the route. Step B therefore derives the copy's flag as the brief asks and
joins it with the instance's creating plugin's (fail-closed conjunction).

## 2. Mechanism

1. **Typed reading** (`crates/qcore-c-gateway/src/typed_dispatch_census.rs`,
   new). For each compile command it streams Clang's `-ast-dump=json` one
   top-level declaration at a time (tracking Clang's de-duplicated
   `file`/`line` attributes in document order; no recursion limit, a 256 MiB
   reader stack) and records, keyed by the spelling file and line:
   a **dispatch** (a call whose callee is, through parentheses, implicit
   conversions and `*`, a member `base->F`), with the record declaring the
   field Clang resolved (`referencedMemberDecl`, so typedefs are resolved by
   construction; an anonymous record takes the typedef naming it); a
   **store** (`base->F = f`, and each record initializer slot, mapped to the
   record's fields in order); a **parameter store** (how a registration
   function stores its callback); and the string literal an instance
   constructor is called with and each descriptor's `.name`. A record is
   **unmatched** for a union field, an anonymous record no typedef names, an
   explicit cast on the dispatch's object path or around its callee, a stored
   function that is converted, or initializer slots that do not match.
2. **Type-keyed census** (`thread_entry_census.rs`, `read_program_typed`).
   A registration is keyed by the records the typed reading stored it in, or
   by *any* record when no typed fact names the site (the field-keyed
   reading); a collector registration (hop R) is keyed
   `struct flb_input_collector` only when the typed reading shows a parameter
   stored in its `cb_collect` (`collector_create`, `src/flb_input.c:2245`). A
   dispatch reaches only the registrants keyed by one of its records or by
   *any*; an untyped dispatch reaches every registrant (unchanged); an
   **unmatched dispatch reaches none and makes every registrant of the field
   unresolved**, and an unmatched store makes its function unresolved and
   keeps it keyed *any*. Without typed facts the reading is byte-identical to
   the pre-card one (§4).
3. **Step B** (`derive_instance_flags`). For a collector registrant
   registered outside `plugins/`: it must be registered in a function `P` on
   `P`'s first parameter; `P` must be stored only in fields of a local plugin
   copy `L` (never called directly, spawned or address-taken); in the
   function `G` storing it, every use of `L` must be modelled (a whole copy
   `L = *X->p` or `L = D` or `memcpy(&L, &D, …)`, field stores, `.flags`
   writes by `=` or `|=` of numbers and numeric macros, and exactly one
   installation `X->p = &L`), and the object holding `L` must not escape. The
   instance `X` must be assigned once in `G` from `flb_input_new` with a
   plugin name the typed reading reads, naming exactly one descriptor, and
   `runs_in_coroutine` must be written by `flb_input_new` alone. The route is
   dropped only when the copy's flag **and** the instance's creating plugin's
   derive no coroutine; the coroutine flag anywhere, or any underived part,
   keeps it.
4. **Runners**: `examples/typed_dispatch_facts.rs` (new; POSIX-splits each
   recorded command, reads units 8 at a time) and `loop_owner_census.rs`
   (`--typed-facts=PATH`, repeatable; the typed section of its output lists
   dispatch typing, Step B's derivations, each yielding body's shortest yield
   path and the seeds the typed reading adds). Scripts:
   [`facts.py`](scripts/facts.py), [`census.py`](scripts/census.py),
   [`same_census.py`](scripts/same_census.py),
   [`arbiters.py`](scripts/arbiters.py), [`identity.py`](scripts/identity.py),
   [`gate.py`](scripts/gate.py), [`update_ledger.py`](scripts/update_ledger.py).

## 3. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_typed_dispatch.rs`
(10 tests; compilable C fixtures of the `flb_input.h`, `flb_input.c`,
in_tail, a coroutine plugin, the Go proxy's other struct and the test's
shapes, compiled by Clang and read by the census):

| Control | Outcome |
|---|---|
| two struct types with the same field name | field-keyed: `other_dispatch` reaches in_tail's pause and T runs it; typed: it reaches only `struct other_plugin`'s registrant, which stays **not discharged** on T |
| a dispatch through a cast base; through a union field | every `cb_pause` registrant **unresolved** (record not matched) |
| a store through a conversion | typed fact unmatched; the lexical reading already refuses it (address taken) |
| a file without a typed reading | its dispatch and its registrant keep the **field-keyed** reading |
| the fixed-test shape (`ins` the list head, `ins->data = &test`) | flag **underived** (both reasons named); **unresolved** while a yield is reachable |
| a copy whose `.flags` is written with `FLB_INPUT_CORO` | flag derived *coroutine*; route kept, **unresolved** |
| a copy passed to an unmodelled function | **underived**, route kept |
| a second writer of `runs_in_coroutine` | **underived** |
| the same shape on `flb_input_new(…, "tail", …)`, holder local | **discharged** on `(flb_lib_worker, NT)` and `(input_thread, T)`; without the typed reading, unresolved |
| typedef'd and anonymous-typedef records, the test's own dispatch | keyed `struct flb_input_plugin` / `typedef anonymous_t` |

## 4. Census: `90978ac6~1` against `90978ac6`

Same 1,685 files and four readings as the coroutine-hop census. BEFORE
(base example binary `2ad0cf46…`) reproduced that archive byte for byte, and
this card's binary run **without** typed facts reproduced it again (all four
SHA-256s, [`same_census.py`](scripts/same_census.py)). The typed reading
([artifacts/facts/](artifacts/facts/)): 555 of 559 selected build units read
(4 fail on headers this reading does not find in the build tree: three
bison-generated parser headers and `backtrace.h`;
`plugins/processor_sql/parser/sql_parser.c`, `src/flb_dump.c`,
`src/record_accessor/flb_ra_parser.c`,
`src/stream_processor/parser/flb_sp_parser.c`), plus the test file (with
`src/flb_lib.c`'s arguments and a configured `flb_tests_runtime.h`) and the
previous test (copied beside that header, keyed as the test). AFTER archive:
[artifacts/census/](artifacts/census/); comparisons:
[artifacts/compare/](artifacts/compare/).

| `90978ac6` configured census | BEFORE | AFTER (Step A = Step A + B) |
|---|---|---|
| edges | 37,188 | 33,573 |
| unresolved seeds | 2,849 | 2,854 |
| field dispatch sites typed / unmatched / untyped | — | 333 / 1 / 577 |
| sites discharged / not discharged / unresolved | 0 / 130 / 18 | **8** / 130 / 10 |

Raw readings and `~1` move the same way (raw `90978ac6` 37,051 → 33,493
edges; at `~1` 0 / 132 / 15 → 8 / 132 / 7 raw and 0 / 136 / 15 → 8 / 136 / 7
configured). Owners unchanged (engine `src/flb_lib.c::flb_lib_worker`, worker
`src/flb_input_thread.c::input_thread`).

**Every moving site** (16 of 148, the same at both revisions but the three
test sites):

| Sites | BEFORE | AFTER | Attribution |
|---|---|---|---|
| in_docker_events ×4 (`:87`, `:125`, `:159`, `:197`), in_event_test `:124`, `:163`, in_fluentbit_metrics `:82`, in_stdin `:272` | unresolved (only the Go proxy's post-yield reason) | **discharged** on `(flb_lib_worker, NT)` and `(input_thread, T)` | `go.c:248` is typed `struct flbgo_input_plugin`, whose only stores are `flb_plugin_proxy_symbol(…)` results: it reaches no registrant of the read files |
| in_calyptia_fleet ×3, in_event_test `:188`, `:218` | unresolved (own + proxy) | unresolved, own post-yield reason only | the proxy's reason no longer reaches them |
| fixed test ×3 | unresolved (own + proxy) | unresolved, own post-yield reason only | its yield path is now all same-type: `flb_input_pause > flb_input_plugin_pause > (in_emitter) cb_emitter_pause > in_emitter_ingest_ring_buffer > flb_input_log_append > … > flb_processor_run > (filter_ecs) cb_ecs_filter > get_ecs_cluster_metadata > flb_http_do > … > flb_io_net_write > net_io_write_async` |

No site moves between not discharged and anything else, so there is no
Safe → non-Safe (discharged → non-discharged) correction. 12 refused sites
change witness only: 8 lose a cross-type `flb_output_exit > <input plugin>_exit`
edge (output plugins' `cb_exit` reached input plugins') for the same-type
`flb_input_exit_all > flb_input_instance_exit` path, and 4 in_calyptia_fleet
sites lose a cross-type `downstream_worker_thread > cb_log_to_metrics_init`
edge for the same-type `flb_filter_init > cb_log_to_metrics_init` path. The 105 other-plugin
refusals (109 configured) all run through the threaded test's T call of
`flb_input_plugin_pause` (56) or `_resume` (49), typed `struct flb_input_plugin`
(every input plugin's callbacks are that type's registrants), plus 4
in_calyptia_fleet sites refused on `(input_thread, NonThreaded)`, a mode the
census cannot exclude on the worker thread (F without configuration flow). The +5 unresolved seeds come from this card's unmatched stores (8
functions carry the new reason, 3 of them already seeds; converted: `flb_malloc`/`flb_realloc`/`flb_free` into the WASM allocator,
four Monkey plugin API slots; a union: `supervisor_parent_signal` into
`sa_handler`); none holds a declared site. The one unmatched dispatch
(`src/stream_processor/flb_sp.c:1115`, `cb_func` through a cast) has no
registrant.

**Step B** derives no flag at this export (so after Step A and after Step B
are the same census): `cb_tail_pause_test` is underived because the object
holding the copy escapes (`ins->data = &test`, `:2557`) and the instance is
not tied to its creation (`ins = mk_list_entry(head, …)`, `:2552`; the test
creates it with `flb_input(ctx->flb, "tail", NULL)`, `:226`, which returns a
descriptor number); `flb_proxy_input_cb_collect` is also stored in a field;
`flb_input_ingress_collector` is registered from a function called directly.

**Old against fixed.** At `90978ac6~1` the previous test's NC1 path stays
**not discharged** (T; its six `running` reads and the plugin path). At
`90978ac6` the fixed callback's three sites are **unresolved at this path
under these derived premises**: every derived context is an owner context
(`(flb_lib_worker, NT)`, `(input_thread, T)`, and the coroutine route's
resumers), and what remains is the post-yield resumer of a coroutine route
its instance is not derived not to take. They would discharge if the
instance flow were derived (the fixture with `flb_input_new(…, "tail", …)`
does), resting on: the typed and lexical readings, P-CORO-HANDLE,
P-PLUGIN-DIR, and this card's P-CALLBACK-INSTANCE (below). The in_tail
plugin's 16 collector sites and NC3's 2 stay **not discharged** at both
revisions.

## 5. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| fixed test ×3 after A | unresolved, proxy reason gone (~60%) | **as predicted** |
| fixed test ×3 after B | unresolved (~90%) | **as predicted**, both named blockers |
| in_tail ×16, NC3 ×2, previous test's ×6 | not discharged | **as predicted** |
| Go-proxy-held ×8 | discharged (~65%) | **discharged** |
| calyptia ×3, event_test `:188`, `:218` | own yields only (85%) | **as predicted** |
| other-plugin 105 / 109 | ≥ 95 stay | **all stay**; 12 witnesses re-attributed |
| edges / seeds | edges fall; seeds may rise | 37,188 → 33,573; +5 seeds |
| corpus / system / slice | byte-identical | see §6 |

## 6. Corpus, system and tail-pause slice

The per-unit routes read no census and no typed fact, so none may move.
[`scripts/arbiters.py`](scripts/arbiters.py) ran all three with `--jobs 8`
BEFORE (base binary `46400329…`, at `27854776`) and AFTER (binary `26834011…`,
at `61cd5dda`); [`scripts/identity.py`](scripts/identity.py) compares each
with the W47-LOOP-OWNER-CONTRACTS archive and composes it (compose3);
[`scripts/before_after.py`](scripts/before_after.py) compares the two runs file
by file. Summary: [artifacts/arbiters.json](artifacts/arbiters.json).

| Measure | BEFORE | AFTER |
|---|---|---|
| corpus (185 units, `66910c10`) | `resource-diagnostics.json` `a843a690…`, byte-identical to the archive; 526 / 10 / 4,298 functions, units 6 / 8 / 167 / 4, leak 36, unknown_alias 7,936, unsupported 35,737; compose3 0 mismatches | **byte-identical to BEFORE** (report and every sidecar; `parity.jsonl` identical once sorted, `audit.log` differs only in its output path) |
| system (`046accf6`) | `b057e5cc…`; compose3 0 mismatches (31 / 2 / 201; unknown_alias 1,020, unsupported 2,888); differs from the archive only in provenance digests (30 + 15 lines) | **byte-identical to BEFORE** |
| tail-pause slice (`90978ac6` export) | `4a911b52…`; compose3 0 mismatches (units Unknown 12, Unsafe 1; functions Safe 24, Unsafe 1, Unknown 245); provenance digests only (32 + 16 lines) | **byte-identical to BEFORE** |

No function, unit, leaf or count moves; no Safe → non-Safe correction; no
leak or violation is added or removed, so none needs classifying; no new
baseline archive.

## 7. Lean

`proof/QCore/LoopOwner.lean`, audited in `proof/AuditLoopOwner.lean` (34 → 43
theorems, no new audit module, no placeholders, no project axioms):
**`type_keyed_is_a_sub_relation`** (type-keyed dispatch only removes edges),
`an_untyped_dispatch_is_field_keyed`,
**`a_call_through_a_record_reaches_only_its_registrants`**,
**`a_removed_edge_is_cross_type_or_refused`** (only spurious edges are
removed; an unmatched dispatch's registrants are unresolved),
`an_unmatched_dispatch_never_discharges`, `no_coroutine_needs_both`,
`a_coroutine_copy_keeps_the_route`, **`an_underived_part_keeps_the_route`**,
`the_route_is_dropped_only_when_both_derive_none`.

## 8. Premises stated, not proved

Trust boundary 28, beside rule 10p's: the recorded build's compile commands
are the program's, and a site the typed reading does not see keeps the
field-keyed reading (577 dispatch sites: 572 in files the build does not
compile, 4 in headers, 1 in `lib/monkey/mk_server/mk_http.c:746`); C's
effective-type rule (an object is not called through a record type other than
its own); the test file is typed with `src/flb_lib.c`'s arguments and a
configured `flb_tests_runtime.h`; **P-CALLBACK-INSTANCE** (a callback stored
in an input plugin object's field runs with an instance whose `p` is that
object); and `runs_in_coroutine` is the route switch, set from the creating
plugin (checked lexically: `flb_input_new` is its only writer).

## 9. Follow-ups

[`followups.json`](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 547 → 548,
registering `W47-LOOP-OWNER-UNTYPED-SITES` (577 untyped dispatch sites and 4
units Clang cannot read keep the field-keyed reading) and amending
`W47-LOOP-OWNER-FIELD-KEYED-V-AND-F` (the cross-type part discharged, 8 sites;
121 / 125 refusals remain, all same-type or F), `W47-LOOP-OWNER-INSTANCE-FLOW`
(Step B built; the 3 fixed-test sites held by the list-head instance and the
escaping holder) and `W47-LOOP-OWNER-COROUTINE-POST-YIELD` (4 bodies now hold
8 declared sites; the proxy holds none). Nothing is retired: neither
DISCHARGE clause is met.

## 10. Gate

GATE_PLACEHOLDER

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/census.py OUT CENSUS_BINARY \
  --typed-library=OUT/library.json --typed-fixed=OUT/fixed-test.json --typed-previous=OUT/previous-test.json
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/same_census.py RUN ARCHIVE
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/identity.py OUT IDENTITY_OUT
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/gate.py LOGS rust|records
```
