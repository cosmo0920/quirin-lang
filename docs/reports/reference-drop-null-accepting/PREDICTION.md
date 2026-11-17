# REFERENCE-DROP-NULL-ACCEPTING: prediction

Committed before any implementation (base `32336c33d`). The card adds a
`resource_reference_dropped(p, nullable)` form (contract specification
C04.17, grammar G05, gateway rule 13b) whose NULL edge is a no-op on the
family, and uses it on the OpenSSL user crate's six frees (`X509_free`,
`EVP_PKEY_free`, `SSL_free`, `SSL_CTX_free`, `BIO_free`, `ENGINE_free`).

## What can move on the pinned routes

* The only declarations that will carry the nullable form live in
  `providers/quirin-provider-openssl`, which is loaded only through the
  generated user load path (`providers/user-crates-load-path.json`); no
  pinned route (corpus, system, slice) passes it.
* The gateway change is confined to rule 13b's drop: a drop clause marked
  nullable reads a definitely-NULL actual as a no-op and a name bound to one
  live family or NULL as that family. A drop **not** marked nullable decides
  exactly as at base: the new binding classes (definitely NULL; one family or
  NULL) refuse a keep and a drop with the reason and text the `Other` class
  gives today. Every shipped reference declaration (librdkafka's keeps and
  drops) is non-nullable.
* The schema field is omitted from every serialized form when false, so every
  report, sidecar and lock without the form is byte-identical.

So every measured route is verdict-identical to its base:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run (prose)

`quirin-audit current --load-path providers/user-crates-load-path.json` at
Fluent Bit `046accf6` (the OpenSSL callers of OPENSSL-PROVIDER-PLUGIN's
census, `current` route). Read at that revision, every caller of the six
frees stays blocked by something the nullable form does not touch:

| Rows | What blocks them after the card |
|---|---|
| `rd_kafka_ssl_ctx_term`, `rd_kafka_transport_ssl_close`, Monkey `context_unset`, `contexts_free` | the drop's actual is a member read (`rk->rk_conf.ssl.ctx`, `rktrans->rktrans_ssl`, `ctx_head->ssl`, `cur->ssl`): a reference the function does not hold (`reference-drop-unheld`, `REFCOUNT-MEMBER-REFERENCE-DROP`), whatever its nullability |
| `rd_kafka_transport_ssl_verify`, `rd_kafka_ssl_X509_from_string`, `rd_kafka_ssl_PKEY_from_string`, `rd_kafka_ssl_read_cert_chain_from_BIO`, `rd_kafka_cert_new`, `rd_kafka_cert_destroy`, `rd_kafka_ssl_set_certs`, `rd_kafka_oidc_assertion_create`, `rd_kafka_ssl_ctx_init`, `rd_kafka_ssl_ctx_init_engine`, Monkey `tls_load_builtin_credentials` | the dropped object comes from an uncontracted OpenSSL producer (`SSL_get1_peer_certificate`, `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey`, `BIO_new_mem_buf`, `X509_new`, `ENGINE_by_id`, …) or a member, so the drop is of a reference the function does not hold; each row also carries other leaves |

Prediction for the opt-in run: **no status move**, and no leaf added or
removed on any row. Content changes are expected only where the declaration
text itself is recorded: the six modules' digests and each binding's checker
diagnostic, which names the declaration's range (`documented "1.1.1"`, and
`documented "3.0"` for `ENGINE_free`, from the series whose page states the
NULL behaviour). No row can become `Safe` through this card: a bound
OpenSSL contract still publishes at most `Pending` while no trusted
certificate checker exists, and the nullable form only stops refusing a drop
whose actual is NULL or the caller's own family.

What the form does admit is shown by the gateway's own controls, not by a
measured row: a caller that acquires a reference from a declared nullable
reference-returning call (`SSL_CTX_new`, `SSL_new`) and drops it without a
NULL test, and a drop of a definitely-NULL pointer.
