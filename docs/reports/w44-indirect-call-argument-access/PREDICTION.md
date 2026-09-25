# W44-INDIRECT-CALL-ARGUMENT-ACCESS: frozen prediction

Frozen before the admission is committed or measured. The base is `c1e69bb5`;
the census instrumentation is commits `d359d3fd`, `43dd4933` and `ba3a22fa`
(recogniser, probe and stream; every census run reproduces all five BEFORE
artifacts byte for byte). The forecast is the census's own: the probe
classifies every refused statement again with the candidate actuals admitted,
using the classifier the admission will use, and records the answer.

## The family at the base

W43-INDIRECT-ACCESS-GENERAL's `residuals.py` class `call-argument`, re-measured
on the combined baseline (census run of `ba3a22fa`): **9,522 leaves / 2,407
Unknown functions / 250 sole-blocked** (the ledger's 9,539 / 2,408 / 242 was
measured on the W43-INDIRECT-ACCESS-GENERAL AFTER tree).

Split by the statement's first call ([census/census.txt](census/census.txt)):

| Class | Leaves | Functions |
|---|---:|---:|
| callee has no contract in force (`uncontracted`) | 5,991 | 1,856 |
| contracted, no refusing actual is a typed read at `Borrowed`/tail (`no-candidate`) | 1,963 | 875 |
| a mix of the two on one line | 1,148 | 248 |
| **admitted by the probe** (contract call) | **147** | **88** |
| not a simple statement (a `for` clause, a condition) | 118 | 107 |
| the refused access is the destination (`p->f = g()`), no refusing actual | 77 | 68 |
| probe refused (another actual still refused, excess actuals, nested call, discarded result) | 57 + 18 mixed | — |

The refusing actuals of contracted calls are dominated by shapes this rule
does not type: member addresses whose record the reader did not complete
(2,406 `Borrowed`, mostly `mtx_lock(&(rkbuf->rkbuf_rkb)->…)` chains and
`rd_kafka_broker_s` members), functions whose declarations are unreadable
(581 + 147), chains (142 + 183, W44-INDIRECT-CHAIN-ACCESS), `sizeof (*v)`
(W44-COLLECTION-ALLOCATION), untyped members and bases, and casts. Typed reads
at custody-taking positions (`Released` 121, `Owned` 20 actuals) stay refused
by design.

## Forecast

* **Leaves removed: 144** `indirect memory access` leaves (the census's
  forecast set, listed in [census/census.json](census/census.json) under
  `forecast.leaves`): librdkafka 91, chunkio 15, cmetrics 15,
  monkey-libevent 13, fluent-bit-chunk 10, in 88 functions. Admitted actuals
  (distinct statements, 185): tail scalar 63, `Borrowed` pointer 60,
  `Borrowed` sub-object address 49, `Borrowed` scalar 42, tail pointer 8.
* **Exchanges: at most 117.** Of the 144, 117 read through a base the body
  may bind; each can come back as `UnknownAlias` at the same line when the
  base is possibly null, poisoned or unresolved there. The census cannot
  decide that; the AFTER attributes it leaf by leaf.
* **Functions: at most 1 Unknown → Safe**, `cio_file_hash_print`
  (`chunkio/cio_file.c`), the only function all of whose leaves are forecast
  leaves.
* **Units: 0 moves.**
* **Leak pin: 271, unchanged.** An admitted call no longer poisons the
  bindings outside the refused statement's frame (`POISON-UNMASK-ON-ADMISSION`),
  so a later release may be credited or refused differently; the forecast is
  that no leak leaf moves, and any move is attributed, not absorbed.
* **Violation candidates: 0.** No forecast statement reads through a base the
  census sees released.
* **`unsupported` falls by about 144**, `unknown_alias` rises by at most 117.

## What would falsify it

A removed leaf outside the forecast set that is not attributable to a
downstream effect (a new binding, an edge, a summary); an added
`use_after_release` or leak; a function or unit move other than the one named.
