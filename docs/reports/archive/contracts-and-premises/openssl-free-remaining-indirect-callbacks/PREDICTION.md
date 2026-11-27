# OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS: prediction

Committed before any declaration, checker, test or Lean change. Base main
`9b107caf1` (integrate/r45 with the SP1 landing record). The card is the
named follow-up recorded by OPENSSL-FREE-INDIRECT-CALLBACKS and appended by
OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS, run with the user decisions of
2026-11-21 on its decision items.

## What was read before this prediction

The OpenSSL 3.5.5 release tarball (`openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`, from
openssl.org, unpacked in `private/src/`, not committed): the frees of
`RSA`, `DSA`, `DH` and `EC_KEY` (`rsa_lib.c`, `dsa_lib.c`, `dh_lib.c`,
`ec_key.c`), `engine_free_util` and `engine_unlocked_finish`
(`eng_lib.c`, `eng_init.c`), `engine_pkey_meths_free` and
`engine_pkey_asn1_meths_free` (`tb_pkmeth.c`, `tb_asnmth.c`),
`evp_pkey_free_it` (`p_lib.c`), `EVP_PKEY_CTX_free` (`pmeth_lib.c`),
`evp_md_ctx_clear_digest` (`digest.c`), `ASYNC_WAIT_CTX_free`
(`async_wait.c`), `ossl_quic_free` (`quic_impl.c`), and the 3.5 manual
sources of the setters' pages (`RSA_meth_new`, `DSA_meth_new`,
`DH_meth_new`, `ENGINE_add`, `EVP_PKEY_ASN1_METHOD`, `ASYNC_WAIT_CTX_new`,
`EVP_MD_meth_new`, `EVP_PKEY_meth_new`, `SSL_set_quic_tls_cbs`).

## The plan

* Declare the setters of the slots the frees run as slot registrations:
  the documented `RSA_meth_set_finish`, `DSA_meth_set_finish`,
  `DH_meth_set_finish`, `ENGINE_set_finish_function`,
  `ENGINE_set_destroy_function`, `EVP_PKEY_asn1_set_free`,
  `ASYNC_WAIT_CTX_set_wait_fd`, `EVP_MD_meth_set_cleanup`,
  `EVP_PKEY_meth_set_cleanup`; and, by the user decision "Admit pinned
  source", the source-derived `EC_KEY_METHOD_set_init` (it has no 3.5 page
  either), `ENGINE_set_pkey_meths`, `ENGINE_set_pkey_asn1_meths`,
  `SSL_CTX_set_client_cert_engine` and `X509_CRL_METHOD_new`, each citing
  the pinned source by file and line, where the vocabulary can state them.
* State the slots at every declared free that can run them, beside a
  library-value premise for the method tables, engines and built-in
  providers the library defines itself, and the one named premise the user
  decided for any other loaded provider.
* Trace the QUIC free path past `quic_impl.c`; state what it runs or name
  what stays unstated.
* Add `RSA`, `DSA`, `DH`, `EC_KEY` and `ENGINE` to the `EX_DATA_AUDIT`
  classes of `SSL_free`, `SSL_CTX_free` and `X509_STORE_free`.
* A free drops `openssl_unstated_callback_slots` only when every item it can
  run is stated; otherwise it keeps it and the report names what is left.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (it is on the
user load path only), and no gateway code is planned to change.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current` run.** No status move and no row content change.
Premises are metadata that no verdict-deciding consumer reads (SP1,
"Semantics unchanged"). The new slot invocations are decided over the run's
registrations, and no Fluent Bit or Monkey source calls any of the new
setters (a search of the local Fluent Bit checkout for the fifteen names
finds none), so each new slot's run set is the empty closed set and
invokes nothing. The bindings stay `Pending`; their count grows by the new
setter declarations and nothing else.

**Rows most likely to prove this wrong:** a row naming one of the declared
frees whose refusal text changes because a new slot key segment
(`rsa_meth_finish`, `engine_destroy`, ...) is read as a member pre-pass
token in some unit.
