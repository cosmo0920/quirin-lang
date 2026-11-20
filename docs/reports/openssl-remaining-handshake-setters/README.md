# OPENSSL-REMAINING-HANDSHAKE-SETTERS: the handshake setters still undeclared

* **Base:** `64114a447` (main, r37).
* **Card:** the named follow-up recorded by OPENSSL-UNDECLARED-HANDSHAKE-CALLBACKS.
* **Commits:**
  * [PREDICTION](PREDICTION.md) `8678baa8b`, committed before any implementation;
  * the implementation, `8fffb83ae`;
  * the store's cleanup key re-spelled after the opt-in run, `f1ad773c0` (§4).

## 1. What was declared and stated

**Page evidence.** Every claim cites a fetched page:

* **Snapshots.** Nine pages were added to `doc-snapshots.json` with
  `fetch_docs.py --merge` (2026-11-20, 28 entries over the series where the
  page exists, absences recorded as not found).
* **Prototypes.** The 28 setters were pinned with `pin_prototypes.py` over
  every recorded function plus the new ones. Every existing prototype and
  every header digest is byte-identical.
* **NAME check.** Each page's NAME list is exactly its slots' writers and
  not-writers (`slot_writers_declared`).

**Where things live.**

* The setters are in two new modules: `ssl_handshake_setters.qv` (16) and
  `x509_store_slots.qv` (12).
* The invocations are on `SSL_write`, `SSL_read`, `SSL_connect` and
  `SSL_do_handshake`. Each now states 67 slot invocations, up from 41.
* `SSL_shutdown` adds the async slot: 18 invocations, up from 17.

| Slot | Writers (3.5 page) | Range | Invocation actuals | Invoked by |
|---|---|---|---|---|
| `ssl_ctx_st.ticket_gen_cb`, `.ticket_dec_cb` | `SSL_CTX_set_session_ticket_cb` (registers both, keeps `arg`) | `since "1.1.1"` | `(ssl, _)`, `(ssl, _×5)` | 4 calls |
| `ssl_ctx_st.generate_session_id` | `SSL_CTX_set_generate_session_id`, `SSL_set_generate_session_id` | `since "0.9.7"` (1.0.2 HISTORY) | `(ssl, _, _)` | 4 calls |
| `ssl_ctx_st.tmp_dh_callback` | `SSL_CTX_set_tmp_dh_callback`, `SSL_set_tmp_dh_callback` | `documented "1.0.2"` | `(ssl, _, _)` | 4 calls |
| `.gen_stateless_cookie_cb`, `.verify_stateless_cookie_cb` | `SSL_CTX_set_stateless_cookie_{generate,verify}_cb` | `since "1.1.1"` | `(ssl, _, _)` | 4 calls |
| `.app_gen_cookie_cb`, `.app_verify_cookie_cb` | `SSL_CTX_set_cookie_{generate,verify}_cb` | `documented "1.1.1"` | `(ssl, _, _)` | 4 calls |
| `ssl_st.session_secret_cb` | `SSL_set_session_secret_cb` (keeps `arg`) | `documented "3.2"` (no page at 3.0, 3.1) | `(ssl, _, _, supplied_nullable, _, _)` | 4 calls |
| `.srp_username_callback`, `.srp_client_pwd_callback`, `.srp_verify_param_callback` | the three `SSL_CTX_set_srp_*_callback` | `since "1.0.1"` (deprecated 3.0, not removed) | `(ssl, _, _)`, `(ssl, _)`, `(ssl, _)` | 4 calls |
| `ssl_ctx_st.async_cb` | `SSL_CTX_set_async_callback`, `SSL_set_async_callback` | `since "3.0.0"` | `(ssl, _)` | 4 calls and `SSL_shutdown` |
| `ssl_ctx_st.new_pending_conn_cb` | `SSL_CTX_set_new_pending_conn_cb` (keeps `arg`) | `since "3.5.0"` | `(_, _, _)` | 4 calls |
| `x509_store_st.verify_cb`, `.verify`, `.get_issuer`, `.check_issued`, `.check_revocation`, `.get_crl`, `.check_crl`, `.cert_crl`, `.check_policy`, `.lookup_certs`, `.lookup_crls`, `.cleanup_fn` | one `X509_STORE_set_*` each | `since "1.1.0"` | every actual `_` | 4 calls |

**Readings.**

