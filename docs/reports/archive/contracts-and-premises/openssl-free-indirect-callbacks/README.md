# OPENSSL-FREE-INDIRECT-CALLBACKS: callbacks the frees run on objects they free in turn

* **Base:** `f916d3007` (integrate/r38 merged with main `7c026b527`). Main
  `ffb41c748` (r38 landed, plus two record-only commits) was merged in as
  `03c338011`. The gate and the amendments check use `ffb41c748`.
* **Card:** the named follow-up recorded by
  OPENSSL-FREE-SESSION-REMOVE-INVOCATION and appended by r38.
* **Commits:**
  * [PREDICTION](PREDICTION.md) `edad80228`, committed before any
    implementation;
  * the implementation, lock and Lean, `c282a01f7`.

## 1. What was declared and stated

**Page evidence.**

* **Snapshots.** `X509_LOOKUP_meth_new` was recorded with `fetch_docs.py
  --merge` (2026-11-20) at four series:
  * 1.1.1, sha256 `7f59d722…`;
  * 3.0, sha256 `58e13678…`;
  * 3.5, sha256 `a0024207…`;
  * 1.0.2: not found.

  HISTORY at all three found series reads "The functions described here
  were added in OpenSSL 1.1.0i."
* **Prototypes.** The nine setters were pinned with `pin_prototypes.py`
  over every recorded function plus the new ones. Every existing prototype
  and every header digest is byte-identical.
* **NAME check.** The 3.5 page's NAME list is exactly the nine writers plus
  their not-writers (`slot_writers_declared`):
  * the type, the constructor and the destructor;
  * the nine getters;
  * the four function pointer types;
  * `X509_OBJECT_set1_X509` and `X509_OBJECT_set1_X509_CRL`.

**Range.** The version vocabulary is `MAJOR.MINOR[.PATCH]`, so it cannot
spell the letter release 1.1.0i. `since "1.1.0"` would admit 1.1.0 to
1.1.0h, where the setters do not exist. The range is therefore
`documented "1.1.1"`, the oldest series checked that has a page.

**Keys.** Each key is `x509_lookup_method_st.lookup_meth_*`, named after
the setter rather than the library's field (`free`, `init`, `shutdown`,
`ctrl`). A key segment is a member pre-pass token, and r38's `cleanup`
showed what a common word costs (MEMBER-PREPASS-UNREADABLE-TOKEN-UNIT). No
Fluent Bit source names any of the new segments, and the checker test pins
that no segment is a bare field word.

### Slot, writers and stating frees

