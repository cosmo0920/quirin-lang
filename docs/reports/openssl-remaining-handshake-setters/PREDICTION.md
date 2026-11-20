# OPENSSL-REMAINING-HANDSHAKE-SETTERS: prediction

Committed before any declaration, checker, gateway test or Lean change (base
`64114a447`, main r37). The card is the named follow-up
`OPENSSL-REMAINING-HANDSHAKE-SETTERS`, recorded by
OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS.

## The plan the prediction is about

The 3.5 pages were read before this prediction (`private/docs-read/`, not
committed). The snapshots enter the record with `fetch_docs.py --merge`.

**New opaque slots** (C04.19.3, with rule 12n amended in place). There is one
key per documented callback. Every writer the slot's 3.5 page names is
declared as its setter, and each page's NAME list must be exactly its slots'
writers and not-writers.

* **Session tickets** (`SSL_CTX_set_session_ticket_cb`, one setter, two
  slots): `ssl_ctx_st.ticket_gen_cb` and `ssl_ctx_st.ticket_dec_cb`,
  retaining `arg`.
* **Session id generation**: `ssl_ctx_st.generate_session_id`, written by
  `SSL_CTX_set_generate_session_id` and `SSL_set_generate_session_id`.
* **Temporary DH**: `ssl_ctx_st.tmp_dh_callback`, written by
  `SSL_CTX_set_tmp_dh_callback` and `SSL_set_tmp_dh_callback`.
* **Cookies**: four context slots, one setter each. These are the stateless
  TLS 1.3 pair and the DTLS pair.
* **Session secret**: `ssl_st.session_secret_cb`, written by
  `SSL_set_session_secret_cb`, retaining `arg`. The callback's
  `peer_ciphers` is `supplied_nullable`, from the page's "On the client side
  the peer_ciphers stack will always be NULL".
* **SRP**: three context slots (username, client password, verify param),
  one setter each.
* **Async**: `ssl_ctx_st.async_cb`, written by `SSL_CTX_set_async_callback`
  and `SSL_set_async_callback`.
* **QUIC**: `ssl_ctx_st.new_pending_conn_cb`, written by
  `SSL_CTX_set_new_pending_conn_cb`, retaining `arg`.
* **The certificate store's callbacks**: twelve `x509_store_st.*` slots,
  one `X509_STORE_set_*` setter each. Per `X509_STORE_set_verify_cb_func`
  (3.5) they are verify_cb, verify, get_issuer, check_issued,
  check_revocation, get_crl, check_crl, cert_crl, check_policy,
  lookup_certs, lookup_crls and cleanup. The page's three macro aliases are
  not-writers, because a call to one is a call to its writer after
  preprocessing.

That is 28 new setter declarations in two new modules:
`ssl_handshake_setters.qv` (16) and `x509_store_slots.qv` (12).

**New stated invocations.** `SSL_write`, `SSL_read`, `SSL_connect` and
`SSL_do_handshake` invoke all 26 new slots: 41 invocations become 67.
`SSL_shutdown` adds the async slot, because an asynchronous SSL call may run
it. Its invocations go from 17 to 18.

**What stays under `openssl_unstated_callback_slots`** (on the same seven
declarations), each item with a follow-up:

* the `SSL_CTX_callback_ctrl` arms (OPENSSL-CTRL-COMMAND-ARMS);
* CT validation (OPENSSL-CT-VALIDATION-LIBRARY-WRITER);
* `SSL_CTX_set_not_resumable_session_callback`,
  `SSL_set_not_resumable_session_callback` and
  `SSL_set_session_ticket_ext_cb`, which have no OpenSSL manual page (a new
  follow-up);
* the functions of an `X509_LOOKUP_METHOD` of a lookup the store holds,
  coordinated with OPENSSL-FREE-INDIRECT-CALLBACKS, which declares their
  setters (a new follow-up for the handshake invocations);
* ex_data.

The premise's statement is rewritten to name these kinds. Its digest
therefore also changes on `SSL_free` and `SSL_CTX_free`. Those two keep the
premise, which is OPENSSL-FREE-INDIRECT-CALLBACKS' discharge.

## Lock

The expected `contracts.lock` diff (`quirin-audit pin`):

* 28 entries added (the two new modules);
* 7 changed digests:
  * `SSL_connect`, `SSL_do_handshake`, `SSL_read`, `SSL_write` and
    `SSL_shutdown` (new invocations and the new statement);
  * `SSL_free` and `SSL_CTX_free` (the statement only);
* every other entry unchanged, and `pinned`, `library_versions` and every
  other top-level key unchanged.

## Rule and specification changes

* C04.18: the OpenSSL premise paragraph gets this card's narrowing.
* C04.19.3: the opaque slot bullet gets three readings:
  * a holder reached through the context (the certificate store);
  * a macro alias as a not-writer;
  * a setter no page documents, which stays unstated.
* Rule 12n is amended in place. No new rule id is needed.
* Lean gets a new module `QCore.OpensslRemainingSetters` with its audit. It
  covers four points:
  * an undocumented writer leaves its slot unstated;
  * a macro alias adds no candidate;
  * a premise kind is dropped from the statement only when its slots are
    stated;
  * stating more invocations never improves a verdict.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (it is on the
user load path only), and no gateway code changes.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current` run (this card's arbiter).** It runs `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with the user load path,
base `64114a447` against the tip, using OPENSSL-SSL-CTX-CALLBACK-SLOTS'
script. The expected base is 1098 / 2 / 3699 / 5.

* **No status move**: no row goes to `Safe`, none goes from `Safe` to
  non-`Safe`, and there is no new `Unsafe`. Bindings grow by 28, all
  `Pending`.
* **Run sets.** Every new slot is closed and empty. None of the 28 setters
  is called in Fluent Bit's or Monkey's sources (searched over a nearby
  Fluent Bit checkout; the run's member facts confirm).
* **Content.** No row changes content. Every row of the selection that
  applies an I/O call refuses first on its object operand, as at r30 to
  r37. So no row reaches candidate classification on a new slot, and an
  empty closed set with a registering declaration decides nothing new.
* **Rows most likely to prove this wrong.** An I/O row that reaches slot
  decisions and now refuses `no declaration registers` would mean a module
  was not loaded. A Monkey row calling `X509_STORE_set_*` would also prove
  this wrong.
