# OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS: what the object frees run on what they free

* **Base:** `ecc03d7a4` (integrate/r39: main `ffb41c748` plus
  OPENSSL-FREE-INDIRECT-CALLBACKS with its amendments). Main `9a769c6b0`
  (r39 landed, then SP1-SYMBOLIC-OPENSSL-PREMISES) was merged in and the
  card reworked onto SP1's formula premises (§9). The second gate and the
  amendments check use `9a769c6b0`.
* **Card:** the named follow-up recorded by OPENSSL-FREE-INDIRECT-CALLBACKS.
* **Commits:**
  * [PREDICTION](PREDICTION.md) `d1382ca88`, committed before any
    implementation;
  * the implementation, lock and Lean, `b80ee7664` (prose statements,
    the measured tip);
  * the merge of main `9a769c6b0` with the rework onto SP1's formulas,
    `cc18f4c59`.

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
| `EVP_PKEY_free` | at the last reference, `evp_pkey_free_it` (`crypto/evp/p_lib.c`): the provider key management's free (`evp_keymgmt_freedata`, `OSSL_FUNC_keymgmt_free`, provider-keymgmt(7)), and `EVP_KEYMGMT_free`, whose last reference calls `ossl_provider_free` and so may run the provider's teardown (`keymgmt_meth.c`, `provider_core.c`, provider-base(7)); `evp_pkey_free_legacy`: the `EVP_PKEY_ASN1_METHOD`'s `pkey_free` (EVP_PKEY_ASN1_METHOD(3)), which for a legacy key is `RSA_free`, `DSA_free`, `DH_free` or `EC_KEY_free`, each running its method's finish function (RSA_meth_new(3): finish "will be called in response to the application calling RSA_free()"; `DH_meth_new`, `DSA_meth_new`; `EC_KEY_METHOD`'s finish, no 3.5 page) and `ENGINE_finish` of its engine; then `ENGINE_finish` of the key's `engine` and `pmeth_engine` | **new** `openssl_unstated_callback_slots`, the free statement `UNSTATED_FREE_STATEMENT`; `openssl_ex_data_callbacks` kept |
| `X509_free` | at the last reference, `x509_cb` `ASN1_OP_FREE_POST` (`crypto/x509/x_x509.c`) frees only library structures and ex_data; the template frees the `X509_PUBKEY`, whose `x509_pubkey_ex_free` (`x_pubkey.c`) calls `EVP_PKEY_free` on the key decoded with the certificate (`x509_pubkey_ex_d2i_ex`), so everything in the row above, for that key | **new**, the free statement `UNSTATED_FREE_STATEMENT`; ex_data kept |
| `PKCS12_free` | the authenticated safes are PKCS#7 objects: a signed one's certificates (and so their keys) and CRLs; a signer info's `pkey` (`si_cb`, `pk7_asn1.c`); a recipient info's `cert` (`ri_cb`). A CRL's free runs its `X509_CRL_METHOD`'s `crl_free` (`crl_cb`, `x_crl.c`), the method `X509_CRL_set_default_method` installs (no 3.5 page) | **new**, the free statement `UNSTATED_FREE_STATEMENT`; ex_data kept |
| `ENGINE_free` | `engine_free_util` (`crypto/engine/eng_lib.c`) at the last structural reference: the engine's public key method and ASN.1 method callbacks, called to find the dynamic methods to free (`engine_pkey_meths_free`, `tb_pkmeth.c`; `engine_pkey_asn1_meths_free`, `tb_asnmth.c`; setters `ENGINE_set_pkey_meths` and `ENGINE_set_pkey_asn1_meths`, no 3.5 page), then its destroy function (`ENGINE_set_destroy_function`, ENGINE_add(3)). The free is structural, so no finish function runs | **new**, the free statement `UNSTATED_FREE_STATEMENT`; ex_data kept |
| `EVP_MD_CTX_free` | `EVP_MD_CTX_reset` (`crypto/evp/digest.c`): `EVP_PKEY_CTX_free` of the context's `pctx` (`pmeth_lib.c`: the `EVP_PKEY_METHOD`'s cleanup, EVP_PKEY_meth_new(3); the provider signature, key exchange, KEM or asymmetric cipher freectx, provider-signature(7); `EVP_KEYMGMT_free`; `EVP_PKEY_free` of its key and peer key; `ENGINE_finish`), then `evp_md_ctx_clear_digest`: the provider digest's freectx (provider-digest(7)), the legacy `EVP_MD`'s cleanup (EVP_MD_meth_new(3)), `ENGINE_finish` of the digest's engine, `EVP_MD_free` of the fetched digest | **new** (the survey's find), the free statement `UNSTATED_FREE_STATEMENT`; ex_data kept |
| `SSL_free`, `SSL_CTX_free`, `X509_STORE_free` | r39's survey (keys, certificates, stores, CRLs, the client certificate engine, the async wait context, QUIC) | unchanged: `UNSTATED_FREE_STATEMENT` |
| `BIO_free` | the BIO callback and method destroy (stated slots); a built-in method's destroy may free more (a `BIO_f_ssl` BIO's SSL object, a `BIO_f_md` or `BIO_f_cipher` context) | unchanged: `openssl_builtin_bio_methods` already names "any callback it runs in turn on an object other than the arguments of the call" |
| `OPENSSL_sk_free` | frees the stack only, never its elements | none needed |

