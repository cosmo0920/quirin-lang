# OPENSSL-UNSTATED-CALLBACK-SLOTS: the slots the narrowed OpenSSL premise covered, stated

Base `bd6b63ead` (main). Named follow-up `OPENSSL-UNSTATED-CALLBACK-SLOTS`
(recorded by OPENSSL-SSL-CTX-CALLBACK-SLOTS), with `RETENTION-NULL-CONSTANT-ACTUAL`;
user decision 2026-11-18. [PREDICTION](PREDICTION.md) was committed before any
implementation (`c6445d1d5`); the implementation is `d9acdc80e`.

## 1. The slots

Every claim cites a fetched page. Six snapshots were added with
`fetch_docs.py --merge` (2026-11-18): `SSL_CTX_set_msg_callback` and
`SSL_CTX_sess_set_get_cb` at 3.5, 1.1.1 and 1.0.2, `SSL_CTX_set_verify`,
`SSL_CTX_set_info_callback` and `SSL_CTX_set_client_cert_cb` at 1.1.1 and
1.0.2, and `SSL_CTX_set_security_level` at 1.1.1 (no 1.0.2 page: the functions
were "added in OpenSSL 1.1.0"). The 3.5 pages the SSL_CTX slots card recorded
were read again at their recorded digests (each one's SHA-256 is unchanged)
and were not refetched into the record. The session page is
`SSL_CTX_sess_set_get_cb`; the URL `…/SSL_CTX_sess_set_new_cb/` serves a
not-found page. The prototypes were pinned with `pin_prototypes.py` over every
recorded function plus the twelve setters, and every existing entry is
byte-identical. The checker reads the pointer typedef `SSL_verify_cb` as
`fnptr` (`FUNCTION_POINTER_TYPEDEFS`) and gives no spelling to a pointer to
it.

Each documented slot is one key, shared by the context's slot and the SSL's
copy. The pages say the SSL's setting overrides the context's, or `SSL_new`
copies it, so the slot's set is every value either holder can hold.

| Slot (key) | Writers (all the 3.5 NAME names) | Page; range | Invocation stated (actuals) | Invoked by |
|---|---|---|---|---|
| `ssl_ctx_st.verify_callback` | `SSL_CTX_set_verify`, `SSL_set_verify` | `SSL_CTX_set_verify`; `documented "1.0.2"` | `(_, _)`: `verify_callback(preverify_ok, x509_ctx)` | the four handshake and I/O calls |
| `ssl_ctx_st.info_callback` | `SSL_CTX_set_info_callback`, `SSL_set_info_callback` | `SSL_CTX_set_info_callback`; `documented "1.0.2"` | `(ssl, _, _)`: "called as callback(SSL *ssl, int where, int ret)" | handshake/I/O, `SSL_shutdown` ("an alert appears") |
| `ssl_ctx_st.msg_callback` | `SSL_CTX_set_msg_callback`, `SSL_set_msg_callback` | `SSL_CTX_set_msg_callback`; `since "0.9.7"` (HISTORY 1.0.2) | `(_, _, _, _, _, ssl, _)`: `ssl` is "The SSL object that received or sent the message" | handshake/I/O, `SSL_shutdown` |
| `ssl_ctx_st.sec_cb` | `SSL_CTX_set_security_callback`, `SSL_set_security_callback` | `SSL_CTX_set_security_level`; `since "1.1.0"` | connection: `(ssl, _, _, _, _, _, _)`; context: `(_, ctx, _, _, _, x, _)` / `(_, ctx, _, _, _, _, _)` | handshake/I/O; `SSL_CTX_use_certificate` ("other is the X509 certificate"); `SSL_CTX_use_certificate_chain_file` |
| `ssl_ctx_st.new_session_cb` | `SSL_CTX_sess_set_new_cb` | `SSL_CTX_sess_set_get_cb`; `documented "1.0.2"` | `(ssl, _)` | handshake/I/O (TLSv1.3: "after the handshake") |
| `ssl_ctx_st.remove_session_cb` | `SSL_CTX_sess_set_remove_cb` | same | `(_, _)`: the session's context, not a formal | handshake/I/O |
| `ssl_ctx_st.get_session_cb` | `SSL_CTX_sess_set_get_cb` | same | `(ssl, _, _, _)` | handshake/I/O |
| `ssl_ctx_st.client_cert_cb` | `SSL_CTX_set_client_cert_cb` | `SSL_CTX_set_client_cert_cb`; `documented "1.0.2"` | `(ssl, _, _)` | handshake/I/O |

**Why one key per slot.** The pages give these readings:

* `SSL_set_verify` with NULL keeps "the default callback for the underlying
  ctx … that was valid at the time ssl was created".
* The info callback set for the context "is overridden from the setting for a
  specific SSL object".
* The message callback's context settings "will be copied to new SSL objects
  by SSL_new".

The session and client certificate slots have one writer each. The `*_arg`
setters keep the callback's argument, not the callback (an undeclared
retention, which fails closed), and are listed as not-writers. `SSL_trace` is
a callback a setter may be handed: listed, and uncontracted if handed.

