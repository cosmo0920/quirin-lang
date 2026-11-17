# OPENSSL-PROVIDER-REMAINING-CALLEES: prediction

Committed before any declaration is written or bound (base `32336c33d`). The
census is OPENSSL-PROVIDER-PLUGIN's
([census](../openssl-provider-plugin/census/census.md)): 61 OpenSSL callees
that its twelve contracts leave uncontracted (corpus and `current`
together), 38 blocked rows on `current`, 6 of them sole.

## What can move

The crate `providers/quirin-provider-openssl` is loaded only through the
generated user load path `providers/user-crates-load-path.json`. No pinned
route (corpus, system, slice) and no configured route passes a user load
path, and this card changes no code outside the crate, its checker and its
scripts. So every measured route is verdict-identical to its base:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The plan the prediction is about

Contract the census's remaining callees that the existing vocabulary states
exactly from the official manual pages and the pinned 3.5.5 headers; leave
the rest uncontracted with a named reason:

- **Contracted (45).** Plain (no custody): `ERR_peek_error`,
  `ERR_peek_last_error`, `ERR_error_string_n`, `EVP_sha256`, `EVP_sha512`,
  `EVP_EncodeBlock`, `SSL_write`, `SSL_read`, `SSL_connect`,
  `SSL_do_handshake`, `SSL_get_error`, `SSL_set_accept_state`,
  `SSL_CTX_set_default_verify_paths`, `SSL_CTX_load_verify_locations`,
  `SSL_CTX_set_options`, `SSL_CTX_use_PrivateKey_file`,
  `SSL_CTX_use_certificate_chain_file`, `SSL_CTX_use_certificate` and
  `SSL_CTX_use_PrivateKey` (the documentation says the context takes its own
  reference), `X509_STORE_CTX_get_error`, `X509_STORE_CTX_get_error_depth`,
  `X509_STORE_CTX_set_error`, `X509_STORE_set_flags`. Borrowed results
  (`resource_return_borrowed_from`): `SSL_CTX_get_cert_store`,
  `SSL_get_certificate`, `SSL_get_client_CA_list`,
  `X509_STORE_CTX_get_current_cert`, `X509_STORE_get0_objects`,
  `X509_get_issuer_name`, `OPENSSL_sk_value`. Returned references
  (reference-counted objects): `X509_new`, `BIO_new_file`, `ENGINE_by_id`,
  `SSL_get1_peer_certificate`, and `PEM_read_bio_X509` /
  `PEM_read_bio_PrivateKey` (bounded to a NULL `x` by the literal-argument
  premise, their password callback stated as a synchronous invocation).
  Fresh heap objects: `EVP_MD_CTX_new`, `OPENSSL_sk_new_null`,
  `d2i_PKCS12_bio` (NULL `p12` premise), with the releases `EVP_MD_CTX_free`,
  `OPENSSL_sk_free`, `PKCS12_free`. A counted drop: `X509_STORE_free`.
  Retentions (C04.14): `OPENSSL_sk_push` (the stack keeps `data`),
  `SSL_CTX_set_default_passwd_cb_userdata` (the context keeps `u`).
- **Left uncontracted (16).** `BIO_new_mem_buf` (a nullable result retaining
  the caller's buffer: C04.14.2 admits only a non-null holder result),
  `SSL_CTX_ctrl` and `SSL_ctrl` (custody depends on `cmd`),
  `SSL_set_fd` (the SSL keeps the descriptor), `SSL_CTX_set_cert_cb` and
  `SSL_CTX_set_default_passwd_cb` (registrations into one of several
  callback slots of one context), `SSL_CTX_use_cert_and_key` (the page
  states the custody of the key only), `OPENSSL_sk_pop_free` (a
  synchronous callback that releases each element), `CRYPTO_free` (the
  OpenSSL allocator is not a declared kind), `X509_NAME_oneline` (two arms
  split on a non-literal pointer), `ERR_peek_last_error_all` (out slots
  receiving pointers into the error queue), `OSSL_PROVIDER_load` and
  `OSSL_PROVIDER_unload` (the page does not state who holds the provider
  after a failed unload), and `ENGINE_load_ssl_client_cert`,
  `X509_OBJECT_get0_X509`, `SSL_certs_clear` (no official manual page
  documents them).

## The opt-in `current` run (the arbiter of this card)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the
contract selection moved to the user load path (the profile manifest's
modules plus `quirin-provider-openssl`,
[`opt_in_providers.py`](../openssl-provider-plugin/evidence/opt_in_providers.py)),
base (this card's base, twelve contracts) against tip, one binary (the card
changes nothing the binary is built from).

- **No row becomes `Safe`.** Every function applying a bound contract joins
  the binding's `Pending` (`plugin-claim-uncertified`).
- **No status move is predicted.** Every row that OpenSSL alone blocks on
  `current` keeps a blocker this card leaves: `rd_kafka_ssl_PKEY_from_string`
  (`BIO_new_mem_buf`; its `PEM_read_bio_PrivateKey` passes a callback that
  must compose as the identity), and the five rows OPENSSL-PROVIDER-PLUGIN
  already contracted keep the readings that refuse them (member-read drops,
  a nullable returned reference handed to `return`). Every co-blocked row
  keeps at least one non-OpenSSL diagnostic; on 20 of the 32 the OpenSSL
  callees become fully contracted, so the row's uncontracted-callee
  diagnostics leave and its other diagnostics stay. The rows with the fewest
  other diagnostics, `rd_kafka_ssl_read_cert_chain_from_BIO` (1) and
  `rd_kafka_ssl_cert_issuer_match` (2), are the ones most likely to prove
  this wrong: if their other diagnostics were consequences of the
  uncontracted calls, they may move `Unknown -> Pending`.
- **Risk in the other direction.** A returned reference or fresh object a
  row acquires and does not release on a path with no refusal is a leak
  candidate (provisional `Unsafe`, never an upstream defect). None is
  predicted; any measured `Unknown -> Unsafe` is reported row by row with
  the declaration it rests on.
- The `Unsafe` row `context_new` (Monkey) stays `Unsafe`.
