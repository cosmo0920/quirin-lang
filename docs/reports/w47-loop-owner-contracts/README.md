# W47-LOOP-OWNER-CONTRACTS: loop-owner obligation over derived thread identity (C1–C3)

Card W47-LOOP-OWNER-CONTRACTS (wave 47, round 3; card-high), gateway rule
**10p**, contract specification **§C04.9**. The specification is
[W47-TAIL-PAUSE-BINDINGS](../w47-tail-pause-bindings/README.md) §6.
Base: local `main` at `8eba46d1` (the brief's base; this worktree's HEAD was
already there). Frozen prediction: [PREDICTION.md](PREDICTION.md) (commit
`1bed9c39`, before any contract, loader, gateway or route change).

Nothing here is a verified race, a verified fix, an `Unsafe` claim or an
upstream defect claim. Every old-vs-fixed statement below reads: *the owner
obligation is (not) discharged at this path under these derived premises*.

## 1. Mechanism

Five parts, each fail-closed.

1. **Kernel** `crates/qcore-core/src/loop_owner.rs`. A source-independent
   `ThreadProgram` (roots, call edges with a hop kind and a mode effect,
   spawns, functions the adapter could not close) is derived to per-function
   contexts `(thread, mode)`: an edge keeps the thread, a loop dispatch (hop
   L) fixes the mode, a threaded-flag branch (hop F) requires one and prunes
   an infeasible branch, and a spawn (hop S) starts a thread named by its
   entry. Unresolved reasons flow along call edges; an unresolved spawner
   roots its entry in every mode. The owner of each loop is the single
   derived thread of its dispatcher with no unresolved path. `owner_verdict`
   gives **Discharged** only when every context runs on its mode's owner and
   nothing is unresolved, **NotDischarged** as soon as one resolved context
   does not, and **Unresolved** otherwise (an absent caller included). C2
   (`control_completion`, `claim_discharged`: an enqueue discharges only
   "request available") and C3 (`pairing`: a joint collector state only at the
   callback boundary, both results succeeded, a resume only under active
   ingestion) are here too. `reduce_to_reactor` writes one loop's accesses as
   an `event_reactor` program whose single dispatcher is the derived owner;
   its L05 and L04 refuse exactly the foreign accesses.
2. **Vocabulary** `requires resource_loop_owner(p, read|mutate);`
   (`contract_loader.rs`, §C04.9): at most one, on an admitted pointer
   parameter; any other spelling rejects the module; an inherited declaration
   must carry its parent's; a review marker cannot wrap it.
3. **Contracts** `providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv`:
   `flb_input_collector_running` (`read`), `_pause`, `_resume`, `_delete`
   (`mutate`), custody exact (`in` borrowed, scalar result, user-neutral).
   The four bodies are byte-identical at all three revisions
   ([`scripts/bodies.py`](scripts/bodies.py)). Pinned with `quirin-audit pin
   --function …` at `66910c10` (the corpus source), `046accf6` (the prepared
   system clone) and the `90978ac6` export (synthetic commit `34155955`, tree
   `8d1f3cd7`); `pin --check` at all three: 4 recorded, 0 not in force (exit
   0 each). Registration (`flb_input_set_collector_*`), `mk_event_*` and the
   public `flb_input_pause/resume` stay undeclared (rule 10j: a callback or a
   retained event).
4. **Per-unit route hook** (`legacy_literal_premise.rs`): the resource route
   derives no thread, so every call of a loop-owner callee is rewritten under
   the literal premise's reserved prefix with a `loop_owner__` mark and
   refused as `call to X outside its declared loop-owner obligation (no
   derived thread on this route)`. The clause can only refuse there; every
   literal-premise refusal keeps its wording byte for byte.
