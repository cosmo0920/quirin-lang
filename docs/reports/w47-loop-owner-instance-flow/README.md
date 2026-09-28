# W47-LOOP-OWNER-INSTANCE-FLOW: the instance a plugin copy is installed on

Card W47-LOOP-OWNER-INSTANCE-FLOW (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `fefb9199`; this worktree's HEAD was already
there. The frozen prediction is [PREDICTION.md](PREDICTION.md), commit
`ec07a1a7`, made before any census, gateway, contract or Lean change. The
integrator later added a step (2026-09-28, the user's decision): observations
the derivation needed but could not derive are written down as provider
contracts.

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement below has
the form *discharged / not discharged / unresolved at this path under these
derived premises*.

## 1. Result

At the `90978ac6` export, the fixed test's three sites (`cb_tail_pause_test`,
`tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) stay **unresolved**. That
holds for the raw reading and for the configured reading. The card derives
two of the three parts of the instance flow that the typed-dispatch card
left open:

- the **holder escape** (`ins->data = &test`);
- **P-CALLBACK-INSTANCE**.

It also builds the third part, the **list-head instance**. Against the
actual code, that part stops on one call on the test's path:
`flb_service_set(ctx->flb, "Flush", …, "Parsers_File", …)`
(`tests/runtime/in_tail.c:218`). `flb_service_set` hands each key to
`flb_config_set_property`, which dispatches on the key's text. Its
`Plugins_File` branch loads plugins and registers them into
`config->in_plugins` (`src/flb_config.c:903-907`, `src/flb_plugin.c:251`),
and it also calls a registration entry point through a function pointer.

The census reads calls, not the keys a variadic call carries, so it cannot
exclude that branch. The route therefore stays kept. That is correct under
fail-closed, not a gap in the other derivations. A diagnostic reading
(§5) blanks that one call: everything then derives, and the three sites
discharge. The diagnostic is not a result.

At `90978ac6~1`, the old test's paths stay **not discharged** (T). No site
moves at either revision.

## 2. Mechanism

**Typed facts.** In `typed_dispatch_census.rs`, each unit's Clang AST now also
records five new fact kinds:

- every access of a listed member, with its use: read, write, address,
  address read through, or address passed to a named call. The listed
  members are:
  - `struct flb_input_instance::{p, runs_in_coroutine, data}`;
  - `struct flb_config::{inputs, in_plugins}`;
  - `struct flb_lib_ctx::config`;
  - `struct flb_input_collector::instance`.
- every access of a listed field name through an unmatched record;
- every function-pointer member read as a value, with its record;
- the functions Clang read;
- every call through a function pointer that is neither a named function
  nor a member (an unmodelled indirect call).

The earlier fact kinds are byte-identical to the typed-dispatch archive (§4).

**List-head instance.** The derivation is in `loop_owner_instance.rs`, a
child module of `thread_entry_census.rs`, and Step B calls it through a
thin hook. It applies when `X = mk_list_entry(H, struct flb_input_instance,
_head)` and `H` is a context's `config->inputs.next`, either inline or in a
local assigned once. The flag is derived only when every one of these
checks holds:

1. The context is a local of the test, assigned once from a function in the
   test's file. That function returns only NULL or one local, and that
   local's field is assigned exactly once from `flb_create()`.
2. On the path to the read, exactly one call reaches a hazard. The path is
   the test up to the read, that function, and `flb_create`, all read in
   place. Two kinds of `return NULL;` block are pruned:
   - in the maker, only where the test returns on a null context
     (acutest's `TEST_CHECK` taken as its argument's truth);
   - in `flb_create`, and in a callee whose null result it returns as null
     (P-HEAD-INSTANCE).
   Each function read in place must itself be free of hazards.
   That one call is not inside a loop, and it is a wrapper. The wrapper
   passes one unassigned parameter as the plugin name to one
   `flb_input_new` call and is called with a literal that the typed
   reading records.
3. Reachability runs over the census graph: every direct call, every
   registrant a dispatch may reach with the coroutine route unpruned,
   spawns, and coroutine entries. A **hazard** is any of:
   - the constructor;
   - an `inputs` or `in_plugins` use other than a read or the three listed
     initialisations (`flb_config_init`'s two `mk_list_init`,
     `flb_input_new`'s `mk_list_add`);
   - a write of a context's `config` other than by `flb_create`;
   - an untyped function naming those members;
   - a function in a file the reader cannot structure, when that file names
     a member, the constructor, or a function that reaches a hazard;
   - a call to a name that only such a file may define.
4. When the path's closure makes an unmodelled indirect call, or contains a
   function the typed reading could not read, no escaped function may reach
   a hazard. An escaped function is one whose address is taken, one named
   in a macro, or a registrant of a function-pointer field read as a value
   through a matching record. The typed reading refines the census's
   field-name poisoning here.
5. Every descriptor whose name matches the literal, ignoring ASCII case as
   `check_protocol` compares, derives no coroutine. The existing literal
   constructor path now matches the same way. That is a strict tightening:
   nothing at the export is affected.

**Holder escape.** The escape is admitted only as `X->data = &holder` into
the installed instance, and only when all of these hold:

- the holder's `struct` is named in the test's file alone;
- in that file, the copy member is written, incremented or has its address
  taken nowhere but in the function Step B models;
- the typed writers of `struct flb_input_instance::p` are exactly
  `flb_input_new` and that installation.

`runs_in_coroutine` keeps its existing whole-program single-writer check.

**P-CALLBACK-INSTANCE.** Before, this was stated. Now it is derived, and if
it cannot be derived the flag stays underived. Every typed or untyped
dispatch of the copy's callback field that may reach the callback must take
one of these forms:

- `X->p->F(X, …)`;
- `Q->F(X, …)` with `Q = X->p` assigned once before it;
- the callback on its own unassigned first parameter.

In addition, `struct flb_input_collector::instance` must be written only by
`collector_create`, from a parameter store. Each registration function must
pass its unassigned first parameter to its one `collector_create` call.

The failure reason names every blocker and, beside them, what did derive
(`derived nonetheless: …`). The census example needs no new flag.

## 3. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_instance_flow.rs` has 16
tests. Each fixture compiles the `mk_list`, `flb_input.h`, `flb_config` and
`flb_lib_ctx` shapes, `flb_create`, `flb_input` → `flb_input_new` appending to
`config->inputs`, `collector_create`, the pre-run dispatch, and the test's
maker, list-head read and holder. Clang builds the typed reading.

| Control | Outcome |
|---|---|
| two `flb_input` calls before the read (head ambiguous) | **unresolved**: more than one call reaches a hazard |
| a name that is not a literal (`flb_input(ctx->flb, name, NULL)`) | **unresolved** |
| the creation inside a `for` loop | **unresolved**: reaches a hazard inside a loop |
| a list mutation on the path (`mk_list_add(…, &ctx->config->inputs)`) | **unresolved** |
| the same mutation in the maker itself (a function read in place) | **unresolved** |
| a creator reachable through a function pointer, with an unmodelled call on the path | **unresolved** |
| the same escaped creator, with no unmodelled call on the path | **discharged** (the escape cannot be called from the path) |
| a call into a file the reader cannot structure | **unresolved** |
| a holder callback that writes `runs_in_coroutine` | **unresolved** (a second writer) |
| a holder callback that re-points `ins->p` | **unresolved** (an instance's `p` written elsewhere) |
| a holder callback that writes the copy member | **unresolved** |
| a dispatch that passes another instance (`a->p->cb_pre_run(b, …)`) | **unresolved**: P-CALLBACK-INSTANCE not derived |
| a coroutine descriptor at the head (`"coro"`) | route **kept** (flag derived coroutine) |
| a descriptor named `"TAIL"` with the coroutine flag | route **kept** (case-insensitive match) |
| the old test's shape (collector operations from the test root after the start) | **not discharged** (T) |
| the fixed shape | **discharged** on `(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`, all three derivations named |

`loop_owner_typed_dispatch.rs` has one re-attributed assertion. Its
fixed-shape control (`ins = first_input(0)`) no longer names the holder
escape. It stays underived only because it is "not tied to its creation".

## 4. Census: `90978ac6~1` against `90978ac6`

The census reads the same 1,685 files and four readings as before
([`census.py`](../w47-loop-owner-typed-dispatch/scripts/census.py)). BEFORE
used the base binary `a99c62a9…` with the typed-dispatch facts. It
reproduced that card's archive byte for byte on all four SHA-256s. After the
card, the two `~1` readings are still byte-identical to it (`d05ebc0e…`,
`a5edf1ed…`); the two `90978ac6` readings differ only in the fixed test's
instance-flag reason. The final binary is `db63d582…`.

The typed facts were regenerated in 52 s. The run read 555 of 559 units;
the same 4 fail. For both tests and the library, every earlier fact kind is
byte-identical to the archive, with two exceptions: the tests' literal
calls gain the `flb_input` wrapper's, and the previous test's scratch path
differs ([`compare_census.py`](scripts/compare_census.py)). The AFTER
census ([artifacts/census/](artifacts/census/)) and facts
([artifacts/facts/](artifacts/facts/)) are archived.

| Reading | sites discharged / not / unresolved | edges | seeds |
|---|---|---|---|
| `90978ac6` configured | 8 / 130 / 10, unchanged | 33,573, unchanged | 2,854, unchanged |
| `90978ac6` raw | 8 / 126 / 10, unchanged | 33,493, unchanged | 2,902, unchanged |
| `~1` configured | 8 / 136 / 7, unchanged | 33,546, unchanged | 2,854, unchanged |
| `~1` raw | 8 / 132 / 7, unchanged | 33,466, unchanged | 2,902, unchanged |

No site changes verdict or first reason, so no site needs attribution and
no correction applies. The only changed field is the fixed test's instance
flag reason, which is still underived:

**Configured reading.** There are two blockers, and both lie on
`flb_service_set`'s closure:

- `flb_service_set` at `:218` reaches a hazard through
  `flb_config_set_property > flb_plugin_load_config_file >
  flb_plugin_load_router > flb_plugin_load`, which mutates `in_plugins` at
  `src/flb_plugin.c:251`. Beside it, `flb_input` at `:226` is the one real
  creation.
- Two closure functions make an unmodelled call or have no typed reading.
  The first is reached through `flb_parser_conf_file > … > flb_cf_yaml.c::dirname`,
  which is Windows-only and inside a compiled file. So an escaped function
  that reaches a hazard (`output_pre_cb_flush`) may be called from the path.

**Raw reading.** `flb_config.c` is unstructured when read unconfigured, and
the census does not read its `flb_config_task_map_resize`. The inlined
`flb_config_init` calls it at `:531`, so the call may reach a definition
only that file holds.

Both readings add *derived nonetheless*: P-CALLBACK-INSTANCE, and the holder
escape. The readers of `ins->data` are `cb_emitter_init`,
`cb_emitter_pause`, the test's two callbacks, `flb_input_instance_init`
(which passes it to `cb_init`) and `input_thread`.

Stated as *discharged / not discharged / unresolved at this path under these
derived premises*:

- **`90978ac6`, fixed test's 3 sites: unresolved.** The holder escape and
  P-CALLBACK-INSTANCE are derived. The list-head instance is not, because
  `flb_service_set`'s keyed branch is not derived.
- **`90978ac6~1`, old test: not discharged.** Its NC1 path
  (`flb_input_collector_running` from T) stays not discharged. It has no
  plugin copy, so Step B never applies.
- **Other sites.** in_tail's 16 sites, NC3's 2 and the other-plugin
  refusals are unchanged at both revisions.

## 5. The one blocker, named by a diagnostic reading

[`counterfactual.py`](scripts/counterfactual.py) is a diagnostic reading,
never a result, archived in [artifacts/counterfactual/](artifacts/counterfactual/).
It blanks lines 218-223 (the `flb_service_set` statement) in a copy of the
test and keeps line numbers, so the unmodified test's typed facts still key
every other site. It then runs the configured census.

In that reading, the flag derives no coroutine, with all three derivations
named: the one creation `flb_input(…, "tail", …)` at `:226` on the context
`test_tail_ctx_create` returns. The three sites are then discharged on
`(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`.

The Fluent Bit test is not changed. This reading only locates the call that
holds the derivation; it is not evidence that the fixed test is safe. What
would discharge it is a derivation of the literal keys: follow-up
`W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS`.

## 6. The observations as contracts (the integrator's step)

The list-head fact did not derive, and the census states premises it cannot
derive. So the observations are written down in
[`fluent_bit/input_instance.qv`](../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_instance.qv)
as four declarations. Each bare clause is a custody fact read against the
body at all three trees. [`bodies.py`](scripts/bodies.py) shows the bodies
of `flb_create`, `flb_input` and `flb_service_set` are byte-identical there.
`flb_input_new` differs only in one TLS initialisation (`tls_crl_file`).
Every other clause is held under W45-CONTRACT-DEFERRED-REVIEW, with the
observation as its reason.

| Declaration | Checked (bare) | Held for review (7 markers) |
|---|---|---|
| `flb_create()` | — | `return_owned(nullable)`, carrying P-FRESH-CONTEXT and P-REGISTERED-DESCRIPTORS (empty `inputs`, the build's descriptors registered once) |
| `flb_input(ctx, input, data)` | `ctx`, `input` borrowed | `data` (retained in `instance->data`, `src/flb_input.c:496`); `return_none`, carrying P-HEAD-INSTANCE |
| `flb_input_new(config, input, data, public_only)` | `input`, `public_only` borrowed | `config` (retained, `:408`, `:677`); `data` (retained); `return_interior(config, nullable)`, carrying the creation facts (one instance per case-insensitive name match, appended at `:677`, `p` at `:490`, `runs_in_coroutine` from `FLB_INPUT_CORO` at `:501`, `:626`, written nowhere else) |
| `flb_service_set(ctx, …)` | `ctx` borrowed, `return_none` | the variadic tail, carrying the key-dependent effect (only `Plugins_File` registers plugins) |

Each "retained" reason says the clause must not be unwrapped as written: v1
has no retained-pointer mode (rule 10j). The declarations are pinned with
`quirin-audit pin --function …` at `66910c10`, at `046accf6` (the system
clone) and at the export (commit `34155955`), using
[`pin.py`](scripts/pin.py). `pin --check` reports 4 recorded, 0 not in
force, exit 0 at each tree.

The module is selected by `cycle19-resource-contracts.json` and by the
fluent_bit resource view, because the loader's view must equal the cycle19
selection. It is published by the provider crate. The resource-view count
goes 56 → 57, with pins updated in:

- `providers/tests`;
- `crates/qcore-audit/tests/provider_selection.rs`;
- the gateway's `neighbours.rs` and `prod_contracts.rs`;
- `compiler/tests/contract_loader.rs`, whose mode test now skips a withheld
  declaration and asserts that it is withheld;
- `qcore_contract_corpus.rs`, which gains four table rows.

P-HOLDER-OPAQUE and P-INSTANCE-FIELDS-BY-MEMBER have no function to carry
them and stay stated in rule 10p.

## 7. Corpus, system and tail-pause slice

[`arbiters.py`](../w47-loop-owner-typed-dispatch/scripts/arbiters.py) ran
with `--jobs 8`:

- BEFORE at `fefb9199`, binary `874c5da9…`;
- AFTER at `69e41646`, binary `0d8638b5…`, with the contract module in.
  The later census commits (hidden definitions, the functions read in place)
  touch only `loop_owner_instance.rs`, which no per-unit route calls.

[`identity.py`](scripts/identity.py) compares each file with the baseline
`w47-condvar-join-publication/integration/artifacts` and composes it
(compose3). [`before_after.py`](scripts/before_after.py) attributes every
function.

**BEFORE** reproduces the baseline. The corpus `resource-diagnostics.json`
is byte-identical and composes with 0 mismatches. The system and slice
differ only in `resource_contract_sources` and `inventory_sha256` lines:
30 + 15 in the system, 32 + 16 in the slice. The parity log is equal once
sorted.

| Measure | BEFORE | AFTER | Moves (all attributed) |
|---|---|---|---|
| corpus (185 units, `66910c10`) | 526 / 10 / 4,298; units 6 / 8 / 167 / 4 | **identical `resource-diagnostics.json`**, compose3 0 mismatches | none; 0 held call sites (no corpus unit calls the four callees); the contract sidecar and two console tally lines name the new held declarations |
| system (`046accf6`) | 31 / 2 / 201 functions (unique) | same | `src/flb_storage.c::flb_storage_create` (both variants, `:744`): leaf reworded from `call to uncontracted function flb_input_new` to `call to flb_input_new whose declared contract awaits human review`; still **Unknown** |
| tail-pause slice (`90978ac6`) | 24 Safe / 1 Unsafe / 245 Unknown functions (unique) | 24 / 1 / 244 / **1 Pending** | `src/flb_lib.c::flb_input` (both variants): **Unknown → Pending**; the held `flb_input_new` call replaces the statement's `indirect memory access is outside the resource projection` leaf (`ctx->config`, `:272`) |

No function moves Safe → non-Safe, so no correction is named. No leak or
violation is added or removed, so none needs classifying.

The one status move is an `Unknown → Pending` that the review mechanism
produces. It needs a caveat. Resolving the `flb_input_new` review would
**not** make `flb_input` Safe, because the held rewrite reports the held
call in place of the same statement's argument refusal, which would return.
That is registered as `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`. The new
baseline is archived in [artifacts/after/](artifacts/after/).

## 8. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| fixed test ×3 | discharged ~55%; unresolved with a narrower named blocker ~45% | **unresolved**, one named blocker (`flb_service_set`'s keyed branch, not the predicted `flb_create`/`flb_destroy` closure, which pruning removed) |
| old test | not discharged (99%) | **as predicted** |
| every other site, edges, seeds | unchanged | **as predicted** |
| raw and configured alike | alike | the verdicts are alike; the first blocker differs (raw: `flb_config.c` unstructured) |
| case-insensitive descriptor match | moves nothing | **as predicted** |
| corpus / system / slice | only held callees move; `Unknown → Pending` where the held call is the only blocker; never Safe | corpus none; system 1 function reworded; slice 1 function `Unknown → Pending` (§7's caveat) |

## 9. Lean

`proof/QCore/LoopOwner.lean` is audited in `proof/AuditLoopOwner.lean`. The
theorem count goes 43 → 53, with no new audit module, no placeholders and no
project axioms (`propext` and `Quot.sound` only). The new theorems:

- **`a_derived_head_is_the_one_creation`** and
  **`every_path_reads_the_one_creation`**: from the fresh empty list, a path
  that creates once and makes no unmodelled mutation leaves exactly that
  instance;
- **`a_second_creator_keeps_the_route`**,
  **`an_unmodelled_mutation_keeps_the_route`**,
  `a_path_without_a_creation_derives_nothing` and
  `an_underived_head_keeps_the_route`;
- **`an_escape_that_writes_neither_keeps_the_derivation`** and
  `an_escape_that_writes_the_flag_or_the_instance_keeps_the_route`;
- **`the_route_is_dropped_only_through_one_non_coroutine_creation`**;
- **`the_old_test_path_is_never_discharged`**.

## 10. Premises stated, not proved

These sit under trust boundary 28, beside rule 10p's:

- **P-HEAD-INSTANCE**: the list the test reads is not empty, because its
  context's creation and its one input's creation succeeded;
- **P-FRESH-CONTEXT**;
- **P-REGISTERED-DESCRIPTORS**: the build's descriptors, registered once
  each by the generated `flb_plugins_register`, which no census reads;
- **P-HOLDER-OPAQUE**;
- **P-INSTANCE-FIELDS-BY-MEMBER**;
- acutest's `TEST_CHECK` as its argument's truth;
- the typed reading's premises (the recorded build is the program; C's
  effective-type rule);
- a `.c` file that no compile command names and no read unit includes is
  outside the program.

The first three are recorded as held contracts (§6). **P-CALLBACK-INSTANCE**
is no longer a premise: it is derived, or the flag stays underived.

## 11. Follow-ups

[`followups.json`](followups.json) is applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 551 → 554. It
registers three entries:

- `W47-LOOP-OWNER-SERVICE-PROPERTY-KEYS`: 1 call holding 3 sites;
- `W47-INSTANCE-FLOW-CONTRACT-REVIEW`: 4 declarations and 7 held clauses,
  holding 2 system and 2 slice call sites;
- `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`: 1 slice function, 2
  variants.

It amends `W47-LOOP-OWNER-INSTANCE-FLOW` and
`W47-LOOP-OWNER-COROUTINE-POST-YIELD`. It retires nothing, because the
DISCHARGE clause is not met for the list-head instance.

## 12. Gate

[`scripts/gate.py`](scripts/gate.py) runs `rust` then `records`. Exits are
in [gate/](gate/) and figures in [gate.json](gate.json). The `rust` phase ran
at `a5e1c799` and every step exited 0:

- submodule; fmt clean; the Rust suite: 350 units / 363 summaries, **6,266
  passed, 0 failed**, 11 ignored; clippy clean; both probes 0;
- **223 Lean audits** (no new audit module);
- the provider gate: provider cargo test 129 passed / 0 failed / 1 ignored;
  provider fmt and clippy clean; `contracts.py check` 0; package build and
  all five verifies 0; provider Python 24 tests OK; `pin --check` at the three
  trees (4 recorded, 0 not in force each) and `pin --verify-contracts` 0.

A first run at `d2679795` (private `gate-2`) failed on three contract-corpus
pins and two clippy lints. The pins were the corpus module count 120 → 121,
the declaration counts 56 / 242 → 57 / 246 (with the withheld declarations
counted beside the function map) and the structured-clause table's order.
All were repaired in `a5e1c799`. The `records` phase ran after the matrix
and index regeneration (matrix: 554 follow-ups, 223 Lean audits, 121 contract
modules). Its first run failed `scripts/tests`'s corpus pins (module count
120 → 121, quirin-provider modules 47 → 48), a test file only, repaired and
rerun: **365 tests OK**, docs index 0, Clang 21.1.8 recorded (private
`gate-records3`).

**Files likely to conflict** with the parallel W47-CONDVAR-JOIN-STOP-PATHS:

- `docs/agents/gateway.md` (the 10p amendment beside 10q's) and
  `docs/agents/README.md` (the 10p row);
- `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md` and `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs` (the Step B hooks, the
  poison-site list and the reach builder) and `typed_dispatch_census.rs`
  (member facts);
- the module-count pins (56 → 57) in `providers/tests/test_contract_load_path.py`,
  `crates/qcore-audit/tests/provider_selection.rs`,
  `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs` and
  `compiler/tests/contract_loader.rs`;
- `cycle19-resource-contracts.json`, `providers/contract-load-path.json` and
  the fluent-bit `contracts.lock`.

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/census.py OUT CENSUS_BINARY \
  --typed-library=OUT/library.json --typed-fixed=OUT/fixed-test.json --typed-previous=OUT/previous-test.json
python3 docs/reports/w47-loop-owner-instance-flow/scripts/compare_census.py BEFORE AFTER [FACTS_BEFORE FACTS_AFTER]
python3 docs/reports/w47-loop-owner-instance-flow/scripts/counterfactual.py OUT CENSUS_BINARY FACTS 218 223
python3 docs/reports/w47-loop-owner-instance-flow/scripts/bodies.py [OUT]
python3 docs/reports/w47-loop-owner-instance-flow/scripts/pin.py OUT [--check]
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-instance-flow/scripts/identity.py RUN OUT
python3 docs/reports/w47-loop-owner-instance-flow/scripts/before_after.py BEFORE.json AFTER.json
python3 docs/reports/w47-loop-owner-instance-flow/scripts/gate.py LOGS rust|records
```
