# OPENSSL-PROVIDER-PLUGIN: prediction

Committed before any contract is written or bound (base `0f6e17118`; census
at `291d4f8fd`, [README](README.md) §2).

## User decisions (verbatim)

- 2026-11-16: "We need to implement OpenSSL related contracts should be built
  on top of quirin-provider-plugin. This should be the testing for the first
  time to depend on the plugin crate."
- 2026-11-16: "We need to pin the OpenSSL identity as per the requirements of
  OpenSSL version contracts. This should be tied the function contract with
  supported since."
- 2026-11-16: "We need to refer the official doxygen like documentation for
  OpenSSL to determine since each of the functions."
- 2026-11-16, answers to the card's stop (relayed): version range = "Contract
  clause (Recommended)"; held status = "Split (Recommended)"; load path =
  "User load path (Recommended)"; missing HISTORY = "Documented-present
  (Recommended)".

## What can move

The crate `providers/quirin-provider-openssl` is a providers-workspace member
loaded **only** through a generated user load path; it is never on
`providers/contract-load-path.json`. None of the three measured routes
(corpus, system, slice) nor the user's default `current` passes that load
path, and the core additions (the `resource_library_version` clause, the
version probe, the `plugin_checks` preservation fields) do nothing unless a
selected declaration carries the clause or a checker is bound. So every
measured route is **verdict-identical** to its base:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run (prose; the integrator's rerun)

Rerun `current` as the integrator ran it at 89fa31d21, with the contract
selection moved from the profile's manifest to the generated user load path
(`providers/user-crates-load-path.json`, written by this card) selecting the
same shipped modules plus `quirin-provider-openssl` (exact command in the
report's hand-back). At OpenSSL 3.5.5, every bound contract
whose range contains 3.5.5 is in force, and every function applying one
publishes the join of its own status with its binding's verdict. The checker
runs with no trusted certificate checker, so its `Safe` claim is `Pending`
(`plugin-claim-uncertified`). **No row can become `Safe`**: Unknown -> Pending
at most.

Of the six rows OpenSSL alone blocks:

| Row | Prediction | Why |
|---|---|---|
| `rdkafka_ssl.c::rd_kafka_transport_ssl_clear_error` | Unknown -> Pending | `ERR_clear_error` contracted; nothing else blocks it |
| `tls_openssl.c::tls_init_library` | Unknown -> Pending | `OPENSSL_init_ssl` contracted (`SSL_load_error_strings` expands to it) |
| `tls_openssl.c::tls_create_server_context` | Unknown -> Pending | `SSL_CTX_new` (a returned reference, handed back) and the nested `TLS_server_method` contracted |
| `rdkafka_ssl.c::rd_kafka_ssl_ctx_term` | stays Unknown | `ENGINE_free`/`SSL_CTX_free` are reference drops (OpenSSL counts these objects), and a drop through a member read is refused (`reference-drop-unheld`, rule 13b), never read as a final free |
| `rdkafka_ssl.c::rd_kafka_transport_ssl_close` | stays Unknown | `SSL_free` likewise, on `rktrans->rktrans_ssl` |
| `rdkafka_ssl.c::rd_kafka_ssl_PKEY_from_string` | stays Unknown | `PEM_read_bio_PrivateKey` takes a password callback; it stays uncontracted until a callback contract (CG22) states what the callback may do |

The 32 co-blocked rows keep other diagnostics and stay at their status (31
Unknown, 1 Unsafe), and their units keep theirs; the OpenSSL callees covered
leave the uncontracted list on those rows. Corpus, system and slice: no move.

## Correction after the evidence run (contracts bound)

The table above is unchanged as committed (`dbec50aae`). The user-path
evidence run ([README](README.md) §10) audited a fixture shaped like the six
rows through the real headers, the generated user load path and the real
checker: `clear_error`, `init_library` and an acquired-and-dropped context
are `Pending`, and the member-read drops are `Unknown` as predicted, but
`return SSL_CTX_new(TLS_server_method());` is `Unknown` (`nullable resource
... used by a non-null contract`: a nullable returned reference handed
straight to the caller's return is refused). So
`tls_openssl.c::tls_create_server_context` is expected to **stay Unknown**,
and the opt-in run moves at most **two** rows (`rd_kafka_transport_ssl_clear_error`,
`tls_init_library`) `Unknown -> Pending`. The measured routes' block is
unchanged: no move.
