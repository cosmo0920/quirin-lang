# W48-CALLEE-CLOSURE-R3: every blocking callee ranked, two libc facts declared

Card W48-CALLEE-CLOSURE-R3, wave 48 round 7, 2026-10-01, base local `main`
`c374de96`. Plan P4, round 3, in the
[unsupported-coverage queue](../../plans/unsupported-coverage/README.md).
Round 2 ranked external callees; this round ranks every callee that blocks an
Unknown function on the corpus, the system route and the tail-pause slice,
names what keeps the top in-corpus bodies Unknown, contracts what existing
vocabulary can state, and sizes the decision classes for the user. The
[prediction](PREDICTION.md) was committed (`74a5d4b0`) before the
implementation commit (`76908268`) and before any AFTER run. No route
changes, so no gateway rule is taken (11e stays free) and no Lean module is
due; the two contracts use the existing checkers.

## Headline

- **BEFORE reproduces the baseline.** Corpus `3a25c4e9…`, 608 / 10 / 4,216,
  byte for byte; system 41 / 2 / 191 and slice 28 / 1 / 240 / 1 Pending by
  totals (those two files embed the worktree path, so their raw digests
  cannot recur across worktrees; `artifacts/after/identity.json` records a
  path-normalised digest).
- **The ceiling is not contracts.** 2,579 Unknown corpus functions carry an
  uncontracted or nested call, 469 carry nothing else. Every summary route
  together frees 6 of them. Taking every in-corpus callee's own body Safe
  frees 390, in one round. The rest of the corpus waits on bodies, not on
  declarations.
- **Class (iii) is exhausted.** In existing vocabulary, no callee without a
  body frees 2 functions alone. The one that did,
  `rd_kafka_topic_partition_get_private` (2), is an interior-reading decision.
- **Two libc facts declared**, neutral and flexible, pinned with
  `quirin-audit pin` and `pin --check` exit 0 at `66910c10` and `046accf6`:
  - `lstat`, which frees the slice's `tail_file_lstat`;
  - `clock_gettime`, the only call in librdkafka's `rd_clock` header body.
    That body still refuses at its `return` (section 3).
- **Movement:**
  - corpus: no status move; one condition leaf removed (`mk_file_get_info`);
  - system: verdict-identical;
  - slice: **+1 Unknown → Safe** (`tail_file_lstat`); two callers lose 8
    leaves and stay Unknown.
  - No Safe → non-Safe move, no correction, no leak or violation added.
- **Cross-unit route:** measured at 5 corpus functions (6 at its ceiling) and
  0 on the two Fluent Bit routes. Registered with that size, not built.
- **Decision table:** the eight classes, sized on all three routes
  (section 5), registered as `W48-CALLEE-DECISION-TABLE`.

## 1. BEFORE

[`scripts/measure.sh`](scripts/measure.sh) runs the brief's three routes with
`--jobs 4` (the round-7 host limit), each into a fresh directory under
`private/w48cc/`:

- the corpus through `scripts/run_pinned_audit.py … 66910c10 --inputs-root
  …/build/cycle19`;
- the system route: `quirin-audit current` with `system-selection.json` over
  the shared prepared `046accf6` attempt;
- the tail-pause slice at the `90978ac6` export.

All exits 0. The binaries' SHA-256 are in
[`artifacts/before-binary.sha256`](artifacts/before-binary.sha256) and
[`artifacts/after/binary.sha256`](artifacts/after/binary.sha256).

| Route | BEFORE | Digest |
|---|---|---|
| corpus | 608 / 10 / 4,216 | `3a25c4e9…` = the brief's (CG10-BOUND-UNTRACKED-ROOTS' AFTER) |
| system | 41 / 2 / 191 | `b389e288…` (normalised `10bc9560…`); totals = CG10's AFTER |
| slice | 28 / 1 / 240 / 1 Pending | `a19e3d65…` (normalised `c8fd5951…`); totals = CG10's AFTER |

The matrix on the base had `dimensions.named_followups` 658 against 659
entries (a merge on `main` left it). The ledger script restores the equality.

## 2. The ranking

