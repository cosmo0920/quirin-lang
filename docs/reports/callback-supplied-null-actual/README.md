# CALLBACK-SUPPLIED-NULL-ACTUAL: a callee-supplied callback actual that may be NULL

Base `062974679` (main, r33). Named follow-up `CALLBACK-SUPPLIED-NULL-ACTUAL`
(recorded by OPENSSL-UNSTATED-CALLBACK-SLOTS); user decision 2026-11-19.
[PREDICTION](PREDICTION.md) was committed before any implementation
(`fba589bcc`); the implementation is `509ee5b1e`.

## 1. The change

**The word.** `supplied_nullable` is a new actual word in every invocation
actual list (C04.19 placement, grammar G05): a value the callee supplies, as
`_`, that its documentation says may be a null pointer. It is reserved as a
parameter name, as `_`, `element` and `supplied_owned` are, and is never a
member's object. In the schema it is `CallbackActual::SuppliedNullable`, a
variant beside `Supplied` and `SuppliedOwned`, so the supplied-output marker
CALLBACK-SUPPLIED-OUTPUT-OBLIGATION needs is one more variant beside it
rather than a flag on `_`.

**The composition** (C04.19 item 2; gateway rule 13u amended in place, no
new rule id). `callback_contracts::classify` admits `resource_borrowed` and
`resource_owned(nullable)` at a `supplied_nullable` position, as at `_`. A
candidate requiring `resource_owned(nonnull)` there is the new class
`RequiresNonNullSupplied`, and `decide` refuses the call with
`callback-supplied-null`, naming the candidate. The position puts no
requirement on the call (`requirements` reads parameter actuals only). The
other readers follow `_`:

* the body check (rule 13y) matches it as `_`;
* rule 15w's `corresponds` forwards it only to itself, so a forward from a
  `_` to a `supplied_nullable` deviates and refuses;
* a hand-off candidate reads it as `_` (`Handed::Own`);
* the clause-standing rendering prints it.

A plain `_` is unchanged.

**Lean.** `QCore.CallbackSuppliedNull` (`proof/AuditCallbackSuppliedNull.lean`,
8 audited theorems, 18 in the module) sits beside `QCore.CallbackContracts`:

* a `nonnull` requirement at a nullable actual is never admitted, and
  `decide` refuses it;
* an admitted candidate accepts NULL at every nullable actual;
* marking `_` actuals nullable keeps the class or refuses
  (`marking_never_admits_more`, `a_marked_identity_was_an_identity`);
* a list with no nullable actual classifies exactly as
  `CallbackContracts.classifyModes`;
* the classify and decision vectors, bound to `NULLABLE_CLASSIFY_VECTORS`
  and `NULLABLE_DECISION_VECTORS` by the unit test
  `nullable_vectors_hold_and_are_the_lean_vectors`.

Axioms are within `propext`, `Classical.choice` and `Quot.sound`.

**Tests.**

* `qcore-contract` `tests/op_callback_clause.rs`
  (`the_nullable_actual_loads_where_a_supplied_one_does`): the word in a
  synchronous, table, member and slot invocation; the retained-text reload;
  a parameter named `supplied_nullable` rejects; it is no member object.
* `qcore-c-gateway` `tests/openssl_unstated_callback_slots.rs`
  (`a_candidate_requiring_a_live_object_where_the_library_may_pass_null_refuses`),
  on the crate's own modules. A security candidate requiring a live `ctx`
  refuses `SSL_write` (`callback-supplied-null`) but not the context's
  loaders, where `ctx` is the caller's. One requiring `s` refuses the
  loaders, not `SSL_write`. One requiring `other` refuses `SSL_write` and
  the chain loader, not `SSL_CTX_use_certificate` (`other` = `x`). One
  accepting NULL decides all three, `SSL_write` `Safe`.
* `quirin-provider-openssl` `check.rs`
  (`a_drifted_nullable_argument_is_refused`): every entry cited; an uncited
  sentence, a vacuous position, an unknown slot and a snapshot without its
  argument sentences refuse; a dropped mark, an extra mark and an invented
  mark on `SSL_read` and `PEM_read_bio_X509` refuse.
  `a_drifted_unstated_slot_reading_is_refused` is updated to the new
  spelling.

