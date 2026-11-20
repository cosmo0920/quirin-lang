# OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: what the object frees run on what they free

* **Base:** `ecc03d7a4` (integrate/r39: main `ffb41c748` plus
  OPENSSL-FREE-INDIRECT-CALLBACKS with its amendments). The gate and the
  amendments check use `ecc03d7a4`.
* **Card:** the named follow-up recorded by OPENSSL-FREE-INDIRECT-CALLBACKS.
* **Commits:**
  * [PREDICTION](PREDICTION.md) `d1382ca88`, committed before any
    implementation;
  * the implementation, lock and Lean, `b80ee7664`.

## 1. Sources

OpenSSL 3.5.5 release tarball `openssl-3.5.5.tar.gz`, sha256
`b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89`
(unpacked into `private/src/`, not committed). The 3.5 manual citations
are its `doc/man3` and `doc/man7` sources, the pages the 3.5 site renders,
so no new `fetch_docs.py` snapshot was needed: no declaration's range or
shape changed, and the checker reads snapshots only for those.

## 2. The frees: what each runs, and what it now names

The crate declares ten frees (every `Release`, `CountedRelease` and
`NullAcceptingRelease` in `DOCUMENTED`). `X509_CRL_free`,
`X509_STORE_CTX_free`, `SSL_SESSION_free` and `BIO_free_all` are not
declared.

| Free | What it runs on what it frees (3.5.5) | Premise and statement now |
|---|---|---|
| `EVP_PKEY_free` | at the last reference, `evp_pkey_free_it` (`crypto/evp/p_lib.c`): the provider key management's free (`evp_keymgmt_freedata`, `OSSL_FUNC_keymgmt_free`, provider-keymgmt(7)), and `EVP_KEYMGMT_free`, whose last reference calls `ossl_provider_free` and so may run the provider's teardown (`keymgmt_meth.c`, `provider_core.c`, provider-base(7)); `evp_pkey_free_legacy`: the `EVP_PKEY_ASN1_METHOD`'s `pkey_free` (EVP_PKEY_ASN1_METHOD(3)), which for a legacy key is `RSA_free`, `DSA_free`, `DH_free` or `EC_KEY_free`, each running its method's finish function (RSA_meth_new(3): finish "will be called in response to the application calling RSA_free()"; `DH_meth_new`, `DSA_meth_new`; `EC_KEY_METHOD`'s finish, no 3.5 page) and `ENGINE_finish` of its engine; then `ENGINE_finish` of the key's `engine` and `pmeth_engine` | **new** `openssl_unstated_callback_slots`, `UNSTATED_KEY_FREE_STATEMENT`; `openssl_ex_data_callbacks` kept |
| `X509_free` | at the last reference, `x509_cb` `ASN1_OP_FREE_POST` (`crypto/x509/x_x509.c`) frees only library structures and ex_data; the template frees the `X509_PUBKEY`, whose `x509_pubkey_ex_free` (`x_pubkey.c`) calls `EVP_PKEY_free` on the key decoded with the certificate (`x509_pubkey_ex_d2i_ex`), so everything in the row above, for that key | **new**, `UNSTATED_CERTIFICATE_FREE_STATEMENT`; ex_data kept |
| `PKCS12_free` | the authenticated safes are PKCS#7 objects: a signed one's certificates (and so their keys) and CRLs; a signer info's `pkey` (`si_cb`, `pk7_asn1.c`); a recipient info's `cert` (`ri_cb`). A CRL's free runs its `X509_CRL_METHOD`'s `crl_free` (`crl_cb`, `x_crl.c`), the method `X509_CRL_set_default_method` installs (no 3.5 page) | **new**, `UNSTATED_PKCS12_FREE_STATEMENT`; ex_data kept |
| `ENGINE_free` | `engine_free_util` (`crypto/engine/eng_lib.c`) at the last structural reference: the engine's public key method and ASN.1 method callbacks, called to find the dynamic methods to free (`engine_pkey_meths_free`, `tb_pkmeth.c`; `engine_pkey_asn1_meths_free`, `tb_asnmth.c`; setters `ENGINE_set_pkey_meths` and `ENGINE_set_pkey_asn1_meths`, no 3.5 page), then its destroy function (`ENGINE_set_destroy_function`, ENGINE_add(3)). The free is structural, so no finish function runs | **new**, `UNSTATED_ENGINE_FREE_STATEMENT`; ex_data kept |
| `EVP_MD_CTX_free` | `EVP_MD_CTX_reset` (`crypto/evp/digest.c`): `EVP_PKEY_CTX_free` of the context's `pctx` (`pmeth_lib.c`: the `EVP_PKEY_METHOD`'s cleanup, EVP_PKEY_meth_new(3); the provider signature, key exchange, KEM or asymmetric cipher freectx, provider-signature(7); `EVP_KEYMGMT_free`; `EVP_PKEY_free` of its key and peer key; `ENGINE_finish`), then `evp_md_ctx_clear_digest`: the provider digest's freectx (provider-digest(7)), the legacy `EVP_MD`'s cleanup (EVP_MD_meth_new(3)), `ENGINE_finish` of the digest's engine, `EVP_MD_free` of the fetched digest | **new** (the survey's find), `UNSTATED_DIGEST_CONTEXT_FREE_STATEMENT`; ex_data kept |
| `SSL_free`, `SSL_CTX_free`, `X509_STORE_free` | r39's survey (keys, certificates, stores, CRLs, the client certificate engine, the async wait context, QUIC) | unchanged: `UNSTATED_FREE_STATEMENT` |
| `BIO_free` | the BIO callback and method destroy (stated slots); a built-in method's destroy may free more (a `BIO_f_ssl` BIO's SSL object, a `BIO_f_md` or `BIO_f_cipher` context) | unchanged: `openssl_builtin_bio_methods` already names "any callback it runs in turn on an object other than the arguments of the call" |
| `OPENSSL_sk_free` | frees the stack only, never its elements | none needed |

