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
`flb_input_new` is also declared in `input_instance.qv` (resource v1), so it is
a registered `null_deref_beside` relation (`contract_views.rs`,
`contract_view_census.rs`); `qcore_contract_corpus.rs` pins the fourteen.

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

## 4. Gates

See [gate section](#gate) below (filled at the gate).

## Files

Scripts: [build.sh](scripts/build.sh), [measure.sh](scripts/measure.sh),
[pin.sh](scripts/pin.sh), [scope_candidates.py](scripts/scope_candidates.py),
[compare.py](scripts/compare.py), [gains.py](scripts/gains.py),
[combined.py](scripts/combined.py), [totals.py](scripts/totals.py),
[runs.py](scripts/runs.py), [update_ledger.py](scripts/update_ledger.py),
[gate.sh](scripts/gate.sh), [provider-gate.sh](scripts/provider-gate.sh).
Private outputs: `private/fbn/` (runs, null streams, pin logs, gate logs) and
`/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/fbn-{before,stable,after}/`.
