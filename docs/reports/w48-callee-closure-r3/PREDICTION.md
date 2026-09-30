# W48-CALLEE-CLOSURE-R3: frozen prediction

Frozen before the AFTER run and committed before the implementation commit.
The two libc sections and the control test already existed, uncommitted, in
the worktree while this was written: the controls ran once as a probe of
`rd_clock` (section 3 below). No arbiter had run on them.

Base: local `main` `c374de96`. BEFORE (`scripts/measure.sh before`, binary
`private/w48cc/before.binary.sha256`) reproduces the brief's baseline:

| Route | BEFORE | Digest |
|---|---|---|
| corpus (185 units, `66910c10`) | 608 / 10 / 4,216 | `3a25c4e9…`, the brief's digest, byte for byte |
| system (`046accf6`) | 41 / 2 / 191 | `b389e288…`; totals equal CG10's AFTER, the file embeds this worktree's path (8 occurrences), so CG10's `f41b88cb…` cannot recur |
| tail-pause slice (`90978ac6`) | 28 / 1 / 240 / 1 Pending | totals equal CG10's AFTER; same path embedding |

## 1. Ranking (task step 1)

`scripts/closure_r3.py` over each route (`artifacts/ranking/*-before.{json,txt}`),
classes by `decisions.json`. A callee's class is (i) same-unit, (ii) one
other unit, (iii) no body in the route, (iv) no body and one of round 2's
eight decision classes (placed by reading, section 5 of the report), (v) a
compiler builtin. `alone`: functions blocked by that callee alone;
`plus-one`: by it and exactly one other callee or non-call leaf.

**Corpus:** 2,579 Unknown functions carry a call blocker, 469 have nothing
else.

| Class | Callees | Alone | Plus one | Sites | Callees freeing ≥ 2 alone |
|---|---:|---:|---:|---:|---:|
| (i) same-unit, body Unknown | 1,451 | 247 | 178 | 3,541 | 41 |
| (ii) cross-unit | 676 | 67 | 68 | 3,254 | 8 |
| (iii) external, no decision | 255 | 4 | 23 | 1,397 | 0 |
| (iv) a decision class | 96 | 28 | 31 | 570 | 3 |
| (v) compiler builtin | 15 | 6 | 2 | 180 | 0 |

**System:** 107 / 16 call-only; alone (i) 10, (iv) 4, (ii) 0, (iii) 0.
**Slice:** 144 / 24 call-only; alone (i) 8, (ii) 3, (iii) 2 (`lstat` and
`XXH_INLINE_XXH3_64bits`), (iv) 2 (`flb_config_map_create`,
`flb_sqldb_close`).

**The in-corpus closure** (functions freed per round):

| Variant | Corpus | System | Slice |
|---|---|---|---|
| existing same-unit summary rule | 0 | 0 | 0 |
| a cross-unit route (Safe home body) | 5, one round | 0 | 0 |
| cross-unit, freed functions taken Safe | 6: 5, then 1 | 0 | 0 |
| every class-(i)/(ii) body taken Safe | 390, one round | 12 | 16 |

The 5 are `cio_file_real_size`, `cmt_initialize`, `ctxTypeID_to_size`,
`LZ4_createStreamHC`, `rd_kafka_Uuid_hash`; round 2 adds
`rd_kafka_Uuid_map_hash`. The 390 are the ceiling of the bottom-up closure:
none arrives without its callee's own body being modelled.

**Class (iii), existing vocabulary.** Only one class-(iii) callee frees 2 or
more functions alone, `rd_kafka_topic_partition_get_private` (2), and its only
faithful v1 reading is the interior reading of class 8 (a nullable child the
partition allocates lazily and keeps in `_private`), so it is registered, not
declared. Every other class-(iii) row frees at most 1. By the brief's stopping
rule nothing more is owed; the card still takes the checked-route rows that
cost no decision:

- `lstat` (libc, flexible): `tail_file_lstat` in the slice, alone.
- `clock_gettime` (libc, flexible): not a reported blocker anywhere. It is
  the only uncontracted call in librdkafka's `rd_clock` header body, which
  W4 derives and cannot validate; `rd_clock` blocks 51 corpus functions (65
  sites). Declared as the libc fact, not by declaring `rd_clock` (user rule:
  no contract for a corpus body).

Declined here (existing-vocabulary contracts that would free a function, but
only by declaring a corpus header body instead of checking it):
`rd_clock` (1: `rd_kafka_timer_next`) and `rd_kafka_buf_write_i8` (1:
`rd_kafka_buf_upgrade_flexver_request`, whose body chain ends in the
cross-unit `rd_buf_write`).

## 2. Predicted movers

| Route | Prediction |
|---|---|
| corpus | **no move**; `resource-diagnostics.json` byte-identical (`3a25c4e9…`). `clock_gettime` is called only inside `rd_clock`, which is not a reported row, and `rd_clock` still refuses at its `return` (section 3), so W4 publishes no summary and no caller changes. The contracts sidecar and the audit log change (two more flexible libc contracts: 111 → 113 under the standing assumption) |
| system | **no move**, byte-identical: neither callee is called in the eight files |
| slice | **+1 Unknown → Safe**, `plugins/in_tail/tail_file.c::tail_file_lstat` (its one leaf is the `lstat` call). Its one caller that names it loses that leaf and stays Unknown (13 other callees, 158 other leaves). No leak or violation added |

No Safe → non-Safe move is predicted: both declarations are neutral
(`resource_borrowed` everywhere, `resource_return_none()`), so a body that
was Safe did not call them (they were uncontracted, which never yields Safe).

## 3. `rd_clock` (probe, recorded before the arbiter)

With `clock_gettime` in force the controls show `rd_clock`'s shape still
Unknown with one leaf, `unmodeled call, aggregate or indirect access in
return expression`: the return reads `ts.tv_sec` and `ts.tv_nsec` of the local
`struct timespec` whose address the call took. The parity stream names the
reader: `elaboration-return` "place: the typed access-path reader refused it
(CG10)", residual feature `aggregate-value`. The same read copied in a
statement first (`s = ts.tv_sec; return s;`) is Safe. That positional
refusal belongs to the aggregate-value card (CG14, running in this round);
if it lands, `rd_clock` and the helpers it alone blocks (`rd_interval0`,
`rd_interval_reset_to_now`, `rd_kafka_buf_set_abs_timeout0`,
`rd_kafka_buf_set_timeout`, `rd_timeout_init`, `rd_timeout_init_us`,
`rd_timeout_remains_us`, then `rd_timeout_remains`,
`rd_timeout_remains_limit`) can validate, and `rd_kafka_timer_next` is the
one reported function that frees (`scripts/helper_closure.py`,
`artifacts/ranking/helper-closure-rd-clock.json`, an upper bound).

## 4. Cross-unit route (task step 2(ii))

A route would free 5 corpus functions in one round and 6 at its ceiling,
0 in the system and slice routes. It needs a two-phase pinned run (export
each unit's validated summaries of external-linkage definitions keyed by
the home unit's source and environment digests, then re-analyse importing
units), a stated external-linkage premise against rule 4a's interposition
and C99 `inline` objections, certificate re-derivation of both halves and a
Lean module. Predicted decision: too large for 6 functions; measured and
registered with this size, not built.
