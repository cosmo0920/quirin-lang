# LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING

A fail-closed defect (rule 53, contract specification C06.2), launched by the
user on 2026-11-18. Registered by
[LIBCURL-PROVIDER-PLUGIN](../libcurl-provider-plugin/README.md) (section 6;
[comparison](../libcurl-provider-plugin/evidence/callers-comparison.json)).
Base `681fca01a`.

## 1. Prediction (committed before the measurement)

Written after the cause was localised and fixed (sections 2 and 3), before
any Fluent Bit run.

* **The opt-in libcurl run** (the registering card's `callers.sh` over
  librdkafka's two libcurl callers at Fluent Bit 046accf6, on this card's own
  preparation). With the base binary the defect reproduces: `with-default`
  and `with-full` publish `rd_http_global_init` `Pending` with the
  human-resolvable `contract-pin-mismatch` text. With the tip binary:
  `without` 6 Safe / 0 Pending / 39 Unknown / 4 Unsafe as before;
  `with-selected` (every libcurl declaration in force) `rd_http_global_init`
  `Pending` (the binding's `plugin-claim-uncertified`), 6 / 1 / 38 / 4;
  `with-default` and `with-full` (unreadable) `rd_http_global_init`
  `Unknown` with "call to `curl_global_init` whose contract is not in force
  at the audited library version (library-version-unreadable)", 6 / 0 / 39 /
  4. No other row moves on any side; no row reaches `Safe`; nothing moves
  into or out of `Unsafe`.
* **The pinned and live routes** (corpus, system, tail-pause, `pinned`): no
  row can move, by construction (section 4), so they are not measured.

```measure-prediction
route corpus
none
route system
none
route tail-pause
none
```
