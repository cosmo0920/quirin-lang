# W47-PREMISES-CORO-PLUGIN: P-CORO-HANDLE, P-PLUGIN-DIR and the immutable threaded flag derived

Card W47-PREMISES-CORO-PLUGIN (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `fe603d49` (the worktree's HEAD at start). The
frozen prediction is [PREDICTION.md](PREDICTION.md), commit `3623c02e`, made
after BEFORE was measured and before any Rust, Lean, AFTER census, arbiter or
contract change (only the export was read, with `grep`/`sed`).

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement has the form
*discharged / not discharged / unresolved at this path under these derived
premises*. The Fluent Bit implementation and test are unchanged; the
fluent-bit-dev checkout was not opened (the prepared exports were read).

## 1. Result

With a typed reading (every measured census reading has one) the census now
**derives** the three premises from the program at `90978ac6` and at
`90978ac6~1`, and makes every discharge that rests on one depend on its
derivation: a premise it does not derive leaves the dependent sites
unresolved (§2). At both revisions all three derive, each down to a named
residual (§3). The fixed test's three sites (`cb_tail_pause_test`,
`tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) stay **discharged at this
path under the derived premises** on `(flb_lib_worker, NonThreaded)` and
`(input_thread, Threaded)`; the old test's 8 sites at `90978ac6~1` stay **not
discharged** (`T Either`). No site moves in any of the six readings; 27 edges
are added in each, every one attributed to a registrant whose registration
the census now declines to derive (§5). Corpus, system slice and tail-pause
slice sidecars are byte-identical to BEFORE (§6).

No premise was found false for this program. The derivation is fail-closed:
eight negative controls (§4) show each premise's violation removing the
dependent discharge.

## 2. Mechanism

`crates/qcore-c-gateway/src/loop_owner_coro_plugin.rs` (new, child of
`thread_entry_census.rs`), reading the census's own lexical reading and new
typed member facts: the typed options (`instance_flow_options`,
`property_key_options`) now also record every access of
`struct flb_input_coro::coro`, `struct flb_libco_in_params::{coro,coll}`,
`struct flb_input_instance::input_coro_list`,
`struct flb_input_thread_instance::input_coro_list`,
`struct flb_input_plugin::flags` and `struct flb_input_instance::is_threaded`
(with every unmatched-record access of those field names), the key literal of
each `flb_input_set_property` call, and the key literal of each
`prop_key_check` inside it. A premise the typed reading did not record is not
derived.

**P-CORO-HANDLE** (the census's hop C takes an input coroutine's first
resumers to be its constructor sites). Derived when: every typed access of
the three handle members stands in the coroutine's closed set
(`flb_input_coro_create`, `flb_input_coro_collect`, `input_params_set`,
`input_pre_cb_collect`, `flb_input_coro_resume`, `flb_coro_resume`,
`flb_input_coro_destroy`, `flb_coro_destroy`); inside those bodies a handle
(a handle member read, a local assigned from one or from `flb_coro_create`,
a listed parameter) is only dereferenced, tested, stored whole into a handle
member or handle local, or passed to a listed parameter of a listed function
or to `flb_free`/`flb_trace`/`FLB_CORO_DATA`/`flb_coro_set`; the creation and
the hand-off (`flb_input_coro_create`, `input_params_set`) are called only
inside the set and never named otherwise; both `input_coro_list`s are only
`mk_list_init`/`mk_list_add` arguments; no unmatched-record access of `coro`,
`coll` or `input_coro_list` exists; and no function a unit Clang could not
read may compile (the failed unit and every header its `#include`s reach by
file name) names the member. **Not derived ⇒** every coroutine entry is
unresolved (`P-CORO-HANDLE is not derived: …`) and no coroutine route is
dropped.

**P-PLUGIN-DIR.** (a) Descriptor flags: every typed write of a
`struct flb_input_plugin`'s `flags` goes into a fresh allocation (a local
assigned once from an allocator, behind at most a cast, named before the
write only in its own member stores and null tests); its address is never
taken; no unmatched-record `flags` write and no untyped function a failed
unit may compile writes `flags`; `runs_in_coroutine` is written by
`flb_input_new` alone; and every `flb_input_coro_collect(C, …)` stands in the
then-block of `if (C->instance->runs_in_coroutine) {…}`, `C` not reassigned
between. **Not derived ⇒** no directory flag and no Step B flag is used, and
no route is dropped. (b) Registrations, per collector registrant registered
only in `plugins/D`: every registration's instance is an unassigned,
never-address-taken parameter of the registering function, chained through
direct calls passing an unassigned parameter to either a callback stored only
in `D`'s `struct flb_input_plugin` initializers (index 0; P-CALLBACK-INSTANCE
derived for those fields by W47-LOOP-OWNER-INSTANCE-FLOW's check; every
typed write of an instance's `p` is `flb_input_new`'s or installs a copy of
the same instance's plugin, `H.M = *X->p;` then `X->p = &H.M;`, into which no
function defined under `plugins/` is stored) or a collector callback of `D`
so registered, whose every `cb_collect` dispatch passes `C->instance` of its
own collector (and a collector's `instance` is its registration's). **Not
derived ⇒** that registrant's directory flag is not used: it keeps the
coroutine route.