No test target was added, so the read census was not re-recorded.

## 2. The OpenSSL marks, page-cited

Nullability is a property of a slot's argument, so it is one table,
`documented::NULLABLE_ARGUMENTS` (slot, position, the page's exact
sentence). `expected_clauses` renders `supplied_nullable` at every
invocation of the slot whose actual there is `_`; a parameter actual is
unchanged. `check::nullable_argument_cited` refuses an entry whose sentence
the slot page's snapshot at 3.5 does not record (in `null_sentences` or the
new `argument_null_sentences`), and an entry no documented invocation reads
as `_`. It runs for every slot a declaration states.

**Snapshots.** `null_sentences` records DESCRIPTION and RETURN VALUES only,
and the security and BIO sentences are in sections of their own.
`fetch_docs.py` therefore records `argument_null_sentences` (every other
section's NULL sentences) on new fetches. Its new `--argument-sentences`
mode refetches a page the record holds and refuses unless the SHA-256 is
the recorded one. Run for `SSL_CTX_set_security_level`,
`SSL_CTX_set_psk_client_callback` and `BIO_set_callback` at 3.5: all three
matched, so no digest changed and no page was replaced.

## 3. The survey

Every `_` argument of every stated invocation was read against the page its
slot cites, at 3.5 and the recorded digest. Each of the 20 slot pages was
refetched for this card, and each matched its recorded SHA-256. Every
section's NULL sentences were listed (`private/null-survey/`), looking for
a sentence saying the callback may receive NULL there.

**Marked** (`NULLABLE_ARGUMENTS`, nine entries over five slots):

| Slot (page) | Position | Sentence, as recorded |
|---|---|---|
| `ssl_ctx_st.sec_cb` (`SSL_CTX_set_security_level`, APPLICATION-DEFINED SECURITY CALLBACKS) | 0 `s`, 1 `ctx` | "One of the two is NULL: checks originating from a connection pass s (with ctx NULL), and checks originating from a context pass ctx (with s NULL)." |
| same | 5 `other` | "It is NULL when no object is associated with the operation." |
| same | 6 `ex` | "ex The extra data pointer previously set with SSL_CTX_set0_security_ex_data() or SSL_set0_security_ex_data(), or NULL." |
| `ssl_ctx_st.psk_client_callback` (`SSL_CTX_set_psk_client_callback`, NOTES) | 1 `hint` | "Note that parameter hint given to the callback may be NULL ." |
| `ssl_ctx_st.psk_use_session_cb` (same page, DESCRIPTION) | 1 `md` | "The first time the callback is called for a connection the md parameter is NULL." |
| `bio_st.bio_callback_ex` (`BIO_set_callback`, CALLBACK OPERATIONS) | 2 `argp`, 7 `processed` | "BIO_free(b) callback_ex(b, BIO_CB_FREE, NULL, 0, 0, 0L, 1L, NULL) or callback(b, BIO_CB_FREE, NULL, 0L, 0L, 1L) is called before the free operation." The table of calls also passes NULL at `processed` before every read, write, gets, puts and ctrl. |
| `bio_st.bio_callback` (same) | 2 `argp` | same sentence |

Where the actual at a marked position is the caller's parameter, it is
unchanged:

* `s` at a connection's security check;
* `ctx` at a context's check;
* `other` = `x` at `SSL_CTX_use_certificate`;
* the PSK callbacks' `ssl`.

The 30 invocation lines in the 10 declarations listed in section 4 change.

**Not marked: no page sentence says so.** These go to the follow-up
`CALLBACK-UNSET-ARGUMENT-NULL`.

* **User argument pointers** a setter keeps and the slot hands back. They
  are NULL when the application never set one, but no page says the
  callback receives NULL.
  * OpenSSL: `cert_cb`'s `arg`; `default_passwd_cb`'s `u` (the page states
    only that the getter "returns NULL" when unset); `msg_callback`'s `arg`
    ("optionally defined"); `arg` of the ALPN select, NPN select and
    advertised, cert verify, ClientHello, record padding and early data
    callbacks; the custom extensions' `add_arg` and `parse_arg`.
  * librdkafka: the rk_conf callbacks' `opaque`
    ("the opaque set with rd_kafka_conf_set_opaque()").
* **Custom extension callbacks' `x`.** The page says it "will indicate the
  current certificate" only when a Certificate message is constructed or
  parsed, and says nothing for the other messages.
* **BIO method functions** (`BIO_meth_new`): `meth_ctrl`'s `parg` is the
  BIO_ctrl caller's (`BIO_flush` passes NULL), but the page says nothing.

**Not NULL by their pages** (`_` correct):

* the verify callback's `x509_ctx`;
* the info callback's scalars;
* the message callback's `buf`;
* the session callbacks' `sess`, `data` and `copy`;
* the keylog `line`;
* the PSK server and find-session `identity`.

The client certificate callback's `x509`/`pkey`, the PSK use/find-session
callbacks' `sess` and `id`, and the ALPN and NPN `out` are out pointers the
callback writes through. Those are CALLBACK-SUPPLIED-OUTPUT-OBLIGATION's.

**Other crates.** None of their invocations has a source sentence saying a
`_` argument may be NULL:

* libc's `bsearch` and `qsort` comparators receive array elements (C11
  7.22.5), and `pthread_once` passes none;
