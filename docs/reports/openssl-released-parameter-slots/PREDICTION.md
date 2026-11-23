# OPENSSL-RELEASED-PARAMETER-SLOTS: prediction

Committed before any loader, gateway, declaration, checker, test or Lean
change. Base main `9193b196a`. The card discharges the released-parameter
part of OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS under the USER DECISION of
2026-11-23 ("if yes, EVP_MD_CTX_free drops its premise, and PKCS12_free
keeps it only for the CRL item. -> launch them, too").

## What was read before this prediction

The OpenSSL 3.5.5 release tarball (`openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`, the
digest r45 and r53 recorded; fetched from the release's GitHub download,
unpacked in `private/src/`, not committed): `EVP_MD_CTX_free`,
`evp_md_ctx_reset_ex`, `evp_md_ctx_clear_digest` and `cleanup_old_md_data`
(`crypto/evp/digest.c`), `EVP_PKEY_CTX_free` and `evp_pkey_ctx_free_old_ops`
(`crypto/evp/pmeth_lib.c`), the PKCS#12 and PKCS#7 templates and free
callbacks (`crypto/pkcs12/p12_asn.c`, `crypto/pkcs7/pk7_asn1.c`) and the
CRL free callback (`crypto/x509/x_crl.c`). The loader's slot reading
(`contract_callback.rs`), the gateway's call route
(`legacy_retained_by.rs`, `legacy_transfer.rs`) and the OpenSSL checker's
free audit.

## The plan

* Loader (C04.19.3, rule 12n): a slot invocation's object may be a
  `resource_released` parameter, read before the release; such a
  declaration states nothing beside its custody contract, its premises and
  slot invocations. Every other object mode and every other clause beside a
  released slot invocation keeps refusing.
* Gateway: at a call stating a slot invocation through a released
  parameter, a candidate's effect on the object (a member-path state, a
  member invocation deferred to its constructed state, a registry use)
  refuses the call, and a candidate's `resource_owned(nonnull)` requirement
  on the released position strengthens the release to non-null, read before
  the release.
* `EVP_MD_CTX_free` states the key's nine teardown slots and the digest,
  cipher and public key method cleanups (the digest's handed `ctx`), and
  drops `openssl_unstated_callback_slots`; `PKCS12_free` states the key's
  nine and keeps the free statement (`#teardown(#o) \ (#stated |
  #settable)`) for a CRL's `crl_free` alone. No setter is added (all
  twelve are declared since r53).

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (user load path
only), no pinned module carries a slot invocation, and the gateway's new
refusal and strengthening read only declarations stating a slot invocation
through a released parameter, which the loader refused before this card.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current` run** (Fluent Bit 046accf6, `--monkey-linked`, the
user load path). No status move. The two frees become callback
declarations, withheld from the function map and read by the callback route;
no Fluent Bit or Monkey source calls any of the twelve setters, so each
slot's run set is the closed empty set and invokes nothing, and the custody
contract reads as before. The bindings are unchanged in number and stay
`Pending`.

Content changes predicted only in the dependency lines of rows resting on
the two frees: `EVP_MD_CTX_free` (`src/flb_crypto.c`,
librdkafka's `rdkafka_sasl_oauthbearer_oidc.c`) loses
`openssl_unstated_callback_slots`; `PKCS12_free` (librdkafka's
`rdkafka_cert.c`, `rdkafka_ssl.c`) names it with the free statement instead
of the released one. Rows whose dependency lines do not list premises
change nothing.

**Rows most likely to prove this wrong:** a row in `src/flb_crypto.c`
calling `EVP_MD_CTX_free` that is `Safe` or `Pending` at base and reads the
callee through the function map alone, which now sees an uncontracted
callee (it would move to `Unknown`).
