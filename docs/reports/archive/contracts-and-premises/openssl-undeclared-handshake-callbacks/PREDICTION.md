# OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: prediction

Committed before any declaration, checker, gateway test or Lean change (base
`65fc06fca`, main r30). User decision 2026-11-18: launch the named follow-up
`OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS).

## The plan the prediction is about

The 3.5 pages were read before this prediction (`private/docs-read/`, not
committed); the snapshots enter the record with `fetch_docs.py --merge`.

* **New opaque slots** (C04.19.3, rule 12n amended in place), one key per
  documented callback, shared by the context and the SSL's copy where the page
  names an SSL writer, each with every writer its 3.5 page names declared as
  its setter, and the page's NAME list exactly its writers and not-writers:
  * keylog `ssl_ctx_st.keylog_callback` (`SSL_CTX_set_keylog_callback`);
  * ALPN selection `ssl_ctx_st.alpn_select_cb` (`SSL_CTX_set_alpn_select_cb`)
    and, from the same page, the NPN slots `ssl_ctx_st.npn_select_cb`
    (`SSL_CTX_set_next_proto_select_cb`) and `ssl_ctx_st.npn_advertised_cb`
    (`SSL_CTX_set_next_protos_advertised_cb`), each setter retaining `arg`;
  * certificate verify `ssl_ctx_st.app_verify_callback`
    (`SSL_CTX_set_cert_verify_callback`, retaining `arg`);
  * PSK: `psk_client_callback`, `psk_use_session_cb`, `psk_server_callback`,
    `psk_find_session_cb`, each with its context and SSL setter (eight);
  * client hello `ssl_ctx_st.client_hello_cb` (retaining `arg`);
  * record padding `ssl_ctx_st.record_padding_cb` (context and SSL setter);
  * early data `ssl_ctx_st.allow_early_data_cb` (context and SSL setter,
    retaining `arg`);
  * custom extensions: six slots (`add`, `free`, `parse`, old and `_ex`
    style), each of the three setters registering three of them and retaining
    `add_arg` and `parse_arg`.

  21 new setter declarations. The checker gains a registration beside the
  shape (a setter registering several slots), the `unsigned int` spelling
  (`u32`) and the new function pointer typedefs.
* **New stated invocations.** `SSL_write`, `SSL_read`, `SSL_connect` and
  `SSL_do_handshake` invoke every new slot with the actuals the pages name;
  `SSL_shutdown` adds the record padding slot ("invoked for every record
  before encryption", type `SSL3_RT_ALERT` among them).
* **What stays under the premise.** `openssl_unstated_callback_slots` stays,
  with its statement unchanged, on the five declarations: the
  `SSL_CTX_callback_ctrl` arms (OPENSSL-CTRL-COMMAND-ARMS); the CT validation
  slot, which `SSL_CTX_enable_ct` / `SSL_enable_ct` write with the library's
  own callback through no function pointer formal, so no declaration can
  register what they write; other slots whose setters remain undeclared
  (session ticket, session id generation, DH parameter, cookie, X509_STORE
  verify callbacks); a BIO's methods and callbacks, which need their own
  reading (a method table is an object of its own, the built-in methods are
  the library's values, and `BIO_free` runs the BIO callback beside a
  reference drop); ex_data. Each becomes a named follow-up.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (the user load path
only), and no gateway code changes.

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
load path, base (base binary and tree `65fc06fca`) and tip, with
OPENSSL-SSL-CTX-CALLBACK-SLOTS' script (base expected: 1098 / 2 / 3699 / 5;
78 bindings, all `Pending`).

* **No status move, no row to `Safe`, no new `Unsafe`.** Bindings 78 to 99,
  all `Pending`.
* **Run sets.** `ssl_ctx_st.alpn_select_cb` =
  {`tls_context_server_alpn_select_callback`} (`src/tls/openssl.c`'s
  `tls_context_alpn_set` and `tls_context_reload`); `keylog_callback` =
  {`ssl_key_logger`} only if the production database defines `FLB_HAVE_DEV`
  (otherwise empty); every other new slot closed and empty (no setter of
  them is called in the compiled sources: the libevent sample and the
  nghttp2 example that call them are not compiled).
* **Content.** `tls_context_alpn_set` and `tls_context_reload` lose their
  uncontracted `SSL_CTX_set_alpn_select_cb` and gain its link and its
  retention of `ctx` by `ctx->ctx` (which may itself refuse,
  `retention-undecided`, an indirect holder); `tls_context_create` likewise
  for the keylog setter if compiled. Rows applying the I/O calls keep
  refusing first on the object operand, so their content does not change, or
  changes only in the refusal text at the same call.
* Rows most likely to prove this wrong: an I/O row reaching candidate
  classification on the ALPN slot (`callback-uncontracted` naming
  `tls_context_server_alpn_select_callback`), and a Monkey row calling one of
  the new setters.
