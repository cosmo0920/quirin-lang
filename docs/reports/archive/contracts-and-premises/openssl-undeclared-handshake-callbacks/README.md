# OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS: the handshake slots whose setters were undeclared

Base `65fc06fca` (main, r30). Named follow-up
`OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS` (recorded by
OPENSSL-UNSTATED-CALLBACK-SLOTS); user decision 2026-11-18.
[PREDICTION](PREDICTION.md) was committed before any implementation
(`a92868259`); the implementation is `4b8b31361`.

## 1. The slots

Every claim cites a fetched page. Nine pages were added to `doc-snapshots.json`
with `fetch_docs.py --merge` (2026-11-18), 24 entries in all (three series
where they exist; `SSL_CTX_set_keylog_callback@1.0.2` and
`SSL_extension_supported@1.0.2` are recorded as not found). The 21 setters'
prototypes were pinned with `pin_prototypes.py` over every recorded function
plus the new ones; every existing entry and every header digest is
byte-identical.

Each documented callback is one key. Where the page names an SSL setter
beside the context's, both share the key (the pages say the SSL inherits the
context's value at `SSL_new`). The new module is `ssl_handshake.qv`.

| Slot (`ssl_ctx_st.…`) | Writers (the 3.5 page's) | Page; range | Invocation (actuals) |
|---|---|---|---|
| `keylog_callback` | `SSL_CTX_set_keylog_callback` | `SSL_CTX_set_keylog_callback`; `documented "1.1.1"` | `(ssl, _)` |
| `alpn_select_cb` | `SSL_CTX_set_alpn_select_cb` (keeps `arg`) | `SSL_CTX_set_alpn_select_cb`; `documented "1.0.2"` | `(ssl, _, _, _, _, _)` |
| `npn_select_cb` | `SSL_CTX_set_next_proto_select_cb` (keeps `arg`) | same page; `documented "1.1.1"` | `(ssl, _, _, _, _, _)` |
| `npn_advertised_cb` | `SSL_CTX_set_next_protos_advertised_cb` (keeps `arg`) | same page; `documented "1.1.1"` | `(ssl, _, _, _)` |
| `app_verify_callback` | `SSL_CTX_set_cert_verify_callback` (keeps `arg`) | `SSL_CTX_set_cert_verify_callback`; `since "0.9.7"` (HISTORY 1.0.2: `arg` ignored before 0.9.7) | `(_, _)` |
| `psk_client_callback` | `SSL_CTX_set_psk_client_callback`, `SSL_set_psk_client_callback` | `SSL_CTX_set_psk_client_callback`; `documented "1.0.2"` | `(ssl, _, _, _, _, _)` |
| `psk_use_session_cb` | `SSL_CTX_set_psk_use_session_callback`, `SSL_set_…` | same page; `since "1.1.1"` | `(ssl, _, _, _, _)` |
| `psk_server_callback` | `SSL_CTX_set_psk_server_callback`, `SSL_set_…` | `SSL_CTX_use_psk_identity_hint`; `documented "1.0.2"` | `(ssl, _, _, _)` |
| `psk_find_session_cb` | `SSL_CTX_set_psk_find_session_callback`, `SSL_set_…` | same page; `since "1.1.1"` | `(ssl, _, _, _)` |
| `client_hello_cb` | `SSL_CTX_set_client_hello_cb` (keeps `arg`) | `SSL_CTX_set_client_hello_cb`; `since "1.1.1"` | `(ssl, _, _)` |
| `record_padding_cb` | `SSL_CTX_set_record_padding_callback`, `SSL_set_…` | `SSL_CTX_set_record_padding_callback`; `since "1.1.1"` | `(ssl, _, _, _)` |
| `allow_early_data_cb` | `SSL_CTX_set_allow_early_data_cb`, `SSL_set_…` (keep `arg`) | `SSL_read_early_data`; `since "1.1.1"` | `(ssl, _)` |
| `custom_ext_{add,free,parse}_cb_ex` | `SSL_CTX_add_custom_ext` (keeps `add_arg`, `parse_arg`) | `SSL_extension_supported`; `since "1.1.1"` | add `(ssl, _×8)`, free `(ssl, _×4)`, parse `(ssl, _×8)` |
| `custom_ext_{add,free,parse}_cb` | `SSL_CTX_add_client_custom_ext`, `SSL_CTX_add_server_custom_ext` (keep both args) | same page; `documented "1.1.1"` | add `(ssl, _×5)`, free `(ssl, _×3)`, parse `(ssl, _×5)` |

`SSL_write`, `SSL_read`, `SSL_connect` and `SSL_do_handshake` invoke all 18,
beside the nine stated before (27 slot invocations each). `SSL_shutdown` adds
the record padding slot: the callback "is invoked for every record before
encryption", whose type may be `SSL3_RT_ALERT`, and the close_notify alert is
a record.

**The checker** (`documented.rs`, `check.rs`):

* 18 new `SLOTS`, each with `not_writers` covering the rest of its page's
  NAME list. The r30 exactness check holds for every new page: getters,
  callback type names, the `_arg` setters, block padding, the early data I/O
  and limit functions, the ClientHello accessors and
  `SSL_extension_supported` are not-writers.
* A setter may register several slots. `Premise::SlotRegistered` states a
  registration beside the shape's, and `Premise::RetainedBy` a second
  retention. `registered_slots` feeds both directions of
  `slot_writers_declared`, so each custom extension setter is a writer of
  each of its three slots, and nothing else may register them.
* `unsigned int` maps to `u32` (the custom extension `ext_type` and
  `context`). The 16 callback typedefs the setters take are
  `FUNCTION_POINTER_TYPEDEFS`, each checked as `typedef R (*NAME)(P…);` in the
  pinned `openssl/ssl.h`.
* Two discrepancies between page and header are resolved by the header, the
  cross-check: `SSL_CTX_set_client_hello_cb`'s synopsis spells
  `SSL_client_hello_cb_fn *f`, and HISTORY says
  `SSL_CTX_set_record_padding_callback` returns `int` since 3.0, but the
  3.5.5 header declares it `void` (the SSL setter returns `int`).

Drift tests (`a_drifted_handshake_slot_reading_is_refused`,
`a_multi_slot_writer_writes_each_slot`) refuse each of these:

* each of the 18 new invocations dropped from `SSL_connect`;
* the keylog's connection read as `_`, or the verify callback handed the
  connection;
* the handshake premise dropped;
* `SSL_shutdown`'s record padding invocation dropped;
* a retained `arg` dropped;
* each of the custom extension setter's three registrations and two
  retentions dropped, or an `_ex` callback read as an older-style one;
* a page naming one more custom extension writer (refuses all six slots);
* the CT validation slot (not one this checker documents).

## 2. What stays under the premise, and why

`openssl_unstated_callback_slots` keeps its statement, which already names
what is left by kind ("a callback slot … whose invocation the declaration
does not state, …, a method or callback of a BIO the SSL object holds, or an
ex_data callback"). It stays on the same five declarations (the four
handshake and I/O calls, `SSL_shutdown`). Each remaining part is a named
follow-up:

* **The `SSL_CTX_callback_ctrl` arms**: OPENSSL-CTRL-COMMAND-ARMS, out of
  scope.
* **CT validation.** `SSL_CTX_set_ct_validation_callback` (3.5) names
  `SSL_CTX_enable_ct` / `SSL_enable_ct`, which set the library's own
  permissive or strict callback. They take no function pointer, so no
  registration can declare them as writers, and a run set built from what
  setters are handed would miss that value. Declaring only the two function
  pointer setters would contradict premise `P-OPAQUE-SLOT-WRITERS`, and
  listing `enable_ct` as a not-writer would hide a writer. So the slot is not
  stated (Lean `a_library_written_slot_is_never_accepted`).
  OPENSSL-CT-VALIDATION-LIBRARY-WRITER.
* **BIOs.** The brief allowed this to stay under a narrowed premise; it does,
  because no sound reading fits this card:
  * a method table is a separate object (`BIO_meth_new`, the `BIO_meth_set_*`
    writers) that reaches the SSL through `BIO_new` and `SSL_set_bio`;
  * the built-in methods are library values no setter is handed (the CT
    problem again);
  * a BIO's callback (`BIO_set_callback_ex`) is also run by `BIO_free`
    (`BIO_CB_FREE`) beside a reference drop, where no callback clause may
    stand.

  That last point is a finding: the crate's `BIO_free` reads that callback
  silently today. OPENSSL-BIO-METHODS-AND-CALLBACKS.
* **Setters still undeclared.** The pinned header has more function pointer
  setters of handshake slots outside the follow-up's list: session ticket,
  not-resumable, session id generation, temporary DH, cookie and stateless
  cookie, session secret, ticket extension, SRP, async, QUIC pending
  connection, and the context store's `X509_STORE_set_verify_cb`.
  OPENSSL-REMAINING-HANDSHAKE-SETTERS.
* **ex_data**: OPENSSL-EX-DATA-REGISTRY, out of scope.

**A boundary the new pages expose.** Several pages put obligations on what
the callback stores through a pointer the library supplies:

* ALPN `out` "may point directly into in, or to a buffer that outlives the
  handshake";
* NPN advertisement: "the SSL does keep a reference to it";
* PSK `*sess`: "Ownership of the SSL_SESSION object is passed to the OpenSSL
  library";
* a custom extension add callback's `*out` goes to its free callback.

C04.19 reads a `_` actual as the callee's own, so a candidate whose contract
is silent about the pointee is admitted as the identity. That gap is in the
candidate's own row, like r30's CALLBACK-SUPPLIED-NULL-ACTUAL, and is
registered as CALLBACK-SUPPLIED-OUTPUT-OBLIGATION.

**Nothing becomes `Safe` by this card alone.** Each stated invocation adds an
obligation. A run that registers an uncontracted or effectful candidate in a
new slot now refuses calls that before rested on the premise alone. A slot
nothing registers refuses even when its set is empty.

**Lock.** `quirin-audit pin` (release binary, toolchain env;
`--source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture
--crate quirin-provider-openssl`; `private/logs/pin-2.log`):

* 21 entries added (`openssl/ssl_handshake.qv`);
* the 5 re-stated declarations' digests changed (`SSL_connect`,
  `SSL_do_handshake`, `SSL_read`, `SSL_shutdown`, `SSL_write`);
