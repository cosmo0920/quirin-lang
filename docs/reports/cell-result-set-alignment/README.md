# CELL-RESULT-SET-ALIGNMENT

Card brief: `private/integrator-briefs/r10-common.md` (round 10, 2026-11-04, steps 1-10). Base `32169290d`, baseline `baseline:32169290d`. This card is new, registered and retired through its [amendments](integration-amendments.json).

> **USER REQUEST 2026-11-04** (verbatim): "We need to write down a prompt to align the result set for our contracts."
>
> **USER DECISIONS 2026-11-04** (verbatim):
> - Q1/Q2: "Basically, Q1(a) and Q2(i) But we must not handle descriptor 0 as a valid handle. So, we need to exclude the 0 descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in [1, MAX]".
> - After the stdin-closed counter-evidence was put to the user: "Write [1, MAX]".
> - Q3: "(α) live_set suffix".
> - On patch v3's native leak: "Accept the leak".
> - On the patches: "Also, we must show the pseudo contracts for having the evidence to fix them."; "So, we need to add a planned fix contracts to include this patch's fxes."; "we need to follow the format of the patch to change the our contracts which are related to this patch."

**The bound is a declared policy.** `live_set([1, MAX])` and `pipe`'s `written_set(pipedes[k], [1, MAX])` are the user's choice, not POSIX's set, which is `[0, MAX]`. Descriptor 0 occurs natively: with stdin closed, ch_data is `pipe2([0, 3])` (§6). Every reading the bound decides is attributed to it (follow-up CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY).

Prediction: [PREDICTION.md](PREDICTION.md). It was committed at `7f6d7209c` before any code. Its amendment, for the user's bound, was committed at `da24fdcee`, before the declarations that move rows.

## 1. Inventory (step 1)

`census/inventory.py`, with `census/cells.json` (cells, creators, sources of the sets) and `census/annotations.json` (one reading per non-read site, 80 keys). It covers 40 cells:
- the declared per-end and flag groups of `teardown_destroy.qv`;
- the withheld `flb_config` and `flb_sched` texts;
- the census-only ends: `ch_data`, `shutdown_fd`, `flush_fd`, `timer_fd`, `fd_timer`, `window.fd`;
- Monkey's `signal_channel_r/w`, `lib_ch_start`, and the local `timeout_fd`.

The census binds itself to every file it reads, with a tree digest. Absent sets come from the annotated stores, never from script constants. An allocation's zero that is overwritten before any reader is marked superseded, with its reason. Live sets are parsed from the named contract clause, which must be present verbatim, or marked MISSING. Every production `live_set`/`absent_set` is checked against the stores and creators.

```sh
python3 -I docs/reports/cell-result-set-alignment/census/inventory.py \
  --root 66910c10=<corpus tree> --root 046accf6=<system tree> --root 34155955=<slice tree> \
  --root monkey-66910c10=<corpus monkey-source> \
  --check docs/reports/cell-result-set-alignment/census/inventory.json
```

It exits 0 with 0 problems. Trees: 66910c10 has 1,267 files and 174 sites; 046accf6 1,275 and 175; 34155955 1,277 and 175; Monkey 92 and 20.

**Creators:**
- `mk_event_channel_create` and `flb_pipe_create`: declared through `pipe`'s written set `[1, MAX]`. The creator's own derivation is pending (§3).
- `mk_event_timeout_create`: MISSING. `timerfd_create` is undeclared (RULE-38-VALIDITY-READING-RESULT-SET).

**End tests, per Fluent Bit tree.** Both columns are kept, as the decision requires:

| Column | Exact | Redundant | Wrong | Witness |
|---|---|---|---|---|
| declared `[1, MAX]` | 17 | 3 | 6 | 3 |
| POSIX `[0, MAX]` | 0 | 2 | 24 | – |

The corpus Monkey has 2 exact and 1 wrong under the declared set, and 3 wrong under POSIX.

