# OPENSSL-UNSTATED-CALLBACK-SLOTS: prediction

Committed before any declaration, checker, gateway or Lean change (base
`bd6b63ead`). User decision 2026-11-18: launch the named follow-up
`OPENSSL-UNSTATED-CALLBACK-SLOTS` (recorded by OPENSSL-SSL-CTX-CALLBACK-SLOTS),
and with it `RETENTION-NULL-CONSTANT-ACTUAL`.

## The plan the prediction is about

* **New slot setters** (C04.19.3's opaque slot, rule 12n; every writer each
  slot's 3.5 manual page names): `SSL_CTX_set_verify` / `SSL_set_verify`
  (`ssl_ctx_st.verify_callback`), `SSL_CTX_set_info_callback` /
  `SSL_set_info_callback` (`ssl_ctx_st.info_callback`),
  `SSL_CTX_set_msg_callback` / `SSL_set_msg_callback`
  (`ssl_ctx_st.msg_callback`), `SSL_CTX_set_security_callback` /
  `SSL_set_security_callback` (`ssl_ctx_st.sec_cb`),
  `SSL_CTX_sess_set_new_cb`, `SSL_CTX_sess_set_remove_cb`,
  `SSL_CTX_sess_set_get_cb` (three session slots) and
  `SSL_CTX_set_client_cert_cb` (`ssl_ctx_st.client_cert_cb`).
* **New stated invocations.** `SSL_write`, `SSL_read`, `SSL_connect` and
  `SSL_do_handshake` invoke every one of those slots beside the certificate
  slot; `SSL_shutdown` the info and message slots;
  `SSL_CTX_use_certificate` and `SSL_CTX_use_certificate_chain_file` the
  security slot (the context's check: `s` NULL, `ctx` the context, `other`
  the certificate).
* **The premise.** `openssl_unstated_callback_slots` is dropped where every
  slot the call may run is then stated: `SSL_CTX_use_certificate_chain_file`,
  which keeps only the ex_data registry (`CRYPTO_get_ex_new_index`, a global
  registry, not a slot) under a narrower premise. It stays, narrowed, on the
  handshake and I/O calls and `SSL_shutdown` (the `SSL_CTX_callback_ctrl` arms
  and other undeclared slots, e.g. Fluent Bit's keylog and ALPN callbacks) and
  on `SSL_free` / `SSL_CTX_free` (the vocabulary admits no callback clause
  beside their reference drop, so the session-remove invocation cannot be
  stated there).
* **The null-constant retention reading** (gateway rule 12t, amended in
  place): a null pointer constant at a retained position (after the cast
  reading, `(void *) 0`) retains nothing, so the call decides with no record;
  every other actual (a local, a formal, a call result that may be null) is
  decided as before.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (the user load path
only). The retention change is gateway-wide, but it only decides calls that
pass a null constant at a retained position of a shipped declaration
(`mk_event_add` and its siblings, `pthread_setspecific`, `msgpack_packer_init`,
the librdkafka retentions, `cJSON_ParseWithLengthOpts`,
`append_to_ring_buffer`); a search of the corpus's sources (Fluent Bit's
`lib/librdkafka-2.15.0/src`, `lib/monkey`, chunkio, cmetrics, ctraces) finds
none, and the system selection does not include the one Fluent Bit file that
does (`out_stackdriver/stackdriver.c`'s `pthread_setspecific(key, NULL)`).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run (this card's arbiter)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the user
load path, base (base binary, base load path) and tip, exactly as
OPENSSL-SSL-CTX-CALLBACK-SLOTS ran it (base: 1098 / 2 / 3699 / 5; 66
bindings, all `Pending`).

* **No status move, no row to `Safe`, no new `Unsafe`.** Every row applying a
  re-stated declaration is `Unknown` at base on other diagnostics (many on the
  object operand, an indirect `session->ssl`), and every row calling a new
  setter also calls an uncontracted `SSL_CTX_ctrl` macro or another
  uncontracted callee.
* **Run sets.** `ssl_ctx_st.verify_callback` =
  {`rd_kafka_transport_ssl_cert_verify_cb`} (`rdkafka_ssl.c`'s conditional
  actual; both arms read, the null one adds nothing; Fluent Bit and Monkey
  pass NULL); `ssl_ctx_st.info_callback` = {`tls_info_callback`} (Fluent
  Bit's `tls_session_create`); the message, security, session and client
  certificate slots closed and empty.
* **Content.** Rows calling the new setters (Fluent Bit's
  `tls_context_set_verify_client`, `tls_context_create`,
  `tls_session_create`; Monkey's context setup; `rd_kafka_ssl_ctx_init`) lose
  their uncontracted-setter diagnostics and gain the bindings' links. Rows
  applying the I/O calls where the candidates are reached gain
  `callback-uncontracted` refusals for `tls_info_callback` (internal to
  `src/tls/openssl.c`) or `rd_kafka_transport_ssl_cert_verify_cb` outside
  their units. `SSL_CTX_use_certificate` rows (`rdkafka_ssl.c`, Monkey's
  `tls_openssl.c`) move from the function map to the callback route (their
  refusal text may change at the same call, as the I/O rows' did). Fluent
  Bit's `oauth2_cache_cleanup` (`out_stackdriver`) loses its three
  `retention-undecided` refusals for `pthread_setspecific(key, NULL)` and
  stays `Unknown` on its other calls.
* Rows most likely to prove this wrong: a row whose only refusals were a
  null-constant retention (none identified), and a Monkey row whose I/O call
  reaches candidate classification and now refuses on the info or verify
  slot.