5. **Thread-entry census** `crates/qcore-c-gateway/src/thread_entry_census.rs`
   (+ `examples/loop_owner_census.rs`, [`scripts/census.sh`](scripts/census.sh)).
   A lexical reading of one program's files with the gateway tokenizer:
   definitions (`static` ones named `path::name`), direct calls, hops A (the
   `TEST_LIST` table → thread `T`), S (`mk_utils_worker_spawn`,
   `flb_worker_create`, `flb_tp_thread_create`, `pthread_create`), V/V\*
   (field-keyed: `.F = f` / `x->F = f` registers `f` on `F`, every `->F (`
   dispatches it — the fixed test's copied vtable is a runtime field store,
   so **V\* is followed as a derived hop**), R (`flb_input_set_collector_*`
   argument 1 on `cb_collect`), L (`flb_input_collector_fd` fixes
   non-threaded, `input_collector_fd` threaded) and F (`if
   (flb_input_is_threaded(x))` arms). Every other address use, a name in a
   `#define` body, a hop field read as a value and an unstructured
   conditional group make the reached functions unresolved; a parameter of
   the same name shadows a function.

## 2. Negative controls (fixtures first)

`crates/qcore-core/tests/unit/loop_owner.rs` (10 tests, kernel programs) and
`crates/qcore-c-gateway/tests/loop_owner_census.rs` (11 tests, C fixtures
built from the bound shapes B01–B60 and run through the census):

