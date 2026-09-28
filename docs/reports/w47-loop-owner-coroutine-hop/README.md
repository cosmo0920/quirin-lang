# W47-LOOP-OWNER-COROUTINE-HOP: the coroutine hop and a configured conditional reading for the thread-entry census

Card W47-LOOP-OWNER-COROUTINE-HOP (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `51fb793c` (this worktree's HEAD was already
there). Frozen prediction: [PREDICTION.md](PREDICTION.md) (commit `e4620daf`,
before any census, kernel or Lean change).

Nothing here is a verified race, a verified fix, an `Unsafe` claim or an
upstream defect claim. Every old-vs-fixed statement reads *the owner
obligation is discharged / not discharged / unresolved at this path under these
derived premises*.

## 1. What blocked the three fixed-test sites

W47-LOOP-OWNER-CONTRACTS left `cb_tail_pause_test`'s three sites
(`tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) and 13 other-plugin sites
unresolved on two reasons. Reading the census before changing it:

1. **"`flb_input.h`: a conditional group this reader cannot structure"** was
   not about `flb_input.h`'s groups (all four end at their entry depth). An
   include guard's closing `#endif` after a file's last token is marked at
   `token_index == tokens.len()`, which the scan's token loop never reached,
   so the guard stayed open and the whole file read as unstructured: every
   function of 221 files at `90978ac6` carried the reason (measured with a
   one-off reading probe, not shipped; the archived evidence is the drop in
   unresolved seeds below).
2. **"address of `input_pre_cb_collect` taken at `flb_input.h:721`"**:
   the coroutine constructor passes the entry to `co_create`, which the
   census read as an escaping address.

## 2. Mechanism

All in the existing modules, each fail-closed.

1. **Hop C** (`crates/qcore-c-gateway/src/thread_entry_census.rs`, hop tables
   `FLUENT_BIT_HOPS`). A coroutine runs on the thread that resumes it. The
   listed constructor `flb_input_coro_collect` names its entry at
   `co_create`'s entry argument (consumed, not escaped). Each function that
   calls it as `v = flb_input_coro_collect(…)` and resumes `v` through
   `flb_input_coro_resume` is a derived resumer and gets a `Coroutine` edge to
   the entry, with hop L's mode inside a loop dispatcher. The entry's calls up
   to its single switch back (`co_switch(x->caller)`, outside any loop) are
   placed in the constructor (the hand-off). Unresolved otherwise: a
   constructor call not so resumed, a wrapper call on any other handle, an
   unreadable hand-off.
2. **Post-yield resumer** (`crates/qcore-core/src/loop_owner.rs`). A function
   the entry dispatches after the hand-off is a *coroutine body*. The census
   reads the yields: a function (not an entry) that switches back to its
   resumer is a yield primitive, a macro that does is a yield macro, and a
   call of either is a yield, except in `flb_input_return_do` (the terminal
   `FLB_INPUT_RETURN`). The kernel marks every body that reaches a yield over
   call edges (not into another coroutine) unresolved:
   `` `B` runs in coroutine `E` and may yield (…): its resumer after the yield is not derived ``.
3. **Which inputs run in a coroutine**: the `.flags` of each
   `struct flb_input_plugin NAME = { … }` descriptor (`|` of numbers and
   macros whose every definition is a number; `FLB_INPUT_CORO` sets it; no
   `.flags` is zero) decide a plugin directory's flag; a collector registrant
   whose registrations all sit in directories decided *no coroutine* is not
   dispatched from the entry. One registered outside `plugins/` (the fixed
   test's callback, the Go proxy's) keeps the coroutine route.
4. **Structure**: a directive after the last token now closes its group.
5. **Configured reading** (`read_program_configured`, `BuildConfiguration`;
   example `loop_owner_census --compile-commands=DB --generated-root=DIR
   --owned-prefix=FLB_`): each read `.c` file's compile-command `-D` set, the
   intersection for every other file; a condition of `defined`, `!`, `&&`,
   `||` and parentheses is decided, anything else is not; an arm is dropped
   only when decided false or after a decided-true arm; an `FLB_` macro is
   undefined only when no command defines it and no read file or generated
   header `#define`s it. At the export: 557 read units, `FLB_HAVE_VALGRIND`
   defined (so `flb_input.h`'s VALGRIND arm is read), `FLB_SYSTEM_WINDOWS`
   undefined.

The shared edit is a side channel: `ConditionalMark` carries its directive's
byte span (`legacy_c.rs`, and the one struct literal in
`legacy_parenthesised_callee.rs`); the token stream is unchanged.

## 3. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_coroutine.rs` (10 tests, C
fixtures of the `flb_input.h` / `flb_input.c` / `flb_input_thread.c` shapes)
and 5 kernel tests in `crates/qcore-core/tests/unit/loop_owner.rs`:

| Control | Outcome |
|---|---|
| a coroutine resumed from a non-owner thread (T constructs and resumes) | **not discharged** on T |
| a handle not derived from the constructor; a constructed coroutine not resumed | **unresolved** (the resumer is not derived) |
| a body that may yield | **unresolved** (post-yield resumer) |
| a hand-off inside a loop | **unresolved** |
| T's resume under `#ifdef FLB_HAVE_NOT_BUILT` | raw: not discharged; configured (no unit defines it): **not read**, discharged on the owners; defined by the build, undecidable (`defined(__linux__)`) or `#define`d in a read file: still read, not discharged |
| a coroutine resumed only by the loop dispatchers | **discharged** on `(flb_lib_worker, non-threaded)` and `(input_thread, threaded)` |
| in_tail (`.flags = 0`) whose collector may yield | **discharged**: not dispatched through the hop; with `FLB_INPUT_CORO`, or with a flags operand that is not a numeric macro, unresolved |
| the fixed test's callback (flag not derived) | discharged when no yield is reachable, **unresolved** when a pause callback may yield |
| an include guard closed after the last token | structured; a group whose arms end at different depths stays unreadable raw and is read under a configuration that decides it |

## 4. Census: `90978ac6~1` against `90978ac6`

[`scripts/census.sh`](scripts/census.sh) reads the same 1,685 files as
W47-LOOP-OWNER-CONTRACTS, raw and configured. BEFORE reproduced the archived
census byte for byte (`fixed.json` `76f75dc9…`, `previous.json` `1f02be39…`).
AFTER (archive: [artifacts/census/](artifacts/census/)); comparisons by
[`scripts/compare_census.py`](scripts/compare_census.py):

| `90978ac6` census | BEFORE | AFTER raw | AFTER configured |
|---|---|---|---|
| functions / edges | 11,964 / 37,122 | 11,964 / 37,051 | 11,951 / 37,188 |
| unresolved seeds | 3,914 | 2,897 | 2,849 |
| unstructured files | (221, probe) | 11 | 4 |
| `input_pre_cb_collect` | unresolved | `(flb_lib_worker, NT)`, `(input_thread, T)`, nothing unresolved | same |
| coroutine bodies / yields | — | 16 / 17 | 16 / 17 |
| sites not discharged / unresolved | 126 / 18 | 126 / 18 | 130 / 18 |

The owners are unchanged (engine `src/flb_lib.c::flb_lib_worker`, worker
`src/flb_input_thread.c::input_thread`). No site changes status between BEFORE
and AFTER raw, and no not-discharged witness changes. The configured reading
adds 4 not-discharged sites (in_kafka `:471`, `:478`, in_statsd `:382`, `:388`):
those files were unstructured raw, so their pause/resume callbacks were not
read at all; now read, they are refused through the threaded test's T call
exactly like the other field-keyed V sites. The 16 unresolved sites at
`90978ac6` (13 at `~1`) all lose both old reasons and are held by these:

| Sites | reason(s) after |
|---|---|
| in_docker_events ×4, in_event_test `:124`, `:163`, in_fluentbit_metrics, in_stdin (8) | only the Go proxy collector `flb_proxy_input_cb_collect`'s post-yield reason: it is runtime-flagged (`in->flags = def->flags | FLB_INPUT_THREADED`), reaches `flb_time_sleep`'s yield, and its `plugin->cb_collect(&data, len)` (`src/proxy/go/go.c:248`, a `struct flbgo_input_plugin` field) dispatches every collector registrant field-keyed |
| in_calyptia_fleet ×3 | its own collector may yield (`flb_http_do > flb_io_net_write > net_io_write_async`, `src/flb_io.c:487`) + the proxy |
| in_event_test `:188`, `:218` | its own collectors may yield (`flb_time_sleep`) + the proxy |
| fixed test `cb_tail_pause_test` ×3 | the callback's own flag is not derived (registered in the test on a runtime copy of the tail plugin) and it reaches a yield only through field-keyed dispatch (`flb_input_pause > flb_input_plugin_pause > in_emitter cb_emitter_pause > filter_multiline cb_emitter_pause > … > out_gelf cb_gelf_flush > flb_io_net_write`) + the proxy |
| in_podman_metrics ×2 | unchanged: no derived caller |

docker_events, fluentbit_metrics and stdin run no coroutine and are no longer
dispatched through the hop (their own collectors, which reach yields through
field-keyed processor dispatch, are not coroutine bodies); in_tail's own
collectors are likewise not bodies. The 16 bodies are the 13 collectors of
the eight `FLB_INPUT_CORO` directories plus three registered outside
`plugins/` (`flb_input_ingress_collector`, the Go proxy, the fixed test).

**Old against fixed.** At `90978ac6~1` the previous test's NC1 path stays not
discharged (T). At `90978ac6` the fixed callback's path is unresolved at this
path under these derived premises: every derived context of it is an owner
context, including the coroutine route, which is now derived; what remains
is the post-yield resumer of a coroutine route its instance is not derived to
take. The in_tail plugin's own 16 collector sites and NC3 stay not discharged
at both revisions (the threaded test's T call, field-keyed V and F).

## 5. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| old reasons at the 16 sites | both gone | **both gone** at all 16 (13 at `~1`) |
| fixed test ×3 | unresolved, own post-yield reason (~70%) | **unresolved**, own post-yield reason — and the proxy's |
| docker ×4, metrics, stdin | **discharged** | **unresolved**: missed. The Go proxy's reason reaches them through field-keyed `cb_collect` dispatch in `go.c`, which the prediction did not foresee; their own route no longer goes through the hop |
| calyptia ×3, event_test ×4 | unresolved (post-yield) | unresolved; event_test `:124`, `:163` hold only the proxy's reason, not their own |
| not-discharged sites | no status move | none (raw); +4 newly read sites configured, attributed above |
| corpus / system / slice | unchanged | see §6 |

## 6. Corpus, system and tail-pause slice

The per-unit routes read no census, so the three measurements must not move.
[`scripts/arbiters.sh`](scripts/arbiters.sh) ran them BEFORE (base binary) and
AFTER; [`scripts/identity.sh`](scripts/identity.sh) compares each with the
W47-LOOP-OWNER-CONTRACTS archive
([`scripts/manifest_identity.py`](scripts/manifest_identity.py): that archive's
manifests are filename-keyed, which `baseline_identity.py` does not read) and
composes it with compose3.

| Measure | BEFORE (`51fb793c`, binary `bb8870de…`) | AFTER (`7d4e0999`, binary `45400e38…`) |
|---|---|---|
| corpus (185 units, `66910c10`) | `resource-diagnostics.json` byte-identical to the archive (`a843a690…`); 526 / 10 / 4,298 functions, units 6 / 8 / 167 / 4, leak 36, unknown_alias 7,936, unsupported 35,737 | **byte-identical to BEFORE** and to the archive |
| system (`046accf6`) | compose3 0 mismatches (31 / 2 / 201 unique; unknown_alias 1,020, unsupported 2,888) | **byte-identical to BEFORE** |
| tail-pause slice (`90978ac6` export) | compose3 0 mismatches (units 0 / 1 / 15; functions 24 / 1 / 245 / 1 not projected) | **byte-identical to BEFORE** |

The system and slice reports differ from the archive only in provenance
digests ([`scripts/provenance_diff.py`](scripts/provenance_diff.py): 30 + 15
and 32 + 16 lines of `resource_contract_sources` / `inventory_sha256`). The
digest hashes the loaded contracts' source assumptions, which carry absolute
paths: the contracts sidecar is identical once this worktree's root is
aliased to the baseline's. So no function, unit, leaf or count moves; no
Safe → non-Safe correction; no leak or violation added or removed; no new
baseline archive. Summary: [artifacts/arbiters.json](artifacts/arbiters.json).

## 7. Lean

`proof/QCore/LoopOwner.lean`, audited in `proof/AuditLoopOwner.lean` (22 → 34
theorems, no new audit module, no placeholders, no project axioms):
`a_coroutine_runs_on_its_resumers`,
**`a_coroutine_resumed_only_on_the_owner_runs_on_the_owner`**,
`a_non_owner_resumer_refuses`, **`an_undetermined_resumer_never_discharges`**,
`a_body_that_may_yield_is_never_discharged`,
`the_dispatcher_resumes_on_its_own_thread`,
`an_input_without_a_coroutine_is_not_dispatched_by_the_hop`,
`an_underived_flag_keeps_the_hop`, `taken_within_kept`,
**`the_taken_arm_is_read`** (whatever arm a build agreeing with every decided
condition takes is kept), `a_decided_false_arm_is_not_read`,
`nothing_after_a_decided_arm_is_read`.

## 8. Premises stated, not proved

Trust boundary 28, beside rule 10p's: **P-CORO-HANDLE** (an input coroutine's
handle reaches a generic resume site — `flb_coro_resume(connection->coroutine)`
and the other sites in §4's follow-up — only through its own `flb_coro_get()`,
after it yields, and `ins->input_coro_list` is never resumed from; the census
enumerates the two `flb_input_coro_resume` calls, both beside their
constructor call); **P-PLUGIN-DIR** (a plugin directory's code registers
collectors only on instances of the plugins it declares, and descriptor flags
are not changed at run time); the configured reading's premises (the test file,
absent from the export's compile database, is configured as the library's
common set; no unread file defines an `FLB_` macro, checked for `lib/` outside
Monkey and the build's generated headers).

## 9. Follow-ups

[`followups.json`](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 547 → 547,
**retiring `W47-LOOP-OWNER-COROUTINE-HOP`** (its DISCHARGE — a derived hop,
in_tail's flag, a structured `flb_input.h`, measured — holds; the sites it
named stay unresolved on the reasons above), registering
`W47-LOOP-OWNER-COROUTINE-POST-YIELD` (the post-yield resumer: 5 bodies holding
declared sites) and amending `W47-LOOP-OWNER-FIELD-KEYED-V-AND-F` (the Go
proxy's cross-struct `cb_collect` dispatch alone holds 8 sites; the test's yield
path) and `W47-LOOP-OWNER-INSTANCE-FLOW` (the test callback's flag, 3 sites).

## 10. Gate

[`scripts/gate.sh`](scripts/gate.sh) `rust` then `records`; exits in
[gate/](gate/), figures in [gate.json](gate.json). All steps exit 0 on
`92e160dd`: the Rust suite 347 units / 360 summaries, **6,213 passed, 0
failed**, 11 ignored (+15: 10 census fixtures, 5 kernel tests); fmt and clippy
clean; both probes 0; **222 Lean audits** (no new audit module; the matrix's
module-count pins are unchanged). No `providers/` change, so the provider gate
is not due; the provider cargo test ran once for the matrix figure (129 passed).
The Python suite and the docs-index check ran after the matrix and index
regeneration. No contract was added or changed, so no pin is due.

**Files likely to conflict** with the parallel W47-CONDVAR-JOIN-PUBLICATION:
`docs/agents/gateway.md` (10p amendment beside its new 10q),
`docs/agents/README.md`, `docs/STATUS.md`,
`docs/development/increments/wave-47.md`, `docs/reports/INDEX.md`,
`docs/reviews/r0_3/acceptance_matrix.json` (gate block and follow-ups),
and `crates/qcore-c-gateway/src/legacy_c.rs` (the `ConditionalMark` span
field). Module counts: Lean audits 222, contract modules 119 (both unchanged
by this card).

## Reproduce

```sh
docs/reports/w47-loop-owner-coroutine-hop/scripts/census.sh OUT
python3 docs/reports/w47-loop-owner-coroutine-hop/scripts/compare_census.py BEFORE.json AFTER.json
docs/reports/w47-loop-owner-coroutine-hop/scripts/arbiters.sh OUT COMMIT
docs/reports/w47-loop-owner-coroutine-hop/scripts/identity.sh OUT SLICE_DIR IDENTITY_OUT
docs/reports/w47-loop-owner-coroutine-hop/scripts/gate.sh LOGS rust|records
```
