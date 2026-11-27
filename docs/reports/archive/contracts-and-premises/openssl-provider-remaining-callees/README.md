# OPENSSL-PROVIDER-REMAINING-CALLEES: the census's remaining OpenSSL callees, and the measured opt-in arbiter

Base `32336c33d` (origin/main). Follow-up `OPENSSL-PROVIDER-REMAINING-CALLEES`
(OPENSSL-PROVIDER-PLUGIN's narrowing of `W48-OPENSSL-CONTRACT-PLUGIN`),
launched by the user 2026-11-17. The user crate
`providers/quirin-provider-openssl` grows from twelve to 57 version-ranged,
checker-bound declarations; 16 census callees stay uncontracted, each with a
named reason and a sized follow-up. The fixture run found that the plugin fold
published a function `Safe` over an uncertified binding when the bound call
was a scalar `return f();`; the card fixes the fold. [PREDICTION](PREDICTION.md)
was committed before any declaration (`f1677b59c`), its correction before the
measurement (`46976389f`).

## 1. What was contracted

Every declaration is flexible, states one `resource_library_version(openssl,
…)` range read off the official manual pages' HISTORY sections (`documented`
at the oldest series checked that documents the function when HISTORY names
no version: user decision 2026-11-16), and is bound to the crate's checker.
Pages were fetched on 2026-11-17 with `scripts/fetch_docs.py --merge` (new:
existing entries are never refetched, so the twelve reviewed digests stay;
each new entry carries its own `fetched_on` and its NAME section); 102 new
page snapshots, 4 of them 404 records. The custody of every declaration is
read off the page of the pinned series (3.5), as OPENSSL-PROVIDER-PLUGIN read
it; the range is where the function is documented. The prototypes come from
the pinned 3.5.5 headers (`scripts/pin_prototypes.py`, now also over
`stack.h`, `pkcs12.h`, `provider.h`, `x509_vfy.h`, `pem.h`; the twelve
existing entries are byte-identical).

| Shape (vocabulary) | Functions | Module |
|---|---|---|
| Plain, no custody | `ERR_peek_error` (documented 1.0.2), `ERR_peek_last_error` (since 0.9.7), `ERR_error_string_n` (since 0.9.6) | `err.qv` |
| Plain | `EVP_sha256`, `EVP_sha512` (a const algorithm object the caller does not free), `EVP_EncodeBlock` | `evp_digest.qv` |
| Fresh object / release (heap) | `EVP_MD_CTX_new`, `EVP_MD_CTX_free` (nullable; both since 1.1.0) | `evp_digest.qv` |
| Plain + registered-callbacks premise | `SSL_write`, `SSL_read`, `SSL_connect`, `SSL_do_handshake` | `ssl_io.qv` |
| Plain | `SSL_get_error`, `SSL_set_accept_state` | `ssl_io.qv` |
| Borrowed result (C04.3.4) | `SSL_get_certificate` (documented 3.0: "owned by the SSL object"), `SSL_get_client_CA_list` ("should not be freed") | `ssl_io.qv` |
| Returned reference (C04.17) | `SSL_get1_peer_certificate` (since 3.0.0) | `ssl_io.qv` |
| Plain | `SSL_CTX_set_options` (since 3.0.0: the `uint64_t` prototype), `SSL_CTX_set_default_verify_paths` (1.1.1), `SSL_CTX_load_verify_locations` | `ssl_ctx.qv` |
| Plain; the context takes its own reference | `SSL_CTX_use_certificate`, `SSL_CTX_use_PrivateKey` ("On success the reference counter … is incremented") | `ssl_ctx.qv` |
| Plain + registered-callbacks premise | `SSL_CTX_use_PrivateKey_file`, `SSL_CTX_use_certificate_chain_file` (may run the password callback) | `ssl_ctx.qv` |
| Borrowed result | `SSL_CTX_get_cert_store` | `ssl_ctx.qv` |
| Retention (C04.14) | `SSL_CTX_set_default_passwd_cb_userdata` (the context keeps `u`) | `ssl_ctx.qv` |
| Returned reference | `X509_new` | `x509_store.qv` |
| Borrowed result | `X509_get_issuer_name` ("internal pointer which MUST NOT be freed"), `X509_STORE_get0_objects` (since 1.1.0), `X509_STORE_CTX_get_current_cert` | `x509_store.qv` |
| Counted drop (C04.17) | `X509_STORE_free` (`X509_STORE_up_ref` exists) | `x509_store.qv` |
| Plain | `X509_STORE_set_flags`, `X509_STORE_CTX_get_error`, `X509_STORE_CTX_get_error_depth`, `X509_STORE_CTX_set_error` | `x509_store.qv` |
| Returned reference, premises `x = 0`, `cb = 0` (C04.5) | `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` | `pem.qv` |
| Fresh object, premise `p12 = 0`; release (nullable) | `d2i_PKCS12_bio`, `PKCS12_free` | `pkcs12.qv` |
| Fresh object / release (nullable) / borrowed result / retention | `OPENSSL_sk_new_null`, `OPENSSL_sk_free`, `OPENSSL_sk_value`, `OPENSSL_sk_push` (documented 3.0: the 1.1.1 page documents only the macros) | `stack.qv` |
| Returned reference | `BIO_new_file`, `ENGINE_by_id` | `lookup.qv` |