**Why each kind has its own statement.** r39's `UNSTATED_FREE_STATEMENT`
names the client certificate engine of an `SSL_CTX`, the async wait context
and QUIC callbacks of an SSL object, and a store's CRLs, none of which these
five reach. So each object kind has a statement naming only what its free
can run. The key functions are spelled once (`key_functions!` in
`documented.rs`) and composed into the key, certificate, PKCS#12 and
digest context statements. Each statement keeps r39's general head ("a
function of a ... of an object the call frees ..., whose setter no
declaration of this crate states or which no setter is handed, among
them ..."), so the list is a naming, and nothing the free runs falls
outside the premise.

**What was narrowed: nothing.** Each of the five previously named only
`openssl_ex_data_callbacks`, which still stands. The new premise is an
addition.

**The ex_data audit's classes.** The `EX_DATA_AUDIT` readings of
`EVP_PKEY_free`, `X509_free`, `PKCS12_free` and `EVP_MD_CTX_free` now list
`RSA`, `DSA`, `DH`, `EC_KEY` and `ENGINE` too (and `EVP_PKEY` for the
certificate frees), since a legacy key's free runs its RSA, DSA, DH or
EC_KEY class's free callbacks and `ENGINE_finish` can drop an engine's
last reference. That list is checker metadata, read only for membership in
`EX_DATA_CLASSES`. It changes no declaration text. The same omission in
the three r39 frees is recorded as a follow-up observation (§7).

## 3. The checker's expectation

`FREE_AUDIT` (`documented.rs`) is read like `EX_DATA_AUDIT`, by
`free_audited` (`check.rs`), for every declaration:

* a declaration whose shape releases an object (`releases`) is listed, and
  so is one naming a free statement; otherwise it is refused (`its ...
  frees an object, and the free audit does not list it`);
* `Frees::Runs { statement }`: the declaration names
  `openssl_unstated_callback_slots` in exactly that statement, once, and
  not the handshake's statement as well;
* `Frees::NamedBy { premise }` (`BIO_free`): it names that premise and no
  free statement;
* `Frees::Nothing` (`OPENSSL_sk_free`): it names no free statement;
* a second listing refuses.

`Premise::UnstatedFreeSlots` now carries its `FreeStatement`, and
`expected_clauses` derives the premise text from it, so a declaration
naming another kind's statement fails the clause-set check too.

**Checker tests** (22 in the crate's unit tests, two new):

* `every_free_names_what_it_runs_on_the_objects_it_frees`: every audited
  free is documented once, audited once and a release; each running free
  names exactly its statement in its module; dropping it, swapping it for
  each of the five other statements, or replacing it with the handshake's
  refuses through `check_function`; the audit covers exactly the releases
  in `DOCUMENTED`; each statement is named in the modules exactly as often
  as the audit reads it; and lexically, none of the five new statements
  names `SSL`, `QUIC`, `async`, ex_data or a lookup method, only the
  PKCS#12 statement names a CRL method, only the digest context's names an
  `EVP_PKEY_CTX`, the engine statement names no finish function and no key,
  and the four key-bearing statements each name the five key functions;
* `a_drifted_free_reading_is_refused`: each drift of `free_audited`, and
  the crate's own `OPENSSL_sk_free` with a free statement added.

## 4. Lock, load path and generated files

The regeneration followed r39's order (`private/regen.sh`, logs
`private/logs/regen-*.log`, exits in `private/logs/regen.exits`, captured
unpiped):

