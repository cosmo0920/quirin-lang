# W47-CALLEE-CLOSURE-R2: prediction (frozen before implementation)

Frozen at base `3ba72320` (local `main`, "Merge runtime imports and explicit
contract checks after complete gate") before any provider file was edited.

## 1. BEFORE is the post-merge baseline

`scripts/run_pinned_audit.py private/w47cc/runs/before 3ba72320 --jobs 5`
(the W43 `run_audit.py` named by the brief drives the retired
`cycle19_fluent_bit_libraries_audit` example, removed by `b64be69e`; the live
runner is `quirin-audit pinned`), with `QCORE_TYPED_PATH_PARITY` inside the
run: exit 0. Against the P3 AFTER archive
(`w47-access-path-positions/artifacts/after/corpus/`, built at `9b2ac682`):

- diagnostics, chunk contracts, Monkey source, the sorted parity stream and
  its contexts are **byte-identical** (`artifacts/before/identity.json`,
  `parity-identity.json`);
- the contracts and source-effects sidecars differ, and every difference is
  attributed (`scripts/attribute_sidecars.py`, exit 0,
  `artifacts/before/sidecar-attribution.json`): `f4018a92` ("Migrate contract
  sources and lock paths to .qv", the only commit on `9b2ac682..3ba72320`
  touching a provider contract text) renamed every module path, and the
  loader's blank padding of an expanded `#include` shrinks with the directive;
  what remains is 34 (contracts) and 26 (source-effects) digest leaves that
  bind those texts, paths and the worktree root (module and lock SHA-256s,
  `resource_contracts:` ids of three chunk rows, build and configuration
  digests, as `audit-pinned-replay-cli` recorded).

So the P3 AFTER identity holds on `main`: **507 Safe / 9 provisional Unsafe /
4,318 Unknown**, leak 34, `unknown_alias` 7,440, unsupported 36,262, Pending 0.

## 2. Closure re-measure (no new trust)

`scripts/closure_r2.py` over BEFORE (`artifacts/ranking/closure-before.*`).
P1's closure predicted access removal from census streams; after P3 those
leaves are measured, so the closure is recomputed over the diagnostics with
P1's callee classes and variants. 447 Unknown functions are blocked only by
call leaves: **374 only by in-corpus callees**, 43 only by external callees,
30 by both.

| Variant | Freed | Rounds |
|---|---:|---|
| same-unit (existing summary rule) | 0 | 1 |
| cross-unit (a Safe body in one other unit) | 3 | 1 |
| ceiling (every freed function taken Safe) | 3 | 2 (3, then 0) |

The in-corpus closure is empty under the existing rules and three functions
deep under a cross-unit route (`cio_file_real_size`, `cmt_initialize`,
`ctxTypeID_to_size`; `W44-CALLEE-CROSS-UNIT-SUMMARY`). The 374 wait on
Unknown callees (largest: `cJSON_New_Item` 10, `LZ4_decompress_generic` 9,
`cmt_mpack_unpack_array` and `ctr_mpack_unpack_array` 5 each).
`snappy.c::hash` waits on a Safe same-unit callee in a nested position, which
no summary serves.

## 3. External ranking against W46

**33 functions blocked alone by one external callee** (367 callees / 2,128
sites by W46's script; `artifacts/ranking/w46-rank-before.txt`), against
W46-EXTERNAL-RESIDUAL-RANKING's 30. The 30 are unchanged, every one behind
its named decline. P3 exposed three new ones, and each can be declared
without a product decision:

| Callee | Alone | Sites | Owner |
|---|---:|---:|---|
| `cfl_arena_calloc` | 2 | 13 | cfl, `quirin-provider-cfl` |
| `rd_file_read` | 1 | 2 | librdkafka (`static RD_INLINE` in `src/rd.h`), `qcore-provider-librdkafka` |

With one other blocker (`plus_callee` / `plus_leaf` in the JSON) nothing else
is reachable by a contract alone: the other callee or leaf is itself a
declined class (`pthread_setspecific`, `abort`, the va builtins,
`getrlimit`/`setrlimit`, `regcomp`/`regfree`, the arena constructor, refcount
drops such as `rd_kafka_enq_once_trigger`), an access/alias leaf, or a callee
in a return expression (`bsearch`, `fabs`).

## 4. The contracts this card will write

Both bodies are identical at `66910c10` and `046accf6` (`cmp` of
`lib/cfl/src/cfl_arena.c`, `lib/cfl/include/cfl/cfl_arena.h` and
`lib/librdkafka-2.15.0/src/rd.h` against W46's read-only `046accf6` clone;
the pins are taken on this card's own fresh clone), each has one copy in the
tree, and both are pinned in revision mode.

- **`cfl_arena_calloc`** (`cfl/sds_lifetime.qv`, the crate's only resource-view
  module that states memory lifetimes). `arena`, `count`, `size` borrowed;
  result **`resource_return_interior(arena, nullable)`**. The body allocates
  inside chunks the arena owns (a new chunk is linked into `arena->head`), the
  result is never `free`d by the caller and is invalidated by
  `cfl_arena_destroy`; NULL on overflow or allocation failure. The derived
  result's six consumer rules (contract spec C04.3.1) hold for it. A probe
  (`crates/qcore-c-gateway/tests/w47_callee_closure_r2.rs`, uncommitted
  probe, `private/w47cc/logs/probe1.log`) measured the alternative
  `resource_return_none()` **admitting a use of the result after the base is
  freed as `Safe`**, where the interior mode reports a use-after-release
  candidate; `none` is therefore not declared. The reading "interior to the
  arena's storage" rather than "inside `*arena`" is stated in the module.
- **`rd_file_read`** (`librdkafka/memory.qv`, beside `rd_malloc`/`rd_free`).
  `file_path`, `size`, `max_size` borrowed; result
  **`resource_return_owned(nullable)`**: `rd_malloc` then `return buf`, NULL
  on open/seek/size/read failure after `rd_free(buf)`. A `static RD_INLINE`
  header body: an unchecked premise pinned to the header text (rule 10j,
  `W46-HEADER-INLINE-DECLARED-UNCHECKED`).

Existing effect kinds only (borrowed, derived interior result, owned
nullable result): **no analyzer change and no Lean theorem**.

## 5. Predicted moves

| Function | Move | Unblocked by | Confidence |
|---|---|---|---|
| `cmt_encode_opentelemetry.c::initialize_attribute_list` | Unknown → Safe | `cfl_arena_calloc` | high (probe body of the same shape is `Safe`) |
| `cmt_encode_opentelemetry.c::initialize_summary_value_at_quantile_list` | Unknown → Safe | `cfl_arena_calloc` | high |
| `rdkafka_sasl_oauthbearer_oidc.c::rd_kafka_oidc_assertion_read_from_file` | Unknown → Safe | `rd_file_read` | high (null test of the formal, then a whole-call return of the owned result) |
| `rdkafka_sasl_oauthbearer_oidc.c::ut_sasl_oauthbearer_oidc_assertion_parse_from_file` | Unknown → provisional Unsafe **possible** | the owned-result summary of the function above | medium: if the summary serves its three calls, `result` leaks on the `RD_UT_ASSERT` failure returns at 1592, 1600 and 1605 (the assertion's own early `return 1`), which would be **real** test-path leaks of the kind its sibling `ut_sasl_oauthbearer_oidc_assertion_process_template_file` already carries; a leak at 1590 (the `result != NULL` failure branch, where `result` is NULL) would be an artifact |

**+3 Safe, the same fewer Unknown; units unchanged (6 / 8 / 167 / 4); no
Safe → non-Safe move.** Leaves: the 13 `cfl_arena_calloc` and 2 `rd_file_read`
uncontracted-call leaves removed; 1 `initialize_attribute_list` leaf
(`append_sample_to_metric`) and 3 `rd_kafka_oidc_assertion_read_from_file`
leaves removed through the summary rule if the new Safe bodies publish
summaries; `unknown_alias` leaves at the bound results (`data_point`,
`instance`, `template_content`, `result`) may change as identities resolve or
the next refusal is exposed. Leak rises by 0 to 4, all in the unit-test
function above. No other function's status moves.

## 6. Fluent Bit system slice

Off route: none of the eight slice files calls either callee at `66910c10`
or at `046accf6` (`grep -c` 0 in each). The slice is not re-run.