* **Shared keys.** Where a page names the SSL's setter beside the
  context's, both share one key, as in the earlier slot cards.
* **The connection as an actual.** It is passed (`ssl`) where the page
  names the connection: "The SSL object for the current connection is
  supplied" (DH), "the SSL connection s" (SRP), and "the SSL_SESSION ss has
  not yet been assigned to the SSL s" (tickets). The QUIC callback gets the
  listener's context and a preallocated connection, so every actual there
  is `_`.
* **`SSL_shutdown` and async.** `SSL_shutdown` invokes async because an
  "asynchronous SSL call" runs it "if a retry occurs", and `SSL_shutdown` is
  such a call under `SSL_MODE_ASYNC`.
* **Cookies.** The cookie pages name `SSL_stateless` and `DTLSv1_listen` as
  their users, and neither is declared. Stating the cookie slots on the
  handshake covers a DTLS server handshake with cookie exchange too, and
  stating more never improves a verdict.
* **The certificate store.** "All the callbacks from a X509_STORE are
  inherited by the corresponding X509_STORE_CTX structure when it is
  initialized", and the handshake verifies the peer's chain through one.
  * The store reaches the connection only through other calls
    (`SSL_CTX_new`, `SSL_CTX_set_cert_store`). Its keys are run-wide, as for
    a BIO method table.
  * A NULL store slot makes the library use "the internal default
    function", which is the call's own behaviour, not a value of the slot.
* **Macro aliases.** `X509_STORE_set_verify_cb_func`,
  `X509_STORE_set_verify_func` and `X509_STORE_set_lookup_crls_cb` are
  "available as macros" over their writers. They are listed as not-writers:
  they have no pinned prototype and no declaration, and a call through one
  is the writer's call.
* **HISTORY contradiction.** The page names `X509_STORE_set_verify_cb`
  both as "added in OpenSSL 1.0.0" and in the 1.1.0 sentence. The later
  bound is taken.
* **Header over page.** Two synopses disagree with the pinned header, which
  is the cross-check:
  * `SSL_set_generate_session_id`: the page has `GEN_SESSION_CB , cb`;
  * `SSL_CTX_set_new_pending_conn_cb`: the page has
    `SSL_set_new_pending_conn_cb_fn * f`, the header
    `SSL_new_pending_conn_cb_fn cb`.

**The checker** (`documented.rs`, `check.rs`):

* 26 `SLOTS`;
* 28 `DOCUMENTED` entries, the session ticket setter with a second
  registration (`Premise::SlotRegistered`);