**The immutable threaded flag.** Every typed write of `is_threaded` is
`flb_input_set_property`'s, in the innermost `if (…) {` block whose one
`prop_key_check` the typed reading keys `"threaded"` (its key parameter
never assigned), or into an instance its writing function created itself (a
local assigned once from an allocator or from `flb_input_new`); a parameter
or field base is a run-time write. Every setter call whose key the typed
reading reads as one literal other than `threaded` (ignoring ASCII case, one
call on its line) cannot write it; the setter is never named other than by a
call. **Not derived ⇒** every edge that fixes a mode (hop L's dispatch, hop F's
requirement, hop C's dispatching constructor) makes its callee unresolved.

**Hooks** in `thread_entry_census.rs` (lines named in §9): the
`CensusReading::premises` report; the options; after Step B, the derivation,
the entry escape and the route gate; the directory/Step B flag gate in
`runs_in_coroutine`; the mode escape before the edges are published. The
`loop_owner_census` example prints `typed.premises`. A reading without typed
facts derives none and prints none (the earlier cards' lexical-only reading;
no measured reading is untyped).

## 3. What each premise derived, and what is left

At `90978ac6`, configured. The previous test's configured reading and both
controls report exactly the same. The two raw readings read 4 files
(`src/flb_config.c`, `plugins/in_kafka/in_kafka.c`, `plugins/in_statsd/statsd.c`,
`plugins/in_tail/tail_file.c`) as unstructured, so their functions carry that
reason and are not read for these derivations: they list 2 fewer residual
setter calls (`src/flb_config.c:1384`, `:1388`), 29 other-literal calls and
fewer registrants (the functions not read), with every premise still derived.

- **P-CORO-HANDLE: derived.** `struct flb_input_coro::coro` is accessed in
  `flb_input_coro_create` (write), `flb_input_coro_collect`,
  `flb_input_coro_resume` and `flb_input_coro_destroy` (reads);
  `struct flb_libco_in_params::{coro,coll}` only in `input_params_set`
  (writes) and `input_pre_cb_collect` (reads); `input_coro_list` only
  `mk_list_init` (`src/flb_input.c:516`, `src/flb_input_thread.c:251`) and
  `mk_list_add` (`include/fluent-bit/flb_input.h:624`). **Residual
  P-CURRENT-HANDLE**: the running coroutine's handle of itself — the
  thread's current coroutine `flb_coro_resume` sets, read by `flb_coro_get()`,
  and `co_active()` — is never used to resume the coroutine while it runs
  (25 call sites in the program, listed in `typed.premises`). Before its
  first resume a new input coroutine is nobody's current coroutine
  (`input_params_set` switches in without `flb_coro_set`), so this concerns
  only a resume racing the running body; a resume after a yield is already
  the unresolved post-yield case (`W47-LOOP-OWNER-COROUTINE-POST-YIELD`).
- **P-PLUGIN-DIR (a): derived.** The one run-time `flags` write is the Go
  proxy's, `in->flags = def->flags | FLB_INPUT_THREADED` into its fresh
  `flb_calloc` before `mk_list_add` publishes it
  (`src/flb_plugin_proxy.c:405`); `runs_in_coroutine` is written by
  `flb_input_new` alone; both constructions (`src/flb_input.c:3314`,
  `src/flb_input_thread.c:163`) stand under their collector's
  `runs_in_coroutine`. **Residual P-DESCRIPTOR-FLAGS-BY-MEMBER** (a
  descriptor's flags are written only through member accesses: no
  whole-object store or byte copy into a registered descriptor), beside
  W47-LOOP-OWNER-INSTANCE-FLOW's P-REGISTERED-DESCRIPTORS.
- **P-PLUGIN-DIR (b): derived for 61 of 88** registrants registered only
  under `plugins/`, among them every registrant of the 8 discharged plugin
  sites (in_docker_events' `cb_reconnect` and `in_de_collect`, in_event_test's
  `cb_collector_time` and `cb_collector_fd`, in_fluentbit_metrics'
  `cb_metrics_collect_start`, in_stdin's `in_stdin_collect`) and all of
  in_tail's. The 27 not derived keep the coroutine route: 22 whose chain's
  P-CALLBACK-INSTANCE meets `plugins/in_node_exporter_metrics/ne.c:157` (ne's
  own collector-table `cb_init` dispatch, which does not pass the instance
  whose `p` it goes through), 4 registering on a value that is not one
  variable (`ctx->ins` and the like: in_etw, in_forward, in_tcp, in_udp), 1
  whose chain reaches no descriptor callback (in_podman_metrics).
- **Immutable threaded flag: derived.** Writers: `flb_input_set_property`'s
  `threaded` arm (`src/flb_input.c:1002`), `flb_input_new` into its fresh
  `flb_calloc` (`:631`), `trace_chunk_pipeline_thread` into the instance it
  created with `flb_input_new` (`src/flb_chunk_trace.c:189`); for both
  creation writes no call between the creation and the write reaches
  `collector_create` over the census graph. 31 setter calls pass another
  literal key. **Residual P-THREADED-SET-BEFORE-START**: the 2 creation
  writes and the 4 setter calls whose key is not one literal
  (`src/flb_lib.c:328` `flb_input_set`, `src/flb_config.c:1384` and `:1388`,
  `plugins/custom_calyptia/calyptia.c:264`) run before the instance they
  write starts.

## 4. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_coro_plugin.rs` (13 tests;
Clang-compiled fixtures of the export's shapes: the coroutine set, the
guarded loop dispatchers, `cb_init` through `p = ins->p`, `flb_input_new`,
the keyed setter, a plugin whose collector pauses itself and may yield):

| Control | Outcome |
|---|---|
| a function outside the set resumes `c->coro` | P-CORO-HANDLE **not derived**; the plugin site **unresolved** (`P-CORO-HANDLE is not derived`) |
| the constructor passes its handle to an unlisted call | not derived (`leaves flb_input_coro_collect`); site unresolved |
| `ins->input_coro_list.next` read | not derived (`a coroutine may be taken from it`); site unresolved |
| `input_params_set` called outside the set | not derived |
| `ins->p->flags \|= FLB_INPUT_CORO` in the collector | P-PLUGIN-DIR flags **not derived**; route kept, site **unresolved** (may yield) |
| a coroutine constructed without testing the flag | flags not derived; site unresolved |
| `ins->is_threaded = 1` in the collector | threaded flag **not derived** (`a run-time write`); site **unresolved** (`the threaded flag is not derived immutable after start`) |
| plugin `in_other`'s `cb_init` registers `in_dummy`'s collector on its own instance | that registrant **not derived**, route kept, its site unresolved; `in_dummy`'s own site still discharged |
| the same helper called from `in_dummy`'s own `cb_init` | derived, discharged |
| the test thread's own call (the old test's shape) | **not discharged** on `T` |
| a non-literal key, a `"Threaded"` literal | listed in the residual; another literal is not |
| every premise derived | the plugin site **discharged** on both owners |
| no typed reading | nothing derived, nothing reported |