Under the declared set these remain wrong:
- flb_config.c:607 `if (config->ch_event.fd)`: the absent -1 of `MK_EVENT_ZERO` is read as live;
- flb_engine.c:1497 and :1563, `shutdown_fd`;
- flb_input.c:3145, `fd_timer > 0`;
- flb_sp.c:670, `window.fd > 0`;
- mk_server.c:585, `timeout_fd > 0`.

The timers' live set is MISSING, so those are classified under the proposed `[0, MAX]`.

Under POSIX the 16 per-end tests and flb_config.c:612 (`ch_data[0]`) are WRONG, as the earlier censuses found.

The three witness tests are the creator's own `== -1` result tests (`shutdown_fd`, `flush_fd`). The redundant ones are `ch_data[0] != 0`, `sched->ch_events[0] != -1` and `timer_fd != -1`.

The CELL-GUARD-RESULT-RANGE census's stored-result tests are this census's end tests. Its other parts (bodies, immediate tests) are unchanged by this card.

## 2. The forms (steps 2-3; Q1 (a), Q2 (i), Q3 (α))

- **G05i.** A per-end sentinel's own `#fresh` clause may end with `live_set(SET)` and then `absent_set(SET)`. The loader refuses:
  - an absent set on the guard's live edge (`quant-cell-absent-set`);
  - a live set that meets the absent set (`quant-cell-live-set`);
  - a suffix on a `#same` clause or a record guard, and any other order (`quant-cell-set`).

  `>= 0` is not added (step 10: the third patch uses `> 0`).
- **G05j.** `ensures normal written_set(SLOT[K], SET) when #result == C;` uses the accepted per-element and per-class refinements. `pipe` states `[1, MAX]` for both elements, with the decisions quoted in the comment (`libc/result_sets.qv`).
  - `providers/contracts.py sync` updated the libc inventory lock.
  - `quirin-audit pin --bounds` re-pinned pipe's flexible entry.
  - The libevent lock's `mk_event_channel_create` derivation was re-derived at the three trees. It is `[-1, 0]` unchanged; only the digest it rests on for `pipe` moved.
- **Production declarations.** `teardown_destroy.qv`'s per-end `> 0` sentinels state `live_set([1, MAX])` and their pinned absent sets: `0`, and `0 | -1` for `ingress_queue_channels`. The host `flb_config_set_program_name` was re-pinned at 046accf6, 34155955 and 66910c10. The contract digest is `3e680dbf…`, and the sites are unchanged.

## 3. The reading (step 4), Rust and Lean

`crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs` provides `ValueSet`, `Guard::dead_edge_reads_absent`, `Guard::live_edge_reads_live`, `Protocol::live`/`absent`/`dead_edge_decides`/`live_edge_decides`/`guard_follows_live`/`undecided_refusal`, and `PER_END_SET_VECTORS`.

**How an edge decides.**
- A dead edge decides "absent" only when it meets the live set nowhere (and, over live | absent, lies in the absent set).
- A live edge decides "live" only when it meets the absent set nowhere.
- Otherwise the names are introduced undecided.

**Defaults.** Without a declared set, the live set is rule 38's premise and the absent set is the guard's dead set. This is rule 38f exactly: the 17 old `PER_END_VECTORS` and every earlier fixture hold.

**Stores and creations.**
- A sentinel store is admitted only at a value of the absent set.
- A creation establishes `> 0` only where the live set lies in its live edge.
- An undecided edge names `descriptor-cell-zero-undecided` when 0 is the value it leaves open, and `descriptor-cell-edge-undecided` otherwise.
- A creation into a cell with a declared live set needs the creator's in-force written set (`FUNCTION#SLOT` in the route's result table) inside it. Otherwise it refuses `descriptor-cell-live-set-unproved`.

