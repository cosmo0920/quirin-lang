# CALLBACK-SUPPLIED-NULL-ACTUAL: prediction

Committed before any loader, gateway, declaration, checker, test or Lean
change. Base `062974679` (main, r33). User decision 2026-11-19: launch the
named follow-up `CALLBACK-SUPPLIED-NULL-ACTUAL` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS).

## The plan the prediction is about

C04.19's loop-safe composition reads a candidate's `resource_owned(nonnull)`
at a `_` actual as met by what the callee supplies. Where the library's page
says that argument may be NULL, that reading is false.

* **The marker.** A new actual word, `supplied_nullable`: a value the callee
  supplies that may be a null pointer. It stands wherever `_` does (every
  invocation actual list) and is reserved as a parameter name, as `_`,
  `element` and `supplied_owned` are. It is `CallbackActual::SuppliedNullable`
  beside `Supplied` and `SuppliedOwned`, so a later supplied-output word
  (CALLBACK-SUPPLIED-OUTPUT-OBLIGATION) is one more variant beside it, not a
  flag on `_`.
* **The composition (C04.19 item 2, gateway rule 13u amended in place).** At
  a `supplied_nullable` position a candidate is admitted as at `_` only when
  it accepts NULL there: `resource_borrowed` or `resource_owned(nullable)`.
  `resource_owned(nonnull)` there refuses the call with the new key
  `callback-supplied-null`, naming the candidate. Every other mode is
  `callback-effectful`, as at `_`. The body check (rule 13y) and the
  forwarding check (rule 15w) read the word as they read `_`, and a forward
  matches it only against itself. A hand-off candidate reads it as `_`.
* **OpenSSL marks, page-cited.** Nullability is a property of a slot's
  argument, so it is one table in `documented.rs`, keyed by (slot, position)
  and citing the slot page's exact sentence at 3.5. Every invocation of that
  slot whose actual there is `_` renders `supplied_nullable`, and a parameter
  actual is unchanged. The checker refuses an entry whose sentence the
  page's snapshot does not record, and an entry no invocation uses. The
  marks are:
  * `ssl_ctx_st.sec_cb` (`SSL_CTX_set_security_level`). Position 1 (`ctx`)
    and position 0 (`s`): "One of the two is NULL". Position 5 (`other`):
    "It is NULL when no object is associated with the operation". Position
    6 (`ex`): "…, or NULL".
  * `ssl_ctx_st.psk_client_callback`, position 1 (`hint`): "Note that
    parameter hint given to the callback may be NULL".
    `ssl_ctx_st.psk_use_session_cb`, position 1 (`md`): "The first time the
    callback is called for a connection the md parameter is NULL".
  * `bio_st.bio_callback_ex` positions 2 (`argp`) and 7 (`processed`), and
    `bio_st.bio_callback` position 2 (`argp`), from `BIO_set_callback`'s
    table of the calls it makes, which passes NULL there.

  The security and BIO sentences are outside DESCRIPTION, which is all
  `null_sentences` records. So `fetch_docs.py` gains a mode that refetches a
  recorded page, refuses unless its SHA-256 is the recorded one, and records
  the NULL sentences of the page's other sections as
  `argument_null_sentences`.
* **Survey.** The other stated slots in the OpenSSL crate (r27, r30–r33)
  and every other crate's invocations are surveyed for page-documented NULL
  arguments at a `_` position. What has no page sentence is listed in the
  report and not marked. That includes the user `arg` pointers a setter
  keeps, which are NULL when unset, and librdkafka's `opaque`.
* **Lean.** A new `QCore.CallbackSuppliedNull` sits beside
  `QCore.CallbackContracts`. A `nonnull` requirement at a nullable actual
  is refused, and an admitted candidate accepts NULL at every nullable
  actual. Marking a position never admits more, and a list with no nullable
  actual classifies exactly as `CallbackContracts.classifyModes`. The Rust
  classify vectors are bound to the Lean ones.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate, and no pinned
module's text gains the word, so the composition reads every pinned
invocation exactly as before.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The OpenSSL lock.** The digests of every declaration stating a marked
invocation change: the handshake and I/O calls (`SSL_read`, `SSL_write`,
`SSL_connect`, `SSL_do_handshake`: security, PSK and BIO), `SSL_shutdown`
(BIO), `SSL_CTX_use_certificate` and `SSL_CTX_use_certificate_chain_file`
(security), and the three readers `PEM_read_bio_X509`,
`PEM_read_bio_PrivateKey` and `d2i_PKCS12_bio` (BIO). No entry is added or
removed, and `pinned` and `library_versions` do not change. The load path
stays byte-identical.

**The opt-in `current` run (this card's arbiter).** `quirin-audit current
--monkey-linked` over Fluent Bit 046accf6 with the user load path, main
`062974679` against the tip, with OPENSSL-SSL-CTX-CALLBACK-SLOTS' script.
The expected base is r33's: 1098 / 2 / 3699 / 5, with 113 bindings, all
`Pending`.

* **No status move and no content change.** Fluent Bit 046accf6 registers
  no security, PSK or BIO callback. r30–r33 observed `sec_cb` empty, and
  the source holds no `SSL_CTX_set_security_callback`,
  `SSL_set_security_callback`, PSK setter or `BIO_set_callback*` call. So
  no marked slot has a candidate, and a closed empty set invokes nothing
  whatever its actuals.
* The row most likely to prove this wrong is one that names the invocation
  text in a refusal (an SSL I/O or reader row whose detail prints the
  clause). That would be a content change with no status move.

**The plain `_` question (supplementary, not this card's change).** A third
run on the tip with one line changed reads every `_` as `supplied_nullable`,
the stricter default, to show what that default would move. The expectation
is a small number of `Safe` or `Pending` to `Unknown` moves: rows whose
synchronous or member invocation has a declared or summarised candidate
with `resource_owned(nonnull)` at a `_` position. Possible sources are
cfl/cmetrics comparators and librdkafka rk_conf candidates. The number is
reported as measured, and the default is not changed here. If the stricter
default looks right, that is a user decision.
