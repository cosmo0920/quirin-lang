# W47-CALLEE-CLOSURE-R2: callee closure and external contracts after P3

Card W47-CALLEE-CLOSURE-R2, 2026-09-28, base local `main` `3ba72320` ("Merge
runtime imports and explicit contract checks after complete gate"). Plan P4,
round 2, in the [unsupported-coverage queue](../../plans/unsupported-coverage/README.md):
re-measure the in-corpus callee closure on the post-P3 corpus, then contract
the external callees that now block the most functions. The
[prediction](PREDICTION.md) was frozen and committed (`1905f391`) before any
provider file was edited. Gateway rule [10j](../../agents/gateway.md#constraint-10j)
is amended; no new rule number is taken.

## Headline

- **The post-merge baseline holds.** BEFORE at `3ba72320` reproduces the P3
  AFTER archive byte for byte in diagnostics, chunk contracts, Monkey source
  and the sorted parity stream; the two sidecars differ only by `f4018a92`'s
  `.qv` rename and the digests that bind it (§1).
- **The in-corpus closure is empty.** 374 Unknown functions are blocked only
  by in-corpus callees. The existing summary rule frees none of them. A
  cross-unit route would free 3 in one round, and nothing more even with every
  freed function taken Safe (§2).
- **External ranking.** 33 functions are blocked by one external callee alone,
  against W46's 30. Those 30 are unchanged, each behind its named decline. P3
  exposed three more, whose callees need no product decision:
  `cfl_arena_calloc` (2) and `rd_file_read` (1).
- **Two contracts**, pinned at `66910c10` and `046accf6` (identical bodies):
  - `cfl_arena_calloc` (`quirin-provider-cfl`): result interior to the borrowed
    arena, nullable;
  - `rd_file_read` (`qcore-provider-librdkafka`): result owned and nullable.
- **Corpus: +3 Safe, +1 provisional Unsafe.** Functions **507 / 9 / 4,318 →
  510 / 10 / 4,314**; units unchanged; leak 34 → 36; unknown_alias 7,440 →
  7,442; unsupported 36,262 → 36,250. Every leaf is attributed. There is **no
  correction**. Of the two added leaks, one is real (a unit-test failure
  path) and one is a summary-approximation artifact (§4).
- **The gain is small.** Nothing else in the ranking can be contracted without
  one of the decisions listed in §6.
- **Lean:** none, and no analyzer change. Both clauses are existing v1 modes
  (borrowed, derived interior result, owned nullable result).
- **Slice:** off route (§5).
- **Follow-ups 523 → 526:** three registered, four amended, none retired.

## 1. BEFORE and the post-merge identity

The W43 `run_audit.py` named by the brief drives
`cycle19_fluent_bit_libraries_audit`, which `b64be69e` retired after exact
archive parity (`audit-pinned-replay-cli`). The live runner is
`scripts/run_pinned_audit.py`, which runs `quirin-audit pinned` over the same
pinned inputs. Command:

```sh
QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl scripts/run_pinned_audit.py OUT 3ba72320 \
  --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 5
```

Exit 0, 8 minutes. The binary's SHA-256 is `2db21ed2…`
([run.json](artifacts/before/run.json)).

Against `w47-access-path-positions/artifacts/after/corpus/`, built at
`9b2ac682`:

- **Byte-identical:** `resource-diagnostics.json`, the chunk contracts and
  `monkey-source.json` ([identity.json](artifacts/before/identity.json),
  `baseline_identity.py`, exit 0, `verdict_bearing_identical: true`). The
  sorted parity stream and its contexts are identical too
  ([parity-identity.json](artifacts/before/parity-identity.json)).
- **Contracts and source-effects sidecars:** they differ, and
  [`scripts/attribute_sidecars.py`](scripts/attribute_sidecars.py) accounts
  for every difference (exit 0,
  [sidecar-attribution.json](artifacts/before/sidecar-attribution.json)):
  - `f4018a92` ("Migrate contract sources and lock paths to .qv") is the only
    commit on `9b2ac682..3ba72320` that touches a provider contract text. It
    renamed every module path in keys, the selection manifest and
    `contract_revision`. It also renamed the `#include` parents, and the
    loader's blank padding of an expanded directive shrinks with them.
  - After that rewrite only digest leaves remain: 34 in contracts and 26 in
    source-effects. They are the module and lock SHA-256s that bind those
    texts and paths, the `resource_contracts:` ids of three chunk rows, and
    the cwd-bound build and configuration digests that
    `audit-pinned-replay-cli` explained.

So `main`'s first post-merge measurement is the P3 AFTER: **507 Safe / 9
provisional Unsafe / 4,318 Unknown**. There is no unattributable difference.

## 2. Closure re-measure (no new trust)

P1's closure (`w46-access-path-census/scripts/predict.py`) worked from a
prediction of which access leaves P2 and P3 would remove, so it needed the
census streams. After P3 those leaves are measured. The re-run therefore
recomputes the closure over the diagnostics themselves, with P1's callee
classes and variants: [`scripts/closure_r2.py`](scripts/closure_r2.py),
[before](artifacts/ranking/closure-before.txt),
[after](artifacts/ranking/closure-after.txt), JSON beside them.

A callee is classed as follows:

- **external:** no unit lists it (W46's test);
- **same-unit:** the calling unit defines it;
- **cross-unit:** exactly one other unit defines it. The diagnostics carry no
  storage class, so a single definition is read as the external-linkage body
  the call binds; this is a stated premise;
- **ambiguous:** several units define it.

A `nested call` leaf is refused for its position. No summary serves it, so it
always blocks: `snappy.c::hash` waits only on the Safe same-unit
`get_unaligned32` in that position.

| Measure (BEFORE) | Value |
|---|---:|
| Unknown functions blocked only by call leaves | 447 |
| only in-corpus callees / only external / both | **374** / 43 / 30 |
| freed by the existing same-unit summary rule | **0** |
| freed by a cross-unit route (a Safe body in one other unit) | **3** (`cio_file_real_size`, `cmt_initialize`, `ctxTypeID_to_size`), one round |
| ceiling (every freed function taken Safe) | 3, then 0 |

The 374 wait on callees that are themselves Unknown. Each count below is the
number of functions a callee blocks alone:

| Callee | Blocks alone | Home |
|---|---:|---|
| `cJSON_New_Item` | 10 | same unit |
| `LZ4_decompress_generic` | 9 | same unit |
| `cmt_mpack_unpack_array`, `ctr_mpack_unpack_array` | 5 each | cross-unit |
| `LZ4_compress_HC`, `LZ4_compress_HC_extStateHC` | 4 each | same unit |
| `rd_kafka_admin_result_ret_groups`, `sds_cat_safe` | 4 each | same unit |

P1 forecast 870 such functions after P3. That figure counted access leaves it
predicted removable. The 374 here have only call leaves.

The external closure is round 0 by construction: a declared contract serves
its caller in the same run.

## 3. External ranking and the contracts

The W46 script on BEFORE ([w46-rank-before.txt](artifacts/ranking/w46-rank-before.txt))
finds 367 external callees, 2,128 sites and **33 functions blocked alone**.
The extended ranking in `closure-before.json` adds three columns:

- `alone` counts `nested call` leaves naming the callee too (W46's
  `pack_string` miss);
- `plus_callee` counts functions whose only other blocker is one more callee;
- `plus_leaf` counts functions whose only other blocker is one non-call leaf.

**Against W46-EXTERNAL-RESIDUAL-RANKING (30):** the same 30 functions remain,
each in its declined class:

| Declined class | Functions |
|---|---:|
| cfl in-place reallocation | 3 |
| Monkey backend internals | 7 |
| libc declined by earlier cards | 9 |
| compiler builtins | 6 |
| librdkafka refcount drop and add | 2 |
| OpenSSL / libcurl | 2 |
| function-pointer formal called by name | 1 |

P3 exposed three new ones:

| Callee | Alone | Sites | Decision |
|---|---:|---:|---|
| `cfl_arena_calloc` | 2 | 13 | **declared** |
| `rd_file_read` | 1 | 2 | **declared** |

The one-other-blocker tier holds nothing a contract alone reaches. The other
blocker in each case is one of these:

- a declined callee: `pthread_setspecific`, `abort`, the va builtins,
  `getrlimit`/`setrlimit`, `regcomp`/`regfree`, or the arena constructor
  pair;
- a refcount drop: `rd_kafka_enq_once_trigger`, 6 functions, each with an
  alias leaf;
- a retained field allocation: `rd_kafka_topic_partition_get_private`, 3;
- a callee in a return expression: `bsearch`, `fabs`;
- an access or alias leaf.

**The contracts.** Both bodies were read at `66910c10` and at this card's own
fresh `046accf6` clone, and they are identical there (one copy each):

- the clone was made by `scripts/prepare-attempt.sh attempt-1`, exit 0, under
  `private/w45-fb/w47-callee-closure-r2/`;
- [`scripts/pin.sh`](scripts/pin.sh) ran `quirin-audit pin --crate … --function …`
  at both revisions and then `pin --check` at both: all exit 0, every crate
  0 not in force and 0 stale;
- the corpus run reports **127 pinned in force** (125 before) and 105 libc
  contracts under the standing assumption.

- **`cfl_arena_calloc`** (`cfl/sds_lifetime.qv`; the crate's other
  resource-view module is `cfl/time.qv`, and `arena_lifetime.qv` is not a v1
  module):
  - `arena`, `count` and `size` are borrowed;
  - the result is `resource_return_interior(arena, nullable)`.

  The body allocates inside chunks the arena owns: a new chunk is linked into
  `arena->head`, and `cfl_arena_destroy` frees them all. It returns NULL on
  overflow, a zero size or a failed chunk allocation. All six consumer rules
  of C04.3.1 hold for it.

  Before declaring, a probe compared the two candidate result modes
  ([probe](artifacts/probe/probe-output.txt), source kept as
  `probe.rs.txt`). With `resource_return_none()`,
  `p = cfl_arena_calloc(m, 1, 8); free(m); … return p[0];` is **`Safe`**:
  that mode forgets the result's dependence on the arena. With the interior
  mode the same body is a use-after-release candidate. So the interior mode
  is declared.

  That mode reads "interior" as the storage the arena owns, not the bytes of
  `struct cfl_arena`. The reading is stated in the module and in rule 10j, and
  registered as `W47-ARENA-INTERIOR-READING` for the user to confirm.
- **`rd_file_read`** (`librdkafka/memory.qv`, beside `rd_malloc`/`rd_free`):
  - `file_path`, `size` and `max_size` are borrowed;
  - the result is `resource_return_owned(nullable)`: an `rd_malloc` buffer,
    or NULL after `rd_free` on every failure arm.

  It is `static RD_INLINE` in `rd.h`, so it is an unchecked premise pinned to
  the header text (`W46-HEADER-INLINE-DECLARED-UNCHECKED`, amended).

**Controls.** [`crates/qcore-c-gateway/tests/w47_callee_closure_r2.rs`](../../../crates/qcore-c-gateway/tests/w47_callee_closure_r2.rs)
loads the production text byte for byte:

- the three moved shapes, plus the rd_file_read release pattern, are `Safe`;
- without the card's sections each of them has an uncontracted call;
- a use of the arena result after the arena is released is a
  use-after-release candidate;
- a released arena argument, and `free` of the arena result, are never `Safe`;
- borrowing an owned arena still leaks the arena;
- a dropped file buffer leaks;
- a double release of the buffer is never `Safe`;
- the NULL-return arm needs no release.

**Deliberate pin updates:**

- `qcore_contract_corpus.rs`: two rows, 5 clauses each;
- `contract_loader.rs`: v1 declarations 230 → 232, and both modes added to
  the mode table;
- the hand-written `librdkafka_memory_hand_written.qv` fixture gains
  `rd_file_read`.

## 4. Corpus arbiter (185 units)

AFTER uses the same command at `7c0e38f3` (binary `8065837d…`), exit 0, and is
archived as the new baseline under [artifacts/after/corpus/](artifacts/after/corpus/)
(manifest with raw, sorted-row and gzip digests).

| Measure | BEFORE (= P3 AFTER) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 507 / 9 / 4,318 | **510 / 10 / 4,314** | +3 / +1 / −4 |
| Units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | 6 / 8 / 167 / 4 | 0 |
| leak / unknown_alias / unsupported / summary_conflict | 34 / 7,440 / 36,262 / 1 | 36 / 7,442 / 36,250 / 1 | +2 / +2 / −12 / 0 |
| Functions blocked alone by one external callee | 33 | 30 | −3 |
| Pinned contracts in force | 125 | 127 | +2 |

Chunk contracts, Monkey source and source effects are byte-identical to
BEFORE ([tallies.json](artifacts/attribution/tallies.json)). The parity
stream gains 47 rows and loses 1, all in the eleven changed functions and in
`rd_kafka_oidc_token_jwt_bearer_refresh_cb`, whose leaves do not change
([parity-diff.json](artifacts/attribution/parity-diff.json)). No parity is
claimed.

**Attribution.** [`scripts/attribute.py`](scripts/attribute.py) (W46's, with
this card's callees and a reviewed-by-reading arm) exits 0 with **0
unattributed**. The output is [arbiter.json](artifacts/attribution/arbiter.json),
every leaf is in [changed-leaves.jsonl](artifacts/attribution/changed-leaves.jsonl),
and the readings are in [reviewed.json](artifacts/attribution/reviewed.json).

- **Removed, 15 leaves:** uncontracted calls to the declared callees
  (`cfl_arena_calloc` 13, `rd_file_read` 2).
- **Removed, 4 cascade leaves:** the summary rule now serves callers of the
  new Safe functions: `initialize_attribute_list` in
  `append_sample_to_metric`, and `rd_kafka_oidc_assertion_read_from_file`
  three times in `ut_sasl_oauthbearer_oidc_assertion_parse_from_file`.
- **Removed, 3 alias leaves, identities resolved:**
  - `template_content` at the null test and the early release in
    `rd_kafka_oidc_assertion_parse_template_file`;
  - `result` at the `rd_free` in the unit test.
- **Added, 7 refusals exchanged at the same calls:**
  - 3 indirect-access leaves where the arena result is stored into a field
    (`attribute->value = …`, `data_point->bucket_counts = …`), which is P5's
    pointer-valued store;
  - 4 `nested call get_context_arena` leaves in `append_sample_to_metric`,
    whose calls pass `get_context_arena(context)` to a now-modeled callee.
- **Added, 5 alias leaves exposed later:** each is a
  `return data_point;`-shaped exit that returns the arena-derived result from
  a body with other refusals: `initialize_histogram_data_point`,
  `initialize_numerical_data_point`, `initialize_string_attribute`,
  `initialize_summary_data_point` and `initialize_summary_value_at_quantile`.
  These are Unknown-class leaves in the accusing direction.
- **Added, 2 leaks** (below).

**Moves (4, no correction):**

| Function | Move | By |
|---|---|---|
| `cmt_encode_opentelemetry.c::initialize_attribute_list` | Unknown → Safe | `cfl_arena_calloc` |
| `cmt_encode_opentelemetry.c::initialize_summary_value_at_quantile_list` | Unknown → Safe | `cfl_arena_calloc` |
| `rdkafka_sasl_oauthbearer_oidc.c::rd_kafka_oidc_assertion_read_from_file` | Unknown → Safe | `rd_file_read` |
| `rdkafka_sasl_oauthbearer_oidc.c::ut_sasl_oauthbearer_oidc_assertion_parse_from_file` | Unknown → provisional Unsafe | the summary cascade of the function above |

Each `Safe` is relative to the declaration it names. The two cmetrics moves
also rest on the interior-extent reading.

**Leak classification (by reading, `rdunittest.h:93-106`).** `RD_UT_ASSERT`
returns 1 on failure without releasing anything, unless
`rd_unittest_assert_on_failure` aborts first.

- **`:1603`/`:1605`, real.** The code is
  `result = rd_kafka_oidc_assertion_read_from_file("/non/existent/file/path"); RD_UT_ASSERT(result == NULL, …)`.
  If that path exists and reads, the buffer leaks on the assertion's failure
  arm. This is test code on an environment-dependent failure path, and it has
  the shape its sibling `ut_sasl_oauthbearer_oidc_assertion_process_template_file`
  already carries (`:1656`/`:1659`).
- **`:1599`/`:1600`, artifact (summary approximation).** The call is with
  `NULL`, and the callee returns NULL for a NULL argument (`:652-653`), so C
  cannot reach the failure arm. The owned-nullable summary does not say so.
  This is registered as `W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL`.

The function's provisional Unsafe rests on the real leak, not only on the
artifact. It is a provisional candidate in test code, never an upstream defect
claim. No `double_release`, `invalid_release` or `use_after_release` leaf is
added.

## 5. Prediction against outcome

| Predicted | Outcome |
|---|---|
| BEFORE = P3 AFTER | held; sidecar differences attributed to `f4018a92` |
| +3 Safe: the two list initialisers and `rd_kafka_oidc_assertion_read_from_file` | **all three** |
| units unchanged; no Safe → non-Safe | held |
| the unit test may become provisional Unsafe with real leaks at 1592/1600/1605; an artifact at 1590 if the null branch were not refined | **became provisional Unsafe with 2 leaks.** `:1605` is real. `:1600` is an **artifact**: the prediction missed that the callee returns NULL for a NULL argument. No leak appears at 1590 or 1592 |
| 13 + 2 declared leaves removed; 1 + 3 cascade leaves; alias leaves may change | exactly; 3 aliases resolved, 5 exposed, plus 7 exchanged refusals the prediction did not list (field stores and `get_context_arena` nesting) |
| slice off route | not re-run: neither callee is called in the eight slice files at `66910c10` or `046accf6` (`grep -c` 0 in each file of both trees) |

## 6. Decisions this card did not make

Everything left in the external ranking waits on one of these. Each is a
product or vocabulary decision, measured and listed for the user:

1. **Callbacks and retained pointers** (`pthread_setspecific`/`_getspecific`,
   `pthread_once`, `bsearch`, `rd_qsort_r_cmp`). This needs a callback or
   retention vocabulary.
2. **In-place lifecycles** (`pthread_key_create`, `pthread_rwlock_init`/`_destroy`,
   `regcomp`/`regfree`) and the **in-place reallocation** of
   `cfl_sds_cat_safe` (3 alone, 5 more one of two). These need an out-slot or
   lifecycle mode.
3. **Reference counts** (`rd_kafka_q_destroy_owner`, `rd_kafka_toppar_keep0`,
   `rd_kafka_enq_once_trigger`). This needs a counted-reference kind.
4. **Handle kinds and no-return** (`dlopen`/`dlclose`, the arena constructor,
   `abort`, `exit`).
5. **System libraries outside the audited source** (`ERR_clear_error`,
   `curl_global_init`). This would need a flexible mode for a non-libc
   library.
6. **Compiler builtins without an owning crate**: the `__atomic_*` family,
   `__builtin_ctz*`, `__builtin_nanf` and the va builtins.
7. **Monkey backend-selected `static inline` bodies**: `_mk_event_*` and
   `mk_http_thread_resume`.
8. **The interior-extent reading this card did make**
   (`W47-ARENA-INTERIOR-READING`): confirm it, or ask for a pool/arena result
   mode.

The in-corpus closure waits on a cross-unit summary route
(`W44-CALLEE-CROSS-UNIT-SUMMARY`, 3 functions) and on the callees' own
blockers. It does not wait on contracts.

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/). It records the standing gate
and the provider gate by [`scripts/gate.sh`](scripts/gate.sh), with every exit
in its own file and private logs.

## 8. Follow-ups

These are applied by [scripts/update_ledger.py](scripts/update_ledger.py) from
[followups.json](followups.json).

**Registered:**

- `W47-EXTERNAL-RESIDUAL-RANKING-R2`: 30 functions alone, 53 with one other
  blocker;
- `W47-ARENA-INTERIOR-READING`;
- `W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL`: 1 artifact leak.

**Amended:**

- `W46-EXTERNAL-RESIDUAL-RANKING`: re-measured, the same 30;
- `W46-HEADER-INLINE-DECLARED-UNCHECKED`: a third declaration;
- `W44-CALLEE-CROSS-UNIT-SUMMARY`: the post-P3 closure;
- `NESTED-CALL-IN-ARGUMENT-POSITION`: the same-unit Safe inner callee, and the
  4 exchanged leaves.

**Retired:** none. W46-EXTERNAL-RESIDUAL-RANKING's classes are unchanged, so
nothing is discharged.

## 9. Reproduction

```sh
docs/reports/w47-callee-closure-r2/scripts/label_manifest.py ARCHIVE_DIR OUT
docs/reports/w43-return-expression/scripts/baseline_identity.py RUN LABELS --root-alias WORKTREE=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a381f4497377f3776
docs/reports/w47-callee-closure-r2/scripts/attribute_sidecars.py P3_ARCHIVE RUN OUT --root-alias WORKTREE=…agent-a381f4497377f3776
docs/reports/w47-callee-closure-r2/scripts/closure_r2.py RUN/resource-diagnostics.json OUT.json [--previous BEFORE.json]
docs/reports/w47-callee-closure-r2/scripts/prepare-attempt.sh attempt-1 QUIRIN_AUDIT
docs/reports/w47-callee-closure-r2/scripts/pin.sh QUIRIN_AUDIT LOG_DIR
docs/reports/w47-callee-closure-r2/scripts/attribute.py BEFORE AFTER --source SOURCE_ROOT --reviewed artifacts/attribution/reviewed.json --json OUT
docs/reports/w47-callee-closure-r2/scripts/tallies.py BEFORE AFTER OUT
docs/reports/w47-callee-closure-r2/scripts/parity_diff.py BEFORE/parity.jsonl AFTER/parity.jsonl OUT
docs/reports/w47-callee-closure-r2/scripts/archive.py RUN OUT_DIR
```

Private runs are under `private/w47cc/` of this card's worktree. The contracts
sidecar embeds the worktree path in its source keys, as every isolated card's
does.
