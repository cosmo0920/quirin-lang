# OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS: prediction

Committed before any declaration, checker, test or Lean change. Base main
`9b107caf1` (integrate/r45 landed: OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS
and SP1-SYMBOLIC-OPENSSL-PREMISES). The card is the named follow-up recorded
by OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS.

## What was read before this prediction

OpenSSL 3.5.5 (`openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`,
unpacked in `private/src/`, not committed): `ssl/ssl_rsa.c` (`ssl_set_pkey`,
`ssl_set_cert`, `SSL_CTX_use_PrivateKey_file`, `use_certificate_chain_file`),
`crypto/pem/pem_pkey.c`, `crypto/pem/pem_oth.c`, `crypto/pkcs12/p12_utl.c`,
`crypto/asn1/tasn_dec.c`, `crypto/x509/x_pubkey.c`, `crypto/x509/by_file.c`,
`crypto/x509/x509_d2.c`, `crypto/asn1/x_info.c`, and the frees of peer and
ephemeral keys in `ssl/statem/` and `ssl/s3_lib.c`.

## The plan

* The loaders, the PEM and PKCS#12 readers and the two verify-path calls
  gain `openssl_unstated_callback_slots` in r45's free statement, unchanged
  (`quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |->
  #inert`). SP1's `#freed` already counts "a replaced or temporary object it
  frees inside the call", so the formula names exactly what each frees.
* The handshake statement (the four handshake and I/O calls and
  `SSL_shutdown`) is widened by that formula as one more conjunct, since a
  declaration names each premise once.
* `FREE_AUDIT` lists them, and a declaration whose `EX_DATA_AUDIT` reading
  names `EVP_PKEY` or `X509` must be in the free audit or is refused.

## What can move

Nothing. A premise is metadata. SP1 records that no consumer that decides a
verdict reads a premise's statement, and no gateway code changes. The
checker must still accept every declaration it changes, so the bindings stay
`Pending`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Opt-in `current` run.** Not re-measured on this card. r45 measured the same
kind of change (five frees gaining the same premise) against its base and
found no row moved and no row changed content. This card changes only
premise text, and the checker's acceptance of the changed declarations is
covered by the provider tests in the gate. The integrator's `quirin-audit
measure --declared` is the arbiter for pinned routes. No pinned route loads
the OpenSSL crate.

## Lock

Expected `contracts.lock` diff: 14 changed digests (9 declarations gaining
the free statement, 5 with the widened handshake statement), none added or
removed, `pinned` and `library_versions` unchanged.