**The checker** (`documented.rs`, `check.rs`):

* Twelve `RegistersSlot` entries (a `registers` helper) and eight new slots in
  `SLOTS`, each with `not_writers`.
* `slot_writers_declared` now also requires the page's NAME list to be
  exactly the union of the writers and not-writers of the slots citing that
  page. A setter the page names cannot be left out unremarked, which the
  former one-way check allowed.
* The premise `ExDataCallbacks` (`openssl_ex_data_callbacks`), and the new
  statement of `openssl_unstated_callback_slots`.
* `expected_clauses` derives each new clause set exactly.

Drift tests (`a_drifted_unstated_slot_reading_is_refused`,
`a_page_naming_an_unlisted_function_refuses_its_slots`) refuse each of these:

* each of `SSL_read`'s eight new invocations dropped;
* the connection read as a supplied value;
* the handshake premise dropped;
* `SSL_CTX_use_certificate`'s invocation dropped, its `x` read as `_`, or the
  context's check read as the connection's;
* `SSL_CTX_use_certificate_chain_file` given the slot premise back, or its
  ex_data premise dropped;
* a setter registering the wrong slot;
* `SSL_shutdown` without its premise;
* a page naming an extra setter, or no longer naming a listed writer.

## 2. Which declarations lost the premise, and which keep it

| Declaration | Before | Now |
|---|---|---|
| `SSL_CTX_use_certificate` | plain; read the security callback silently | invokes `ssl_ctx_st.sec_cb(_, ctx, _, _, _, x, _)`; withheld from the function map |
| `SSL_CTX_use_certificate_chain_file` | password slot + `openssl_unstated_callback_slots` | password and security slots; **slot premise dropped**: every slot it may run is stated. It names `openssl_ex_data_callbacks` instead (see below) |
| `SSL_write`, `SSL_read`, `SSL_connect`, `SSL_do_handshake` | certificate slot + premise | all nine slots + the narrowed premise |
| `SSL_shutdown` | premise | info and message slots + the narrowed premise |
| `SSL_free`, `SSL_CTX_free` | premise | premise (statement narrowed); no invocation stated |

**Why each keeps it:**

* **Ex_data (`SSL_CTX_use_certificate_chain_file`).** The call creates
  certificates and frees the chain it replaces, so it may run ex_data
  callbacks. `CRYPTO_get_ex_new_index` (3.5): "new_func() is called when a
  structure is initially allocated"; "free_func() is called for every defined
  index". That is a global registry per object class, not a slot of an
  object.
  * No sound reading exists within the vocabulary. A slot needs an object
    parameter holding the callback (`CRYPTO_get_ex_new_index` takes a class
    index and no object). Its key would merge every class. And the callback
    receives the parent being freed and the data `SSL_set_ex_data` kept, a
    retention no clause states.
  * Dropping the coverage would weaken the declaration, so the narrower
    premise `openssl_ex_data_callbacks` names exactly that.
* **Handshake and I/O calls.** They run callbacks whose setters this crate
  does not declare:
  * the `SSL_CTX_callback_ctrl` arms (servername, status, ticket key: card
    OPENSSL-CTRL-COMMAND-ARMS);
  * the keylog and ALPN selection callbacks, which Fluent Bit registers
    (`ssl_key_logger`, `tls_context_server_alpn_select_callback`);
  * the certificate-verify, PSK, client-hello and record-padding callbacks;
  * a BIO's methods, and ex_data.
