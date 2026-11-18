# OPENSSL-PEM-CALLBACK-ARM: prediction

Committed before any code, contract or checker change (base `974434463`).
User decision 2026-11-18: remove the PEM readers' `x = cb = NULL` restriction
that OPENSSL-PROVIDER-REMAINING-CALLEES left
([report](../openssl-provider-remaining-callees/README.md) §1 "The PEM
readers").

## The plan the prediction is about

1. **Loader.** A literal-argument premise (C04.5) is admitted beside callback
   clauses (C04.19) when they are the declaration's only retaining clauses.
   Every other retaining clause beside a premise still rejects.
2. **Route G.** The premise rewrite (`legacy_literal_premise`) today renames
   only callees in the function map. A callback declaration is withheld from
   that map, so the rewrite also reaches a retaining declaration: an unmet
   premise renames the call to an undeclared callee before the callback route
   reads it.
3. **`pem.qv`.** Both readers drop `requires resource_literal_argument(cb, 0)`
   and gain `ensures normal resource_callback_invoked(cb, _, _, _, u)`; the
   `x` premise stays. The OpenSSL checker's documented shape and the crate's
   `contracts.lock` follow (pem entries only).

## What can move

No pinned route reads the user load path, so `pem.qv` reaches none. The code
changes act only on declarations that carry both a premise and a callback
clause, which the loader rejected until now, so none exists in any pinned
contract set; the route G change otherwise only renames, under a poisoned
premise set, retaining callees as well (no pinned route runs poisoned). So:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The opt-in `current` run

The seven-file selection REFERENCE-DROP-NULL-ACCEPTING used
([selection](../reference-drop-null-accepting/artifacts/optin-selection.json)),
base binary and crate against tip binary and crate, contracts from the
generated user load path.

* **No row becomes `Safe`.** Every row applying an OpenSSL binding joins its
  `Pending`; rows stay `Unknown` on their other diagnostics.
* **The seven calls.** Each loses `call … outside its declared
  literal-argument premise` and is decided by the callback route instead.
  `rdkafka_ssl.c:790` passes `&ca` at `x` and a formal at `cb`: it stays
  outside the premise (the `x` reuse arm), unchanged. The other six pass a
  null `x` and a function designator. Predicted remaining blockers:
  - `rd_kafka_transport_ssl_passwd_cb` (`rdkafka_ssl.c:761, 833, 1292`):
    `callback-uncontracted` (its body calls `rd_kafka_dbg`/`rd_kafka_log`,
    so no validated summary is expected) or, if a summary validates, the
    call decides and the row keeps its other diagnostics.
  - `rd_kafka_conf_ssl_passwd_cb` (`rdkafka_cert.c:272, 349, 405`): a
    candidate reading only `conf`'s member and copying into the callee's
    buffer; if its summary validates as borrowed/owned positions with no
    result mode, the call decides; otherwise `callback-uncontracted`.
  Where the call decides, the result becomes a held reference, so
  `X509_free`/`EVP_PKEY_free` of it can stop being `reference-drop-unheld`
  and new readings may surface (interior borrows, member stores of the
  result: `cert->x509`, `cert->pkey`).
* **The null-callback calls regress in content, not in status.** A null
  pointer constant at an invoked callback is an open candidate set today
  (`callback-undecided`, rule 13u), and the PEM null-`cb` arm (OpenSSL's
  default callback) has no spelling in the vocabulary. So Monkey's
  `tls_load_builtin_credentials` (`tls_openssl.c:176, 188`) and the OIDC
  assertion (`rdkafka_sasl_oauthbearer_oidc.c:495, 498`, not in this
  preparation's database) lose their in-premise reading and refuse
  `callback-undecided`; their rows are already `Unknown` on other
  diagnostics and stay so. Recorded as a follow-up and a user decision.
* **Risk in the other direction.** A held reference acquired and not
  released on a refusal-free path is a leak candidate (provisional
  `Unsafe`, never an upstream defect); none is predicted.

## Addendum before the null-default measurement (user decision 2026-11-18)

The table above is unchanged as committed (`bcdb9b3b2`); its first opt-in
run (`fbe57c2d8`) measured it. The card now also states the null-`cb` arm
(`resource_callback_invoked_or_default`), predicted against the same base:

* The null-`cb` reads return to the base reading: Monkey's
  `tls_load_builtin_credentials` loses both `callback-undecided` and the six
  `reference-drop-unheld` drops it regained, so its row equals base (still
  `Unknown` on its other diagnostics); the predecessor fixture's `read_cert`
  is `Pending` again.
* The seven calls read as in the first run (a non-null callback is decided
  unchanged): `rdkafka_ssl.c:761, 833` `callback-uncontracted`,
  `rdkafka_cert.c:349, 405` the member store, the three condition calls
  unchanged. Base to tip: 0 moves, 3 rows changing content.
* The pinned-route block is unchanged (no move): `pem.qv` is read by none,
  and no pinned contract states the new spelling.
