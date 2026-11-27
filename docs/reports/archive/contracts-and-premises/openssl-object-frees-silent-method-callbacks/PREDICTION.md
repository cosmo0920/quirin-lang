# OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: prediction

Committed before any declaration, checker, test or Lean change. Base
`ecc03d7a4` (integrate/r39: main `ffb41c748` plus
OPENSSL-FREE-INDIRECT-CALLBACKS with its amendments). The card is the named
follow-up recorded by OPENSSL-FREE-INDIRECT-CALLBACKS.

## What was read before this prediction

The OpenSSL 3.5.5 release tarball (`openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`,
unpacked in `private/src/`, not committed), its sources and its 3.5 manual
sources (`doc/man3`, `doc/man7`):

* `crypto/evp/p_lib.c` (`EVP_PKEY_free`, `evp_pkey_free_it`,
  `evp_pkey_free_legacy`), `crypto/evp/keymgmt_meth.c`
  (`EVP_KEYMGMT_free`), `crypto/provider_core.c` (`ossl_provider_free`);
* `crypto/rsa/rsa_lib.c`, `crypto/dsa/dsa_lib.c`, `crypto/dh/dh_lib.c`,
  `crypto/ec/ec_key.c` (each `*_free`: method finish, `ENGINE_finish`);
* `crypto/x509/x_x509.c` (`x509_cb`), `crypto/x509/x_pubkey.c`
  (`x509_pubkey_ex_free`, the eager decode of the public key),
  `crypto/x509/x_crl.c` (`crl_cb`, `X509_CRL_set_default_method`),
  `crypto/pkcs7/pk7_asn1.c` (signer and recipient info free callbacks);
* `crypto/engine/eng_lib.c` (`engine_free_util`), `tb_pkmeth.c`,
  `tb_asnmth.c`;
* `crypto/evp/digest.c` (`EVP_MD_CTX_free`, `evp_md_ctx_clear_digest`),
  `crypto/evp/pmeth_lib.c` (`EVP_PKEY_CTX_free`).

## The plan

**The four named frees** each gain `openssl_unstated_callback_slots`.
`UNSTATED_FREE_STATEMENT` (r39) does not fit any of them: it names the
client certificate engine of an `SSL_CTX`, the async wait context and QUIC
callbacks of an SSL object, none of which these frees reach. So each object
kind gets its own statement naming only what its free can run:

* **`EVP_PKEY_free`** (a key statement): the key's provider key management
  free function, and that provider's teardown function when the key holds
  its last reference; the key's `EVP_PKEY_ASN1_METHOD` free; the finish
  function of its `RSA_METHOD`, `DSA_METHOD`, `DH_METHOD` or
  `EC_KEY_METHOD`; the finish and destroy functions of an ENGINE of the key.
* **`X509_free`** (a certificate statement): the same functions, of the
  certificate's public key.
* **`PKCS12_free`** (a PKCS#12 statement): the same functions, of a key it
  frees in turn (a certificate's public key in a signed PKCS#7 safe, a
  signer info's key, a recipient info's certificate's key), and the
  `X509_CRL_METHOD` free function of a CRL a signed safe holds.
* **`ENGINE_free`** (an engine statement): the engine's destroy function and
  its public key method and ASN.1 method callbacks (`ENGINE_set_pkey_meths`,
  `ENGINE_set_pkey_asn1_meths`), which `engine_free_util` calls to free
  their dynamic methods. The free is structural, so no finish function runs.

**The survey of the other declared frees.** The crate declares ten frees:

* `SSL_free`, `SSL_CTX_free`, `X509_STORE_free` already name the premise
  (r39's statement);
* `BIO_free`: whatever a built-in BIO's destroy function frees in turn (the
  SSL object of a `BIO_f_ssl` BIO, a digest or cipher context) is named by
  `openssl_builtin_bio_methods` ("any callback it runs in turn on an object
  other than the arguments of the call");
* `OPENSSL_sk_free` frees the stack only;
* **`EVP_MD_CTX_free` has the same silent gap.** It runs the digest's
  cleanup function (`EVP_MD_meth_set_cleanup`) or its provider's freectx,
  the digest's ENGINE finish, and frees the context's `EVP_PKEY_CTX`, which
  runs its `EVP_PKEY_METHOD` cleanup (`EVP_PKEY_meth_set_cleanup`), its
  provider operation's freectx, its ENGINE finish, and may free its keys.
  It gains the premise with a digest context statement of its own.

`X509_CRL_free`, `X509_STORE_CTX_free`, `SSL_SESSION_free` and
`BIO_free_all` are not declared in this crate.

**The checker's expectation** is a free audit beside `EX_DATA_AUDIT`: every
declaration whose documented shape is a release is listed, as running such
functions (it must name the premise in exactly the audited statement), as
covered by another named premise (it must name that premise), or as freeing
nothing whose functions a user can set (it must not name a free statement).
A free left out of the audit, or one dropping or changing its statement,
fails the checker.

**Not in this card:** stating these slots individually
(OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS, which needs a user decision),
and the calls that are not frees but free keys or certificates inside (the
`SSL_CTX_use_*` loaders, the PEM and PKCS#12 readers on failure, the
handshake calls). Those are recorded as a follow-up.

## Lock

The expected `contracts.lock` diff: 5 changed digests (`X509_free`,
`EVP_PKEY_free`, `PKCS12_free`, `ENGINE_free`, `EVP_MD_CTX_free`), no entry
added or removed. `pinned`, `library_versions` and every other top-level
key unchanged except `flexible`, as in r39.

## Rule, specification and Lean

* C04.18 gains a paragraph; rule 12n is amended in place. No new rule id.
* Lean: a new module for the free audit (an unaudited free refuses, a
  running free names its audited statement, a premise stands only where
  audited, the premise moves no verdict), with its audit.

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
Fluent Bit 046accf6, base `ecc03d7a4` against the tip, both measured here.

* **No status move.** No row goes to `Safe`, none from `Safe` to
  non-`Safe`, and there is no new `Unsafe`. Expected 1139 Safe / 2 Pending /
  3656 Unknown / 7 Unsafe on both sides (r39's tip, whose crates, providers
  and contracts equal this base's).
* **Bindings** unchanged at 150, all `Pending`.
* **Content.** No row changes content: a premise is metadata, and no new
  key segment is introduced.
* **Rows most likely to prove this wrong.** A row naming `EVP_PKEY_free`,
  `X509_free` or `EVP_MD_CTX_free` whose refusal text changes, or a binding
  of one of the five frees that turns `Unknown` (the checker refusing its
  own declaration).
