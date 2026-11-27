# Prediction — W47-SUMMARY-VALIDATION-PROTECTED-FORMALS

Frozen before the refusing build is measured. Base `2d7970a4`; the BEFORE run
reproduces the round-2b baseline (corpus diagnostics byte-identical; system
verdicts and leaves identical, only contract-source digests differ). The
extent comes from the record-only build `adda51b8`, whose verdict-bearing
artifacts are byte-identical to the baseline (corpus) and verdict/leaf
identical (system). It re-checks every W4 candidate on the CFG built with the
candidate as the function's own contract and records the decision without
refusing (`scripts/census.py`).

## Step 1: the extent

**Corpus.** 180 units derive 1,166 summaries. 80 candidates are undeclared
and not all-`Borrowed` (the only ones whose premises can differ), all on the
live-entry route; none is removed-wrapper, derived-result or contextual. The
reported-premise CFG differs for 9. For 8 the candidate stops validating;
the 9th (`mk_config_sanity_check`, protecting `server`) still validates and
its row is Safe.

The summaries that validate while their function's reported row is non-Safe
are exactly these 8, each Unknown. In every case the two runs disagree on the
protected set, which is the candidate's non-`Borrowed` formals, and through
it on every premise that set seeds:

| Unit | Function | Candidate | Formals the reported run protects |
|---|---|---|---|
| `rdkafka_partition.c` | `rd_kafka_topic_partition_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_partition.c` | `rd_kafka_topic_partition_not_regex` | Owned, Borrowed | `elem` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_broker_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_broker_internal_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_metadata.c` | `rd_kafka_metadata_partition_internal_cmp` | Owned, Owned | `_a`, `_b` |
| `rdkafka_sasl_oauthbearer.c` | `check_oauthbearer_extension_value` | Owned, Borrowed, Borrowed | `value` |
| `rdkafka_cert.c` | `rd_kafka_conf_ssl_passwd_cb` | Borrowed ×3, Owned | `userdata` |
| `rdkafka_conf.c` | `rd_kafka_anyconf_is_modified` | Owned, Borrowed | `conf` |

The premises are the lexical never-bound closure (rule 10o), `may_bind` and
the declared locals, and the AST closure of rule 10l. Every other summary
whose function has no reported row is a helper that rule 4a admits for
derivation only. Of those, 479 are all-`Borrowed` and 18 are not, and each of
the 18 validated on the reported premises.

**Safe functions resting on a withdrawn summary**, transitively through the
unit's direct-call graph: **one**, `rd_kafka_topic_partition_cmp_opaque`
(`rdkafka_partition.c`). The other 20 functions reaching a withdrawn summary
are already Unknown (3 in `rdkafka_sasl_oauthbearer.c`, 17 in
`rdkafka_conf.c`).

**System** (`046accf6`): 15 derivations. There are 4 candidate decisions, which
are 2 functions in 2 configurations: `schedule_request_wait` (its CFG
changes and it still validates) and `flb_engine_exit_status` (its CFG is
unchanged). Nothing is withdrawn, and no summary validates while its row is
non-Safe.

## Per-function forecast (refusing build)

1. `rd_kafka_topic_partition_cmp_opaque`: Safe → Unknown, with one leaf,
   "call to uncontracted function `rd_kafka_topic_partition_cmp`". This is a
   **correction**: the Safe rested on a summary whose own premises refused.
2. The 8 withdrawn functions stay Unknown. Their rows become the contract-free
   analysis on the unprotected CFG, so leaves are exchanged: refusals that
   came from protection disappear (for example `rd_kafka_topic_partition_cmp`'s
   return-family leaf), and `unknown_alias` leaves on the formals and their
   copies stay or appear.
3. Direct callers of the 8 that are already Unknown stay Unknown. They may gain
   "call to uncontracted function" leaves where the call was modelled through
   the summary.
4. No other function moves. There is no Safe → non-Safe move beyond (1), and no
   non-Safe → Safe move. Functions go 527 / 10 / 4,297 → **526 / 10 / 4,298**.
   Units stay 6 / 8 / 167 / 4.
5. No leak, double release, use after release or provisional Unsafe is added:
   withdrawing a summary only leaves a call uncontracted. leak stays 36.
   `unknown_alias` and `unsupported` move only at the rows of (1)–(3).
6. System: no move. Verdicts and leaves are identical to the baseline.
7. The parity stream is identical to the baseline except for the census rows
   and the rows of the 8 withdrawn functions, whose reported CFG is now built
   without their own contract. The re-check's rebuild writes no parity rows.
