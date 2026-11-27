# OPENSSL-FREE-SESSION-REMOVE-INVOCATION: prediction

Committed before any loader, gateway, declaration, checker, test or Lean
change. Base `062974679` (main, r33). User decision 2026-11-19: launch the
named follow-up `OPENSSL-FREE-SESSION-REMOVE-INVOCATION` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS, appended by OPENSSL-BIO-METHODS-AND-CALLBACKS).

## The plan the prediction is about

* **The drop ordering, as read at base.** At a call of a retaining
  declaration, the transfer applies (in order) the custody contract
  (`apply_contract_call`), the retentions, the reference keeps and drops
  (`reference_route::apply`), then the callback route
  (`callback_route::apply`). A callback candidate's `resource_owned(n)`
  requirement on a parameter actual is folded into the custody contract's
  mode at that position (`legacy_retained_by.rs`), so it is checked before
  the drop already. What the callback route applies after the drop is a
  candidate's member-path states at a parameter position (rule 30) and the
  deferred member-invocation reading of the object's constructed state
  (rules 13w, 14e). Those would read or change the dropped object after its
  drop.
* **The fix.** The callback route is applied before the reference route.
  Before this card the loader admits no callback clause beside a reference
  clause, and no arm, result state, hand-off or member-path state either, so
  no admitted declaration carries both operand kinds and the swap changes no
  reading. Beside a drop, a slot invocation runs only at the *last* drop,
  which the call does not decide. So a candidate's member-path state or a
  deferred member reading at a dropping call refuses the call (fail-closed),
  and only two things stand: the static decision of the candidates, and the
  `resource_owned(n)` requirement, which holds before the drop whether or
  not the drop is the last.
* **The loader (C04.19 amended in place).** Beside reference clauses, a
  callback clause is admitted only when every reference clause is a drop
  and every callback clause is a slot invocation
  (`resource_callback_slot_invoked`). A keep, a slot move, a derived out, a
  plain member invocation, a synchronous invocation, a registration, a
  table and a registry beside a reference clause still reject.
* **The declarations.**
  * `SSL_CTX_free` states the session-remove invocation (`ctx`,
    `ssl_ctx_st.remove_session_cb`, `_`, `_`).
  * `SSL_free` states the session-remove invocation and the three BIO slots
    a BIO's last drop runs (`bio_st.bio_callback_ex`, `bio_st.bio_callback`,
    `bio_method_st.meth_destroy`), every actual `_`.
  * `BIO_free` states the same three BIO slots on `a`, every actual `_`.
  * `openssl_bio_free_callbacks` is narrowed to the BIO class's ex_data free
    callbacks. The built-in methods' destroy functions move under
    `openssl_builtin_bio_methods`, whose statement widens to a BIO the call
    frees; `BIO_free` and `SSL_free` name it, and the eight declarations
    already carrying it change digest.
  * `openssl_unstated_callback_slots` stays on both frees, with its
    unchanged shared statement. Freeing the context frees its certificate
    store (`SSL_CTX_set_cert_store` 3.5: the context takes ownership), whose
    lookup methods' free and shutdown functions are user-settable
    (`X509_LOOKUP_meth_set_free`, `_set_shutdown`). So the registered
    discharge's reading, "leaving them on `openssl_ex_data_callbacks`", would
    leave a callback the frees may run uncovered. The stated session-remove
    invocation leaves the premise's coverage by its own words ("no clause of
    this declaration states").
* **Lean.** A new module models the amended admission (it only grows, and
  only by slot invocations beside drops), the order (the requirement is read
  on the pre-drop state; the old order checked a last drop's requirement
  against a freed object), and the non-last drop (the decision does not
  depend on whether the drop is the last; a member effect beside a drop
  refuses). `QCore.OpensslBioSlots` is amended: the free premise covers only
  ex_data, and a BIO's free callbacks must be stated.

## What can move

**Pinned routes.** No pinned route loads the OpenSSL crate, and no pinned
module carries a callback clause beside a reference clause, so the swapped
order and the new refusal read nothing there.

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
`docs/reports/openssl-provider-remaining-callees/evidence/fixture` keeps every
fixture function's status. No binding changes status.

## The opt-in `current` run (this card's arbiter)

`quirin-audit current --monkey-linked` over Fluent Bit 046accf6 with the user
load path, base main `062974679` and tip, using OPENSSL-SSL-CTX-CALLBACK-SLOTS'
script. The expected base is r33's: 1098 / 2 / 3699 / 5, with 113 bindings, all
`Pending`.

* **No status move.** No row moves to `Safe`, there is no new `Unsafe`, and
  the binding count stays 113, all `Pending`.
* **Run sets.** The three BIO keys stay closed and empty, as in r32's and
  r33's runs. `ssl_ctx_st.remove_session_cb` stays as at base: no Fluent Bit
  unit in either compile database calls `SSL_CTX_sess_set_remove_cb` that I
  know of, so I expect it closed and empty and every stated invocation at a
  free to invoke nothing.
* **Content.** Most likely no row's content changes. A change would be
  limited to a free's row naming the slot keys or setters it now reads.
* The rows most likely to prove this wrong:
  * a row calling `SSL_free`, `SSL_CTX_free` or `BIO_free` (Fluent Bit's
    TLS teardown, Monkey's, librdkafka's) refusing because the slot decision
    has no run facts on the route that applies it, or because the key's run
    set is open. That would be a `Safe` or `Pending` to `Unknown` move;
  * a free's row that is `Safe` at base reading `Unknown` because the free
    is now read through the callback route at all.