* the cmetrics and chunkio log callbacks receive the formatted message;
* the cmetrics and ctraces mpack unpackers receive an index;
* Monkey's `mk_thread.cb_destroy` receives `MK_THREAD_DATA(th)`, an interior
  pointer;
* librdkafka's op and rk_conf callbacks receive ops, error codes, strings
  and lists, plus `opaque` (above). `rd_kafka_set_logger`'s "rk may be
  passed as NULL in the callback" is about a deprecated setter that no
  declaration invokes, and `log_cb`'s `rk` is a parameter actual.

## 4. Lock

`quirin-audit pin` (debug binary of `509ee5b1e`'s tree, through the
toolchain env; `--source-root
docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate
quirin-provider-openssl`): 0 pinned, 107 flexible. The 10 re-stated
declarations' digests change, exactly the predicted set:

* `SSL_read`, `SSL_write`, `SSL_connect`, `SSL_do_handshake` (security, PSK,
  BIO);
* `SSL_shutdown` (BIO);
* `SSL_CTX_use_certificate`, `SSL_CTX_use_certificate_chain_file`
  (security);
* `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey`, `d2i_PKCS12_bio` (BIO).

No entry is added or removed, and `pinned`, `library_versions` and every
other top-level key are unchanged. The other steps:

* `quirin-plugin load-path --extends contract-load-path.json` over libcurl
  then openssl, run from `providers/`;
* `plugin views --write`, which left `providers/user-crates-load-path.json`
  byte-identical;
* `providers/package.py checkers`: exit 1 before the views were written
  (the regenerated load path had none), exit 0 after the re-pin;
* `providers/contracts.py sync`, which wrote nothing.

## 5. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice read no user load path, and no pinned module carries the word. The
prediction's `measure-prediction` block (none on all three) is for the
integrator's `quirin-audit measure --declared`.

**The opt-in `current` run (this card's arbiter).** `quirin-audit current
--monkey-linked` over Fluent Bit 046accf6 with OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`), driver `private/measure.sh`.
Each side is a `git archive` side tree with a release `quirin-audit` built
in it and its checker installed by its own `package.py checkers`
(`checkers.exit` 0 on all three). Inputs were prepared by the base binary's
`prepare` into `private/fb-046accf6` (exit 0). The three runs overlapped,
so their wall times are not comparable.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base | `062974679` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| tip | `509ee5b1e` | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| experiment | tip, every `_` read nullable | same | 1097 / 2 / 3700 / 5 | 113, all `Pending` |