* **`SSL_shutdown`.** It reads records until the close_notify (post-handshake
  messages, a record padding callback, a BIO's methods), so it keeps the
  premise beside the two stated invocations.
* **The frees.** The session page says the remove callback "also happens for
  all sessions in the internal session cache when SSL_CTX_free(3) is called"
  and "when a connection was not shutdown cleanly" (`SSL_free`). But C04.19
  admits no callback clause beside a reference drop (`contract_loader.rs`: "a
  callback clause stands only beside retentions and plain predicates"), so the
  invocation cannot be stated. Follow-up OPENSSL-FREE-SESSION-REMOVE-INVOCATION.

The narrowed statement of `openssl_unstated_callback_slots` names what is
left: "a callback slot of the SSL object or its SSL_CTX whose invocation the
declaration does not state, among them the SSL_CTX_callback_ctrl arms … a
method or callback of a BIO the SSL object holds, or an ex_data callback".

**The premise moves no verdict, and dropping it adds nothing.** A premise is
metadata (C04.18). Each stated invocation adds an obligation: a run that
registers an uncontracted or effectful candidate now refuses calls that
before rested on the premise alone. What a session callback hands back (a
reference a new-session callback keeps by returning 1, a client certificate
callback's certificate and key) is outside C04.19's identity composition, so
a registered such callback refuses these calls. That is strictly stronger
than before.

**Lock.** `quirin-audit pin` was run with the tip binary
(`--source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture
--crate quirin-provider-openssl`, through the toolchain env, which pin needs
for Clang; log `private/pin-1.log`).

* 12 entries added (the setters).
* The 9 re-stated declarations' digests change (`SSL_CTX_free`,
  `SSL_CTX_use_certificate`, `SSL_CTX_use_certificate_chain_file`,
  `SSL_connect`, `SSL_do_handshake`, `SSL_free`, `SSL_read`, `SSL_shutdown`,
  `SSL_write`).
* 51 entries are unchanged, and so are `pinned`, `library_versions` and every
  other top-level key.

The other steps:

* `providers/user-crates-load-path.json` regenerated (`quirin-plugin
  load-path --extends contract-load-path.json` over `quirin-provider-libcurl`
  then `quirin-provider-openssl`, then `quirin-audit plugin views --write`):
  12 bindings added, views unchanged.
* `providers/package.py checkers` exit 0 (`private/checkers-2.log`; the run
  before re-pinning refused the stale lock as expected, `private/checkers-1.log`).
* `providers/contracts.py sync` wrote nothing further.

## 3. RETENTION-NULL-CONSTANT-ACTUAL

Gateway rule 12t is amended in place (C04.14, "No object"). `retained_by.rs`
gains `Retained::Nothing`, which decides `Outlives` in every holder class, and
`null_pointer_constant`. That reading strips matched parentheses and pointer
casts `( T * )`, then accepts `0`, `NULL` or `nullptr`; the preprocessed
`NULL` is `( ( void * ) 0 )`.

`legacy_retained_by.rs`'s `null_retention` skips such a retention, with no
record and no refusal. It applies to both the parameter-holder and the
path-holder retentions, through `decide(&Retained::Nothing, …)`. Every other
actual is read as before, and a removal naming a null constant still keys on
no object.

The reading is structural: `(const char *) (1 - 1)` and `(const char *) 1`
stay undecided. Test evidence:

* `NULL_DECISION_VECTORS` and `NULL_SPELLING_VECTORS`.
* The unit test `null_vectors_bind_the_table_and_the_reading`: a null
  retention inserted anywhere in every trace vector leaves the replay
  unchanged.
* `tests/retention_null_constant.rs`:
  * a null constant in a caller-supplied, `global` or tracked holder is
    `Safe`;
  * a local initialised to 0, a formal, a call's result, `1 - 1` and a non-null
    cast keep `retention-undecided`.

## 4. Fail-closed arms, each with a test

`crates/qcore-c-gateway/tests/openssl_unstated_callback_slots.rs` (7 tests,
Clang) reads the crate's own `ssl.qv`, `ssl_ctx.qv` and `ssl_io.qv`:

| Arm | Test | Refusal |
|---|---|---|
| a new slot's run candidate has no contract in force (the context's info setter; the SSL's verify setter with a conditional actual) | `a_new_slot_with_an_uncontracted_candidate_refuses` | `callback-uncontracted`, naming the candidate and slot |
| a new slot registered from a formal of an address-taken function | `a_new_slot_registered_from_an_undecidable_value_refuses` | `callback-member-open` |
| a new slot no loaded declaration registers (`ssl_ctx.qv` absent: session and client certificate slots) | `a_new_slot_no_declaration_registers_refuses` | `no declaration registers` |
| the premise-dropped chain loader, and `SSL_CTX_use_certificate`, with a security candidate releasing `other` | `dropping_the_premise_never_skips_an_invocation` | `callback-effectful` |
| the setters load as slot registrations, the I/O calls as nine invocations, `SSL_CTX_use_certificate` leaves the function map | `the_crate_states_every_new_slot` | n/a |

A row is `Safe` only where every candidate is the identity or the slot's run
set is empty (`identity_candidates_decide_the_io_call`).
`a_null_argument_to_a_retaining_setter_retains_nothing` checks
`SSL_CTX_set_cert_cb(c, cert_ok, 0)` against the crate's own declaration.

**Lean.**

* `QCore.OpensslUnstatedSlots` (`proof/AuditOpensslUnstatedSlots.lean`, 9
  theorems):
  * a named function is a writer or a not-writer; an unclassified name, or an
    unnamed writer, refuses;
  * the ex_data premise covers no slot, so dropping the slot premise needs
    every slot stated;
  * the verdict ignores the premise, a row is `Safe` only if every invocation
    decides, and stating an invocation never improves a verdict.
* `QCore.RetentionNullConstant` (`proof/AuditRetentionNullConstant.lean`, 10
  theorems), beside `QCore.RetentionPath`:
  * a null constant retains nothing, never records and never refuses;
  * every other actual is decided as before, and the refusals in an unknown
    or `global` holder stay;
  * a null retention is the identity on records and never changes a replay;
  * the shared vectors hold.
* Axioms are within `propext`, `Classical.choice` and `Quot.sound`.

## 5. The user path, with the real binaries

[`evidence/user-path.sh`](evidence/user-path.sh) runs the committed load path
over this card's two-unit [`evidence/fixture/`](evidence/fixture/) with the
tip binaries, writing nothing to the tree ([logs](evidence/run-1/)). Every
step exits 0: 78 bindings, all `Pending`.

| Fixture function | Status | Why |
|---|---|---|
| `configure` | `Pending` (host `Safe`) | `SSL_CTX_set_cert_cb(ctx, app_certificate, NULL)` no longer refuses: a null `arg` is retained by nothing (it was `retention-undecided` in the SSL_CTX slots card's fixture) |
| `load_certificate`, `load_chain` | `Pending` | the security slot's run set is `{app_security}`, the identity in force in this unit |
| `write_some` | `Pending` | the nine slots: verify NULL (empty), info `{app_info}`, security `{app_security}`, certificate `{app_certificate}`, all identities here; message, session and client certificate closed and empty |
| `shut` | `Pending` | info and message slots, plus the narrowed premise |
| `read_some`, `load_other` (`tls_other.c`) | `Unknown` | the run's candidates are `tls_slots.c`'s internal callbacks, with no contract in force in this unit: `callback-uncontracted` (`app_certificate`, `app_security`) |

## 6. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice take no user load path. The prediction's `measure-prediction` block
(none on all three) is for the integrator's `quirin-audit measure --declared`
against `rev:bd6b63ead`; this host has no prepared pinned-route inputs.

**One correction to the prediction's search.** The null-constant reading is
gateway-wide. The opt-in run below shows a Monkey row,
`mk_http_thread_destroy` (`lib/monkey/mk_server/mk_http_thread.c`), losing a
`pthread_setspecific` retention of `( void * ) ( ( void * ) 0 )` in `global`
(`retention-undecided`). The prediction's grep missed it (a macro-spelled
NULL). `lib/monkey` is among the corpus pin's clean paths, so the same
content change may appear on the corpus route. Here the row stays `Unknown`
on three other diagnostics (unresolved `libco_params` and `th`, an
unpublished `mk_list_del` unlink), so no move is expected there either. The
integrator's arbiter decides.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(the user load path's selection, `SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).
Inputs were prepared on this host by the base binary's `prepare` into
`private/fb-046accf6`. There were two runs, each in a `git archive` side tree
whose checker its own `package.py checkers` installed (exit 0 both):

* base: base binary and tree `bd6b63ead`;
* tip: tip binary and tree `d9acdc80e`.

The two runs overlapped in time, so their wall times are not comparable.

| Run | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings | Wall |
|---|---|---|---|---|
| base | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 66, all `Pending` | 26 min |
| tip | same | 1098 / 2 / 3699 / 5 | 78, all `Pending` | 25 min |

Both runs exit 0 with no unmatched row ([summaries](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`). **No row moved,
as predicted**: no row to `Safe`, no new `Unsafe`.

**The slot sets observed** (sidecar `callback_members.members`, all closed)
are exactly the predicted ones:

* `verify_callback` = {`rd_kafka_transport_ssl_cert_verify_cb`};
* `info_callback` = {`tls_info_callback`};
* `cert_cb` = {`rd_kafka_ssl_cert_callback`};
* `default_passwd_cb` = {`rd_kafka_transport_ssl_passwd_cb`};
* `msg_callback`, `sec_cb`, the three session slots and `client_cert_cb`:
  empty.

**5 rows change content:**

* `rd_kafka_ssl_ctx_init`, Monkey `mk_tls_init`: gain the
  `SSL_CTX_set_verify` link (their uncontracted-setter diagnostic was already
  hidden behind an earlier refusal).
* `rd_kafka_transport_ssl_close`, Monkey `context_unset`: `SSL_shutdown` now
  goes through the callback route, which refuses the object operand as an
  indirect access (`indirect memory access is outside the resource
  projection`), as the I/O rows did in the SSL_CTX slots card.
* `mk_http_thread_destroy`: the null retention refusal leaves (above).

**Where the prediction was wrong in detail:**

* No row reached candidate classification on a new slot, so no
  `callback-uncontracted` refusal for `tls_info_callback` or
  `rd_kafka_transport_ssl_cert_verify_cb` appears: every row applying the
  I/O calls refuses first on its object operand.
* Fluent Bit's own setter rows (`tls_context_set_verify_client`,
  `tls_context_create`, `tls_session_create`) and the `SSL_CTX_use_certificate`
  rows did not change content.
