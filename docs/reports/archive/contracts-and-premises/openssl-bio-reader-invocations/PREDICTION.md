# OPENSSL-BIO-READER-INVOCATIONS: prediction

Committed before any loader, gateway, declaration, checker, test or Lean
change. Base `dd0225012` (integrate/r32: main `f659b24f5` plus
OPENSSL-BIO-METHODS-AND-CALLBACKS and its amendments). User decision
2026-11-19: launch the named follow-up `OPENSSL-BIO-READER-INVOCATIONS`
(recorded by OPENSSL-BIO-METHODS-AND-CALLBACKS).

## The plan the prediction is about

`PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` and `d2i_PKCS12_bio` read
through the caller's BIO, so they run its callback ("called during most
high-level BIO operations", `BIO_set_callback` 3.5) and functions of its
method table, and their declarations say neither. They carry a
literal-argument premise (`x = 0`, `p12 = 0`), and rule 57 admits only a
synchronous invocation beside one.

* **Rule 57 amended in place.** The loader admits an opaque slot invocation
  (`resource_callback_slot_invoked`, `CallbackClause::MemberInvoked { slot:
  true }`) beside a literal-argument premise, as it admits a synchronous
  invocation. Both hold nothing after the call. Route G already renames
  every unmet call of a retaining declaration before the callback route
  reads it (r29), so a slot invocation is decided only at calls the premise
  holds for, exactly as rule 12n decides it there. A plain member
  invocation (rule 12n without `slot`), a registration, a table, a registry
  and every other retaining clause beside a premise still reject.
* **The readers state the BIO slots.** Each of the three declarations
  states all 14 BIO slot invocations on its BIO formal (position 0) with
  every actual `_`, exactly the set the SSL I/O calls state, and names
  `openssl_builtin_bio_methods`. The pages do not list which BIO
  operations a read performs, and a filter BIO forwards operations down
  its chain, so every key is stated. `_` is kept because a chained BIO, not
  only the caller's, is handed to the slot functions, and `_` refuses every
  candidate that is not the identity at that position.
* **The built-in premise's statement** is widened from "a BIO the SSL
  object holds" to cover the caller's BIO argument too. The five SSL calls
  carry the same statement, so their digests change.
* **Other declarations.** No other declaration in the crate reads or writes
  through a caller's BIO. `BIO_free` is already under
  `openssl_bio_free_callbacks`. The setters do not run the slots, and
  `BIO_new_file` creates a fresh BIO with a built-in method and no
  callback.
* **Lean.** `QCore.CallbackLiteralPremise`'s admission gains the slot
  invocation. A new `QCore.OpensslBioReaders` covers the reader shape: an
  unmet premise decides no slot, a met one defers to rule 12n's decision,
  and applied implies every slot set is closed with every candidate the
  identity.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate (it is on the
user load path only). The loader change only admits declarations it
rejected before, and no pinned module carries a slot invocation beside a
premise.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The user path on the predecessor's fixture

The `contracts` step over
`docs/reports/openssl-provider-remaining-callees/evidence/fixture` keeps
`read_cert` and `pkcs12_cycle` `Pending`, as at base, and `read_cert_reuse`
`Unknown` (outside the premise). No binding changes status.

## The opt-in `current` run (this card's arbiter)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the user
load path, base `dd0225012` and tip, using OPENSSL-SSL-CTX-CALLBACK-SLOTS'
script. The expected base is r32's tip: 1098 / 2 / 3699 / 5, with 113
bindings, all `Pending`.

* **No status move.** No row moves to `Safe`, there is no new `Unsafe`, and
  the binding count stays 113, all `Pending`.
* **Run sets.** All 14 BIO slots stay closed and empty, as in r32's run.
  So a stated slot invocation at a met reader call invokes nothing.
* **Content.** Rows applying a reader within its premise include Monkey's
  `tls_load_builtin_credentials` (two `PEM_read_bio_X509` calls with a null
  `cb`) and librdkafka's certificate loading. Most likely their content does
  not change at all. If it does change, the change is limited to naming the
  premise or the BIO setter declarations the decision reads. The r29
  null-callback reading of `tls_load_builtin_credentials` does not regress.
* The rows most likely to prove this wrong:
  * a reader row refusing because the slot decision has no run facts on
    the route that applies it. That would be a `Pending` or `Safe` to
    `Unknown` move;
  * the widened premise statement changing an SSL row's content.