The 81 existing loop-owner census tests pass unchanged (their fixtures carry
no violating access).

## 5. Census at `90978ac6~1` and `90978ac6`

[`scripts/census.py`](scripts/census.py) runs W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's
six readings with every reading's function and edge listing; facts from
W47-LOOP-OWNER-TYPED-DISPATCH's `facts.py` and the predefined card's
`predefined_facts.py`, run with each run's own binaries.

**BEFORE** (base binaries `loop_owner_census` `3c2e0d28…`,
`typed_dispatch_facts` `4b7655a4…`, `predefined_macro_facts` `9923a0cc…`;
facts regenerated, byte-identical to the archived digests): all six readings
**byte-identical** to the W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS archive
([`scripts/identity.py`](scripts/identity.py)).

**AFTER** (release binaries at the tip, `loop_owner_census` `fb3e89eb…`,
`typed_dispatch_facts` `417141e9…`, `predefined_macro_facts`
`fb069f28…`; the typed facts gain only this card's members and literals,
[`scripts/facts_diff.py`](scripts/facts_diff.py); predefined facts
byte-identical):

| Reading | sites discharged / not / unresolved | edges | seeds |
|---|---|---|---|
| `90978ac6` configured | 11 / 130 / 7, unchanged | 33,529 → 33,556 | 2,852 |
| `90978ac6~1` configured | 8 / 136 / 7, unchanged | 33,503 → 33,530 | 2,852 |
| `90978ac6` raw | 8 / 126 / 10, unchanged | 33,493 → 33,520 | 2,902 |
| `90978ac6~1` raw | 8 / 132 / 7, unchanged | 33,466 → 33,493 | 2,902 |
| both controls (configured without the predefined facts) | unchanged | +27 each | |