[`scripts/closure_r3.py`](scripts/closure_r3.py) extends round 2's
`closure_r2.py` to every callee. The outputs are in
`artifacts/ranking/{corpus,system,slice}-{before,after}.{json,txt}` (the corpus JSON gzipped; `closure_r3.py` loads the plain file it writes).

- **Merging rows.** A function's rows are merged by (unit, function). The
  Fluent Bit routes carry one row per configuration.
- **Classes.** A callee is placed relative to the calling unit:
  - (i) same unit;
  - (ii) one other unit;
  - (iii) no body in the route;
  - (iv) no body, and one of round 2's eight decision classes, placed by
    reading ([`decisions.json`](decisions.json), section 5);
  - (v) a compiler builtin.
- **Nested calls.** Since CG05 (rule 10y) a nested call is applied through
  any contract in force, so the closure serves a `nested call` leaf exactly
  as it serves an uncontracted one. Round 2 read it as always blocking. The
  reading is an upper bound: 10y also refuses for reasons other than a
  missing contract. `strlen`, which is declared, has 5 such leaves.

**Corpus.** 2,579 Unknown functions have a call blocker, and 469 have nothing
else.

| Class | Callees | Blocked alone | + one other | Leaves | Callees freeing ≥ 2 |
|---|---:|---:|---:|---:|---:|
| (i) same unit (body Unknown) | 1,451 | 247 | 178 | 3,541 | 41 |
| (ii) cross-unit | 676 | 67 | 68 | 3,254 | 8 |
| (iii) no body, no decision | 255 | 4 | 23 | 1,397 | 0 |
| (iv) decision class | 96 | 28 | 31 | 570 | 3 |
| (v) builtin | 15 | 6 | 2 | 180 | 0 |

**System:** 107 functions with a call blocker, 16 call-only. Blocked alone:
(i) 10, (iv) 4, (ii) and (iii) 0.

**Slice:** 144 functions with a call blocker, 24 call-only. Blocked alone:
(i) 8, (ii) 3, (iii) 2 (`lstat`, `XXH_INLINE_XXH3_64bits`), (iv) 2.

**The in-corpus closure** (functions freed, per round):

| Variant | Corpus | System | Slice |
|---|---|---|---|
| existing same-unit summary rule | 0 | 0 | 0 |
| + a cross-unit route over Safe home bodies | 5 | 0 | 0 |
| cross-unit, every freed function taken Safe | 6 (5, then 1) | 0 | 0 |
| every class-(i)/(ii) body taken Safe | 390 (one round) | 12 | 16 |

A second round adds nothing in any variant. The in-corpus closure is
therefore a set of independent bodies, not a chain: each of the 390 waits on
its callee's own non-call leaves.

## 3. What keeps the top bodies Unknown (task step 2(i))

[`scripts/body_leverage.py`](scripts/body_leverage.py) takes one callee body
Safe at a time and runs the closure
([body-leverage-corpus-before.json](artifacts/ranking/body-leverage-corpus-before.json)).
[`scripts/body_reasons.py`](scripts/body_reasons.py) names the reader that
refuses each statement from the parity stream
([body-reasons-top.json](artifacts/ranking/body-reasons-top.json)).

| Body | Frees | Its blocker (reader) | Owner |
|---|---:|---|---|
| `cJSON.c::cJSON_New_Item` | 10 | `hooks->allocate(sizeof(cJSON))`, an indirect call (CG22) | callbacks, a decision |
| `lz4.c::LZ4_decompress_generic` | 12 | 117 leaves (`__builtin_memcpy`, assignments, aliases) | several cards |
| `rdkafka_admin.c::rd_kafka_admin_result_ret_{groups,resources,topics}` | 4 + 3 + 3 | a union-member pointer return behind a cast, refused by the typed reader (CG10/CG13), and a declaration initializer | open Clang-gap residuals |
| `ctr_encode_text.c::sds_cat_safe` | 4 | `cfl_sds_cat_safe` | decision class 2 |
| `lz4hc.c::LZ4_compress_HC_extStateHC` | 4 | two same-unit callees, each Unknown | chain |
| `cmt_metric_histogram.c::metric_sum_exchange` | 2 | `cmt_atomic_*`, whose bodies are `__atomic_*` builtins | decision class 6 |
| `rdxxhash.c::XXH_readLE64_align` | 1 (28 nested leaves) | a `?:` of cast derefs (CG06 connective, CG10 cast path) | Clang-gap residual |