**One statement for every free.** The free statement is SP1's formula
(C04.18.9) `quant all #o in #freed: #teardown(#o) \ (#stated | #settable)
|-> #inert`. `#freed` is the call's own set of objects whose last reference
it drops, directly or in turn, and `#teardown(o)` is what the free of `o`
runs from its method tables, provider and engine, or holds itself (SP1
report, vocabulary table). So the one statement names exactly what each
free runs: for `EVP_PKEY_free` the key and its RSA, DSA, DH or EC_KEY and
ENGINE objects; for `X509_free` the certificate and its key; and so on per
the table above. Before SP1 this card wrote five prose statements, one per
object kind (`b80ee7664`, §9), because r39's prose named SSL-only objects;
the formula has no such names, so it is reused rather than narrowed. A
narrower domain (for instance `#of(EVP_PKEY, #freed)`) would be wrong: it
would drop the RSA, ENGINE and other objects a key's free frees in turn.

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
* `Frees::Runs`: the declaration names `openssl_unstated_callback_slots`
  in the free statement, and not in the handshake's statement as well;
* `Frees::NamedBy { premise }` (`BIO_free`): it names that premise and no
  free statement;
* `Frees::Nothing` (`OPENSSL_sk_free`): it names no free statement;
* a second listing refuses.

**Checker tests** (23 in the crate's unit tests after the merge, two new;
SP1's `a_prose_premise_is_refused` now counts 58 premise clauses, its 53
and these five; `qcore-frontend`'s `the_openssl_formulas_are_the_lean_definitions`
counts 8 clauses of `unstatedFreeFormula` and 58 in all):

* `every_free_names_what_it_runs_on_the_objects_it_frees`: every audited
  free is documented once, audited once and a release; each running free
  names the free statement in its module; dropping it, or replacing it
  with the handshake's, refuses through `check_function`; the audit covers
  exactly the releases in `DOCUMENTED`; and the modules name the free
  statement exactly as often as the audit reads a run (8);
* `a_drifted_free_reading_is_refused`: each drift of `free_audited`, and
  the crate's own `OPENSSL_sk_free` with a free statement added.

## 4. Lock, load path and generated files

The regeneration on the merged tree followed r39's order
(`private/regen.sh`, logs `private/logs/regen-*.log`, exits in
`private/logs/regen.exits`, captured unpiped). `quirin-audit` was rebuilt
in release at the merged tree first, since SP1 changed the premise reader.

| Step | Exit |
|---|---|
| build `quirin-audit` (release, merged tree) | 0 |
| build `quirin-plugin` (`quirin-provider-plugin`) | 0 |
| `quirin-plugin load-path --extends contract-load-path.json --output user-crates-load-path.json quirin-provider-libcurl quirin-provider-openssl`, from `providers/` | 0 |
| `providers/package.py checkers` | 1, as expected (stale views) |
| `quirin-audit plugin views --load-path providers/user-crates-load-path.json --write` | 0 |
| `quirin-audit pin --load-path providers/user-crates-load-path.json --source-root docs/reports/openssl-provider-remaining-callees/evidence/fixture --crate quirin-provider-openssl` | 0 |
| `providers/contracts.py sync` | 0 |
| `providers/package.py checkers` | 0 |

The reviewed `contracts.lock` diff against main `9a769c6b0`
(`private/lockdiff.py`):

* **5 entries changed** (`contract_sha256` and `pinned_at`):
  * `ENGINE_free` `5f1bfe6a67b4a7d3aae28699546a1f0234118fe41b23fa85eb5170d8628bbe9f`
  * `EVP_MD_CTX_free` `5ece855d28d538d4b9a920e6dc2b72f6b15b3d642adb5eb4b50baf1bbcdf3b17`
  * `EVP_PKEY_free` `c260fd439b581f3b01d3f8dc90a1d650172bd8790c43c4f7d7213ff3fd275ec4`
  * `PKCS12_free` `d4b7e9be30544dc588a7d2ac40915ddcdd98375c4ba07aa6f2e6085ac8608b6b`
  * `X509_free` `c8153b1edf4d84dd6c07f8c6ed722b72736303c5790eead5e08ca1cf8eba7ebe`
* **0 added, 0 removed, 139 unchanged.** `pinned`, `library_versions`,
  `fluent_bit_revision`, `generated_by`, `lock_version`, `normalisation`,
  `note` and `owner` are unchanged; `flexible` holds the entries.

(The prose-era tip `b80ee7664` changed the same five entries against
`ecc03d7a4`, with other digests; those are superseded.)