**Attribution** ([`scripts/attribute_edges.py`](scripts/attribute_edges.py),
archived in [artifacts/census/](artifacts/census/)): no site moves (verdict
or first reason) in any reading; every added edge, 27 per reading, is
`input_pre_cb_collect → R` for one of the 27 registrants of §3 whose
registration is not derived (their coroutine route kept); no edge is
removed; functions and seeds are unchanged. No site moves Safe → non-Safe
(the census publishes no `Safe`), and no correction is named.

**Old against fixed.** At `90978ac6` configured the fixed test's 3 sites are
discharged at this path under the derived premises of §7; at `90978ac6~1`
the old test's 8 sites stay not discharged (`T Either`), at both the raw and
the configured reading.

**Condvar/join and lifecycle census** (`w47-flb-lib-lifecycle`'s
`census.sh`, raw reading without typed facts):
all four JSONs (fixed, previous, system, corpus) equal the
W47-FLB-LIB-LIFECYCLE archive once the worktree root is aliased
([`condvar_identity.py`](../w47-loop-owner-predefined-macro-arms/scripts/condvar_identity.py),
exit 0); the old test's C5 status is unchanged. That census has no typed
reading, so none of this card's derivations runs there.

## 6. Corpus, system and tail-pause slice

The per-unit routes read no census. [`arbiters.py`](../w47-loop-owner-typed-dispatch/scripts/arbiters.py)
(`--jobs 8`) and [`identity.py`](../w47-loop-owner-predefined-macro-arms/scripts/identity.py)
against `w47-flb-lib-lifecycle/integration/artifacts`, then compose3:

ARBITER_RESULT

## 7. Premises of the three discharged sites

Stated as: **discharged at `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`
(the `90978ac6` export, configured reading) under these derived premises**,
never a verified fix. Derived at this path: everything
W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS §6 lists as derived, and now
P-CORO-HANDLE, P-PLUGIN-DIR (descriptor flags; the test's callback takes
Step B's route, whose `descriptors_flag("tail")` the derived flags now
guard) and the immutable threaded flag, each down to its residual.

Stated, not proved (trust boundary 28): **P-CURRENT-HANDLE**,
**P-THREADED-SET-BEFORE-START**, **P-DESCRIPTOR-FLAGS-BY-MEMBER** (this
card's residuals); P-HEAD-INSTANCE, P-FRESH-CONTEXT,
P-REGISTERED-DESCRIPTORS, P-HOLDER-OPAQUE, P-INSTANCE-FIELDS-BY-MEMBER,
P-KEY-FOLD, P-INSTANCE, P-RECORDED-COMPILER, P-DD-COMPLETE, acutest's
`TEST_CHECK`, the lexical and typed readings' faithfulness (the typed
reading covers the build's units; 4 failed units, whose files and reachable
headers the lexical fallback reads), and the region-macro-call premise
(`W47-LOOP-OWNER-REGION-MACRO-CALLS`).