All three exit 0 with no unmatched row
([runs and comparisons](evidence/opt-in/), compared by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**Base to tip: as predicted, no row moved and no row changed content.**
There is no Safe-to-non-Safe move. Fluent Bit registers no security, PSK or
BIO callback, so no marked slot has a candidate.

## 6. Should a plain `_` be read as possibly NULL?

The experiment ran the tip with one line added to `classify`
(`private/experiment_patch.py`, never committed): `resource_owned(nonnull)`
at a plain `_` is `RequiresNonNullSupplied` too. Against the tip:

* **One Safe-to-Unknown move.** librdkafka's `rd_kafka_ApiVersion_check`
  (`rdkafka_feature.c`) gains "call to `bsearch`: callback
  `rd_kafka_ApiVersion_key_cmp` … requires a live object where the callee
  may pass NULL (callback-supplied-null)".
* **Three content changes.**
  * `rd_kafka_features_check` now reads its callee
    `rd_kafka_ApiVersion_check` as uncontracted, because the callee's
    summary is no longer published.
  * `rd_kafka_populate_metadata_topic_racks` refuses first on its `bsearch`
    comparator `rd_kafka_metadata_broker_internal_cmp`, which hides two
    later diagnostics.
  * `rd_kafka_parse_Metadata0` changes in its links only.

Both refusals are false. libc's `bsearch` declaration states
`resource_callback_invoked(compar, key, _)`, and the `_` is a pointer to an
element of the caller's array, which C11 7.22.5 never passes as NULL.

So the stricter default loses a correct `Safe` row and gains nothing in this
run. No row a plain `_` admits is known to receive NULL. The ones that might
are the user `arg` and `opaque` pointers below, and those are better marked
where a source says so than refused everywhere. **My recommendation is to
keep the default.** A plain `_` reads as a value the callee supplies and
meets a `nonnull` requirement; `supplied_nullable` marks the documented
exceptions. Changing the default is the user's decision. It would move the
row above, and it would refuse every comparator that states
`resource_owned(nonnull)`.

## 7. Gate

**First run.** `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base 062974679 --out private/gate-1` on `509ee5b1e` (Clang 21, Lean
4.35.0) chose full scope with the provider gate, because the diff reaches
`qcore-c-gateway`. It exited 1. Every step passed but the Rust suite, which
had 662 units and 707 summaries: 9703 passed, **2 failed**, 21 ignored, 148
filtered. The other steps:

* Lean: 453 audits.
* Python: 779 tests.
* Providers: package build and verify, 159 passed and 1 ignored, fmt and
  Clippy all exit 0.

The two failures were my stale test expectations, not flakes.
`tests/openssl_undeclared_handshake_callbacks.rs`'s
`the_crate_states_every_bio_slot` and `the_readers_state_every_bio_slot`
asserted every BIO slot actual is `_`. They now assert exactly the
documented `supplied_nullable` positions (`bio_actuals`), and the target
passes 17/17. The record is [`gate/gate-1.json`](gate/gate-1.json).

**Second run.** The same command was run on `b8d154d6a` (the fix, the
report and the evidence) with `--out private/gate-2`, in full scope with
the provider gate. Result: `gate_scope: 14 steps passed`, exit 0. Record:
[`gate/gate-2.json`](gate/gate-2.json).

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | 1 submodule (`third_party/libevent` at `d82464a27`) |
| Rust suite | 0 | 662 units, 707 summaries; 9705 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 453 audits (new `AuditCallbackSuppliedNull`) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 159 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

No test target was added, so the read census was not re-recorded.

**After merging main `3b12177a3`** (r34 OPENSSL-EX-DATA-REGISTRY, r35
CALLBACK-CANDIDATE-LITERAL-PREMISE; merge `1d75fe250`):

* **The merge.** `check.rs` conflicted in its imports and its tests module,
  and both cards' imports and tests were kept. `callback_contracts.rs`
  merged cleanly with r35. The 30 marked invocation lines sit beside r34's
  `openssl_ex_data_callbacks` premises, and r34's ex_data declarations are
  unchanged.
* **The lock.** `contracts.lock` was not hand-merged. It was regenerated in
  the documented order: load-path, checkers (failing on the absent views,
  as expected), `views --write`, `pin`, `contracts.py sync`, checkers.
  Against `3b12177a3`, exactly the same 10 `flexible` entries change
  `contract_sha256` and `pinned_at`:
  * `PEM_read_bio_PrivateKey`, `PEM_read_bio_X509`;
  * `SSL_CTX_use_certificate`, `SSL_CTX_use_certificate_chain_file`;
  * `SSL_connect`, `SSL_do_handshake`, `SSL_read`, `SSL_shutdown`,
    `SSL_write`;
  * `d2i_PKCS12_bio`.

  107 entries are on both sides. `pinned`, `library_versions` and every
  other top-level key are unchanged, and the load path is byte-identical to
  main's.
* **The gate.** `gate_scope.py --base 3b12177a3 --out private/gate-3`
  ([`gate/gate-3.json`](gate/gate-3.json)): `14 steps passed`, exit 0.
  * Rust: 662 units and 707 summaries; 9710 passed, 0 failed, 21 ignored,
    148 filtered.
  * Lean: 455 audits. Python: 779 tests.
  * Providers: 161 passed, 1 ignored. Every other step exits 0.
* **The amendments.** `apply_amendments.py --check --profiles-base
  3b12177a3` exits 0.

The opt-in measurement was not rerun on the merge.

**After merging main `80e4b0a25`** (r36 OPENSSL-FREE-SESSION-REMOVE-INVOCATION;
merge `aa11d1991`):

* **The merge.** C04.19's placement paragraph and the handshake-callbacks
  test conflicted, and both sides were kept.
* **The frees.** r36 states the BIO callback slots on `BIO_free` and
  `SSL_free`. Those invocations now state `argp`, and the `_ex` form's
  `processed`, as `supplied_nullable`, from the same cited sentence: it is
  the `BIO_free` row itself. r36's test of the frees' actuals now expects
  those positions. The `NULLABLE_ARGUMENTS` checker and the crate's tests
  pass (18/18).
* **The lock.** It was regenerated in the six-step order, with each step's
  exit captured unpiped:
  * load-path: 0;
  * checkers: 1, the expected missing-views refusal;
  * `views --write`: 0;
  * `pin`: 0;
  * `contracts.py sync`: 0;
  * checkers: 0.

  Against `80e4b0a25`, 12 `flexible` entries change `contract_sha256` and
  `pinned_at`: this card's 10, plus `BIO_free` and `SSL_free`. 107 entries
  are on both sides. `pinned`, `library_versions` and the load paths are
  unchanged.
* **The gate.** `gate_scope.py --base 80e4b0a25 --out private/gate-4`
  ([`gate/gate-4.json`](gate/gate-4.json)): `14 steps passed`, exit 0.
  * Rust: 662 units; 9715 passed, 0 failed, 21 ignored, 148 filtered.
  * Lean: 456 audits. Python: 779 tests.
  * Providers: 161 passed, 1 ignored.
* **The amendments.** `apply_amendments.py --check --profiles-base
  80e4b0a25` exits 0.

## 8. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`CALLBACK-SUPPLIED-NULL-ACTUAL` and registers
`CALLBACK-UNSET-ARGUMENT-NULL`: the user argument pointers and the other
undocumented possibly-NULL `_` arguments listed in section 3.

It also appends a note to OPENSSL-EX-DATA-CLASS-REGISTRY, registered by
OPENSSL-EX-DATA-REGISTRY (r34). `CRYPTO_get_ex_new_index` (3.5) says "The
free_func() may be called with a NULL pointer" and, of `ptr`, "for
new_func() this will typically be NULL". A registry invocation that states
the ex_data callbacks should mark those arguments `supplied_nullable`. This
card leaves r34's ex_data declarations unchanged.

Files likely to conflict:

* `crates/qcore-c-gateway/src/callback_contracts.rs` (`classify`, `decide`,
  `CandidateClass`, the tests module), with CALLBACK-CANDIDATE-LITERAL-PREMISE;
* `crates/qcore-c-gateway/src/legacy_callback_route.rs`
  (`handoff_admission`, the body check);
* `crates/qcore-schemas/src/callback.rs` (`CallbackActual`), with
  CALLBACK-SUPPLIED-OUTPUT-OBLIGATION;
* `providers/quirin-provider-openssl/src/documented.rs` (after `SLOTS`),
  `src/check.rs` (imports, `expected_clauses`, the check loop, tests);
* the `.qv` invocation lines of `ssl_io.qv`, `ssl.qv`, `ssl_ctx.qv`,
  `pem.qv` and `pkcs12.qv`;
* `contracts.lock`, `doc-snapshots.json`: regenerate the lock with the
  steps above;
* with OPENSSL-EX-DATA-REGISTRY and OPENSSL-FREE-SESSION-REMOVE-INVOCATION;
* `docs/agents/rules/13u.md` (regenerate `gateway.md`) and C04.19.