| Case | Outcome |
|---|---|
| NC1 (previous test: T pauses) | **not discharged**: witness `flb_test_inotify_pause_collectors > flb_input_pause > flb_input_plugin_pause > in_tail_pause > flb_tail_fs_inotify_pause`, context `(T, non-threaded)` |
| NC2 (fixed test: callback on E) | **discharged** in the fixture: contexts `(flb_lib_worker, non-threaded)` and `(input_thread, threaded)`, each its loop's owner, reached through V\* (`cb_pre_run`) and R/L |
| NC3 (T enqueues, polls `running`) | **not discharged** at both revisions (T is neither loop's owner, whatever the mode); an enqueue never discharges "applied" |
| NC6 (resume while ingestion off) | the running claim is **refused** (`pairing`, `Ingestion::Inactive`) |
| NC7 (a collector result discarded) | **no conclusion** |
| fail-closed | an escaped address, a macro, a field read as a value, a second engine dispatcher thread: never discharged; a malformed or duplicate clause rejects the module; the per-unit route refuses a declared call |

## 3. The previous test against the fix (census over the test binary)

[`scripts/census.sh`](scripts/census.sh) reads 1,685 files (`src/`, `plugins/`,
`lib/monkey/`, `include/`, `tests/runtime/in_tail.c`; not `src/fluent-bit.c`)
of the `90978ac6` export, and the same tree with the previous test
(`90978ac6~1:tests/runtime/in_tail.c`, blob-identical to `046accf6`'s, SHA-256
`008e32fc…`). 11,962 / 11,964 functions, 37,088 / 37,122 edges, 56 roots, 25
spawns, 3,914 unresolved seeds. The owners derive: **engine loop
`src/flb_lib.c::flb_lib_worker`, worker loop
`src/flb_input_thread.c::input_thread`**, each with no unresolved path.
Archive: [artifacts/census/](artifacts/census/).

| Sites | `90978ac6~1` | `90978ac6` |
|---|---|---|
| in_tail plugin collector ops (16: `tail.c` ×8, `tail_fs_inotify.c` ×4, `tail_fs_stat.c` ×4) | not discharged, through the **non-threaded test**'s T (`flb_test_inotify_pause_collectors > flb_input_pause/resume > flb_input_plugin_* > in_tail_*`) | not discharged, through the **threaded test**'s T call (`flb_test_inotify_threaded_pause_collectors > flb_input_pause > …`): F without configuration flow forks it into the non-threaded branch |
| previous test's own `running` reads (6, `:2493`–`:2506`) | not discharged (T) | absent |
| threaded test's poll (2, `wait_tail_collectors_state` `:2449`, `:2450`) | not discharged (T) | not discharged (T) |
| fixed callback (3, `cb_tail_pause_test` `:2483`, `:2492`, `:2494`) | absent | **every derived path discharges** — `(flb_lib_worker, non-threaded)` via `flb_start > do_start > flb_lib_worker > flb_engine_start > flb_engine_handle_event > flb_input_collector_fd > cb_tail_pause_test`, and `(input_thread, threaded)` via `input_collector_fd` — but the site is **unresolved**: `cb_collect` is also dispatched from the coroutine collector `input_pre_cb_collect`, whose address escapes (`include/fluent-bit/flb_input.h:721`), and that header has a conditional group the census cannot structure |
| other plugins (120) | 105 not discharged, 15 unresolved | 105 / 15 (unchanged) |
| `src/` (3: `flb_input_collector_delete`, the proxy's pause/resume) | not discharged | not discharged |

So: the fix removes the NC1 path (its six `running` reads and the plugin
path through the non-threaded test) and introduces the NC2 path, on which
the owner obligation is discharged under the derived premises; the site stays
unresolved only through the coroutine hop (`W47-LOOP-OWNER-COROUTINE-HOP`).
The in_tail plugin's own collector operations stay not discharged at
`90978ac6`, through the unchanged threaded test's T call and the unfixed
threaded flag (`W47-LOOP-OWNER-FIELD-KEYED-V-AND-F`), and NC3 is present at
both. The other-plugin refusals are field-keyed V attributions (every
`cb_pause`/`cb_resume`/`cb_init` registrant is attributed every dispatcher):
a premise gap, not findings.

## 4. Prediction against outcome

Every predicted figure held ([PREDICTION.md](PREDICTION.md)); the one
qualifier that did not materialise is that `flb_input_collector_delete`'s
own pause call was not reworded (its statement is refused earlier).

| Measure | Predicted | Measured |
|---|---|---|
| corpus (185 units, `66910c10`) | byte-identical verdict-bearing output; the three `flb_input_chunk.c` sites (`flb_input_chunk_set_limits` `:3018`, `flb_input_chunk_protect` `:3050`, `:3085`) stay Unknown on the uncontracted public API | `resource-diagnostics.json` **byte-identical** (`cmp` 0); 6 / 8 / 167 / 4 units, 526 / 10 / 4,298 function rows, leak 36, unknown_alias 7,936, unsupported 35,737 |
| system (`046accf6`) | no status move; `sb.c` leaves may reword | 0 status moves; 3 leaf rewordings (`cb_sb_pause`, `cb_sb_resume`, `cb_sb_exit`: `indirect memory access …` → `outside its declared loop-owner obligation`); unknown_alias 1,020, unsupported 2,888 unchanged |
| tail-pause slice (`90978ac6`) | no status move; rewording in `tail.c`, `tail_fs_inotify.c`, `tail_fs_stat.c` | 0 unit and 0 function status moves (0 / 1 / 15 units; 24 / 1 / 245 / 1 functions); 6 functions, 16 sites reworded the same way; counts unchanged |
| NC outcomes | §3 table of the prediction | as predicted (§3) |

**Attribution.** Every moving leaf is one of the 19 declared-operation call
sites, each refused before by the typed-access gate (`indirect memory access
is outside the resource projection`) and now by rule 10p's premise refusal,
which runs first. The category stays `unsupported`, so no count moves. No
function moves; no Safe → non-Safe correction occurs, because no Safe
function in any measurement calls a loop operation
([`scripts/callers.py`](scripts/callers.py): 27 corpus, 22 system and 42 slice
call sites of loop operations or the public API, all in Unknown functions
except `mk_net_conn_create`'s two provisional-Unsafe `mk_event_*` calls).
**Leaks and violations:** none added or removed in any measurement.

Artifacts: [artifacts/](artifacts/) — `before/` identities, `after/{corpus,system,slice}`
(gzipped, filename-keyed manifest; the new baseline for the next card),
`compare-{corpus,system,slice}.json` ([`scripts/compare.py`](scripts/compare.py)),
`callers-*.json` and `slice-tally.json`. BEFORE reproduces all three
baselines (compose3: 0 mismatches each). Binary SHA-256s: BEFORE `87bd218f…`,
final AFTER `83b90db1…` (built at `795d6757`; an intermediate AFTER at
`903029aa` gave the same verdicts and leaves).

## 5. Lean

`proof/QCore/LoopOwner.lean`, audited in `proof/AuditLoopOwner.lean` (22
theorems, no placeholders, no project axioms): `a_step_keeps_the_thread`,
`a_dispatched_callback_runs_on_the_owner`, `a_worker_callback_runs_on_the_owner`,
`an_unresolved_dispatcher_has_no_owner`, `an_infeasible_branch_is_pruned`,
`unresolved_is_never_discharged`, `no_derived_caller_is_never_discharged`,
`context_discharged_on_owner`, `discharged_means_every_context_runs_on_its_owner`,
`a_non_owner_context_refuses`, `the_per_unit_route_refuses_every_obligated_call`,
`only_a_discharge_admits`, `discharged_accesses_run_on_the_owner`,
**`the_owner_route_derives_the_dispatcher_premise`** (for this route,
`EventDriven.Execution.sameBaseSingleDispatcher` follows from the derived
owner rather than being supplied), `a_discharged_loop_has_no_concurrent_mutator`,
`a_foreign_access_conflicts_with_the_dispatcher`, `an_enqueue_never_applies`,
`a_threaded_control_only_enqueues`, `an_unfixed_mode_claims_nothing`,
`a_boundary_claim_needs_both_results_and_the_owner`,
`a_discarded_result_gives_no_conclusion`, `a_resume_without_ingestion_is_refused`.

## 6. Premises stated, not proved

Trust boundary 28: the lexical reading and the hop tables; nothing outside the
read files names a read function; a function stored in a field is called only
through that field's name; **P-INSTANCE** (along a derived path the instance a
collector operation names is the one whose mode the path fixed: in_tail's
`ctx->ins`; one engine label stands for the engine of the path's config); the
threaded flag is immutable after init; acutest's per-test fork (T is the
child's main thread). Residuals R1 (collector destroy through `config->evl`)
and R4 (a NULL `thi` returns 0, so C2's enqueue premise needs a live worker)
stay registered under `W47-TAIL-PAUSE-BINDING-RESIDUALS`.

## 7. Follow-ups

`followups.json`, applied by the idempotent [`scripts/update_ledger.py`](scripts/update_ledger.py):
543 → 547, **retiring `W47-LOOP-OWNER-CONTRACTS`** and registering
`W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS` (8 functions),
`W47-LOOP-OWNER-COROUTINE-HOP` (3 test sites + 13 plugin sites),
`W47-LOOP-OWNER-FIELD-KEYED-V-AND-F` (16 in_tail + 105 plugin sites),
`W47-LOOP-OWNER-AUDIT-ROUTE` (19 per-unit call sites) and
`W47-LOOP-OWNER-INSTANCE-FLOW`.

## 8. Gate

See [gate.json](gate.json) and `gate/`; commands in [`scripts/gate.sh`](scripts/gate.sh).

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-contracts/scripts/bodies.py
target/release/quirin-audit pin --source-root ROOT --crate quirin-provider-fluent-bit \
  --function flb_input_collector_running --function flb_input_collector_pause \
  --function flb_input_collector_resume --function flb_input_collector_delete [--check]
docs/reports/w47-loop-owner-contracts/scripts/census.sh OUT
QCORE_TYPED_PATH_PARITY=OUT/corpus-parity.jsonl scripts/run_pinned_audit.py OUT/corpus COMMIT \
  --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 8
python3 docs/reports/w46-contract-gap-expansion/scripts/run_system.py OUT/system --jobs 8
bash docs/reports/w47-tail-pause-bindings/scripts/run-current.sh attempt-1 RUN BINARY \
  --selection contract/profiles/fluent-bit/tail-pause-selection.json --jobs 8
python3 docs/reports/w47-loop-owner-contracts/scripts/compare.py BEFORE AFTER OUT.json
python3 docs/reports/w47-loop-owner-contracts/scripts/callers.py RUN SOURCE_ROOT
```