**Not landed: the slot derivation** (follow-up CELL-RESULT-SET-WRITTEN-SET-DERIVATION). This covers forwarding, `*P = fd[k]` after the C library call's failure exit, and the pinned creator declarations. Until it lands, no creator's written set is in force, so every creation into these cells refuses. That is fail-closed. I did not finish it within the card's budget.

Lean: `QCore.CellResultSet` (audited in `proof/AuditCellResultSet.lean`, registered under rule 38). Standard axioms only, no `sorry`. Its theorems:
- `meets_of_mem`;
- `the_dead_edge_reading_is_sound`, `the_live_edge_reading_is_sound`;
- `without_sets_the_reading_is_rule_38f` (parity);
- `the_declared_bound_decides_the_positive_dead_edge`;
- `posix_leaves_descriptor_zero_on_the_positive_dead_edge`;
- `set_vectors_hold`, bound entry for entry to the Rust `PER_END_SET_VECTORS` by `the_set_vectors_are_the_lean_ones`.

## 4. Fixtures

**`tests/cell_result_set_alignment.rs` (9):**
- the declared bound decides `> 0`; POSIX `[0, MAX]` and no-set readings refuse `zero-undecided`;
- the third patch's shape (close then `-1`, and the unconditional reset) is `Safe`;
- a store outside the absent set breaks the protocol;
- the ingress `-1` stores keep `0 | -1`;
- the loader refusals, `>= 0` included;
- the planned-fix module loads as 16 per-end sentinels with `[1, MAX]`/`-1`.

**`tests/cell_protocol_per_end_guards.rs`:** eight vectors change under the production sets, as the prediction's amendment states. The `zero-undecided` releases go; two escapes and one `protocol-broken` go with them; the creator's two `descriptor-cell-undecided` become `descriptor-cell-live-set-unproved`. The module doc points to the POSIX contrast.

## 5. Pseudo contracts and planned fix (steps 7-10)

[pseudo-contracts/](pseudo-contracts/) has one file per holder record: output instance, input instance, input thread instance, `flb_config`, and the census-only ends. Each has four sections:
- (a) as written, with both columns;
- (b) the fixed shape;
- (c) the native evidence;
- (d) the user's third patch (sha256 `c7cd578b…e385`), with its hunks quoted.

