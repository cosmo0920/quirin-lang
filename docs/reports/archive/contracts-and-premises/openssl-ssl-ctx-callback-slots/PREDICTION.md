# OPENSSL-SSL-CTX-CALLBACK-SLOTS: prediction

Committed before any declaration, loader, gateway or checker change (base
`974434463`). User decision 2026-11-18: replace the named premise
`openssl_registered_callbacks` (C04.18, OPENSSL-PROVIDER-REMAINING-CALLEES)
with stated callback registrations and invocations.

## The plan the prediction is about

C04.19.3 already states a setter's registration into a member
(`resource_callback_member_registered(P, TAG.FIELD, F)`) and an invocation of
a member (`resource_callback_member_invoked(OBJ, TAG.FIELD, A…)`), decided
over the run's closed set of stores. It cannot decide an OpenSSL slot as
written: `SSL_CTX` and `SSL` are opaque (`typedef struct ssl_ctx_st SSL_CTX;`
in the public headers), no unit of a run defines the record, and rule 12n
never decides a member of a record the run does not define. Relaxing that
for every undefined record would be unsound: a run that does not compile
librdkafka would then decide `rd_kafka_s` members over the declared setters
alone, missing the library's own stores. So the card adds the smallest
marked variant:

* **An opaque slot** (C04.19.3, rule 12n amended in place):
  `resource_callback_slot_registered(P, TAG.FIELD, F)` and
  `resource_callback_slot_invoked(OBJ, TAG.FIELD, A…)`, the member forms'
  grammar and composition, for a record the library keeps opaque. Decided
  only over the run's facts, only when no unit of the run defines `struct
  TAG` (a unit that does refuses the call), only when a declaration
  registers the slot, over exactly the declared registrations' actuals
  (premise: every writer of the slot is declared, checked by the OpenSSL
  checker against the manual page's NAME list). Without the run's facts, or
  with an open or unregistered slot, the call refuses. Lean module for the
  decision, audited.
* **Declared:** `SSL_CTX_set_default_passwd_cb` (slot
  `ssl_ctx_st.default_passwd_cb`), `SSL_CTX_set_cert_cb` and
  `SSL_set_cert_cb` (slot `ssl_ctx_st.cert_cb`, the SSL's copy keyed with its
  context's: the union is every value either can hold; each also retains
  `arg`).
* **Premise replaced:** `SSL_CTX_use_PrivateKey_file` invokes the password
  slot and drops the premise. `SSL_CTX_use_certificate_chain_file` invokes
  the password slot; the security callback the SSL_CTX_set_security_level
  page says is consulted when a certificate is set has no declared setter,
  so it keeps a narrowed premise `openssl_unstated_callback_slots` (the
  slots it does not state). `SSL_write`, `SSL_read`, `SSL_connect`,
  `SSL_do_handshake` invoke the certificate slot and keep the narrowed
  premise for the slots whose setters are not declared (verify, info,
  message, security, session, client certificate; several are
  `SSL_CTX_ctrl` command arms, out of scope).
* **Silent readings made explicit:** `SSL_shutdown`, `SSL_free`,
  `SSL_CTX_free` name the narrowed premise (SSL_CTX_free's page names the
  session-remove callback); the frees keep their NULL-accepting drop.

## What can move

The crate `providers/quirin-provider-openssl` is loaded only through the
generated user load path. No pinned route (corpus, system, slice) loads it,
and the gateway change decides only the two new slot keywords, which no
shipped module states. So every pinned route is verdict-identical:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run (this card's arbiter)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the
user load path (OPENSSL-PROVIDER-REMAINING-CALLEES' script), two runs: base
(base binary, base load path) and tip (tip binary, tip load path). A base
binary cannot read the tip modules (it rejects the slot keywords), and the
tip binary changes nothing for the base modules, so no third run is needed.

* **No status move, no row to `Safe`, no new `Unsafe`.** Every row applying
  one of the six re-stated declarations is `Unknown` at base on other
  diagnostics (r22's tip run).
* **Content.** The run registers one candidate per slot: librdkafka's
  `SSL_CTX_set_cert_cb(ctx, rd_kafka_ssl_cert_callback, rk)` and
  `SSL_CTX_set_default_passwd_cb(ctx, rd_kafka_transport_ssl_passwd_cb)`
  (`rdkafka_ssl.c:1714, 1946`). Rows applying `SSL_write`, `SSL_read`,
  `SSL_connect` or `SSL_do_handshake` in a unit where
  `rd_kafka_ssl_cert_callback` has no contract in force (every unit but
  `rdkafka_ssl.c`: Fluent Bit's and Monkey's TLS, `rdkafka_transport.c`)
  gain a callback refusal at that call; likewise the `*_file` loaders
  outside the unit defining `rd_kafka_transport_ssl_passwd_cb`.
  `rd_kafka_ssl_ctx_init` loses its two uncontracted-setter diagnostics and
  gains a retention refusal for `rk` kept by the context.
* Rows most likely to prove this wrong: a row in `rdkafka_ssl.c` whose only
  remaining refusals were the uncontracted setters, if the callbacks'
  in-unit summaries compose as the identity (none identified in advance).