| Step | Exit |
|---|---|
| build `quirin-plugin` (`quirin-provider-plugin`) | 0 |
| `quirin-plugin load-path --extends contract-load-path.json --output user-crates-load-path.json quirin-provider-libcurl quirin-provider-openssl`, from `providers/` | 0 |
| `providers/package.py checkers` | 1, as expected (stale views) |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin --load-path providers/user-crates-load-path.json --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate quirin-provider-openssl` | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` | 0 |

`quirin-audit` is the release binary built at the base; the card changes
no crate source outside `providers/`. The reviewed `contracts.lock` diff
against the base (`private/lockdiff.py`):

* **5 entries changed** (`contract_sha256` and `pinned_at`):
  * `ENGINE_free` `2044677ce1e49699e71be09a1dab587154962b91bad97f588bc9f559a686b76f`
  * `EVP_MD_CTX_free` `680d53740317ddb2d5ba704028863ab0e6986ac633450ae2a03aa43fe44136b8`
  * `EVP_PKEY_free` `7ffb87ddd254af1d2936ddbc8124e08aa7bbfb8631f29245abdfcaf2546297db`
  * `PKCS12_free` `c3e9f2ff75fa7d3454e6581b9b36d601e0a025c4fe58afb876d33b3d4aa43602`
  * `X509_free` `fdb8a3e2ffa686c7ede59fd2087036347a0abec0e6f75bbce541a7a6f7d2a9b2`
* **0 added, 0 removed, 139 unchanged.** `pinned`, `library_versions`,
  `fluent_bit_revision`, `generated_by`, `lock_version`, `normalisation`,
  `note` and `owner` are unchanged; `flexible` holds the entries.

`user-crates-load-path.json` and `quirin-plugin.json` are unchanged (no
binding added). `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`
were rerun (`docs/agents/gateway.md`, `proof/QCore.lean`).

**Specification and rule:** C04.18 gains "The object frees"; rule 12n is
amended in place. No new rule id.

## 5. Lean

The checker gains a rule (the free audit), so there is a new module,
`QCore.OpensslObjectFreesAudit`, audited by
`proof/AuditOpensslObjectFreesAudit.lean` (18 theorems; axioms within
`propext`, `Classical.choice` and `Quot.sound`; `private/logs/lean-1.log`,
`lean-audit-1.log`):

* the audit is total over releases, and an unaudited release refuses;
* a running free names exactly its audited statement; dropping it, naming
  another kind's or adding the handshake's refuses;
* a free statement stands only where the audit reads a run, and a free
  whose run another premise names carries it;
* the ex_data premise alone leaves a key's method finish uncovered
  (`key_free_vector_needs_the_premise`);
* the premise moves no verdict;
* vectors from the crate's audit.

## 6. Measurement