The planned-fix declarations are [planned-contracts/teardown_destroy_patch_v3.qv](planned-contracts/teardown_destroy_patch_v3.qv): `> 0 live_set([1, MAX]) absent_set(-1)` for every patched record, `flb_config` included. They are on no load path and no route reads them, and no patched route is added (the user's call). Their store census runs over master 49592c4fd plus v3 in a scratch copy, with 0 problems: [census/inventory-patched.json](census/inventory-patched.json), annotations in `census/annotations-patched.json`. In that census:
- every per-end test is exact;
- the untouched flb_engine.c `ch_notif[1] <= 0` is exact under the bound;
- the compound `[1] > 0 && [1] != [0]` tests are not read by rule 38c, which reads only an exact `r->G OP V`.

**Patch review, all three versions, verified natively** (private/native/logs). Per site, under the declared bound, after v3:

| Site | Decided after v3? | Notes |
|---|---|---|
| `ch_data` | Decided | Closed end by end |
| `ch_manager[0]`, `ch_notif[0]` | Decided | |
| `ch_manager[1]`, `ch_notif[1]` | NOT decided | Compound test shape; the residual suggestion is to split the test |
| :607 alias close | Removed | No double close is left |
| Instance, output and thread ends | Decided | |
| `ch_notif[1] <= 0` | Decided under the bound | Untouched by v3 |

**The patch's own tests and builds:**
- v3 applies cleanly (`git apply --check` exit 0) and builds.
- Its tests are `channel_initialization` and `partial_config_channels`, plus the two log-level tests. They pass with stdin open and closed.
- v3 has no descriptor-0 test: v2's `notification_write_descriptor_zero` and `channel_descriptor_zero` were removed.

**Reload.** Under v1 and v2, descriptor 0 went to the new engine's epoll after a reload (closed unconditionally). Under v3, fd 0 stays held by the leaked ch_data, so no channel takes it. The integrator's inference that a channel takes 0 after a reload is refuted in this configuration.

**-1 stores.** Every -1 store precedes every path to a destroyer. `ingress_queue_channels` keeps master's late `-1` (flb_input.c:553-554), and that is safe: every earlier return frees directly, and publication is at :681. v3 has no second -1 block in flb_input.c.

**Remaining zero-based readers.** None outside the patch except flb_engine.c's `ch_notif[1] <= 0`.

**Windows (source reading only, FLB-PATCH-WIN64-INT-SLOT-HAZARD).** On Win64 the patched fields are `intptr_t`, but `mk_event_channel_create` writes them through `int *`. After the -1 store, a created end reads as negative:
- `> 0` never closes it;
- `ch_notif[1] <= 0` skips the notification;
- `flb_pipe_w` sends to an invalid socket.

ch_data and ingress are not affected. This hazard is unchanged across the three versions.

## 6. Native evidence (step 6)

The runs were stdin closed (`0<&-`), `-Y`, two SIGHUPs, then SIGTERM, under strace. Logs are in `private/native/logs/`.

**Pinned binary 046accf6** (`pinned-closed.strace`, plus gdb at `flb_config_exit`):
- `ch_data` = `pipe2([0, 3])` (flb_lib.c:181) takes descriptor 0 before every census end and is never closed.
- No later channel gets 0.
- Every teardown does `close(13) = 0`, then `close(13) = -1 EBADF`: the :607 alias of `ch_manager[0]`, then :618 (W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE).
- So the census ends need a host that frees descriptor 0 later, or a race inside `pipe2`.
- `ch_data` (:611/:612) is the same class as W43-DESC-FLB-CONFIG-DATA-PIPE-ZERO-GUARD.
- The :607 risk is the double close, not "-1 read as live".

**Master 49592c4fd:** the same.

**v3** (`v3-closed.strace`): fd 0 is never closed, which is the leak the user accepted ("Accept the leak"). Every other end is closed exactly once. The remaining 12 EBADFs are timerfds, on every build, outside this card.

These are recorded as evidence beside the W43 entries, not as named premises. Nothing is filed upstream.

## 7. Prediction and measurement

See [PREDICTION.md](PREDICTION.md). No status move on any route. Content changes:
- system `flb_output_instance_destroy` (×2): `zero-undecided` at :638 goes;
- slice `flb_input_instance_destroy` (×2): :1353 goes;
- slice `input_thread_instance_destroy` (×2): :226 goes;
- slice `input_instance_channel_events_init` (×2): `live-set-unproved` at :1380 in place of `undecided`, and possibly the escapes and `protocol-broken` at :1387/:1401;
- the host-dependent rows: the contract digest.

Each change is attributed to the declared bound.

The worktree's isolation guard refuses the live measurement, so the integrator runs it from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/cell-result-set-alignment-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:32169290de211435f4741d10dc7e35ed625e8634 \
  --side tip=rev:2bf298712 \
  --declared docs/reports/cell-result-set-alignment/PREDICTION.md
```

**Measurement (integrator, 2026-11-04).** Run from the main checkout at
`32169290d` with the command above, `--side tip=rev:2bf298712946618c6777c45628923916b2126fad`,
output `private/measure/cell-result-set-alignment-20261104T090512Z-r2`; exit 0.
The first run refused before measuring: PREDICTION.md carried two
`measure-prediction` blocks (the original prediction was kept under "Original
prediction"), and exactly one is read. The integrator relabelled that
historical block's fence as `text`; both blocks predict `none` on every route,
so the declaration is unchanged. The declared prediction matches on every
route (corpus, system, slice: `match`). Moves 0, unit moves 0, unmatched 0 on
each route; content changes: corpus 0, system 2, slice 6:

- system `flb_output_instance_destroy` (units 0, 1): `descriptor-cell-zero-undecided`
  at :638 removed.
- slice `flb_input_instance_destroy` (units 0, 1): `zero-undecided` at :1353
  removed.
- slice `input_thread_instance_destroy` (units 0, 1): `zero-undecided` at :226
  removed.
- slice `input_instance_channel_events_init` (units 0, 1): two
  `descriptor-cell-live-set-unproved` leaves at :1380 added in place of the two
  `descriptor-cell-undecided` leaves; the `protocol-broken` leaf and the three
  `escapes` leaves at :1401 removed (the row now refuses at the creation). The
  row stays non-Safe.
- No host-dependent row shows a contract-digest change.

[native-comparison.json](native-comparison.json) (sha256
`cf2e2e30e856a2293f2e79dbbbc6fca5c9d598e295f73e174536bb43b3758689`).

## 8. Card gate

The card gate was `python3 scripts/gate_scope.py --base 32169290d --out private/gate4`. It ran on `2bf298712`, the last code commit, with a clean tree ([gate.json](gate.json)). Only this report's README, amendments, gate.json, pseudo contracts and patched-census files were added after it. The scope was full: the diff reaches `qcore-c-gateway`, `qcore-contract` and `qcore-schemas`, a proof root changed (all Lean), and `providers/` changed (provider gate).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 |
| fmt, clippy | 0, 0 | |
| Rust suite | 0 | 602 units, 643 summaries; 9,161 passed, 0 failed, 16 ignored, 135 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean | 0 | 406 audits |
| Python suite | **1** | 577 tests, 1 failure (below) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider `cargo test` | 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The one Python failure is the expected stale record: `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`, `['24.0.0', 405] != ['24.0.0', 406]`. This card adds `AuditCellResultSet.lean`, and the integrator regenerates the records.

Earlier runs:
- `private/gate3`, on `94bca8d9c`, failed on:
  - one clippy `useless_format`;
  - the generated `gateway_rule_registry.rs` not regenerated;
  - two contract tests: pipe's clause count, and the reserved word `written_set` in the teardown comment;
  - a provider test that read a teardown edit made during the run.

  `2bf298712` fixes them.
- `private/gate` and `private/gate2` were stopped by me before completion to fold in late code changes.

The logs are under `private/gate4/` in the worktree. The console log is outside it, at `/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/crsa/gate4-console.log`.

Census digests: `census/inventory.json` sha256 `20a719749d7883effee62254cd1d614245afed8e0b46989525c4c273ab23c822`; `census/inventory-patched.json` sha256 `c28598cad88f4c95cc2e2efe585135e9e4d6db8933fdd23425884859fe090834`. Both `--check` runs exit 0 with 0 problems.

## 9. Follow-ups

- **Registered: CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY.** Every `> 0` reading the declared bound decides, against the recorded native traces (pinned and v3).
- **Registered: CELL-RESULT-SET-WRITTEN-SET-DERIVATION.** The slot derivation and the creator pins. Until they land, every creation into a cell with a declared live set refuses.
- **Registered: FLB-PATCH-WIN64-INT-SLOT-HAZARD.** Source reading.
- **Amended:**
  - CELL-GUARD-DESCRIPTOR-ZERO-READ-AS-ABSENT (both census columns);
  - W43-DESC-FLB-CONFIG-DATA-PIPE-ZERO-GUARD and W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE (native re-observation, v3);
  - RULE-38-VALIDITY-READING-RESULT-SET (the timer cells are MISSING);
  - CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD (flb_config's planned text);
  - CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (under the bound, the window's refusal is now the visible one).
- **Option for the user, not taken: a patched measured route.** It would put the planned declarations in force over master plus v3. My recommendation is not to add it until a pinned tree contains the patch: the declarations are already checked by the patched store census and the fixtures.