New declarations live in new modules (and `err.qv`) so the six frees
REFERENCE-DROP-NULL-ACCEPTING owns (`bio.qv`, `engine.qv`, `evp.qv`,
`ssl.qv`, `x509.qv`) are untouched.

**The registered-callbacks premise.** The handshake and I/O calls, and the
two `*_file` loaders, may run callbacks the SSL object or its SSL_CTX carries
(verification, certificate, password, info, message). No clause states a
hidden registry on a holder with several callback slots (C04.19's
registration replaces "any earlier registration"), so these declarations
name the reading as a C04.18 premise, `openssl_registered_callbacks`,
published with every row that applies them: the callbacks take, release or
retain nothing the caller holds. This makes explicit what OPENSSL-PROVIDER-PLUGIN's
`SSL_shutdown`, `SSL_free` and `SSL_CTX_free` read silently; those are left
as they are (the frees are another card's). A reviewer who rejects the
premise withdraws six declarations, not the rest.

**The PEM readers.** The brief named `PEM_read_bio_PrivateKey` an exception
unless the callback vocabulary states it exactly. `resource_callback_invoked(cb,
_, _, _, u)` does, but the loader admits no callback clause beside the
literal-argument premise the `x` reuse arm needs ("a retention clause stands
only beside plain parameter and result predicates", `contract_loader.rs`, on
`literal_arguments`). So each reader holds only where both `x` and `cb` are a
null pointer constant: Monkey's `tls_load_builtin_credentials` (`tls_openssl.c:176,
188`) and the OIDC assertion (`rdkafka_sasl_oauthbearer_oidc.c:495, 498`).
Every librdkafka call passing `rd_kafka_conf_ssl_passwd_cb` or
`rd_kafka_transport_ssl_passwd_cb` (`rdkafka_cert.c:272, 349, 405`,
`rdkafka_ssl.c:761, 790, 833, 1292`) stays uncontracted.

## 2. Left uncontracted (16), each fail-closed

| Callee | Why no declaration states it | Follow-up |
|---|---|---|
| `BIO_new_mem_buf` | the BIO keeps reading the caller's buffer; C04.14.2 admits a `result` holder only with `resource_return_owned(nonnull)`, and this result may be NULL | `OPENSSL-NULLABLE-RESULT-HOLDER` |
| `PEM_read_bio_*` with a callback | no callback clause beside a literal premise | `OPENSSL-PEM-CALLBACK-ARM` |
| `SSL_CTX_ctrl`, `SSL_ctrl` | custody switches on `cmd` (`SSL_CTX_add_extra_chain_cert` hands over the certificate); a guard exists only on retentions | `OPENSSL-CTRL-COMMAND-ARMS` |
| `SSL_CTX_set_cert_cb`, `SSL_CTX_set_default_passwd_cb` | a registration into one of a context's several callback slots | `OPENSSL-SSL-CTX-CALLBACK-SLOTS` (also discharges the registered-callbacks premise) |
| `OPENSSL_sk_pop_free` | a synchronous callback that releases (drops a reference on) every element; C04.19 lets only a registry's `element` position release | `OPENSSL-STACK-POP-FREE-CALLBACK` |
| `SSL_set_fd` | the SSL keeps the descriptor; no retention form takes an integer | `OPENSSL-DESCRIPTOR-RETENTION` |
| `SSL_CTX_use_cert_and_key` | the page states the custody of the key only, not of the certificate or chain | in `OPENSSL-UNDOCUMENTED-CALLEES` |
| `ERR_peek_last_error_all` | four out slots receive pointers the error queue keeps; C04.16's derived out slot derives from a parameter, not a global | `OPENSSL-QUEUE-OUT-SLOTS` |
| `X509_NAME_oneline` | allocates when `buf` is NULL, returns `buf` otherwise; the premise can bound only the NULL arm | `OPENSSL-NONNULL-ARGUMENT-PREMISE` |
| `CRYPTO_free` | releases what OpenSSL's allocator returned; no allocator kind is declared (C04.6) | `OPENSSL-ALLOCATOR-KIND` |
| `OSSL_PROVIDER_load`, `OSSL_PROVIDER_unload` | the page does not state who holds the provider after a failed unload (returns 0) | in `OPENSSL-UNDOCUMENTED-CALLEES` |
| `ENGINE_load_ssl_client_cert`, `X509_OBJECT_get0_X509`, `SSL_certs_clear` | no official manual page documents them (`SSL_certs_clear` is named once in another page's NOTES) | `OPENSSL-UNDOCUMENTED-CALLEES` |

[`census/rerank.md`](census/rerank.md) (from [`census/rerank.py`](census/rerank.py)
over OPENSSL-PROVIDER-PLUGIN's census, unchanged) ranks what is left: on
`current`, 16 of 72 callees and 22 of 116 pairs; 13 rows still blocked by an
uncontracted OpenSSL callee, one of them with no other diagnostic
(`rd_kafka_ssl_PKEY_from_string`, `BIO_new_mem_buf`); 25 rows have every
OpenSSL callee contracted (by name; the PEM readers only within their
premises).

## 3. The checker

`quirin-openssl-check` now derives each declaration's **exact** clause set
from its documented entry (`src/documented.rs`: page, series, range, shape,
premises) and refuses any other (`check::expected_clauses`): every parameter
borrowed except a non-counted release's own; a counted release a reference
drop; a returned reference `resource_return_owned(nullable)` with
`resource_return_kind(reference)`; a borrowed result
`resource_return_borrowed_from(p, n)`; a retention `resource_retained_by`;
each premise exactly. The prototype check gains `long`, `unsigned long`,
`size_t`, `T **` / `const T **` (C04.1 rule 12v) and a pointer to the
`pem_password_cb` typedef as `fnptr`, and refuses every other spelling. New
tests refuse a borrowed result read as owned, a dropped retention, a returned
reference read as heap, a missing premise, a counted drop read as a final free
and the reverse, and an unsupported `since`; every cited page must be a
fetched snapshot. Lean: `QCore.OpensslCalleeShapes` (audited by
`proof/AuditOpensslCalleeShapes.lean`, 13 theorems, axioms within `propext`
and `Quot.sound`): one accepted clause set per function, no final release on
a counted shape, a borrowed result owning nothing, a retention never
borrowed-and-forgotten, a plain call stating no custody, every premise stated.

## 4. The fold fix (found by the fixture)

`qcore_audit::user_plugins::fold_unit` joined a function's status with the
binding verdicts of the callees its control-flow graph lowered to
`ContractCall`. A scalar `return f();` over a contracted callee is admitted
without such a node, so `peek` (`return ERR_peek_error();`) was published
**`Safe`** over a `Pending` binding ([`evidence/base-binary-run/run.json`](evidence/base-binary-run/run.json):
the new contracts with the base binary). Now, once a unit row carries its
contract dependencies (rule 13f), `fold_dependencies` folds the links of every
bound declaration a function rests on that the first fold did not link
(`report.fold_last_unit_dependencies`, called in `projection.rs` after
`contract_dependencies::annotate_unit`). It only raises; without links it
returns at once, so no pinned or configured route is touched.
`user_contract_plugins.rs::a_bound_contract_applied_in_a_return_expression_joins_its_binding`
pins it (`return f();` and `n = f(); return n;` both `Pending`, each with its
link). Lean: `published_never_lowers`, `resting_on_pending_is_never_safe`,
`dependencies_cover_applied` in the same module. This was latent since
OPENSSL-PROVIDER-PLUGIN (`return SSL_shutdown(s);` would have been `Safe`);
the libcurl user crate's bindings go through the same fold.

## 5. The user path, run with the real binaries

[`evidence/user-path.sh`](evidence/user-path.sh) (OPENSSL-PROVIDER-PLUGIN's,
on this card's fixture [`evidence/fixture/tls_more.c`](evidence/fixture/tls_more.c)),
run with the tip `quirin-audit`, `quirin-plugin` and checker
([logs](evidence/run-1/), [summary](evidence/run-1/run.json)), every step exit
0: `check` 15 modules, 57 checkers; `plugin views --write` (all 15 modules in
the resource view); `pin` 0 pinned / 57 flexible (the lock diff adds 45
entries and changes none); `contracts`: 57 bindings, 0 Safe / 57 Pending;
`pin --verify-contracts`.

| Fixture function (shaped like) | Status | Why |
|---|---|---|
| `peek`, `error_string`, `write_some`, `digest_cycle`, `pkcs12_cycle`, `store_flags`, `engine_cycle`, `peer_check`, `read_cert` | `Pending` | every call contracted; references and fresh objects acquired and released once |
| `keep_userdata` | `Unknown` | the retention into a caller's context is undecided (`retention-undecided`) |
| `read_cert_reuse` | `Unknown` | `&slot` at `x`: outside the premise, uncontracted |
| `verify_depth` | `Unknown` | the borrowed `cert` is used after a call naming its parent (`interior-borrow-ended`, rule 12y) |

## 6. The measured opt-in arbiter

`quirin-audit current --monkey-linked` over Fluent Bit `046accf6` as
`prepare --run` invokes the user's run, the contract selection moved to the
side tree's `providers/user-crates-load-path.json` (the profile manifest's
modules plus `quirin-provider-openssl`, OPENSSL-PROVIDER-PLUGIN's
`opt_in_providers.py`), `SOURCE_DATE_EPOCH=1767225600`,
`QCORE_LIBRARY_VERSION_COMPILE_DATABASE` the run's database, `--jobs 12`
([script](evidence/opt-in/opt-in-current.sh)). Inputs prepared on this host
by `quirin-audit prepare --fluent-bit ../fluent-bit --revision 046accf6…
--clang /usr/lib/llvm-21/bin/clang` into `private/fb-046accf6` (no shared
preparation exists here). Three runs, each a `git archive` of its tree with
its checker installed by that tree's `package.py checkers` step:

| Run | Binary | Load path | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings | Wall |
|---|---|---|---|---|---|---|
| base | `32336c33d` | `32336c33d` (12 contracts) | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1097 / 2 / 3700 / 5 | 12, all `Pending` | 58 min |
| fix-only | tip | `32336c33d` | same | 1097 / 2 / 3700 / 5 | 12, all `Pending` | 55 min |
| tip | tip (`e556fea4f`) | tip (57 contracts) | same | 1097 / 2 / 3700 / 5 | 57, all `Pending` | 54 min |

Every run exit 0, no unmatched row ([run summaries](evidence/opt-in/),
[invocations](evidence/opt-in/invocation-tip-1.json)). The two `Pending`
rows are OPENSSL-PROVIDER-PLUGIN's corrected prediction, now measured:
`rd_kafka_transport_ssl_clear_error` and `tls_init_library`
(`plugin-claim-uncertified`, host status `Safe`).

**No row moved, as predicted.** Base to tip: 0 status moves, 32 rows changed
content only ([comparison](evidence/opt-in/comparison-base-tip.json), each
row with the links it gained, the uncontracted-callee diagnostics it lost and
the other diagnostics it gained or lost). Split:

* **The fold fix alone** (base to fix-only, [comparison](evidence/opt-in/comparison-base-fixonly.json)):
  7 rows, all `Unknown`, gain links to first-card bindings their
  `ContractCall` nodes did not show (`ERR_clear_error` on
  `rd_kafka_transport_ssl_connect`, `_io_event`, `_recv`, `_send`; `SSL_new`
  on `_connect`; `SSL_CTX_new` on `rd_kafka_ssl_ctx_init`; Monkey's
  `mk_tls_exit` `SSL_CTX_free`, `mk_tls_init` `OPENSSL_init_ssl`). No row of
  this run was `Safe` over a missed binding, so the fix moves nothing here;
  it moved the fixture's `peek`.
* **The new declarations** (fix-only to tip, [comparison](evidence/opt-in/comparison-fixonly-tip.json)):
  30 rows. On 17 only the links and the uncontracted-callee diagnostics
  change (`SSL_write`, `SSL_read`, `SSL_connect`, `SSL_do_handshake`,
  `SSL_get_error`, `SSL_set_accept_state`, `SSL_get1_peer_certificate`,
  `ENGINE_by_id`, `ERR_peek_error`, `ERR_error_string_n`,
  `EVP_sha256`/`EVP_sha512`, `EVP_EncodeBlock`,
  `SSL_CTX_load_verify_locations`, the `*_file` loaders; five of the 17,
  outside the census's rows, only gain a link: `rd_base64_encode`,
  `rd_kafka_ssl_set_ca_path`, `mk_tls_read`, `mk_tls_write`,
  `tls_load_credentials`); each keeps other diagnostics and stays `Unknown`.
  On `rd_kafka_transport_ssl_recv` and `_send` the contracted `SSL_read` /
  `SSL_write` now let the analysis reach an `unresolved resource identity
  rktrans` it did not reach before.
  The new readings refuse where the analysis cannot decide them, never
  accuse: `interior-borrow-ended` on borrowed results used after a call
  naming their parent (`rd_kafka_ssl_cert_callback`, `_cert_issuer_match`,
  `_cert_verify_cb`, `_set_certs`); `derived result of OPENSSL_sk_value from
  an unmodeled base expression` (the `sk_X509_value` macro passes
  `ossl_check_X509_sk_type(sk)`, a call); `retention-undecided` for
  `OPENSSL_sk_push` (an `ossl_check_*` actual) and
  `SSL_CTX_set_default_passwd_cb_userdata` (`rk` into the caller's
  context); `call … outside its declared literal-argument premise` for the
  PEM readers given a callback (`rd_kafka_cert_new`,
  `rd_kafka_ssl_PKEY_from_string`, `rd_kafka_ssl_X509_from_string`);
  `reference-drop-unheld` for `X509_STORE_free(cert->store)` (a member
  read, rule 13b). Monkey's `tls_load_builtin_credentials` applies both PEM
  readers within their premise: its `X509_free(cert)`/`EVP_PKEY_free(key)`
  drops are now of held references (two `reference-drop-unheld` and a
  branch-condition refusal leave), and the row stays `Unknown` on its other
  diagnostics. No new `Unsafe`.

**The pinned routes, by construction.** The corpus, system and slice routes
(`run_pinned_audit.py`, the measure plugin's configured selections) take no
`--load-path`; their contracts come from `providers/contract-load-path.json`
and `cycle19-resource-contracts.json`, which this card does not change
(`git diff 32336c33d` touches neither, nor `contract/`). A report's
`plugin_links` exist only when `user_plugins::run` finds checker bindings in
a user load path (`current.rs:368`, `no_build.rs:787`), and both folds return
at once without links, so the fold fix cannot reach them; the twelve
declarations' crate and its new ones are not on the shipped load path
(`providers/contracts.py check`, exit 0). The prediction's
`measure-prediction` block (no move on corpus, system, slice) therefore
holds without a declared measurement on this host, which has no prepared
pinned-route inputs; the integrator's `quirin-audit measure --declared` run
against `rev:32336c33d` is the arbiter of that block.

## 7. Gate

`python3 scripts/gate_scope.py --base 32336c33d --out private/gate-1/gate`
through `private/toolchain/env.sh` (Clang 21.1.8, Lean 4.35.0) on
`46976389f` (the code of `e556fea4f`; the report uncommitted); scoped, not
full: the diff reaches `qcore-audit`, a proof root and `providers/`
(record [`gate/gate-1.json`](gate/gate-1.json), logs `private/gate-1/gate/`).

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (workspace and the four scoped crates) | 0 | |
| Rust suite (scoped: 7 packages, 42 dependent targets) | 0 | 268 units, 275 summaries; 3411 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 443 audits (`AuditOpensslCalleeShapes.lean` among them) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 101 | 32 summaries; 66 passed, 1 failed: `quirin-provider-openssl` `a_drifted_shape_of_the_new_readings_is_refused` |
| provider fmt, provider clippy | 0, 0 | |

The failure was this card's test: after `pem.qv` lost its callback clause
(§1) the test still removed that clause, a no-op the checker rightly
accepted. `4d9d59499` makes it remove the `cb` premise instead (a test-only
change: no module, checker logic or measured input changes). Reruns on
`4d9d59499` (`private/gate-1-reruns/`): `cargo test --manifest-path
providers/Cargo.toml --workspace --locked --offline` exit 0, 46 summaries,
142 passed, 0 failed; provider clippy exit 0; provider fmt exit 0. Earlier,
`qcore-audit`'s `user_contract_plugins` binary alone: exit 0, 10 passed
(`private/logs/ucp-test.log`); `providers/contracts.py check` exit 0.

## 8. Follow-ups

[`integration-amendments.json`](integration-amendments.json) retires
`OPENSSL-PROVIDER-REMAINING-CALLEES` (both halves done: every census callee
contracted or registered with its reason, and the opt-in arbiter measured)
and registers the ten follow-ups of §2, each with its size. Not registered,
named for the integrator: the registered-callbacks premise is a reading a
reviewer may reject (it withdraws six declarations; `OPENSSL-SSL-CTX-CALLBACK-SLOTS`
replaces it); the fold fix also governs the libcurl user crate's bindings
(LIBCURL-PROVIDER-PLUGIN runs in parallel; its rows can only rise).
Files likely to conflict: `providers/quirin-provider-openssl/contracts.lock`,
`providers/user-crates-load-path.json` (regenerate: `quirin-plugin load-path`,
`quirin-audit plugin views --write`, `quirin-audit pin`), `quirin-plugin.json`,
`src/lib.rs`, `proof/QCore.lean` (rerun `scripts/gen_lean_roots.py`),
`crates/qcore-audit/src/user_plugins.rs`.
