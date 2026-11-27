# OPENSSL-FREE-INDIRECT-CALLBACKS: prediction

Committed before any declaration, checker, gateway test or Lean change. Base
`f916d3007` (integrate/r38, OPENSSL-REMAINING-HANDSHAKE-SETTERS, merged with
main `7c026b527`). The card is the named follow-up
`OPENSSL-FREE-INDIRECT-CALLBACKS`, recorded by
OPENSSL-FREE-SESSION-REMOVE-INVOCATION and appended by r38.

## What was read before this prediction

* **Pages.** `X509_LOOKUP_meth_new` at 3.5, 3.0 and 1.1.1. There is no page
  at 1.0.2. Also `SSL_CTX_free`, `SSL_free`, `SSL_CTX_set_cert_store`,
  `X509_STORE_new`, `EVP_PKEY_free`, `RSA_meth_new`, `ENGINE_add` and
  others at 3.5 (`private/pages/`, not committed).
* **Source.** OpenSSL 3.5.5 (`private/src/`, not committed):
  * `crypto/x509/x509_lu.c` (`X509_STORE_free`, `X509_LOOKUP_free`,
    `X509_LOOKUP_shutdown`);
  * the three built-in lookup methods (`by_file.c`, `by_dir.c`,
    `by_store.c`);
  * `ssl/ssl_lib.c` (`SSL_CTX_free`, `SSL_free`,
    `ossl_ssl_connection_free`), `ssl/ssl_cert.c` (`ssl_cert_free`);
  * `crypto/evp/p_lib.c`, `crypto/rsa/rsa_lib.c`, `crypto/x509/x_crl.c`,
    `crypto/async/async_wait.c` and `ssl/quic/quic_tls*.c`.

## The plan

**Slot registrations.** `X509_LOOKUP_meth_new` (3.5) names nine setters,
one slot each, holder the `X509_LOOKUP_METHOD`. They are declared in a new
module, `x509_lookup_slots.qv`:

* `new_item`, `free`, `init`, `shutdown`, `ctrl`;
* `get_by_subject`, `get_by_issuer_serial`, `get_by_fingerprint`,
  `get_by_alias`.

Every other name on the page is a not-writer, so the NAME check passes.

* **Keys.** The keys are `x509_lookup_method_st.lookup_meth_*`, a segment
  no Fluent Bit or Monkey source names. A bare `free`, `init` or `ctrl`
  segment would be a member pre-pass token. That is r38's
  MEMBER-PREPASS-UNREADABLE-TOKEN-UNIT, and it would be far worse here.
* **Range.** HISTORY says "added in OpenSSL 1.1.0i". The version grammar
  has no letter releases, and `since "1.1.0"` would admit 1.1.0 to 1.1.0h.
  So the range is `documented "1.1.1"`, the oldest series with a page.

**Stated invocations at the frees.** Every actual is `_` (the
`X509_LOOKUP`, which no formal names):

* `X509_STORE_free(xs)` invokes `x509_lookup_method_st.lookup_meth_shutdown`
  and `.lookup_meth_free` on `xs`. The last drop runs
  `X509_LOOKUP_shutdown` then `X509_LOOKUP_free` for each lookup.
* `SSL_CTX_free(ctx)` invokes both on `ctx`. Its last drop frees the
  context's store and its `CERT`'s verify and chain stores.
* `SSL_free(ssl)` invokes both on `ssl`. It frees the connection's `CERT`
  stores and may drop the context's last reference.

**The built-in lookup methods.** `X509_LOOKUP_file`, `X509_LOOKUP_hash_dir`
and `X509_LOOKUP_store` are library values, handed to no setter. Their free
functions release only their own method data. A new library-value premise,
`openssl_builtin_lookup_methods`, goes on the three frees, as
`openssl_builtin_bio_methods` does for BIOs.

**The twelve `X509_STORE_set_*` slots are not stated at the frees.**
`X509_STORE_free` runs none of them. The store's cleanup function is run by
`X509_STORE_CTX_cleanup`, that is, by a verification context, which the
handshake states (r38).

**What still stays under `openssl_unstated_callback_slots`** (the survey's
remainder, so the fail-closed rule applies). These are functions the frees
run on objects they free in turn, and no declaration states their setters:

* a key's provider key management free function (a dispatch table, with
  no setter at all);
* a key's `EVP_PKEY_ASN1_METHOD` free function;
* a key's `RSA_METHOD`, `DSA_METHOD`, `DH_METHOD` or `EC_KEY_METHOD` finish
  function;
* the finish and destroy functions of a key's ENGINE, and of the context's
  client certificate engine (`tls_engine_finish`);
* the free function of a CRL's `X509_CRL_METHOD`, for a CRL a store holds;
* the async wait context's fd cleanup functions (`SSL_free`);
* the QUIC TLS callbacks (`SSL_set_quic_tls_cbs`, called through
  `RECORD_LAYER_clear` in `SSL_free`);
* whatever a QUIC object's free runs further, which was not traced.

So `SSL_free` and `SSL_CTX_free` keep the premise, with a new statement
specific to the frees that names this remainder. It drops the ctrl arms, CT,
the undocumented handshake setters and ex_data. None of these run at a free
except ex_data, which `openssl_ex_data_callbacks` already covers.
`X509_STORE_free` read the same remainder silently (its certificates' keys,
its CRLs' methods), so it gains the premise with the same statement. The
handshake calls keep r38's statement unchanged. A follow-up is registered
for the remainder.

**Not in this card.** OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS (the
handshake's `get_by_subject`) stays registered. It is appended to say the
setters and the built-in premise now exist.

## Lock

The expected `contracts.lock` diff:

* 9 entries added;
* 3 changed digests: `X509_STORE_free`, `SSL_CTX_free`, `SSL_free`;
* every other entry unchanged. `pinned`, `library_versions` and every other
  top-level key unchanged.

## Rule and specification changes

* C04.18 gains a paragraph for this card.
* C04.19.3's opaque slot bullet gains the lookup method table as a holder,
  and the library-value premise.
* Rule 12n is amended in place. No new rule id is needed.
* Lean: a new module `QCore.OpensslFreeIndirectCallbacks` with its audit.
  * A premise is dropped from a free only when every callback it covered
    is stated.
  * The frees keep it while the remainder is nonempty.
  * Stating the lookup slots never improves a verdict.
  * A store slot that the free does not run need not be stated.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate, and no gateway
code changes.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current` run (the arbiter).** `current --monkey-linked` over
Fluent Bit 046accf6, base `f916d3007` against the tip. The base is measured
by this card, because the AG landings moved it.

* **No status move.** No row goes to `Safe`, none from `Safe` to
  non-`Safe`, and there is no new `Unsafe`.
* **Bindings** grow by 9, all `Pending`.
* **Run sets.** Both lookup slots are closed and empty. No Fluent Bit or
  Monkey source calls an `X509_LOOKUP_meth_set_*` setter (checked over the
  nearby Fluent Bit checkout).
* **Content.** No row changes content. The rows that call `SSL_free`,
  `SSL_CTX_free` or `X509_STORE_free` refuse first on something other than
  a slot decision, as at r36 to r38.
* **Rows most likely to prove this wrong.**
  * A free's row that now refuses `no declaration registers` would mean
    the module was not loaded.
  * A row whose refusal text changes would mean a key segment pulled in a
    unit, as r38's `cleanup` did.