| Slot | Writer (3.5 page) | Stated at | Invocation actuals |
|---|---|---|---|
| `x509_lookup_method_st.lookup_meth_shutdown` | `X509_LOOKUP_meth_set_shutdown` | `X509_STORE_free`, `SSL_CTX_free`, `SSL_free` | `(_)` |
| `x509_lookup_method_st.lookup_meth_free` | `X509_LOOKUP_meth_set_free` | `X509_STORE_free`, `SSL_CTX_free`, `SSL_free` | `(_)` |
| `.lookup_meth_new_item` | `X509_LOOKUP_meth_set_new_item` | none (run by `X509_LOOKUP_new`) | |
| `.lookup_meth_init` | `X509_LOOKUP_meth_set_init` | none | |
| `.lookup_meth_ctrl` | `X509_LOOKUP_meth_set_ctrl` | none (run by `X509_LOOKUP_ctrl`) | |
| `.lookup_meth_get_by_subject`, `.lookup_meth_get_by_issuer_serial`, `.lookup_meth_get_by_fingerprint`, `.lookup_meth_get_by_alias` | one `X509_LOOKUP_meth_set_get_by_*` each | none (the handshake's lookups are OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS) | |

**Why the frees run these two slots** (OpenSSL 3.5.5 source):

* **`X509_STORE_free`.** At the last drop it runs, for each lookup the
  store holds, `X509_LOOKUP_shutdown` and then `X509_LOOKUP_free`
  (`crypto/x509/x509_lu.c`). These call `method->shutdown(ctx)` and
  `method->free(ctx)`.
* **`SSL_CTX_free`.** It frees the context's store ("SSL_CTX_set_cert_store()
  will take ownership of the store", SSL_CTX_set_cert_store 3.5). Its
  `ssl_cert_free` also frees the `CERT`'s `verify_store` and `chain_store`.
* **`SSL_free`.** `ossl_ssl_connection_free` frees the connection's `CERT`
  (and so its stores) and calls `SSL_CTX_free` on `session_ctx`. `SSL_free`
  then calls `SSL_CTX_free` on `ctx`. Either can be the last reference.

**Actuals.** Each function is handed the library's `X509_LOOKUP`, which no
formal names, so the actual is `_`. The page states no argument NULL, so no
`NULLABLE_ARGUMENTS` entry was added.

**The built-in methods.** `X509_LOOKUP_file`, `X509_LOOKUP_hash_dir` and
`X509_LOOKUP_store` are library values that no setter is handed. Their free
functions release only their own method data (`free_dir`, `by_store_free`;
the file method has none), and none has a shutdown function. The new
library-value premise `openssl_builtin_lookup_methods` covers them on the
three frees, as `openssl_builtin_bio_methods` does for BIOs.

**The twelve `X509_STORE_set_*` slots are not run at a store's free.**
`X509_STORE_free` calls none of them. The store's cleanup function is
`ctx->cleanup`, run by `X509_STORE_CTX_cleanup` (`x509_vfy.c`), which is a
verification context's cleanup. The handshake states that slot (r38). So
the frees do not state them, and the checker test refuses
`x509_store_st.cleanup_fn` stated at a free.

## 2. Survey: what else the frees release, and what stays unstated

Source read: OpenSSL 3.5.5 `ssl/ssl_lib.c` (`SSL_CTX_free`, `SSL_free`,
`ossl_ssl_connection_free`), `ssl/ssl_cert.c`, `ssl/statem/extensions_cust.c`,
`ssl/tls_depr.c`, `ssl/record/rec_layer_s3.c`, `ssl/quic/quic_tls*.c`,
`ssl/quic/quic_impl.c`, `crypto/x509/x509_lu.c`, `by_*.c`, `x_crl.c`,
`crypto/evp/p_lib.c`, `crypto/rsa/rsa_lib.c` and
`crypto/async/async_wait.c`, together with the 3.5 pages of `SSL_CTX_free`,
`SSL_free`, `SSL_CTX_set_cert_store`, `RSA_meth_new`, `ENGINE_add` and
others (private, not committed).

| Object freed | By | User-settable code it runs | Disposition |
|---|---|---|---|
| A store's lookups | all three | lookup method shutdown and free | **stated** (§1) |
| A built-in lookup method | all three | library value | `openssl_builtin_lookup_methods` |
| The sessions in the cache | `SSL_CTX_free` (and `SSL_free` through it) | session-remove callback | stated (r36) |
| SSL, SSL_CTX, SSL_SESSION, X509, X509_STORE, EVP_PKEY, BIO objects | all three | ex_data free callbacks | `openssl_ex_data_callbacks` |
| The connection's BIOs | `SSL_free` | BIO callback and destroy method | stated (r36), built-in BIO premise |
| Keys (`CERT` keys, `dh_tmp`, certificates' public keys, CT log keys) | all three | a provider's key management free function (a dispatch table, with no setter); a legacy key's `EVP_PKEY_ASN1_METHOD` free (`EVP_PKEY_asn1_set_free`); `RSA_METHOD`, `DSA_METHOD`, `DH_METHOD` or `EC_KEY_METHOD` finish (`RSA_meth_set_finish` ...); the key's ENGINE finish and destroy | **unstated**, premise |
| The client certificate engine | `SSL_CTX_free` (`tls_engine_finish`) | ENGINE finish and destroy functions (`ENGINE_set_finish_function`, `ENGINE_set_destroy_function`); `SSL_CTX_set_client_cert_engine` has no 3.5 page | **unstated**, premise |
| A store's CRLs | all three | `X509_CRL_METHOD` `crl_free` (`X509_CRL_METHOD_new`, no 3.5 page) | **unstated**, premise |
| The async wait context | `SSL_free` (`ASYNC_WAIT_CTX_free`) | fd cleanup functions (`ASYNC_WAIT_CTX_set_wait_fd`'s `cleanup`) | **unstated**, premise |
| Records released by `RECORD_LAYER_clear` | `SSL_free` | the QUIC TLS `crypto_release_rcd` callback (`SSL_set_quic_tls_cbs`, a dispatch table) | **unstated**, premise |
| A QUIC connection, stream, listener or domain | `SSL_free` (`ossl_quic_free`) | not traced past `quic_impl.c` | **unstated**, premise |
| Custom extensions | `SSL_CTX_free`, `SSL_free` (`custom_exts_free`) | none: `free_cb` runs during a handshake, not here | nothing to state |
| The keylog BIO | `SSL_CTX_free` (under `SSLKEYLOGFILE`, off by default) | a library-made file BIO, whose callbacks no user can set | nothing to state |
| SRP, DANE, cipher lists, CT log store, token cache | `SSL_CTX_free` | none found | nothing to state |

**So the frees keep `openssl_unstated_callback_slots`** (the fail-closed
rule). The premise could not become `openssl_ex_data_callbacks` alone, as
the registered discharge expected. Its statement on the frees is now one of
their own (`UNSTATED_FREE_STATEMENT`), and it names exactly the unstated
rows above:

> a callback the call may run that no clause of this declaration states (a
> function of a method, provider or engine of an object the call frees in
> turn, whose setter no declaration of this crate states or which no setter
> is handed, among them the free function of the key management of the
> provider of a key, the free function of the EVP_PKEY_ASN1_METHOD of a
> key, the finish function of the RSA_METHOD, DSA_METHOD, DH_METHOD or
> EC_KEY_METHOD of a key, the finish and destroy functions of the ENGINE of
> a key or of the client certificate engine of the SSL_CTX, the free
> function of the X509_CRL_METHOD of a CRL a certificate store holds, a
> cleanup function of the async wait context of the SSL object, the QUIC
> TLS callbacks of the SSL object, and any callback the free of a QUIC
> object runs in turn) takes, releases or retains nothing the caller holds

**What the free statement drops, compared with r38's:**

* the ctrl arms, CT and the undocumented handshake setters, all of which
  run during a handshake and not at a free;
* the lookup methods, which are now stated;
* ex_data, which `openssl_ex_data_callbacks` covers.

The handshake calls keep r38's statement unchanged, because they still run
the lookup methods' get_by functions.

**`X509_STORE_free` gains the premise.** It named only
`openssl_ex_data_callbacks`, yet freeing its certificates and CRLs runs the
key and CRL rows above. It now carries the same free statement, the two
lookup slots and the built-in lookup premise. `X509_free`, `EVP_PKEY_free`,
`PKCS12_free` and `ENGINE_free` have the same silent gap (keys' methods,
engines' finish and destroy). That gap is registered as a follow-up rather
than widened here.

**The ex_data audit.** `SSL_free`'s `EX_DATA_AUDIT` classes now list
`EVP_PKEY`, `X509_STORE` and `SSL_CTX` too, since it may drop the context's
last reference. That list is checker metadata and changes no declaration
text.

## 3. Lock, load path and generated files

The regeneration used r38's driver order (`private/regen.sh`, logs
`private/logs/regen-*.log`, exits captured unpiped):

| Step | Exit |
|---|---|
| build `quirin-plugin` | 0 |
| `quirin-plugin load-path --extends contract-load-path.json` from `providers/`, libcurl then openssl | 0 |
| `providers/package.py checkers` (installs, then refuses the stale views) | 1, as expected |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin --load-path providers/user-crates-load-path.json --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate quirin-provider-openssl` | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` | 0 |
| `providers/package.py checkers`, rerun after `cargo fmt` (`regen-7`) | 0 |

`quirin-audit` is the release binary built at the base. The card changes
no crate source outside tests. The diff was reviewed against base:

* **9 entries added**, with these `contract_sha256` values:
  * `X509_LOOKUP_meth_set_ctrl` `7cb7849d…`
  * `X509_LOOKUP_meth_set_free` `36fab90d…`
  * `X509_LOOKUP_meth_set_get_by_alias` `8a46982f…`
  * `X509_LOOKUP_meth_set_get_by_fingerprint` `d100eb76…`
  * `X509_LOOKUP_meth_set_get_by_issuer_serial` `bbaf633a…`
  * `X509_LOOKUP_meth_set_get_by_subject` `e077c213…`
  * `X509_LOOKUP_meth_set_init` `a4ec0686…`
  * `X509_LOOKUP_meth_set_new_item` `4054e4f9…`
  * `X509_LOOKUP_meth_set_shutdown` `de933860…`
* **3 entries changed** (`contract_sha256` and `pinned_at`):
  * `X509_STORE_free` `8b8cdd4b…`
  * `SSL_CTX_free` `d0c41082…`
  * `SSL_free` `4969559e…`
* **132 entries unchanged.** Of the top-level keys, only `flexible`
  differs. `pinned`, `library_versions` and every other key are unchanged.

Other files:

* **`user-crates-load-path.json`**: 9 bindings, one module.
* **`quirin-plugin.json`**: 9 bindings, added by hand at the module's
  position.
* **Generators.** `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`
  were rerun (`docs/agents/gateway.md`, `proof/QCore.lean`).

**Specification and rule:**

* C04.18 gains "The callbacks the frees run in turn".
* C04.19.3's opaque slot bullet gains the lookup method table as a holder,
  the spelling of key segments, and the narrowing rule.
* Rule 12n is amended in place. No new rule id was needed.

## 4. Tests and Lean

**Gateway tests.** They are in the existing target
`crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`,
so the read census needs no new unit. The target has 29 tests, all passing
with Clang (`private/logs/gtest-1.log`).

| Arm | Test | Reading |
|---|---|---|
| the declarations | `the_frees_state_their_slots_beside_the_drop` (amended), `the_store_free_states_the_lookup_slots` (new) | `SSL_CTX_free` and `SSL_free` gain the two lookup slots. `X509_STORE_free` keeps its non-nullable drop and states exactly the two, actual `_`. No free states an `x509_store_st.*` slot. The nine setters register `lookup_meth_<field>` |
| identity candidates, or nothing registered | `identity_candidates_decide_the_frees` | `free_store`, `free_ctx`, `free_ssl` and `free_bio` are `Safe`, with no `callback-` refusal |
| an uncontracted lookup free function; a shutdown function releasing its lookup | `a_candidate_that_is_not_the_identity_refuses_the_frees` | each free of a store refuses, naming the candidate; `free_bio` stays `Safe` |
| no run facts; `x509_lookup_slots.qv` absent | `a_free_slot_without_registrations_or_run_facts_refuses` | not `Safe`, with `no declaration registers` for the three; `free_bio` stays `Safe` |

**Checker tests.** There are 20, including the new
`a_drifted_free_indirect_reading_is_refused`, run in the gate's provider tests.
`SLOTS` grows from 68 to 77. The new test refuses each of these:

* a lookup registration moved;
* the range read as `since "1.1.0"`;
* a page NAME losing a not-writer;
* on each of the three frees:
  * each lookup invocation dropped;
  * the freed object handed to the callback;
  * a third lookup slot stated;
  * a store slot stated;
  * the built-in premise dropped;
  * the unstated premise dropped, or read in the handshake's statement.

The existing tests read the frees' premise in the free statement.

**Lean.** The new module is `QCore.OpensslFreeIndirectCallbacks`, audited by
`proof/AuditOpensslFreeIndirectCallbacks.lean` (11 theorems). Axioms are
within `propext` and `Quot.sound` (`private/logs/lean-1.log`).

* A free with an unstated remainder keeps the premise, and the ex_data
  premise alone leaves the remainder uncovered.
* A statement narrows only by kinds the call never runs.
* A slot the call does not run needs no statement.
* Stating the lookup slots never improves the outcome.
* The frees' vectors.

## 5. Measurement

**Pinned routes: no move predicted, arbiter deferred.** No pinned route
loads the OpenSSL crate (it is on the user load path only), and no gateway
code changed. The prediction's `measure-prediction` block (none on corpus,
system and slice) is for the integrator's `quirin-audit measure
--declared`. That command cannot run on this host: the pinned inputs and
published baselines are absent, as for r30 to r38.

**The opt-in `current` run (the arbiter).** It ran `current
--monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Inputs.** They were prepared on this host by the base binary's
  `prepare` into `private/fb-046accf6` (exit 0).
* **Side trees.** Each side is a `git archive` side tree, and each tree's
  own `package.py checkers` exited 0. Both runs used the base's release
  `quirin-audit`, since the card changes no crate source outside tests.
  The drivers are [base-run.sh](evidence/opt-in/base-run.sh) and
  [tip-run.sh](evidence/opt-in/tip-run.sh).
* **Host load.** The two runs overlapped each other and the gate, so wall
  times are not comparable. The rows are deterministic.

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base-1 (`private/opt-in/base-1`) | `f916d3007` | 180 / 181: 9 Safe, 0 Pending, 6 Unsafe, 162 Unknown, 4 data-only | 1139 / 2 / 3656 / 7 | 141, all `Pending` |
| tip-1 (`private/opt-in/tip-1`) | `c282a01f7` | same | 1139 / 2 / 3656 / 7 | 150, all `Pending` |

Both runs exit 0 with no unmatched row
([summaries](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**The base moved since r38's 1098 / 2 / 3699 / 5.** That is the other
lane's AG landings, measured here at `f916d3007`, not by this card.

**Against the prediction:**

* **No status move: met.** No row went to `Safe`, there is no `Safe` to
  non-`Safe` move, and there is no new `Unsafe`.
* **Bindings: met.** They went from 141 to 150 (+9), all `Pending`.
* **Content: met.** `rows_changed` is 0. So no new key segment pulled in a
  unit, unlike r38's `cleanup`, and every row naming a free refuses on
  what it refused on at base.
* **Run sets.** None of the nine setters is called in Fluent Bit's or
  Monkey's sources, so both stated lookup sets are the empty closed set. As
  in r33 to r38, the report carries no `callback_members` sidecar, so the
  sets are not listed separately.

## 6. Gate

**Command:** `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base ffb41c748 --out private/gate-1` (Clang 21, Lean 4.35.0). It was run
through `private/run.sh`, with the log at `private/logs/gate-1.log`.

* **Tree.** `03c338011`, the implementation merged with main `ffb41c748`.
  The report, evidence and amendments were uncommitted (docs only).
* **Record.** [`gate/gate-1.json`](gate/gate-1.json), with logs under
  `private/gate-1/`.
* **Scope.** The scope is scoped, not full: 44 Rust targets over 7
  packages, Lean all (`proof/QCore.lean` changed), the provider gate
  included.
* **Result: exit 1.** The only failure was the known flake
  `test_gate_slot_queue.Status.test_each_waiter_shows_its_queue_position`
  (`'holder' != 'waiting for a slot'`, `private/gate-1/python.log`).
* **The rerun.** `python3 -m unittest discover -s scripts/tests -p
  'test_gate_slot_*.py'` passed 115 tests, exit 0
  ([record](gate/gate-1-slot-rerun.txt), `private/logs/gate-1-slot-rerun.log`).
  Every other step passed.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (scoped runs) | 0 | `third_party/libevent` at `d82464a27` |
| Rust suite (scoped, 44 targets, 7 packages) | 0 | 274 units, 281 summaries; 3469 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 462 audits (new `AuditOpensslFreeIndirectCallbacks`) |
| Python suite | 1 | 779 tests; the one failure is the known slot flake, rerun green (115 tests) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 163 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

No test target was added, so the read census was not re-recorded. The
provider tests took 2444 s, because the host was shared with this card's
two opt-in runs.

**Outside the gate**, during the work (`private/logs/`):

* the crate's checker tests: 20 passed;
* `openssl_undeclared_handshake_callbacks`: 29 passed (`gtest-1.log`);
* `lake build QCore.OpensslFreeIndirectCallbacks` and the audit, exit 0
  (`lean-1.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json) (`--check
--profiles-base ffb41c748` exits 0) retires OPENSSL-FREE-INDIRECT-CALLBACKS.

It registers:

* **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**, the survey's remainder that
  keeps the premise on the three frees (§2);
* **OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS**. `X509_free`,
  `EVP_PKEY_free`, `PKCS12_free` and `ENGINE_free` name only
  `openssl_ex_data_callbacks`, yet they run keys' method, provider and
  engine functions, and an engine's destroy function.

It appends to **OPENSSL-LOOKUP-METHOD-HANDSHAKE-INVOCATIONS**: the setters
and the built-in premise now exist, and only the handshake's `get_by_subject`
remains. That card was not done here, because its invocations sit on the
four handshake calls and need the built-in premise's statement widened to a
lookup the call reads through.

**Files likely to conflict** with other OpenSSL cards:

* `providers/quirin-provider-openssl/src/documented.rs` (`Premise`, the new
  statement constants, `SLOTS`, `SSL_FREED`, `SSL_CTX_FREED`,
  `LOOKUP_FREED`, `X509_STORE_FREED`, `DOCUMENTED`, `SSL_free`'s
  `EX_DATA_AUDIT` classes) and `src/check.rs` (`FUNCTION_POINTER_TYPEDEFS`,
  the `pub use`, `expected_clauses`, tests);
* `contracts/openssl/ssl.qv` (`SSL_CTX_free`, `SSL_free`) and
  `x509_store.qv` (`X509_STORE_free`);
* `contracts.lock`, `providers/user-crates-load-path.json`,
  `quirin-plugin.json` and `src/lib.rs`. Regenerate the lock in §3's order;
* `doc-snapshots.json` and `pinned/openssl-3.5.5.json`, which are
  append-only;
* `crates/qcore-c-gateway/tests/openssl_undeclared_handshake_callbacks.rs`
  (the free tests);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and C04.19.3, and
  `docs/agents/rules/12n.md`;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean`. Rerun the
  generators.