* 67 entries unchanged, and `pinned`, `library_versions` and every other
  top-level key unchanged.

The other steps, in order (`private/logs/*-2.log`):

* `quirin-plugin load-path --extends contract-load-path.json` over libcurl
  then openssl: 21 bindings added;
* `plugin views --write`: views unchanged;
* `providers/contracts.py sync`: wrote nothing further;
* `providers/package.py checkers`: exit 0.

A first attempt ran `checkers` first, as the brief lists the steps
(`private/logs/*-1.log`). Its views check refused the stale load path that
lacked the new module, and the first `load-path` call was given a path the
tool resolves relative to the output (`providers/providers/…`). Both were
reverted with nothing committed, and the order above was used.

## 3. Fail-closed arms, each with a test

`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs` (6
tests, Clang) reads the crate's own `ssl.qv`, `ssl_ctx.qv`, `ssl_io.qv` and
`ssl_handshake.qv`:

| Arm | Test | Refusal |
|---|---|---|
| an uncontracted keylog or ALPN candidate (Fluent Bit's shape) | `a_new_slot_with_an_uncontracted_candidate_refuses` | `callback-uncontracted`, naming candidate and slot |
| a multi-slot setter whose free callback releases the `add_arg` it is handed, beside identity add and parse callbacks | `every_registration_of_a_multi_slot_setter_is_read` | `callback-effectful` |
| a new slot no loaded declaration registers (`ssl_handshake.qv` absent) | `a_new_slot_no_declaration_registers_refuses` | `no declaration registers` (also on `SSL_shutdown`) |
| `SSL_shutdown` with an uncontracted record padding candidate | `shutdown_reads_the_record_padding_slot` | `callback-uncontracted` |
| the setters load as their registrations (three for a custom extension setter); the I/O calls as 27 invocations, `SSL_shutdown` as 3 | `the_crate_states_every_handshake_slot` | n/a |

A row is `Safe` only where every candidate is the identity
(`identity_candidates_decide_the_io_call`; null `arg`s retain nothing, rule
12t).

r30's `openssl_unstated_callback_slots.rs` now loads `ssl_handshake.qv` too,
since `ssl_io.qv`'s I/O calls invoke its slots, and expects 27 invocations
instead of 9. Without that module its identity test refused
`ssl_ctx_st.keylog_callback` ("no declaration registers"), as it should.

**Lean.** `QCore.OpensslHandshakeSlots`
(`proof/AuditOpensslHandshakeSlots.lean`, 10 theorems):

* a further registration is a registration;
* an entry registering a slot it is not listed for refuses, and so does a
  listed writer whose entry dropped the registration;
* accepted writers are exactly the registering entries;
* a library-written slot is never accepted, and a slot a declaration may run
  unstated needs the premise;
* stating more invocations never improves a verdict.

Axioms are within `propext`, `Classical.choice` and `Quot.sound`.

## 4. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice take no user load path, and no gateway code changed. The prediction's
`measure-prediction` block (none on all three) is for the integrator's
`quirin-audit measure --declared`.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(the user load path's selection, `SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Inputs.** The prepared 046accf6 inputs of earlier cards were gone with
  their worktrees, so they were prepared again by the binary's `prepare`
  (`--fluent-bit ../fluent-bit --revision 046accf6`) into
  `private/fb-046accf6`, exit 0.
* **Side trees.** Each run used a `git archive` side tree whose checker its
  own `package.py checkers` installed (exit 0 both): base `65fc06fca`, tip
  `4b8b31361`.
* **Binary.** Both runs used one release `quirin-audit` built at the base.
  The card changes no crate source outside tests, so the tip's binary is the
  same program.
* **Timing.** The runs overlapped in time, so wall times are not compared.

| Run | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|
| base | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 78, all `Pending` |
| tip | same | 1098 / 2 / 3699 / 5 | 99, all `Pending` |

Both runs exit 0 with no unmatched row
([runs, invocations](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`). **No row moved,
as predicted**: no row to `Safe`, no new `Unsafe`, and **no row changed
content**.

**The slot sets observed** ([sidecar](evidence/opt-in/slot-sets.txt),
`callback_members.members`, all closed) are as predicted:

* `alpn_select_cb` = {`tls_context_server_alpn_select_callback`};
* `keylog_callback` = {`ssl_key_logger`} (the production database defines
  `FLB_HAVE_DEV`);
* the other sixteen new slots are empty;
* the nine earlier slots are unchanged.

**Where the prediction was wrong in detail.** The predicted content changes
(`tls_context_alpn_set`, `tls_context_reload`, `tls_context_create` gaining
the setter links) did not happen. Those functions are in `src/tls/openssl.c`,
which `src/tls/flb_tls.c` includes, and that unit is not among the run's 180
files. The run's member facts read the whole compile database, which is why
the slot sets name these callbacks, but no row is analysed for that unit. Every
row of the selection applying an I/O call (`rdkafka_ssl.c`, Monkey's
`tls_openssl.c`) refuses first on its object operand, as in r30, so no row
reaches candidate classification on a new slot.

## 5. Gate

Command: `python3 scripts/gate_scope.py --base 65fc06fca --out private/gate-1`,
run through `private/toolchain/env.sh` (Clang 21, Lean 4.35.0).

* Tree: `4b8b31361`, the implementation. The report and amendments were not
  yet written; they are docs only.
* Mode: the scoped card gate (44 test targets and 7 packages selected), with
  the provider gate included.
* Record: [`gate/gate-1.json`](gate/gate-1.json). Logs: `private/gate-1/`
  and `private/logs/gate-1.log`.
* Result: `gate_scope` exit 1, on one Python test (below). Every other step
  exits 0.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (four scoped runs) | 0 | 1 submodule (`third_party/libevent` at `d82464a27`) |
| Rust suite (scoped) | 0 | 274 units, 281 summaries; 3445 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 450 audits (new `QCore.OpensslHandshakeSlots`) |
| Python suite | **1** | 779 tests, 1 failure |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 157 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

**The Python failure, disclosed and not fixed.**
`test_gate_scope_census.CensusAgreesWithTheModel.test_the_census_covers_the_suite`
asserts that the recorded read census
(`docs/reports/gate-scope-precise-selection/read-census.json`, 645 units)
covers more than 90% of the tree's root test units. The card's new test target
`openssl_undeclared_handshake_callbacks` brings the tree to 717 units, and
645 is not greater than 645.3, so the check fails. Nothing is wrong with the
selection: the census is one commit's observation, and any card adding a
target now crosses the line. The discharge is
GATE-READ-CENSUS-REFRESH's: re-record the census with
`scripts/gate_read_census.py` (about 25 minutes, all root units under strace).
That is integrator work, so it was not done here. Neither the
`test_gate_slot_lane` flake nor a stale-matrix check failed in the gate's
Python run.

A plain rerun of the Python suite on the final tree `aff696736`
(`private/logs/python-final.log`) failed two of 779 tests:

* the same census check;
* the stale-matrix check
  `test_the_derived_facts_are_recomputed_rather_than_carried`. The committed
  matrix's `lean_audits` reads 449 against the 450 produced, the new audit.
  It is regenerated when the integrator applies the amendments.

Outside the gate, during the work:

* the crate's checker tests: 14 passed;
* `user_crate.rs`: 2 passed;
* the three OpenSSL gateway test files (`openssl_callback_slots`,
  `openssl_unstated_callback_slots`, `openssl_undeclared_handshake_callbacks`):
  22 passed.

## 6. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS` and registers:

* OPENSSL-BIO-METHODS-AND-CALLBACKS;
* OPENSSL-CT-VALIDATION-LIBRARY-WRITER;
* OPENSSL-REMAINING-HANDSHAKE-SETTERS;
* CALLBACK-SUPPLIED-OUTPUT-OBLIGATION.

Files likely to conflict with other OpenSSL cards (OPENSSL-CTRL-COMMAND-ARMS,
OPENSSL-EX-DATA-REGISTRY, OPENSSL-FREE-SESSION-REMOVE-INVOCATION):

* `providers/quirin-provider-openssl/src/documented.rs` (`Premise`, `SLOTS`,
  `HANDSHAKE`, `SHUTDOWN`, `DOCUMENTED`) and `src/check.rs`;
* `contracts/openssl/ssl_io.qv` and `ssl.qv` (the five declarations);
* `contracts.lock` and `providers/user-crates-load-path.json`. Regenerate
  them in this order: `quirin-plugin load-path` from `providers/`, `plugin
  views --write`, `quirin-audit pin`, `contracts.py sync`, `package.py
  checkers`;
* `doc-snapshots.json` and `pinned/openssl-3.5.5.json` (append-only);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.19.3;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean`. Rerun
  `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`.