* the 18 new callback typedefs in `FUNCTION_POINTER_TYPEDEFS`;
* one `NULLABLE_ARGUMENTS` entry (the session secret's `peer_ciphers`, "On
  the client side the peer_ciphers stack will always be NULL."). No other
  new page states a callback argument NULL.

`a_drifted_remaining_setter_reading_is_refused` refuses:

* each of the 26 invocations dropped from each of the four calls;
* `peer_ciphers` read as `_`;
* the QUIC callback handed `ssl`;
* a store callback handed `ssl`;
* the old premise statement, on `SSL_connect`, `SSL_shutdown`, `SSL_free`
  and `SSL_CTX_free`;
* `SSL_shutdown`'s async invocation dropped;
* the ticket setter's second registration or its retention dropped.

The same test pins two further facts:

* the aliases have no prototype, no entry and are not-writers;
* the undocumented setters have no entry and no fetched page.

## 2. What stays under `openssl_unstated_callback_slots`, and why

The premise stays on the same seven declarations: the four handshake and
I/O calls, `SSL_shutdown`, `SSL_free` and `SSL_CTX_free`. Its statement is
rewritten to name what it still covers, so no coverage was dropped
unremarked. Before, the statement named only the ctrl arms and ex_data
beside the generic clause. It now names:

* **The `SSL_CTX_callback_ctrl` arms** (servername, status, ticket key):
  OPENSSL-CTRL-COMMAND-ARMS.
* **CT validation.** `SSL_CTX_enable_ct` writes the library's own callback:
  OPENSSL-CT-VALIDATION-LIBRARY-WRITER (appended: the phrase now stands in
  the statement).
* **The not-resumable session and session ticket extension callbacks.**
  `SSL_CTX_set_not_resumable_session_callback`,
  `SSL_set_not_resumable_session_callback` and
  `SSL_set_session_ticket_ext_cb` are in the pinned header, but
  `docs.openssl.org/3.5/man3/<name>/` is not found for any of them and no
  page's NAME names them. No page supports a writer list, so they are not
  declared. New follow-up: OPENSSL-UNDOCUMENTED-HANDSHAKE-SETTERS (needs a
  documented source or a user decision).
* **The lookup methods of the store's `X509_LOOKUP`s.** The default issuer
  and certificate lookup reach `get_by_subject` and the other method
  functions, and the built-in methods are library values.
  OPENSSL-FREE-INDIRECT-CALLBACKS (registered, not running) declares the
  `X509_LOOKUP_meth_set_*` setters for the frees, so this card did not
  declare them in parallel. New follow-up:
  OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS.
* **ex_data**, beside `openssl_ex_data_callbacks`.

On `SSL_shutdown` the generic clause also still covers the slots it does
not state (a post-handshake message's callbacks).

`SSL_free` and `SSL_CTX_free` change only by the statement. They state
nothing new and keep the premise, which is OPENSSL-FREE-INDIRECT-CALLBACKS'
discharge (appended).

**Boundaries the pages expose**, each a follow-up:

* **Callbacks outside any call.** The async callback is also run by the
  engine "via a polling mechanism or via an interrupt". QUIC handshake
  processing may run in a library thread, and one QUIC object's call may run
  another connection's callbacks through the shared reactor, against the
  `ssl` actuals stated since r29. No declaration states an invocation
  outside its call: OPENSSL-CALLBACKS-OUTSIDE-CALLS.
* **What a candidate hands back.** The SRP password is "freed by the
  library", `get_issuer` "must up-ref" `*issuer`, the lookup stacks are
  returned, and QUIC's `new_ssl` must not be referenced:
  CALLBACK-SUPPLIED-OUTPUT-OBLIGATION (appended).
* **New `_` arguments that may be NULL** (the kept `arg`s, the ticket
  `dec_cb`'s `ss`): CALLBACK-UNSET-ARGUMENT-NULL (appended).
* **The verification context's own setters.** `X509_STORE_CTX_set_verify_cb`
  and `X509_STORE_CTX_set_verify` write the verification context's copy,
  which during a handshake only a callback can reach. They are not declared,
  so a candidate calling one has no contract in force.

**Nothing becomes `Safe` by this card alone.** Each invocation is one more
obligation (`QCore.OpensslRemainingSetters.stating_the_remaining_slots_never_improves_a_verdict`).

## 3. Lock, load path and generated files

The regeneration ran in this worktree in the fresh-worktree order. The
driver is `private/regen.sh`, re-run as `private/regen2.sh` after
`f1ad773c0`; logs are `private/logs/regen{,2}-*.log`, exits captured
unpiped:

| Step | Exit (both runs) |
|---|---|
| `quirin-plugin load-path --extends contract-load-path.json` from `providers/`, libcurl then openssl | 0 |
| `providers/package.py checkers` (installs, then refuses the stale views) | 1, as expected |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin --load-path providers/user-crates-load-path.json --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate quirin-provider-openssl` | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` | 0 |

`quirin-audit` is the release binary built at the base: the card changes no
crate source outside tests. The diff was reviewed against base:

* 28 entries added;
* 7 `contract_sha256` and `pinned_at` changes:
  * `SSL_connect`, `SSL_do_handshake`, `SSL_read`, `SSL_write`,
    `SSL_shutdown` (invocations and statement);
  * `SSL_free`, `SSL_CTX_free` (statement only);
* 100 entries unchanged;
* `pinned`, `library_versions` and every other top-level key unchanged.

Other files:

* **`user-crates-load-path.json`**: 28 bindings, two modules.
* **`quirin-plugin.json`**: 28 bindings, by hand at their module positions.
* **The attribution tree's lock** has the same digests as the final one;
  only `pinned_at` differs.
* **Generators.** `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`
  were rerun (`docs/agents/gateway.md`, `proof/QCore.lean`).

**Specification and rule.**

* C04.18: the OpenSSL premise paragraph gains "The remaining handshake
  setters".
* C04.19.3: the opaque slot bullet gains the store as a holder, macro
  aliases as not-writers and undocumented setters never declared.
* Rule 12n is amended in place. No new rule id was needed.

## 4. Measurement

**Pinned routes: no move predicted, arbiter deferred.** Corpus, system and
slice read no user load path, and no gateway code changed. The prediction's
`measure-prediction` block (none on all three) is for the integrator's
`quirin-audit measure --declared`. That command cannot run on this host: the
pinned inputs and published baselines are absent, as for r30 to r37.

**The opt-in `current` run (the arbiter).** It ran `current --monkey-linked`
over Fluent Bit 046accf6 with OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Inputs.** They were prepared by the base binary's `prepare` into
  `private/fb-046accf6` (exit 0).
* **Side trees.** Each run used a `git archive` side tree, and that tree's
  own `package.py checkers` exited 0. Drivers are in
  [evidence/opt-in](evidence/opt-in/).

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base-1 | `64114a447` | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| tip-1 | `8fffb83ae` | same | **1092** / 2 / **3705** / 5 | 141, all `Pending` |
| tip-2 (confirmation) | `8fffb83ae` | same | 1092 / 2 / 3705 / 5 (no row differs from tip-1) | 141 |
| variant-1 (attribution, not committed) | `8fffb83ae` with the key `x509_store_st.cleanup` spelled `cleanup_fn` | same | 1098 / 2 / 3699 / 5, no row differs from base | 141 |
| tip-3 (final) | `f1ad773c0` | same | **1098 / 2 / 3699 / 5, no row changed** | 141, all `Pending` |

**At `8fffb83ae`, six `Safe` rows moved to `Unknown`**
([comparison](evidence/opt-in/comparison-base-tip1.json)):

* librdkafka's `rd_kafka_event_destroy` and `ut_op_destroy_free`;
* Monkey's `mk_sched_threads_destroy_all`, `mk_sched_threads_destroy_conn`,
  `mk_sched_threads_purge` and `sched_thread_cleanup`.

128 rows changed content. **None of this was an OpenSSL reading.** The
cause was the member pre-pass:

* The segment `cleanup` of the new key `x509_store_st.cleanup` is a
  closure-graph token.
* WAMR's `wasm_loader.c` names `cleanup` (twice), and no other new key
  segment, so the pre-pass now read it.
* Its Clang AST JSON exceeds the reader's recursion limit ("unsupported
  Clang AST JSON: recursion limit exceeded at line 962996 column 264"), so
  the unit stays unread.
* An unread C unit opens every member (rule 12n,
  `member_prepass.rs`). Every member closure then carried "1 unit(s) of the
  run may name it but were not read". That demoted the six rows,
  fail-closed, and changed the refusal text of 122 Unknown rows. The rows
  calling `SSL_free` and `SSL_CTX_free`, for example, lost their
  `reference-drop-unheld` leaf to an earlier refusal.

The tip-2 rerun showed it deterministic. The attribution variant, which
spelled the key after the page's `X509_STORE_CTX_cleanup_fn`, reproduced
base exactly ([comparison](evidence/opt-in/comparison-base-variant.json)).

A key is only the slot's name: the run never defines `struct x509_store_st`.
So `f1ad773c0` adopts `x509_store_st.cleanup_fn`, and the final run matches
base row for row ([comparison](evidence/opt-in/comparison-base-tip.json)).

**Against the prediction:**

* no status move: met at the final tree;
* bindings: 113 → 141, all `Pending`, as predicted (+28);
* no content change at the final tree: met.

The prediction did not foresee a key segment pulling in an unreadable unit.
The reader's fragility is a follow-up, MEMBER-PREPASS-UNREADABLE-TOKEN-UNIT.
The run does not list slot sets separately: like r33 to r37, the report
carries no `callback_members` sidecar. None of the 28 setters is called in
Fluent Bit's or Monkey's sources, so every new set is the empty closed one.

## 5. Tests and Lean

**Gateway tests.** They live in the existing target
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`, so
the read census needs no new unit. Six new tests, with Clang:

| Arm | Test | Reading |
|---|---|---|
| the declarations | `the_crate_states_every_remaining_slot` | 28 setters' registrations (two for the ticket setter); each of the 26 keys once per call with the actuals of §1; `SSL_shutdown` states async and no store slot |
| identity candidates (store, async, ticket pair, secret), NULL `arg` | `identity_remaining_candidates_decide_the_io_call` | `write_some` and `shut` `Safe`, no `callback-` refusal |
| an uncontracted store or async candidate | `a_remaining_slot_with_an_uncontracted_candidate_refuses` | `callback-uncontracted` naming it (async on `shut` too); the store candidate leaves `shut` `Safe` |
| the ticket setter's second registration | `both_registrations_of_the_session_ticket_setter_are_read` | `callback-effectful` for a `dec_cb` releasing `ss` |
| `peer_ciphers` NULL on a client | `a_session_secret_candidate_requiring_the_peer_ciphers_refuses` | `callback-supplied-null` |
| this card's modules absent | `a_remaining_slot_no_declaration_registers_refuses` | every new key undecided; `no declaration registers` |

The existing counts moved:

* the I/O calls from 41 to 67, `SSL_shutdown` from 17 to 18;
* `openssl_unstated_callback_slots.rs` loads the two modules and expects 67.

**Checker tests.** 19 tests, including the new drift test. `SLOTS` grows
from 42 to 68 and `NULLABLE_ARGUMENTS` from 9 to 10.

**Lean.** The new module is `QCore.OpensslRemainingSetters`, audited by
`proof/AuditOpensslRemainingSetters.lean` (9 theorems):

* a slot with no fetched page is never accepted and needs the premise;
* an alias call and its writer's call give the same candidates;
* a statement drops a kind only when its slots are stated, and stating
  slots keeps it honest;
* stating more invocations never improves a verdict.

Axioms are within `propext`, `Classical.choice` and `Quot.sound`.

## 6. Gate

Command: `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
64114a447 --out private/gate-N` (Clang 21, Lean 4.35.0).

| Gate | Tree | Result | Record |
|---|---|---|---|
| gate-1 | `8fffb83ae` | exit 0, 18 steps passed | [`gate/gate-1.json`](gate/gate-1.json), `private/gate-1/`, `private/logs/gate-1.log` |
| gate-2 | `f1ad773c0` | exit 0, 18 steps passed (same counts: 3468 Rust passed, 457 audits, 779 Python, 162 provider) | [`gate/gate-2.json`](gate/gate-2.json), `private/gate-2/`, `private/logs/gate-2.log` |

Steps and counts at gate-1:

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (five scoped runs) | 0 | `third_party/libevent` at `d82464a27` |
| Rust suite (scoped, 44 targets, 7 packages) | 0 | 274 units, 281 summaries; 3468 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 457 audits |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 162 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

Neither `test_gate_slot_lane` nor `test_gate_slot_queue` flaked. The census
check passed, since no target was added.

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) (`--check
--profiles-base 64114a447` exits 0) retires
OPENSSL-REMAINING-HANDSHAKE-SETTERS.

It registers:

* OPENSSL-UNDOCUMENTED-HANDSHAKE-SETTERS;
* OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS;
* OPENSSL-CALLBACKS-OUTSIDE-CALLS;
* MEMBER-PREPASS-UNREADABLE-TOKEN-UNIT.

It appends to:

* CALLBACK-SUPPLIED-OUTPUT-OBLIGATION;
* CALLBACK-UNSET-ARGUMENT-NULL;
* OPENSSL-FREE-INDIRECT-CALLBACKS;
* OPENSSL-CT-VALIDATION-LIBRARY-WRITER.

**Files likely to conflict** with other OpenSSL cards
(OPENSSL-FREE-INDIRECT-CALLBACKS, OPENSSL-CTRL-COMMAND-ARMS,
OPENSSL-CT-VALIDATION-LIBRARY-WRITER):

* `providers/quirin-provider-openssl/src/documented.rs`
  (`UNSTATED_SLOTS_STATEMENT`, `SLOTS`, `NULLABLE_ARGUMENTS`, `HANDSHAKE`,
  `SHUTDOWN`, `DOCUMENTED`) and `src/check.rs` (`FUNCTION_POINTER_TYPEDEFS`,
  tests);
* `contracts/openssl/ssl_io.qv` and `ssl.qv`. The statement line is on all
  seven declarations, `SSL_free` and `SSL_CTX_free` included;
* `contracts.lock`, `providers/user-crates-load-path.json`,
  `quirin-plugin.json` and `src/lib.rs`. Regenerate the lock in §3's order;
* `doc-snapshots.json` and `pinned/openssl-3.5.5.json` (append-only);
* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`
  and `openssl_unstated_callback_slots.rs`;
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.19.3, and
  `docs/agents/rules/12n.md`;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean`. Rerun the
  generators.
