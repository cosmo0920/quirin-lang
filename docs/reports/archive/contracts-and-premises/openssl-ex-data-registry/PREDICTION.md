# OPENSSL-EX-DATA-REGISTRY: prediction

Committed before any declaration, checker, test or Lean change. Base
`062974679` (main, r33). User decision 2026-11-19: launch the named
follow-up `OPENSSL-EX-DATA-REGISTRY` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS).

## The plan the prediction is about

`CRYPTO_get_ex_new_index` (3.5): "When a structure is initially allocated
(such as RSA_new()) then the new_func() is called for every defined index";
"When a structure is free'd (such as SSL_CTX_free()) then the free_func() is
called for every defined index"; `dup_func` runs when an `SSL`,
`SSL_SESSION`, `EC_KEY` or BIO chain is copied. The ex_data classes are BIO,
DH, DSA, EC_KEY, ENGINE, EVP_PKEY, RSA, SSL, SSL_CTX, SSL_SESSION, UI,
UI_METHOD, X509, X509_STORE and X509_STORE_CTX.

**Priority 1 (this card lands it).** Every declaration of
`quirin-provider-openssl` that allocates, copies or finally frees an object
of one of those classes names the premise `openssl_ex_data_callbacks`. That
includes a replaced or temporary object the call frees inside. The audit,
from the 3.5 pages, and from the 3.5 sources where a page is silent:

* **Constructors:** `SSL_CTX_new` (an `SSL_CTX` and its `X509_STORE`),
  `SSL_new` (an `SSL`, and the memory BIOs of its record layers), `X509_new`,
  `BIO_new_file`, `ENGINE_by_id` (a dynamic engine), the PEM readers (an
  `X509` or `EVP_PKEY`) and `d2i_PKCS12_bio` (a signed PKCS#7 safe decodes
  its certificates).
* **Frees of a last reference:** `SSL_free`, `SSL_CTX_free`, `BIO_free`,
  `X509_free`, `X509_STORE_free`, `EVP_PKEY_free`, `ENGINE_free`,
  `PKCS12_free`, and `EVP_MD_CTX_free` (its `EVP_PKEY_CTX` may hold the last
  reference to a key).
* **Calls that create or free inside:**
  * the handshake and I/O calls and `SSL_shutdown` (sessions, peer
    certificates and keys);
  * `SSL_set_accept_state`, which resets the record layer and so frees and
    creates memory BIOs (`RECORD_LAYER_reset`, 3.5 sources);
  * `SSL_CTX_use_certificate` and `SSL_CTX_use_PrivateKey` (the replaced
    certificate or key, and the other one when the two do not match);
  * `SSL_CTX_use_PrivateKey_file` (a temporary key);
  * `SSL_CTX_load_verify_locations` and `SSL_CTX_set_default_verify_paths`
    (certificates read from a file, freed when already in the store);
  * `OPENSSL_init_ssl` (the configuration it loads by default may create
    engines).
* `SSL_CTX_use_certificate_chain_file` names the premise already.

Eight declarations have an `openssl_unstated_callback_slots` or
`openssl_bio_free_callbacks` statement that mentions ex_data. They keep that
premise and name the ex_data one too.

The checker gains an audit table:

* a declaration whose documented shape creates or frees an object carries
  the premise, or is listed as touching no ex_data class (`EVP_MD_CTX_new`,
  `OPENSSL_sk_new_null`, `OPENSSL_sk_free`, and `SSL_get1_peer_certificate`,
  an up-reference);
* exactly the audited functions carry the premise.

Lean gets a model of the audit check.

**Priority 2 (the registry reading): attempted only if sound, predicted
deferred.** A run-wide set keyed by class needs three things this vocabulary
lacks:

* a registration whose key is the `class_index` literal;
* an invocation with no object parameter, since the constructors take none;
* an invocation beside the frees' reference drops, which C04.19 does not
  admit today.

If it is deferred, a narrower follow-up is registered.

## What can move

A named premise is metadata (C04.18): no consumer that decides a verdict
reads it. Adding one changes each declaration's digest, and the regenerated
pin lock carries the new digests.

**Pinned routes.** No pinned route loads the OpenSSL crate.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The user path on the predecessor's fixture

This is the `contracts` step over
`docs/reports/openssl-provider-remaining-callees/evidence/fixture`. No
function and no binding changes status: `read_cert` and `pkcs12_cycle` stay
`Pending`, `read_cert_reuse` stays `Unknown`, and the 113 bindings stay
`Pending`.

## The opt-in `current` run (this card's arbiter)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the user
load path, base `062974679` against tip, using
OPENSSL-SSL-CTX-CALLBACK-SLOTS' script. The expected base is r33's tip:
1098 / 2 / 3699 / 5, with 113 bindings, all `Pending`.

* **No status move.**
  * No row moves to `Safe`.
  * No row moves away from `Safe` or `Pending`.
  * There is no new `Unsafe`.
  * The bindings stay at 113, all `Pending`.
* **No content change.** r33 changed eight digests with no content change.
  The comparison's row content (status, links, diagnostics, the declarations
  a row rests on) does not carry premises. If a row does change, it will be
  one applying a newly premised declaration whose report lists the premises
  it rests on, and the report will attribute it.

**The lock diff** is predicted as 29 changed `contract_sha256` and
`pinned_at` values, one per newly premised declaration.
`SSL_CTX_use_certificate_chain_file` stays unchanged, and no entry is added
or removed.