**Pinned routes: no move predicted, arbiter deferred.** No pinned route
loads the OpenSSL crate (it is on the user load path only), and no gateway
code changed. The prediction's `measure-prediction` block (none on corpus,
system and slice) is for the integrator's `quirin-audit measure
--declared`. Pinned-route parity cannot run on this host: the pinned inputs
and published baselines are absent, as for r30 to r39.

**The opt-in `current` run (the arbiter).** `current --monkey-linked` over
Fluent Bit 046accf6 with OPENSSL-SSL-CTX-CALLBACK-SLOTS'
[script](../openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
(`SOURCE_DATE_EPOCH=1767225600`, `--jobs 12`).

* **Inputs.** Prepared on this host by the base's release binary:
  `quirin-audit prepare --fluent-bit ../fluent-bit --revision 046accf6aded7906f66a1baf0202da349e521736
  --output private/fb-046accf6 --clang /usr/lib/llvm-21/bin/clang`, exit 0
  (`private/logs/prepare.log`).
* **The base was remeasured** at `ecc03d7a4`, not reused from r38's
  `f916d3007`: r39 changed the contracts since then (+9 bindings).
* **Side trees.** Each side is a `git archive` side tree; each tree's own
  `package.py checkers` exited 0. Both runs used the base's release
  `quirin-audit` (the card changes no crate source). Drivers:
  [base-run.sh](evidence/opt-in/base-run.sh),
  [tip-run.sh](evidence/opt-in/tip-run.sh).

| Run | Tree | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|---|
| base-1 (`private/opt-in/base-1`) | `ecc03d7a4` | 180 / 181: 9 Safe, 0 Pending, 6 Unsafe, 162 Unknown, 4 data-only | 1139 / 2 / 3656 / 7 | 150, all `Pending` |
| tip-1 (`private/opt-in/tip-1`) | `b80ee7664` | same | 1139 / 2 / 3656 / 7 | 150, all `Pending` |

Both exit 0 with no unmatched row ([summaries](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `evidence/compare.py`).

**Against the prediction: met in every part.** No status move, no new
`Unsafe`, bindings unchanged at 150 (all `Pending`, so the checker accepts
its five changed declarations), and `rows_changed` 0.

## 7. Gate

**Command:** `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base ecc03d7a4 --out private/gate-1` (Clang 21, Lean 4.35.0), log
`private/logs/gate-1.log`, record [`gate/gate-1.json`](gate/gate-1.json),
step logs under `private/gate-1/`.

* **Tree.** `b80ee7664`; the report, evidence and amendments were
  uncommitted (docs only).
* **Scope.** Scoped: 44 Rust targets over 7 packages, Lean all
  (`proof/QCore.lean` changed), the provider gate included.
* **Result: exit 1.** The only failure is
  `test_build_matrix_sections.CommittedMatrixIsSelfConsistent.test_each_derived_dimension_matches_the_section_it_counts`:
  `dimensions.named_followups is 1030 against a named_followups of 1031`.
  **It is the base's, not this card's.** The card does not touch
  `docs/reviews/r0_3/acceptance_matrix.json`; at `ecc03d7a4` the file
  already reads 1030 against 1031, while at its parent `85468240d` it reads
  1030 against 1030. So r39's amendments application registered two
  follow-ups and retired one without regenerating the dimension. The
  integrator should regenerate through
  `docs/reports/remaining-d02/build_matrix.py`. The slot flakes did not
  fire, so no slot rerun was needed. Every other step passed.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (scoped runs) | 0 | `third_party/libevent` checked out |
| Rust suite (scoped) | 0 | 273 units, 280 summaries; 3461 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 463 audits (new `AuditOpensslObjectFreesAudit`) |
| Python suite | 1 | 779 tests; 1 failure, the base's matrix dimension (above) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 165 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

The provider tests took 5066 s because the host was shared with this
card's two opt-in runs.

**Outside the gate**, during the work (`private/logs/`): the crate's tests
(`ptest-1.log`, `ptest-2.log`: 22 unit and 2 integration tests passed),
crate clippy (`clippy-1.log`, exit 0), and `lake build
QCore.OpensslObjectFreesAudit` with its audit (`lean-1.log`,
`lean-audit-1.log`, exit 0).

## 8. Follow-ups

[`integration-amendments.json`](integration-amendments.json) (`--check
--profiles-base ecc03d7a4` exits 0) retires
OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS.

It registers **OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS**: the
declarations that are not frees but free keys, certificates or contexts
inside the call and name no free statement. They are the `SSL_CTX_use_*`
loaders (a replaced or temporary key or certificate), the PEM readers and
`d2i_PKCS12_bio` on a failed read, `SSL_CTX_load_verify_locations` and
`SSL_CTX_set_default_verify_paths`, and the handshake and I/O calls (peer
keys, ephemeral keys). The handshake's statement names none of these
functions. The audit added here covers releases only.

It appends to **OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS**:

* the five frees here keep the premise too, with the kinds this report
  names (their setters: `ENGINE_set_pkey_meths`,
  `ENGINE_set_pkey_asn1_meths`, `EVP_MD_meth_set_cleanup`,
  `EVP_PKEY_meth_set_cleanup`, plus the provider dispatch tables), and a
  key's provider teardown is one more provider function;
* `X509_STORE_free` carries r39's shared statement, which also names SSL
  objects' kinds the store free never reaches, so a store statement is the
  narrower reading;
* the `EX_DATA_AUDIT` classes of the three r39 frees omit the legacy key
  classes and `ENGINE`.

**Files likely to conflict** with other OpenSSL cards:

* `providers/quirin-provider-openssl/src/documented.rs` (`Premise`, the
  new statements, `FreeStatement`, `Frees`, `FREE_AUDIT`, the `*_FREED`
  premise arrays, `EX_DATA_AUDIT`) and `src/check.rs` (the `pub use`,
  `expected_clauses`, `free_audited`, tests);
* `contracts/openssl/engine.qv`, `evp.qv`, `evp_digest.qv`, `pkcs12.qv`,
  `x509.qv`;
* `contracts.lock` (regenerate in §4's order);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and `docs/agents/rules/12n.md`;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean` (rerun the
  generators).