## 8. Lean

`proof/QCore/LoopOwnerCoroPlugin.lean` (new, namespace
`QCore.LoopOwner.CoroPlugin`, imported by `QCore.lean`), audited in
`proof/AuditLoopOwner.lean`: 81 → 95 theorems, no new audit module, no
placeholders, no project axioms.

- **`a_census_discharge_covers_every_possible_resumer`**: a discharge under
  the census's resumers is one under every resumer the program may use
  (with `a_confined_handle_has_only_derived_resumers`);
  **`an_escaping_handle_never_discharges`**; `a_use_outside_the_set_escapes`;
- **`a_runtime_flags_write_keeps_the_route`**,
  **`a_registration_on_another_plugins_instance_keeps_the_route`**,
  `a_derived_directory_drops_the_route`,
  `a_parameter_chain_keeps_the_directory`;
- **`no_write_after_start_fixes_the_flag`** (with `valueAt_before_all`,
  `valueAt_after_all`), **`a_runtime_threaded_write_never_discharges`**,
  `another_literal_key_cannot_write_the_flag`;
- **`the_old_test_path_is_never_discharged_whatever_the_premises`**.

## 9. Follow-ups, gate and conflicts

[`followups.json`](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 568 → 572, registering
`W47-PREMISES-CURRENT-HANDLE` (25 sites), `W47-PREMISES-THREADED-SET-BEFORE-START`
(6 sites), `W47-PREMISES-PLUGIN-DIR-UNDERIVED-REGISTRATIONS` (27 registrants,
0 sites) and `W47-PREMISES-CORO-PLUGIN-CONTRACTS` (the residuals as held
provider contracts: not added here, since declaring `flb_coro_get` or
`flb_input_set_property` changes per-unit verdicts at their call sites, an
integrator step with its arbiters); amending
`W47-LOOP-OWNER-COROUTINE-POST-YIELD` and `W47-LOOP-OWNER-INSTANCE-FLOW`.
Retires nothing (no entry tracked these premises by name).

GATE_RESULT

**Files likely to conflict** (the other premise cards amend 10p and extend
`QCore.LoopOwner` too):

- `docs/agents/gateway.md` (a new amendment paragraph after
  W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's, before `constraint-10q`),
  `docs/agents/README.md` (one sentence appended to the 10p row),
  `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md`, `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs`: the `premises` field
  of `CensusReading`; the `#[path]` module declaration after
  `predefined_macros`; three lines each in `instance_flow_options` and
  `property_key_options`; after `derive_instance_flags`, the premise block;
  the first lines of `runs_in_coroutine`'s `from_directory` and its Step B
  fallback; the route gate in the dispatch loop; the mode escape before
  `program.edges`;
- `crates/quirin-source-analysis/examples/loop_owner_census.rs` (the
  `typed.premises` print);
- `proof/QCore.lean` (one import), `proof/AuditLoopOwner.lean` (one import,
  14 lines appended).

No contract, provider, module-count or resource-view pin changes.

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/predefined_facts.py OUT PREDEFINED_BINARY GENERATED_DIR
python3 docs/reports/w47-premises-coro-plugin/scripts/census.py OUT CENSUS_BINARY TYPED_FACTS PREDEFINED_FACTS
python3 docs/reports/w47-premises-coro-plugin/scripts/identity.py BEFORE AFTER
python3 docs/reports/w47-premises-coro-plugin/scripts/attribute_edges.py BEFORE AFTER
python3 docs/reports/w47-premises-coro-plugin/scripts/summary.py CENSUS --premises
python3 docs/reports/w47-premises-coro-plugin/scripts/facts_diff.py BEFORE_FACTS AFTER_FACTS
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/identity.py RUN OUT
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/condvar_identity.py RUN
python3 docs/reports/w47-premises-coro-plugin/scripts/gate.py LOGS rust|records
```
