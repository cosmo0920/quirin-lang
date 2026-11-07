# REFUSED-STORE-OF-CALL-RESULT-ESCAPE prediction

Committed after the census and before anything else in the tree changes.
Measurement side: `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7` (main
checkout `private/baselines/7b726587…/`: `eight-home` corpus,
`fluent-bit-system` system, `fluent-bit-tail-pause` slice).

**What changes: nothing semantic.** The census (README §1) refutes the
hypothesis. `cmt_map_metric_get`'s leak is CMT-GETTER-OWNERSHIP-CLOSURE's
source-validated counterexample, not a refused-store projection leak, so no
rule is implemented. The card's commits touch
`docs/reports/refused-store-of-call-result-escape/` only. The tip binary is
built from code byte-identical to the base.

**Affected rows at the baseline: none.** The leak leaves the rule would
have read, relisted at `baseline:7b726587b`:

* **corpus**: 41 leak leaves in 9 functions.
  * `cmt_map_metric_get` 1 (the getter counterexample, line 5639 = its
    acquisition line).
  * `ctr_random_get` 1.
  * `unittest_http_get_params_append` 1.
  * The three OIDC unit tests 28.
  * `rd_kafka_sasl_safe_string` 1.
  * `rd_kafka_snappy_java_uncompress` 7.
  * `mk_http_error` 1.
  * `mk_net_conn_create` 1.

  Every acquisition except the getter's is bound to a name (`rk = …`,
  `outbuf = …`, `d = safe = …`, `fd = …`, `conn = …`). None is an
  acquisition stored through a refused destination.
* **system**: 0 leak leaves (13 Unknown / 2 Unsafe units, neither on a leak).
* **slice**: 0 leak leaves.

`cmt_map_metric_get` stays provisional Unsafe and `cmt_map.c` stays Unsafe.
This is the refutation's consequence, not a missed acceptance; see README §3.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing:

* No callee leaves the uncontracted list.
* No function leaves the "only uncontracted" set.
* No unit changes status against the same run of the base binary.
  `cmt_map.c` stays provisional Unsafe on its one getter leak
  (`cmt_map.c:210`). The user's 13337d0af-era run read 10 Safe / 0 Pending /
  8 provisional Unsafe / 164 Unknown / 4 data-only; this card changes none of
  those counts.

Any difference is a measurement finding, not an effect of this card.
