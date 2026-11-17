# LIBCURL-PROVIDER-PLUGIN: prediction

Committed before any contract is written or bound (base `32336c33d`). The
pinned prototypes (`pinned/libcurl-8.18.0.json`) and the documentation record
(`doc-snapshots.json`) were generated first; neither is read by any route.

## User decision (verbatim, relayed in the card brief)

- 2026-11-17: libcurl's contracts move onto the user-contract provider SDK,
  exactly as card OPENSSL-PROVIDER-PLUGIN did for OpenSSL.

## What can move

The crate `providers/quirin-provider-libcurl` is a providers-workspace member
loaded **only** through the generated user load path
(`providers/user-crates-load-path.json`, rule 53); it is never on
`providers/contract-load-path.json`. None of the three measured routes
(corpus, system, slice) reads a user load path, and this card changes no
analyzer crate, so every measured route is **verdict-identical** to its base:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run (prose)

`quirin-audit current` over the two librdkafka units that call libcurl
(`rdhttp.c`, `rdkafka_sasl_oauthbearer_oidc.c`; card
W48-LIBCURL-CONTRACT-PLUGIN's selection) at Fluent Bit `046accf6`, with the
profile's default modules as a provider selection, once on the shipped load
path and once on the user load path with `quirin-provider-libcurl` added.
The six contracts are the W48 plugin's six, each now version-ranged
(`since` from the official AVAILABILITY sections) and bound to this crate's
checker; the checker's `Safe` is `Pending` (`plugin-claim-uncertified`), so
**no row can become `Safe`** and none moves into or out of `Unsafe`.

What a declaration's range does depends on where the probe reads
`curl/curlver.h` (rule 53; this host has no libcurl development headers
installed, so the routes' headers are the Ubuntu package extracted privately
and passed to CMake, as W48 did):

| Probe configuration | Prediction |
|---|---|
| the selected units' compile commands (`QCORE_LIBRARY_VERSION_COMPILE_DATABASE`, each passing the extracted include directory): 8.18.0, equal to the lock | **`rd_http_global_init` Unknown -> Pending** (its only blocker is `curl_global_init`); `rd_http_get`, `rd_http_get_params_append`, `rd_kafka_oidc_jwt_bearer_build_request_body` and the three OIDC refresh callbacks link a bound contract and stay `Unknown` on their other blockers (the typecheck stubs, `curl_url_set`/`_get`, `curl_easy_*` option calls). Exactly W48's movement. |
| Clang's default search path (no database; no `curl/curlver.h` on this host) | no move: every libcurl call is held `Unknown` (`library-version-unreadable`, `stale-evidence`) |
| the whole prepared compile database (configurations that pass libcurl no include directory) | no move: the probe cannot read `curl/curlver.h` in those configurations, so it is unreadable and every call is held `Unknown`. Recorded as a finding for LIBRARY-VERSION-PROBE-ROUTE-DATABASE, not changed here. |