`user-crates-load-path.json` and `quirin-plugin.json` are unchanged (no
binding added). `scripts/gen_rule_docs.py` and `scripts/gen_lean_roots.py`
were rerun (`docs/agents/gateway.md`, `proof/QCore.lean`).

**Specification and rule:** C04.18 gains "The object frees"; rule 12n is
amended in place. No new rule id.

## 5. Lean

The checker gains a rule (the free audit), so there is a new module,
`QCore.OpensslObjectFreesAudit`, audited by
`proof/AuditOpensslObjectFreesAudit.lean` (17 theorems; axioms within
`propext`, `Classical.choice` and `Quot.sound`; `private/logs/lean-2.log`,
`lean-audit-2.log`). Reworked after SP1: the statement identity is the one
formula, so the per-kind theorem was dropped and the module now imports
`QCore.PremiseFormula`.

* the audit is total over releases, and an unaudited release refuses;
* a running free names the free statement; dropping it or adding the
  handshake's refuses;
* the free statement stands only where the audit reads a run, and a free
  whose run another premise names carries it;
* the statement is well formed (`QCore.PremiseFormula.openssl_formulas_wf`);
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

## 7. Gate (first run, on `b80ee7664`)

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
* the `EX_DATA_AUDIT` classes of the three r39 frees omit the legacy key
  classes and `ENGINE`.

**Files likely to conflict** with other OpenSSL cards:

* `providers/quirin-provider-openssl/src/documented.rs` (`Frees`,
  `FREE_AUDIT`, `OBJECT_FREED`, `EX_DATA_AUDIT`) and `src/check.rs` (the
  `pub use`, `free_audited`, tests, the prose-premise count);
* `crates/qcore-frontend/tests/premise_formula.rs` (the formula counts);
* `contracts/openssl/engine.qv`, `evp.qv`, `evp_digest.qv`, `pkcs12.qv`,
  `x509.qv`;
* `contracts.lock` (regenerate in §4's order);
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18 and `docs/agents/rules/12n.md`;
* the generated `docs/agents/gateway.md` and `proof/QCore.lean` (rerun the
  generators).

## 9. Rework on main `9a769c6b0` (SP1's formula premises)

Main moved after the first hand-back: r39 landed (`b7f86a0de`), then
SP1-SYMBOLIC-OPENSSL-PREMISES (`9a769c6b0`) stated every OpenSSL premise as a
C04.18.9 formula and made the checker refuse prose (`check::prose_premise`).
The trial merge conflicted on the five modules, the lock and `check.rs`.

**Resolution.** The provider files were taken from main and the card was
reapplied in SP1's form (merge `cc18f4c59`):

* the five frees name SP1's free statement `UNSTATED_FREE_STATEMENT`
  (`quant all #o in #freed: #teardown(#o) \ (#stated | #settable) |->
  #inert`). SP1 converted r39's prose to this formula over the call's own
  `#freed`, which is what the five per-kind prose statements spelled out
  (§2), so they and `FreeStatement` were dropped rather than translated into
  five formulas. No premise was narrowed: the five frees gained the premise
  and keep `openssl_ex_data_callbacks`;
* `FREE_AUDIT` keeps its expectation, now `Runs`, `NamedBy` or `Nothing`
  without a statement kind (§3);
* SP1's own counts move with the five clauses: `a_prose_premise_is_refused`
  58 (was 53) and the frontend's `the_openssl_formulas_are_the_lean_definitions`
  8 `unstatedFreeFormula` clauses, 58 in all;
* the Lean module was reworked (§5); the lock regenerated (§4).

**No re-measurement.** Against the measured tip `b80ee7664`, the rework
changes only the text of the premise on the five declarations, from prose
to SP1's formula. SP1 states that a formula premise is metadata exactly as
a string one is, and no verdict-deciding consumer reads it
(`docs/reports/sp1-symbolic-openssl-premises/README.md`, "Semantics
unchanged"). No gateway code changed, and the checker accepts all five
(the regeneration's `package.py checkers` exit 0, and the gate's provider
tests). So the opt-in result of §6 (no move, `rows_changed` 0) carries over.
The base side would now be main `9a769c6b0`, which SP1's lane measured.

**Gate on the merged tree.** `sh private/toolchain/env.sh python3
scripts/gate_scope.py --base 9a769c6b0 --out private/gate-2`, log
`private/logs/gate-2.log`, record [`gate/gate-2.json`](gate/gate-2.json).
Tree `cc18f4c59`; the report and amendments were uncommitted (docs only).
Scoped, Lean all. **Exit 0, 19 steps passed.**

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (scoped runs, `qcore-frontend` included) | 0 | |
| Rust suite (scoped) | 0 | 274 units, 281 summaries; 3468 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 464 audits |
| Python suite | 0 | 779 tests (the base matrix-count failure of §7 is fixed on main) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 166 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

The slot flakes did not fire. The amendments check is `--check
--profiles-base 9a769c6b0`, exit 0 (the file's `base` is `9a769c6b0`).
