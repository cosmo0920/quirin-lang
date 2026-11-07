# CURRENT-UNSAFE-TRIAGE: the current run's provisional Unsafe findings

Round 26 A (card-medium, records only). Every provisional Unsafe configuration
of the user's `quirin-audit current` run at Fluent Bit `046accf6` (report
`build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json`
in the main checkout), classified per witness from the bodies at that
revision. Prior records read: docs/reports/w44-leak-pin-audit/ (README.md,
classification.md, followups.json) and
docs/reports/cfl-mpack-external-declarations/README.md §4.1.

No analyzer, contract or Lean code changed; no audit ran. The
`QCORE_LEAK_WITNESS` census was not built, so no per-witness path was
reconstructed; where a cause is inferred from source and diagnostics, the
table says so. Every Unsafe below stays provisional. None is presented as an
upstream defect and nothing was filed upstream.

USER DECISION (2026-11-07, verbatim): "Register four as implementation cards"
(the four fix cards of §3).

## 1. How a finding reaches Unsafe

The gateway's custody fixed point emits `Leak` or `DoubleRelease`;
`classify_diagnostics` maps both to Unsafe (legacy_verdict.rs). Leaks are
reported at the settled exit state, so joined states give may-evidence, not
one execution (legacy_leak_witness.rs); W44-LEAK-PIN-AUDIT is the precedent
for classifying each leaf by source reading. The refused hand-on rule
withdraws a leak only when a refused statement names the site's bound place.

## 2. Witness table

| Unit / function | Count | Identity (acquired → reported) | Verdict |
|---|---:|---|---|
| cmt_decode_statsd.c `decode_statsd_message` | 1 double release | `metric_name` from `cfl_sds_create_len` (:393), released :417 and :490 | True positive (CFL-MPACK §4.1; the `decode_counter_entry` failure arm, no excluding guard found) |
| cmt_map.c `cmt_map_metric_get` | 1 | `calloc` :210 stored into `metric->hist_buckets` | Modelling defect (cause inferred: an unbound call result under a refused store) |
| rdhttp.c `unittest_http_get_params_append` | 1 | `rk = rd_calloc` :802 → :805 | True positive, unit-test assertion-failure path |
| rdkafka_sasl_oauthbearer_oidc.c (3 functions) | 28 | `assertion_create` 2 sites × 12 asserts (:1774/:1775 → :1777-:1798); `parse_from_file` :1589→:1592, :1599→:1600, :1603→:1605; `process_template_file` :1656→:1659 | True positives, one cause: a failing `RD_UT_ASSERT` returns over live allocations (W44 class (a)) |
| rdkafka_sasl_scram.c `rd_kafka_sasl_safe_string` | 1 | `d = safe = rd_malloc` :243 → `return safe` :249 | Modelling defect: CHAINED-ASSIGNMENT-INNER-DESTINATION (verified in source by the integrator) |
| snappy.c `rd_kafka_snappy_java_uncompress` | 7 | `outbuf = rd_malloc(uof)` :1705 → :1648, :1660, :1680, :1694, :1701, :1712, :1721 | Modelling defects: :1648/:1660/:1701 pass-2 re-checks of pass-1-accepted bytes (W44 rows 223-225, PATH-INSENSITIVE-SCALAR-CORRELATION); :1712 the `unlikely(!outbuf)` arm (W44 row 226); :1680/:1694/:1721 inferred from the pass-loop generation join (`loop re-acquisition` at :1705) |
| mk_http.c `mk_http_error` | 1 | `fd = open(...)` :1301 → `return MK_EXIT_OK` :1315 | True positive: the stream is set up from `sr->file_fd`, not `fd`; the source comments "This fd seems to be leaked" (verified by the integrator) |
| mk_net.c `mk_net_conn_create` | 1 | `conn = mk_mem_alloc` :247 → `return NULL` :320 | True positive: when `mk_net_fd_connect` does not return -1 the body falls through to `return NULL` without freeing `conn` or closing `fd` (verified by the integrator; W44 class (a)) |

Totals: 41 findings (40 leaks, 1 double release). True positives 32 in 5
units (statsd 1, rdhttp 1, oidc 28, mk_http 1, mk_net 1; 29 of them on unit-test
assertion-failure paths). Modelling defects 9 in 3 units (cmt_map 1, scram 1,
snappy 7). Fixing them moves those units Unsafe → Unknown, never to Safe.

## 3. Implementation cards (registered)

- CHAINED-ASSIGNMENT-INNER-DESTINATION (existing, amended): scram.
- LOOP-REACQUISITION-LEAK-ACCUSATION: snappy :1648, :1660, :1680, :1694,
  :1701, :1721.
- BUILTIN-EXPECT-NULL-TEST: snappy :1712.
- REFUSED-STORE-OF-CALL-RESULT-ESCAPE: cmt_map.

## 4. Open for the user

- Whether the 29 unit-test assertion-failure leaks stay provisional Unsafe or
  form their own class (a semantic policy choice; not decided here).
- Whether anything is taken upstream (mk_http_error, mk_net_conn_create,
  decode_statsd_message). The card's drafts are not upstream-defect claims and
  nothing is sent.