None of these is a small admission under a landed rule. Each is either a
decision or an open Clang-gap residual. No new Clang-gap card is started
here.

**Header bodies.** A class-(iii) callee can be a `static` header body that
W4 derives but cannot validate. Its refusals appear only in the parity
stream: [`scripts/helper_blockers.py`](scripts/helper_blockers.py) collects
them and [`scripts/helper_closure.py`](scripts/helper_closure.py) closes
them. `rd_clock` (51 blocked functions, 65 leaves) failed only on the
undeclared libc `clock_gettime`. So the card declared the libc fact, not
`rd_clock`: the user's rule forbids a contract that replaces a corpus body.

With the fact in force, the controls show one remaining refusal, at the
`return`. The return reads `ts.tv_sec` and `ts.tv_nsec` of a local
aggregate whose address the call took:

- the typed reader refuses that place in a return (parity: "place: the typed
  access-path reader refused it (CG10)", feature `aggregate-value`);
- the same read copied first (`s = ts.tv_sec; return s;`) is Safe.

This is registered as `W48-RETURN-AGGREGATE-MEMBER-READ`. It is in
CG14-AGGREGATE-VALUES-AND-INITIALIZERS' scope, and CG14 is running in this
round. On the corpus BEFORE, 45 return consultations in 10 functions carry
that feature and that refusal. If `rd_clock` validates, nine helpers follow
(`rd_timeout_init`, `rd_interval0`, …), and `rd_kafka_timer_next` is the one
reported function freed ([helper-closure-rd-clock.json](artifacts/ranking/helper-closure-rd-clock.json)).

Two class-(iii) callees would each free one function, but only if a corpus
header body were declared rather than checked, so neither was declared:

- `rd_kafka_buf_write_i8`: its chain ends in the cross-unit `rd_buf_write`;
- `rd_clock` itself.

## 4. Contracts (task step 2(iii), (v))

Both are libc facts. Libc facts go only in `quirin-provider-libc`, as
`revision flexible` declarations (C06.2), and they read as the standing libc
assumption.

- **`lstat`** (`libc/sys_stat.qv`): `stat`'s statement, borrowed path and
  buffer, `resource_return_none()`.
- **`clock_gettime`** (`libc/time.qv`): `gettimeofday`'s statement, a
  borrowed scalar id and a written-through `struct timespec`.

Both prototypes were read on the audited host (`/usr/include`). Their call
sites were read at all three trees:

- `rdtime.h` and Monkey's `mk_file.c` are byte-identical at `66910c10`,
  `046accf6` and `90978ac6` ([`scripts/compare_revisions.py`](scripts/compare_revisions.py));
- `plugins/in_tail/tail_file.c` is identical at the two pinned revisions and
  differs at the slice export; every tree's `tail_file_lstat` has the same
  non-Windows arm, `return lstat(path, st);`. The file is audited only by the
  slice route.

**Pinning.** [`scripts/pin.sh`](scripts/pin.sh) ran `quirin-audit pin
--crate quirin-provider-libc --function lstat --function clock_gettime` at
the read-only `66910c10` source and at this card's own fresh `046accf6`
clone (`scripts/prepare-attempt.sh attempt-1`, exit 0, under
`private/w45-fb/w48-callee-closure-r3/`). It then ran `pin --check` at both:
every step exits 0, and every crate reports 0 not in force and 0 stale. The
lock gains two flexible rows. The corpus run reports 113 libc contracts under
the standing assumption (111 before).

**Controls.** [`crates/qcore-c-gateway/tests/w48_callee_closure_r3.rs`](../../../crates/qcore-c-gateway/tests/w48_callee_closure_r3.rs)
loads the production modules byte for byte:

- the `tail_file_lstat` shape and a clock read are `Safe`;
- without the card's sections, each is an uncontracted call;
- an owned buffer handed to either call and dropped leaks;
- a released buffer is never `Safe`;
- the `rd_clock` probe (section 3) is pinned.

**Deliberate pin updates:**

- `contract_loader.rs`: v1 declarations 253 → 255, and the previous-mode
  table gains both names;
- `qcore_contract_corpus.rs`: two per-file clause rows.

## 5. Decision classes, sized (task step 2(iv))

These are round 2's §6 classes. The card decides none of them.

A callee is placed by reading its body or prototype. A callee is placed only
when no unit of the route has its body; a body in the route is decided by
analysing it, not by a declaration.

Columns:

- **callees:** the callees in the class;
- **alone:** functions blocked by one callee of the class alone;
- **class:** functions blocked only by callees of the class;
- **+1:** functions blocked by the class plus exactly one other blocker;
- **fns:** Unknown functions carrying the class;
- **leaves:** the class's call leaves.

| Class | Corpus: callees / alone / class / +1 / fns / leaves | System | Slice |
|---|---|---|---|
| 1 callbacks, retained pointers | 7 / 5 / 5 / 3 / 39 / 60 | 1 / 1 / 1 / 0 / 1 / 2 | 1 / 0 / 0 / 0 / 1 / 2 |
| 2 in-place lifecycle, reallocation | 7 / 6 / 7 / 6 / 43 / 180 | 1 / 1 / 1 / 0 / 1 / 2 | — |
| 3 reference counts | 5 / 2 / 2 / 10 / 93 / 132 | — | — |
| 4 handle kinds, no-return | 6 / 3 / 4 / 5 / 27 / 40 | 3 / 2 / 2 / 0 / 3 / 6 | 2 / 2 / 2 / 0 / 3 / 9 |
| 5 OpenSSL / libcurl | 59 / 2 / 4 / 2 / 26 / 133 | — | — |
| 6 builtins, no owning crate | 15 / 6 / 7 / 1 / 80 / 180 | — | 3 / 0 / 0 / 0 / 4 / 40 |
| 7 Monkey backend inline bodies | 11 / 8 / 8 / 2 / 11 / 13 | — | — |
| 8 interior reading, extended | 1 / 2 / 2 / 1 / 10 / 12 | — | — |

**Placements this round added, each read:**

- **Class 1:** `mk_sched_get_thread_conf` returns `pthread_getspecific`'s
  retained pointer.
- **Class 2:** `flb_config_task_map_grow` reallocates `config->task_map` in
  place.
- **Class 4:** constructors, destructors and a teardown whose release is
  their own:
  - `flb_config_map_create`, a constructor whose release is
    `flb_config_map_destroy`;
  - `flb_sqldb_close`, which closes a SQLite handle and drops a parent's
    `users` count;
  - `flb_input_coro_finished`, which destroys coroutines found by id.
- **Class 8:** `rd_kafka_topic_partition_get_private` returns a child it
  allocates lazily with `rd_calloc` into `rktpar->_private`:
  - the only faithful v1 reading is `resource_return_interior(rktpar,
    nullable)`;
  - that extends the user's interim arena reading to a new shape, which is
    a decision;
  - its two callers store through the result without a null test, so even
    declared it frees neither.

The user decision of 2026-09-28 already sends class 5 to plugin crates.

**Two rows that are not decisions:**

- `accept4` and `bind` (1 corpus function each) wait on the declared-ABI
  reader's transparent-union gap;
- `XXH_INLINE_XXH3_64bits` (1 slice function) has no owning crate.

## 6. AFTER

Same command at `76908268`. Every run exits 0.
[`scripts/identity.py`](scripts/identity.py) wrote
[identity.json](artifacts/after/identity.json).

| Route | AFTER | Moves | Leaves |
|---|---|---|---|
| corpus | 608 / 10 / 4,216, `c452aa9a…` | none | −1: `mk_core/mk_file.c::mk_file_get_info`'s condition leaf at `:45` (`if (lstat(path, &f) == -1)`), which is now modelled; still Unknown on 2 other leaves |
| system | 41 / 2 / 191 | none | none; only the contract-source digests in the compile records change |
| slice | **29 / 1 / 239 / 1 Pending** | `tail_file.c::tail_file_lstat` Unknown → Safe (its one leaf, `lstat`) | `flb_tail_file_append` −3: the `tail_file_lstat` call and two `file` aliases. `flb_tail_file_is_rotated` −5: the typed actual `file->name` at the call and four `ctx` aliases. The never-bound closure no longer refuses a formal passed to a callee with a validated `Borrowed` summary. Both stay Unknown |

Every change is attributed: the one move and 10 removed leaves, all to the
two declarations or to `tail_file_lstat`'s new summary. No leaf is added,
there is no Safe → non-Safe move, and no leak or violation is added.

**Against the prediction:**

- The slice move was predicted exactly.
- The corpus was predicted byte-identical, and one leaf moved. The ranking
  reads callee names from call leaves only. A call inside a controlling
  expression is reported as a condition leaf with no callee name, so
  `mk_file_get_info`'s `lstat` in an `if` was invisible to it. CG00 sizes
  that class at 587 leaves ("uncontracted callee in a condition").
- The slice's second caller, and the alias leaves released with the summary,
  were not predicted.

## 7. Gate

[`scripts/gate.sh`](scripts/gate.sh) runs the standing gate and the provider
gate, one exit file per step. The exits are in [gate/](gate/), and the
figures are in [gate.json](gate.json).

## 8. Follow-ups

These are applied by [`scripts/update_ledger.py`](scripts/update_ledger.py).

**Registered:**

- `W48-CALLEE-DECISION-TABLE`: section 5's table;
- `W48-RETURN-AGGREGATE-MEMBER-READ`: section 3's refusal.

**Resized or amended:**

- `W45-CALLEE-CASCADE-RESIDUAL`: 469 call-only;
- `W47-EXTERNAL-RESIDUAL-RANKING-R2`;
- `W44-CALLEE-BODY-DOES-NOT-VALIDATE`: 1,451 callees, 3,541 leaves;
- `W44-CALLEE-CROSS-UNIT-SUMMARY`: 676 callees, 3,254 leaves; the route
  frees 5, or 6 at its ceiling, and is not built;
- `W44-CALLEE-NO-CORPUS-BODY`;
- `W44-CALLEE-DESTRUCTOR-SUMMARY`: 233 callees, 1,268 leaves;
- `W44-CALLEE-EXTERNAL-INLINE-DEFINITION`: `mpack_*` 25 callees, 128
  leaves;
- `W47-ALIAS-CALLEE-BORROWED-CLOSURE`;
- `CG05-UNCONTRACTED-INNER-CALLEES`: 164 nested leaves, 88 functions;
- `CG10-BOUND-ROOT-PATH-REFUSED`: unchanged, 5,126;
- `W46-HEADER-INLINE-DECLARED-UNCHECKED`.

**Retired:** none. No listed DISCHARGE clause holds.

## 9. Reproduction

```sh
docs/reports/w48-callee-closure-r3/scripts/measure.sh NAME [corpus,system,slice]
docs/reports/w48-callee-closure-r3/scripts/rank_all.sh NAME LABEL
docs/reports/w48-callee-closure-r3/scripts/identity.py BEFORE AFTER OUT.json
docs/reports/w48-callee-closure-r3/scripts/body_leverage.py DIAGNOSTICS OUT.json [--max-own-leaves N]
docs/reports/w48-callee-closure-r3/scripts/body_reasons.py PARITY.jsonl F1,F2 [--json OUT]
docs/reports/w48-callee-closure-r3/scripts/helper_blockers.py RANKING.json PARITY.jsonl OUT.json
docs/reports/w48-callee-closure-r3/scripts/helper_closure.py DIAGNOSTICS PARITY.jsonl SEED[,SEED] [--json OUT]
docs/reports/w48-callee-closure-r3/scripts/whatif.py DIAGNOSTICS CALLEE[,CALLEE]
docs/reports/w48-callee-closure-r3/scripts/compare_revisions.py FILE...
docs/reports/w48-callee-closure-r3/scripts/prepare-attempt.sh attempt-1 QUIRIN_AUDIT
docs/reports/w48-callee-closure-r3/scripts/pin.sh QUIRIN_AUDIT LOG_DIR [check-only]
docs/reports/w48-callee-closure-r3/scripts/gate.sh LOG_DIR PACKAGE_DIR
```

Private runs are under `private/w48cc/` of this card's worktree. The slice
runs are under the shared prepared attempt, as `w48cc-before` and
`w48cc-after`.
