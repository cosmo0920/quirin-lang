# FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE

Card **FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE** (wave 48, round 28, 2026-10-15/16;
card-high, small). Base: local `main` `ca3d827fe`, rebased onto `5b1ceecd5`
(LEAN-4-35-TOOLCHAIN: no analyzer, contract or provider file changed between
the two). Prediction: [PREDICTION.md](PREDICTION.md), frozen at `05dbc670c`
before any declaration, with an addendum committed with the stable
declarations and before any scope entry. Authority: the ledger entry and its
DISCHARGE, the [FB-QUANT-CONTRACT-CENSUS](../fb-quant-contract-census/README.md)
ranking (card 1), gateway rules
[14x](../../agents/gateway.md#constraint-14x)–[14z](../../agents/gateway.md#constraint-14z)
(amended in place), contract specification C09.3 ("Scoped member
invariants"), grammar G05f, the `nonnull_` naming decision, and "handle as
safe as possible". No new syntax, rule id, analyzer code or Lean module:
the declarations use the existing forms and the existing
`QCore.NullMemberScope` theorems (`stable_fact_persists`,
`a_fresh_store_does_not_reach`, `a_scoped_use_sees_non_null`,
`a_stable_declaration_is_never_validated`, `failure_is_monotone`).

## 1. The stores, and the fresh object's NULL

Every store of the descriptor, read at 66910c10, 046accf6 and the tail-pause
export of 90978ac6 (byte-identical to 046accf6 for every function below):
`instance->p = plugin` in `flb_input_new` (`flb_input.c:490`),
`flb_output_new` (`flb_output.c:779` at 66910c10, `:857` at 046accf6),
`flb_filter_new` (`flb_filter.c:507`), `flb_processor_instance_create`
(`flb_processor.c:2188`) and `flb_custom_new` (`flb_custom.c:214`). Each
constructor finds the plugin in the config's registered list, tests or
dereferences it, `flb_calloc`s the instance and stores the plugin once.
Nothing else under `src/`, `include/` or `plugins/` assigns the member; only
`tests/internal/*.c` do (a plugin's address or pointer), and no selection
analyses them; no file of either selection writes a whole instance.

**The calloc'd NULL (the brief's first risk).** Measured, not adjusted:
rule 14x examines assignments, compound updates, struct assignments and typed
fills; the allocator call is none of them, so the fold reports exactly the two
assignments per route that analyses a constructor (output on the system
route, input on the slice; static and shared configurations), both proved,
`whole_object_stores` 0, nothing withdrawn. This is the rule's own reading,
now stated in 14x: a fresh object starts outside the stable state, so a
zeroed member is never read non-null; only a seen non-null fact persists, and
a scope entry is checked at every direct caller.

## 2. Declarations (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/null_deref.qv`)

| Declaration | Functions | Standing |
| --- | --- | --- |
| `requires nonnull_member_stable(TAG, p);` | `flb_input_new` (`flb_input_instance`), `flb_output_new` (`flb_output_instance`), `flb_filter_new` (`flb_filter_instance`), `flb_processor_instance_create` (`flb_processor_instance`) | `partly-validated` where a route analyses the store (output: system; input: slice; 2 stores each, proved), else `declared-assumption` (bodies read at both revisions); never `validated` (rule 14x) |
| `requires nonnull_member_scope(flb_input_instance, p, ins);` | `flb_input_plugin_pause`, `flb_input_plugin_resume`, `flb_input_pause`, `flb_input_resume`, `flb_input_instance_pre_run`, `flb_input_instance_exit`, `flb_input_thread_instance_pre_run`, `flb_input_thread_instance_pause`, `flb_input_thread_instance_resume`, `flb_input_thread_instance_exit` | `in-force` on every route: each direct caller's actual is a `call-precondition` check |

`flb_custom_instance.p` has no reader on any route and is not declared. The
scope set was chosen mechanically ([scope_candidates.py](scripts/scope_candidates.py),
[scope-choice.json](artifacts/scope-choice.json)) from the stable-only run:
every function whose own row kept only `memory-field:V->p` candidates for one
instance formal `V`, byte-identical at 046accf6 and the export, address taken
nowhere, external linkage. It found the census's seven plus the three thread
signals (each reads `ins->p->name` through `flb_plg_debug`); two static
functions that qualify otherwise (`input_instance_channel_events_init`,
`handle_input_event`) and every output reader (each keeps other candidates or
reads a list element) are left out. All fourteen are pinned with
`quirin-audit pin --bounds` at 66910c10 and 046accf6 (`bounds-contracts.lock`
5 → 19 pinned; `pin --check` 0 at both, `pin --verify-contracts` 0); the pins
are in force on the slice because the export's sites equal 046accf6's.
`flb_input_new` (`input_instance.qv`) and `flb_input_thread_instance_exit`
(`lib_engine_thread.qv`, held) also have resource-v1 declarations, so each is
a registered `null_deref_beside` relation (`contract_views.rs`; relations 23 →
25, `contract_view_census.rs`, `contract_views.rs` tests);
`qcore_contract_corpus.rs` pins the fourteen and `contract_loader.rs` counts
them among the bounds declarations resource v1 rejects (314 → 328).

## 3. Measurement

Binary `f8c75e26…` (base) for BEFORE and the stable-only reading, `91e1d574…`
(tip `a642e71f3`) for AFTER, each under the heavy lock
([measure.sh](scripts/measure.sh), [runs.json](artifacts/runs.json); the
corpus `run.json` `binary_sha256` equals each). BEFORE reproduces round 28
exactly (corpus `71387bd3…`).

**Custody: unchanged on every route.** Report bytes identical BEFORE/AFTER:
corpus `71387bd3…` 936 / 9 / 3,889; system `c85f64ba…` 93 / 4 / 351; slice
`ba0e1970…` 52 / 1 / 347 / 4 Pending (the system and slice reports embed the
worktree's paths, so their digests differ from the census worktree's); no
row's custody column moved in any null stream.

| Null property | BEFORE | stable only | AFTER |
| --- | ---: | ---: | ---: |
| corpus Safe uncond. / cond. / Unknown / Unsafe | 1,113 / 1,435 / 2,355 / 18 | — | **1,114** / 1,435 / **2,354** / 18 |
| corpus candidates | 12,643 | — | 12,644 (−2 +3) |
| system | 84 / 125 / 245 | 86 / 125 / 243 | **86** / 125 / **243** |
| system candidates | 1,005 | 989 | 995 (−16 +6) |
| slice | 71 / 103 / 232 | 71 / 103 / 232 | **69** / **121** / **216** |
| slice candidates | 1,279 | 1,261 | 1,253 (−38 +12) |

[compare-corpus.json](artifacts/compare-corpus.json), [-system](artifacts/compare-system.json),
[-slice](artifacts/compare-slice.json), the stable-only comparisons, and
[moved-rows-*.json](artifacts/moved-rows-slice.json) (every moved row's
proofs, assumptions and dependencies).

### Safe gains, read by hand

* **Slice, 20 rows (10 functions × static and shared), Unknown → Safe
  conditional.** Each row's every check is proved; its scope is
  "declared `nonnull_member_scope(flb_input_instance, p, ins)`; inferred
  `quant null-deref #ins`", its dependencies its own scope declaration and
  the inferred clauses of the in-unit helpers it calls:
  * `flb_input_plugin_pause`, `flb_input_plugin_resume`: the scope entry
    (4 `ins->p` reads) and the existing `effective-type` premise; no call
    intervenes, so no stable dependency.
  * `flb_input_pause`, `flb_input_resume`: the entry fact survives
    `flb_input_buf_paused` / `flb_input_is_threaded` by
    `stable:flb_input_instance.p`; their calls to `flb_input_plugin_*` and
    `flb_input_thread_instance_*` are discharged caller checks (dependencies
    on those declarations).
  * `flb_input_instance_pre_run`, `flb_input_instance_exit`: the entry and
    `stable:flb_input_instance.p` across `flb_input_is_threaded`; the call to
    `flb_input_thread_instance_pre_run` / `_exit` discharged.
  * `flb_input_thread_instance_pre_run`: the entry alone.
  * `flb_input_thread_instance_pause`, `_resume`, `_exit`: the entry and
    `stable:flb_input_instance.p` (the `flb_plg_debug` read follows
    `flb_log_check_level`; `_exit` also the `effective-type` premise).

  Their combined verdicts stay Unknown (custody Unknown): a null gain only.
* **System 2 rows and corpus 1 row, `flb_chunk_route_plugin_matches`,
  Unknown → Safe unconditional (combined Unknown → Safe).** It tests
  `!o_ins || !o_ins->p || !o_ins->p->name` and re-reads `o_ins->p->name`
  after `strlen` calls; `stable:flb_output_instance.p` keeps the tested
  fact (rule 14y). Clauses: the stable declaration (system
  `partly-validated`, both stores proved; corpus `declared-assumption`, no
  constructor analysed, stores read at both revisions) and the
  `effective-type` premise. Custody was already Safe, so the combined row
  is Safe resting on that declaration (row dependency
  `stable:flb_output_instance.p`).

### Named corrections (Safe → non-Safe), the brief's second risk

Rule 14z's caller check, slice, 4 rows: **`flb_input_rate_gate_protect`**
(both configurations, Safe unconditional → Unknown; it tests `ins == NULL`
and calls `flb_input_pause(ins)` without reading `ins->p`) and
**`flb_input_test_pause_resume`** (both, Safe conditional → Unknown;
`flb_input_pause(ins)` first, so its later `flb_input_resume` check is moved
by stability but the first is a candidate). The question that sat in the
callee's first `ins->p` read now sits at the call, as decision 2 moves it.

New caller candidates in rows already Unknown (no verdict move): slice
`flb_input_pause_all`, `flb_input_rate_gate_resume`, `flb_input_pre_run_all`,
`flb_input_exit_all` (both configurations each; the last two test a copy,
`p = ins->p; if (!p) continue;`, which does not discharge the path
`ins->p`: rule 14z now says so); system `flb_input_chunk_protect` (2 sites ×
2) and `flb_input_chunk_set_limits` (× 2); corpus the same three sites once.
`handle_input_event`'s checks are moved to its own `ins->p->cb_*` reads.

### Prediction against outcome

* Slice: predicted up to 10 (addendum) functions Unknown → Safe conditional:
  **10, as predicted** (20 rows). Caller corrections predicted for four
  callers if Safe: two were Safe and moved.
* System: predicted no verdict move and `flb_output_exit` /
  `flb_output_init_all` losing repeated reads; **wrong**: those keep both
  first reads (`p = ins->p` then `p->…` is a first dereference of the copy);
  instead six other output functions (`flb_output_new` and the five
  property checks) lose one re-read each, and `flb_chunk_route_plugin_matches`
  moves to Safe (the addendum's correction).
* Corpus: frozen text "unchanged"; the addendum predicted the
  `flb_chunk_route_plugin_matches` gain and up to 3 caller candidates per
  configuration: **as the addendum predicted** (one configuration).
* Census expectation (7 slice functions, 12 output candidates): the
  descriptor's 34 slice candidates are 38 removed of which 12 are re-added at
  callers; the three output functions it named do not move.

### After the rebase onto `49d6a679c`

The card was rebased onto main `49d6a679c`. Main had moved past
HANDOFF-CANDIDATE-OWN-CALLBACKS, MEMBER-PREPASS-OTHER-ROUTES,
MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS, LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED and
-REMAINDER, HANDOFF-DIRECT-CALL-READING, UNION-OPAQUE-RECORD-WRITES-BY-TYPE,
FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE, RELATIONAL-REWRITE-LINE-ORDER, three of
the user's code commits (`a5e4d39d8`, `b93c8f8b9`, `aa8d8b030`),
FB-EVENT-LOOP-RETENTION-AND-REGISTRATION and FB-TEARDOWN-DESTROY-DECLARATIONS.
Only `contract_loader.rs`'s count (64 / 489 / 314 → 64 / 489 / 328) and the
records conflicted. This card adds no module (its declarations are in the
existing `fluent_bit/null_deref.qv`), so none of the module-count pins those
cards moved changes here.

It was then measured again ([arbiter.sh](scripts/arbiter.sh),
[arbiter_compare.sh](scripts/arbiter_compare.sh)): main's binary `30b052f3…` on
main's tree (worktree detached at `49d6a679c`) against the rebased tip's
`34a81754…` (`b57c4e00e`). Each corpus `run.json` `binary_sha256` matches.
Four earlier passes gave the same result and are superseded: against
`4c0803f1f`, `1c7333db1`, `4285ff7cd` (= `cc3e79559`'s code) and `8b360766c`.

* **Custody is byte-identical** on every route: corpus `f2eb590a…` 939 / 9 /
  3,886, system `ae2ee178…` 95 / 4 / 349, slice `8d36cf32…` 52 / 1 / 347 /
  4 Pending. No custody column moved. The digests and totals differ from the
  first base's because of main's own changes, identically on both sides.
* **The null movement is the first base's, row for row**
  ([rebase-identity.json](artifacts/rebase-identity.json),
  [compare-rebased-*.json](artifacts/compare-rebased-slice.json)): the same
  moving rows, check-outcome changes and added caller candidates on every
  route. Main's tallies equal the first base's BEFORE (corpus 1,113 / 1,435 /
  2,355 / 18, system 84 / 125 / 245, slice 71 / 103 / 232); the tip's equal
  the first AFTER. That covers the 20 slice gains, `flb_chunk_route_plugin_matches`
  (corpus 1, system 2, combined Unknown → Safe), and the 4 named slice
  corrections.
* **No Fluent Bit row lists a premise.** LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's
  `premises` field appears on 339 corpus rows, all through
  `stable:rd_kafka_broker_s.rkb_rk`, and on no row that rests on these
  declarations. The declarations name no `resource_premise` (see the
  reading below).

**Premise reading.** Rule 14x checks the stores the analysed units make. In
the tree, every store of `p` is a constructor's (or a test's), and no in-tree
plugin under `plugins/` writes it. Fluent Bit can also load out-of-tree
plugins: shared objects through `flb_plugin_load` (`src/flb_plugin.c`), and
Go plugins through the proxy (`src/flb_plugin_proxy.c`). Those are code the
audit never reads, and a C shared-object plugin's callbacks receive the
`struct flb_*_instance *` itself. Its writing `ins->p` is the same kind of
unanalysed store that `rk_no_plugin_library` names for librdkafka. The
declarations here state no premise, as the integrator directed. Under "handle
as safe as possible" a disclosed premise, for example "no out-of-tree
(shared-object or Go proxy) plugin writes an instance's `p`", would list this
on the affected rows without changing a verdict (`listing_changes_no_verdict`).
It is recorded for the integrator's decision, not adopted here.

## 4. Gate

The standing gate (g9) and the provider gate (pg7) ran at `b57c4e00e` on main
`49d6a679c`. Every step ran under the heavy lock, with exits written to files
and nothing piped ([gate.sh](scripts/gate.sh),
[provider-gate.sh](scripts/provider-gate.sh)):

| Step | Exit | Count |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update (`--init --recursive`) | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 507 units, 520 summaries; 8,185 passed, 0 failed, 15 ignored (455 s inside the lock) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` (Lean 4.35.0) | 0 | 332 Lean audits (no Lean change in this card) |
| `cargo clean --profile dev` (07z) | 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 374 tests OK |
| `python3 scripts/build_docs_index.py --check` | 0 | |
| provider: `cargo check` / `test` / `fmt` / `clippy` (`providers/`, `--locked --offline`) | 0 / 0 / 0 / 0 | 133 passed, 0 failed, 1 ignored (43 summaries) |
| provider: `contracts.py check`, `package.py build`, `verify` ×5 | 0, 0, 0 ×5 | |
| provider: `python3 -m unittest discover -s providers/tests` | 0 | 25 tests OK |

After the records: `build_matrix.py . --gate gate.json` and the Python suite
and docs index again ([finish-gate.sh](scripts/finish-gate.sh); exits in
[gate/](gate/)). Each earlier gate ran at an earlier base and is superseded.
Each was green, or failed only on a cause since fixed:

* g1–g5 / pg1–pg3 on `5b1ceecd5`:
  * g1/pg1: a `cargo fmt` diff in the pin table;
  * g2/pg2: `cargo check` hit a full shared disk;
  * g3: 4 tests failed, an unregistered second `null_deref_beside` pair and
    the loader's rejected count;
  * g4: `cargo fmt`;
  * g5 / pg3: green (8,098 passed, 324 audits).
* g6 on `4c0803f1f`: green (8,135 passed, 328 audits).
* g7 on `1c7333db1`: green (8,144 passed, 329 audits; steps 05–09 resumed at
  the same HEAD with [gate-resume.sh](scripts/gate-resume.sh) after a stop
  for the disk floor).
* pg5 at `6bfa8f2b7`: green.
* Targeted checks r1 (on `4285ff7cd`) and r2: r1 green
  ([rebase-check.sh](scripts/rebase-check.sh)); r2 stopped when main moved.
* g8 / pg6 on `b08c7c3d3`: green (8,176 passed, 331 audits).

The bold rule-id duplicate check is empty (no new id). Logs:
`private/fbn/gate-g9/`, `private/fbn/tests-gate-g9/`,
`private/fbn/provider-gate-pg7/`.

## 5. Ledger

`named_followups` 790 → 790 on main `49d6a679c` ([update_ledger.py](scripts/update_ledger.py),
idempotent; `dimensions.named_followups` equals the list length). At the first
base it went 783 → 785.

* **Retired:**
  * `FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE`: every DISCHARGE clause holds. The
    declarations are pinned and checked, every stable store is proved and
    nothing is withdrawn, the prediction is frozen, the system, slice and
    corpus null rows are measured before and after with every moved row read,
    and no custody row moved.
  * `FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE-MEASURED`. This card observed that
    `flb_processor_instance_create` allocates `sizeof(struct flb_filter_instance)`
    for a `struct flb_processor_instance` (`flb_processor.c`, before `:2188`).
    Card FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE
    ([report](../fb-processor-instance-allocation-size/README.md)) measured it
    as benign but fragile: the processor instance is 152 bytes and the filter
    instance 216 in every configuration, and none of the 550 accesses goes past
    byte 152. That card registered the `-MEASURED` sibling because this card's
    entry was not yet on main. The sibling resolves the observation, so the
    original entry is not registered.
* **Registered:**
  * `NULL-MEMBER-COPY-TEST-PATH`: a tested copy (`p = ins->p; if (!p)`) does
    not discharge `ins->p` at a scope-entry call (4 slice rows).
  * `FB-PLUGIN-DESCRIPTOR-RESIDUAL-READERS`: the `->p`-sourced candidates the
    declarations leave (system 80 in 19 functions, slice 224 in 71), the
    undeclared static and output readers, member bases, and the caller
    corrections.
* **Amended:** `NULL-MEMBER-SCOPE-ENTRY-INFERENCE` and `NULL-MEMBER-STABLE-CENSUS`,
  with the Fluent Bit sizes.

## Files

Scripts: [build.sh](scripts/build.sh), [measure.sh](scripts/measure.sh),
[pin.sh](scripts/pin.sh), [scope_candidates.py](scripts/scope_candidates.py),
[compare.py](scripts/compare.py), [gains.py](scripts/gains.py),
[combined.py](scripts/combined.py), [totals.py](scripts/totals.py),
[runs.py](scripts/runs.py), [update_ledger.py](scripts/update_ledger.py),
[gate.sh](scripts/gate.sh), [gate-resume.sh](scripts/gate-resume.sh),
[heavy.sh](scripts/heavy.sh), [provider-gate.sh](scripts/provider-gate.sh).
Private outputs: `private/fbn/` (runs, null streams, pin logs, gate logs) and
`/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/fbn-{before,stable,after}/`.
