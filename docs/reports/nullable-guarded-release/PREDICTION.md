# NULLABLE-GUARDED-RELEASE: prediction

Committed before the implementation commit. Base `96d41921c` (main).
USER DECISION 2026-11-22: "launch the guarded-release fix card".

## What was known when this was written

Two base measurements and three probes; no implementation was measured on
the corpus.

* **Base**, opt-in `quirin-audit current --monkey-linked` over Fluent Bit
  046accf6 (binary built from `96d41921c`, side tree `git archive
  96d41921c`): **1,169 Safe / 2 Pending / 3,626 Unknown / 7 Unsafe**.
* **Base with the held branch merged** (`git merge --no-commit 4174d25e5`,
  a throwaway worktree, same binary): **1,171 / 2 / 3,622 / 9**, the held
  card's four moves reproduced on the newer base
  (`rd_kafka_mock_handle_Metadata` and `ut_testSameSubscriptions` provisional
  Unsafe).
* **The Metadata leak is an intermediate-iterate report.** The leak-witness
  census (`indirect-census` build, the mock-handlers unit alone, base with
  the held branch) records the leak at `:1490` with `settled: false` and the
  site `unresolved: true` at the settled exit. `requested_topics` reaches the
  guards as an *unknown* binding: `&requested_topics->elems[i]` (`:1415`)
  drops it from rule 10a's sparing frame, so every refused statement (the
  `rd_kafka_buf_read_*` macros) poisons it. The guard `if (requested_topics)`
  is then refused (rule 10b), the release is refused (unresolved identity and
  the nullable leaf) and marks the site unresolved, but the refused guard's
  null edge reaches `return 0` first on some iterate, where `report_leaks`
  claims the site; the claim is never withdrawn (rule 6,
  C02-P-ALIAS-LEAK-ORDER). The settled analysis does not support it.
* **Local probes** (`crates/qcore-c-gateway/tests`, a reduced preprocessed
  unit per row, the two librdkafka declarations): every one of the 20
  nullable leaves of the 11 rows reproduces; the Metadata row is Unknown with
  no leak (the local schedule happens to settle the marker first). With the
  sparing frame reading `&NAME->F` as C does (the reading rule 12e already
  gives the tie names), the nullable leaves of `rd_kafka_mock_handle_Metadata`,
  `rd_kafka_metadata_topic_match` and `rd_kafka_mock_handle_ShareAcknowledge`
  go and `rd_kafka_mock_handle_ShareFetch` keeps three of six; the other seven
  rows keep theirs (their nullability comes from other shapes: a lazy
  constructor in a loop, a poisoned binding joined at a loop head).
  `ut_testSameSubscriptions` keeps its four leaks.

## Planned changes

* **(A) A leak claim must hold at the settled exit.** A current-generation
  leak `report_leaks` finds is held, as rule 42 holds an earlier
  generation's, and decided once the work list is stable: withdrawn when the
  settled exit state marks the identity unresolved (a refused effect, a
  refused release among them, may have discharged it) and the function
  carries an `Unknown` leaf; inserted otherwise. Only Unsafe -> Unknown moves
  are possible.
* **(B) The sparing frame reads a member address as C does.** Rule 10a's
  frame drops a name any `&` is followed by; `&NAME->F` (`&(NAME)->F`) takes
  the address of a member of `*NAME`, never of `NAME`'s slot, so a refused
  statement cannot write `NAME` through it. The frame reads the body through
  rule 12e's member-address reader. The guard refinement itself (rules 14l,
  10b) is unchanged: a binding that survives decides the guard, its non-null
  edge accepts the `nonnull` release, its null edge removes the site. A
  refusal stays a refusal, so no row can become Safe through (B); a spared
  binding can expose a violation (rule 10a's recorded effect).

## Predicted movements (opt-in route, base vs tip)

**Safe -> non-Safe: 0. Unknown -> Safe: 0** (neither change can admit a
refused function).

**Unsafe -> Unknown: 0 to 2.** Only a leak the settled state withdraws can
move; the base's seven Unsafe rows (`decode_statsd_message`'s double
release, `cmt_map_metric_get`, the two `rdunittest_acknowledge.c` rows,
`rd_kafka_snappy_java_uncompress`, `mk_http_error`, `mk_net_conn_create`)
were not censused, so none is predicted by name.

**Unknown -> Unsafe: 0 to a few possible**, from (B) exposing a violation
behind a formerly poisoned binding; any is reported per row as provisional.

**Leaves:** `unresolved resource identity` leaves fall (bindings spared in
functions spelling `&NAME->F`), by hundreds rather than tens; no status moves
from that alone.

## Predicted movements (merged comparison, private tree only)

Tip with the held branch merged, against base with the held branch merged:
`rd_kafka_mock_handle_Metadata` **Unsafe -> Unknown**; its nullable leaves
gone. `ut_testSameSubscriptions` stays provisional Unsafe (its leaks are on
real `RD_UT_ASSERT` / `verifyValidityAndBalance` failure returns). Of the
other ten nullable-leaf rows, `rd_kafka_metadata_topic_match`,
`rd_kafka_mock_handle_ShareAcknowledge` lose their nullable leaves and
`rd_kafka_mock_handle_ShareFetch` keeps the `forgotten_partitions` ones; all
stay Unknown on other leaves.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). On main the librdkafka
constructors are undeclared, so no nullable-release row exists there; the
expected status moves are none, with the same two possibilities as above
(a withdrawn intermediate-iterate leak, a violation exposed behind a spared
binding), each reported as unpredicted if it occurs.