* Fluent Bit's `oauth2_cache_cleanup` is not in the run's compile database.
* The null retention row is Monkey's, not the one predicted.

## 7. Gate

GATE-PLACEHOLDER

## 8. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`OPENSSL-UNSTATED-CALLBACK-SLOTS` and `RETENTION-NULL-CONSTANT-ACTUAL` and
registers:

* `OPENSSL-FREE-SESSION-REMOVE-INVOCATION`: `SSL_free` / `SSL_CTX_free` may
  run the (now stated) session-remove slot, but no callback clause stands
  beside their reference drop.
* `OPENSSL-EX-DATA-REGISTRY`: ex_data callbacks are a global per-class
  registry. Only the premises on eight declarations name them; every other
  constructor and free of an ex_data class (`X509_new`, `X509_free`,
  `EVP_PKEY_free`, the PEM readers, `SSL_CTX_use_certificate`'s replaced
  certificate) reads them silently.
* `OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS`: the handshake's other slots whose
  setters are not declared (keylog, ALPN selection, certificate verify, PSK,
  client hello, record padding, early data, CT validation, custom
  extensions), and a BIO's methods and callbacks. With
  OPENSSL-CTRL-COMMAND-ARMS, they keep the handshake and I/O calls and
  `SSL_shutdown` on the premise.
* `CALLBACK-SUPPLIED-NULL-ACTUAL`: C04.19 reads a candidate's
  `resource_owned` requirement at a `_` position as met by the callee. The
  security callback's page says one of `s` and `ctx` is NULL, so a candidate
  requiring a live object there is read as satisfied by a NULL the library
  passes. That gap is in the candidate's own row, not the caller's.

Files likely to conflict with OPENSSL-PEM-CALLBACK-ARM (same crate):

* `providers/quirin-provider-openssl/src/documented.rs` (the `Premise` enum,
  the constants, `DOCUMENTED`);
* `src/check.rs` (imports, `expected_clauses`, the tests module);
* `contracts.lock` and `providers/user-crates-load-path.json`. Regenerate
  them: `quirin-audit pin` with the arguments above, `quirin-plugin load-path`
  over libcurl then openssl, `plugin views --write`;
* `doc-snapshots.json` and `pinned/openssl-3.5.5.json` (append-only on both
  sides);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.19.3;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean`. Rerun
  `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`.
