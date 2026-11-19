# OPENSSL-BIO-METHODS-AND-CALLBACKS: prediction

Committed before any declaration, checker, gateway test or Lean change (base
`f659b24f5`, main r31). User decision 2026-11-19: launch the named follow-up
`OPENSSL-BIO-METHODS-AND-CALLBACKS` (recorded by
OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS).

## The plan the prediction is about

The 3.5, 1.1.1 and 1.0.2 pages `BIO_set_callback` and `BIO_meth_new` were
read before this prediction (`private/docs/`, not committed; `BIO_meth_new`
has no 1.0.2 page). The snapshots enter the record with `fetch_docs.py
--merge`.

* **`BIO_free` first.** It runs the BIO's callback (`BIO_set_callback` 3.5,
  CALLBACK OPERATIONS: "callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL)
  ... is called before the free operation"), its method's destroy function
  (`BIO_meth_new` 3.5: "This function will be called in response to the
  application calling BIO_free()") and the BIO class's ex_data free
  callbacks, all beside its reference drop. Stating the callback invocation
  there needs C04.19 to admit a slot invocation beside a reference drop. That
  is not only a loader change: a candidate's requirement on the dropped
  formal is applied by the callback route after the reference route's drop
  in the transfer (`legacy_transfer.rs`), so the order would have to be
  rebuilt and modelled. And the destroy function and ex_data would still need
  a premise. So `BIO_free` gets a named premise, `openssl_bio_free_callbacks`,
  covering exactly those three; `SSL_free`, which frees the read and write
  BIOs, names it too. OPENSSL-FREE-SESSION-REMOVE-INVOCATION stays open.
* **The BIO callback slots** (C04.19.3, rule 12n amended in place), one key
  per callback format: `bio_st.callback_ex` (writer `BIO_set_callback_ex`)
  and `bio_st.callback` (writer `BIO_set_callback`), the page's NAME list
  exactly their writers and not-writers.
* **The method table slots**, one key per function `BIO_meth_new`'s page
  documents, each written by its one setter: `bio_method_st.bwrite`
  (`BIO_meth_set_write_ex`), `.bwrite_old` (`_write`), `.bread`
  (`_read_ex`), `.bread_old` (`_read`), `.bputs`, `.bgets`, `.ctrl`,
  `.create`, `.destroy`, `.callback_ctrl`, `.bsendmmsg`, `.brecvmmsg`. The
  key is run-wide, so the set is every function any table's setter is
  handed. That is a superset of what the SSL's BIOs (`BIO_new`,
  `SSL_set_bio`) can hold, apart from the built-in methods. `BIO_METHOD` has
  been opaque since 1.1.0, so no unit writes a table except through the
  setters, and a unit defining `struct bio_method_st` refuses the slot.
* **The built-ins**, a library-value premise `openssl_builtin_bio_methods`:
  the functions of a method the library defines (`BIO_s_socket`,
  `BIO_s_mem`, `BIO_f_buffer`, ...), which no setter is handed, take,
  release or retain nothing the caller holds.
* **14 new setter declarations**, a new module `bio_slots.qv`.
* **New stated invocations.** `SSL_write`, `SSL_read`, `SSL_connect`,
  `SSL_do_handshake` and `SSL_shutdown` invoke all 14 new slots, on `ssl`,
  with every actual `_` (the BIO, not the connection, is the callee's own
  value): 41 slot invocations on each I/O call and 17 on `SSL_shutdown`.
  Each also names `openssl_builtin_bio_methods`.
* **The narrowed premise.** `openssl_unstated_callback_slots` drops "a method
  or callback of a BIO the SSL object holds" from its statement. It stays on
  the same seven declarations for the ctrl arms, the CT slot, the remaining
  setters and ex_data.
* **Out of scope, and a finding.** `PEM_read_bio_X509`,
  `PEM_read_bio_PrivateKey` and `d2i_PKCS12_bio` read through the caller's
  BIO, so they run its callback and methods, and their declarations do not
  say so. A slot invocation cannot stand beside their literal-argument
  premise (rule 57), so this becomes a named follow-up.

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
load path, base (tree `f659b24f5`) and tip, with
OPENSSL-SSL-CTX-CALLBACK-SLOTS' script (base expected: 1098 / 2 / 3699 / 5;
99 bindings, all `Pending`).

* **No status move, no row to `Safe`, no new `Unsafe`.** Bindings 99 to 113,
  all `Pending`.
* **Run sets.** `bio_st.callback_ex` and `bio_st.callback` closed and empty:
  no compiled source calls the callback setters. The method slots are closed
  and empty unless Monkey's bundled libevent `bufferevent_openssl.c` is in
  the run. If it is, `bwrite_old`, `bread_old`, `bputs`, `ctrl`, `create` and
  `destroy` each hold its `bio_bufferevent_*` function.
* **Content.** Rows applying the I/O calls (`rdkafka_ssl.c`, Monkey's
  `tls_openssl.c`) keep refusing first on the object operand. Their content
  does not change, or changes only in the refusal text at the same call.
  Rows calling `BIO_free` (librdkafka's `rdkafka_ssl.c`, `rdkafka_cert.c`,
  `rdkafka_sasl_oauthbearer_oidc.c`) keep their status. Their content changes
  at most by naming the new premise.
* The rows most likely to prove this wrong:
  * an I/O row reaching candidate classification on a method slot
    (`callback-uncontracted`, naming a `bio_bufferevent_*` function);
  * a `BIO_free` row changing status because the premise changes how the
    declaration is routed.
