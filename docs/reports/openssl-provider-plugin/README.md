# OPENSSL-PROVIDER-PLUGIN: OpenSSL contracts on the user-contract SDK, with version-ranged declarations

Base `0f6e17118` (integrate/r12). The card narrows `W48-OPENSSL-CONTRACT-PLUGIN`:
OpenSSL's contracts now live in `providers/quirin-provider-openssl`, the first
in-repository crate built on the user-contract SDK
(`providers/quirin-provider-plugin`), each declaration states the OpenSSL
versions it holds for, and a checker plugin checks every binding. The measured
routes cannot move (the crate is reachable only through a generated user load
path, [PREDICTION](PREDICTION.md)); an opt-in run moves at most two rows
`Unknown -> Pending`, never `Safe`.

## 1. User decisions (verbatim)

- 2026-09-28 (recorded in `W48-OPENSSL-CONTRACT-PLUGIN`): OpenSSL contracts are
  owned and checked by their own plugin, composed through the four-value
  verdict without erasing any verdict or reason.
- 2026-11-16: "We need to implement OpenSSL related contracts should be built
  on top of quirin-provider-plugin. This should be the testing for the first
  time to depend on the plugin crate."
- 2026-11-16: "We need to pin the OpenSSL identity as per the requirements of
  OpenSSL version contracts. This should be tied the function contract with
  supported since."
- 2026-11-16: "We need to refer the official doxygen like documentation for
  OpenSSL to determine since each of the functions."
- 2026-11-16, answers to the card's stop (relayed; options in §3): version range
  "Contract clause (Recommended)"; held status "Split (Recommended)"; load
  path "User load path (Recommended)"; missing HISTORY "Documented-present
  (Recommended)".

## 2. Census

[`census/census.py`](census/census.py), run by [`census/run.sh`](census/run.sh)
(files read, never executed), writes [`census/census.json`](census/census.json)
and [`census/census.md`](census/census.md). A callee is OpenSSL when it has an
OpenSSL prefix and the pinned `libssl.so.3`/`libcrypto.so.3` export it or the
pinned headers declare or define it (exports are needed: OpenSSL 3 declares
`X509_free` and its kin only through `DECLARE_ASN1_FUNCTIONS`). A row (one
function of one unit) is blocked **sole** when every diagnostic it carries is
an uncontracted call to OpenSSL, **co** when it carries one beside any other
diagnostic.

| Route (report) | Units | OpenSSL callees | Sites | Pairs | Rows blocked | Sole | Co |
|---|---:|---:|---:|---:|---:|---:|---:|
| corpus (`baseline:497bdeb90`, FB 66910c10) | 185 | 68 | 147 | 99 | 29 | 4 | 25 |
| system (`baseline:497bdeb90`) | 15 | 0 | 0 | 0 | 0 | 0 | 0 |
| slice (`baseline:497bdeb90`) | 16 | 0 | 0 | 0 | 0 | 0 | 0 |
| current (linked, binary 89fa31d21, FB 046accf6) | 187 | 72 | 174 | 116 | 38 | 6 | 32 |

The six rows OpenSSL blocks alone on `current` (the corpus has the first four):
`rd_kafka_transport_ssl_clear_error` (`ERR_clear_error`),
`rd_kafka_ssl_ctx_term` (`ENGINE_free`, `SSL_CTX_free`),
`rd_kafka_transport_ssl_close` (`SSL_free`, `SSL_shutdown`),
`rd_kafka_ssl_PKEY_from_string` (`BIO_new_mem_buf`, `BIO_free`,
`PEM_read_bio_PrivateKey`), and Monkey's `tls_create_server_context`
(`SSL_CTX_new`) and `tls_init_library` (`OPENSSL_init_ssl`). Ranked by pairs on
`current`: `X509_free` 7, `BIO_free` 6, `BIO_new_mem_buf` 5, `EVP_PKEY_free` 5,
`SSL_write` 4, `ENGINE_free` 3, `ERR_clear_error` 3, `PEM_read_bio_PrivateKey`
3, `SSL_free` 3, then 63 callees with one or two pairs. One `current` row is
`Unsafe` (Monkey, co-blocked); every other blocked row is `Unknown`.

Sources: Fluent Bit's own OpenSSL callers (`src/tls/openssl.c` 73 functions /
133 sites, `flb_crypto.c` 35/69, `flb_hmac.c` 21/27, `flb_hash.c`,
`flb_oauth2.c`, `flb_fips.c`, `flb_utils.c`, `out_azure_kusto`) are analysed by
**no** route; Monkey's `lib/monkey/tls/tls_openssl.c` (37/54) is analysed by
`current` only (under `--monkey-linked`), and `lib/monkey/plugins/tls/tls_openssl.c`
by none.

**Pinned identity: a version.** `#include <openssl/opensslv.h>` preprocessed by
`/usr/lib/llvm-21/bin/clang` with every `-I` the two prepared compile databases
pass: OpenSSL **3.5.5**, `OPENSSL_VERSION_NUMBER` = `0x30500050` (MAJOR 3,
MINOR 5, PATCH 5, no pre-release), from `/usr/include/openssl/opensslv.h`
(sha256 `7acbe889…`; chain `macros.h`, `x86_64-linux-gnu/openssl/opensslconf.h`,
`configuration.h`, digests in `census.json`). One header set: the CMake cache
names `OPENSSL_INCLUDE_DIR=/usr/include` (OpenSSL found as v3.5.5, librdkafka
linked against `OpenSSL::SSL`), neither database passes an OpenSSL include
directory, and the checkout bundles no `opensslv.h`. Package
`libssl-dev 3.5.5-1ubuntu3.7 amd64`, installed 2026-10-02, before the
2026-11-09 prepare.

## 3. The stop and the choices

The card stopped after the census for two placements fail-closed did not
settle, each changing what the user writes or ships. The user chose:

1. **Where a version range lives: a contract clause.**
   `requires resource_library_version(LIB, since "V"[, removed "W"]);` (or
   `documented "V"`) in the declaration, the crate's version source declared
   once in `library-versions.json`, the pinned version recorded by
   `quirin-audit pin` in the crate's lock and re-read by every audit (contract
   specification C06.2, grammar G05, rule 53). Rejected: the lock alone (ranges
   cannot be derived from source, so a generated file would carry hand input)
   and a `quirin-plugin.json` field (only the checker could enforce it, and the
   checker task carries no identity of the audited build's headers).
2. **Held status: split.** A version that only differs from the pinned one is
   `library-version-changed`, held as `contract-pin-mismatch` (`Pending`,
   review and re-pin); out of range, unreadable or ambiguous hold every call
   `Unknown` (`stale-evidence`).
3. **Load path: a generated user load path.** The crate is a providers-workspace
   member (built, tested, packaged) but never a record of
   `providers/contract-load-path.json`, whose crates carry no checker
   bindings: a contract there would apply without its checker. It is listed in
   `providers/user-crates-load-path.json`, generated by the SDK
   (`quirin-plugin load-path --extends contract-load-path.json`) and its views
   by `quirin-audit plugin views --write`.
4. **No HISTORY entry: documented-present.** `documented "V"` names the oldest
   documentation series checked that documents the function, never an
   invented `since`.

## 4. The crate

`providers/quirin-provider-openssl` (namespace `openssl`), laid out as an SDK
user crate: `Cargo.toml` (depends on `quirin-provider-plugin` by path),
`src/lib.rs` (`NAMESPACE`, `PUBLISHED = []`, the embedded module texts),
`contracts/openssl/*.qv`, `contracts.lock`, `library-versions.json`,
`quirin-plugin.json` (one checker binding per function, all on
`checkers/openssl-check`), the checker `src/bin/openssl_check.rs` on the SDK's
`checker` runtime with its logic in `src/check.rs`, the pinned prototypes
`pinned/openssl-3.5.5.json` (`scripts/pin_prototypes.py`: the headers
preprocessed, since `X509_free` has no lexical declaration) and the fetched
documentation record `doc-snapshots.json` (`scripts/fetch_docs.py`: per page
and series the URL, the SHA-256 of the page as fetched on 2026-11-16, its
HISTORY text and its sentences on NULL and reference counts; pages are not
committed). `checkers/` is a build product (`providers/package.py checkers`).

Twelve contracts, every one flexible (no OpenSSL source is audited to pin),
version-ranged and checker-bound. Reference-counted objects follow the
2026-10-05 class-3 decision (C04.17): a free is `resource_reference_dropped`,
never a final free, and a constructor returns `resource_return_kind(reference)`.

| Function | Module | Range | Official page (series read) | Custody |
|---|---|---|---|---|
| `ERR_clear_error` | `err.qv` | `documented "1.0.2"` (HISTORY: all versions) | ERR_clear_error (3.5, 1.1.1, 1.0.2) | none |
| `OPENSSL_init_ssl` | `init.qv` | `since "1.1.0"` | OPENSSL_init_ssl (3.5, 1.1.1) | none |
| `TLS_server_method` | `ssl.qv` | `since "1.1.0"` | SSL_CTX_new (3.5, 1.1.1) | none (static table) |
| `SSL_CTX_new` | `ssl.qv` | `documented "1.0.2"` | SSL_CTX_new (3.5, 1.1.1, 1.0.2) | returned reference, nullable |
| `SSL_CTX_free` | `ssl.qv` | `documented "1.0.2"` | SSL_CTX_free | reference drop |
| `SSL_new` | `ssl.qv` | `documented "1.0.2"` | SSL_new | returned reference, nullable |
| `SSL_free` | `ssl.qv` | `documented "1.0.2"` | SSL_free | reference drop |
| `SSL_shutdown` | `ssl.qv` | `documented "1.0.2"` | SSL_shutdown | none (`ssl` borrowed) |
| `ENGINE_free` | `engine.qv` | `documented "1.1.1"` (1.0.2 page 404; deprecated 3.0, not removed) | ENGINE_add (3.5, 1.1.1) | reference drop |
| `X509_free` | `x509.qv` | `documented "1.0.2"` (HISTORY: all versions) | X509_new | reference drop |
| `EVP_PKEY_free` | `evp.qv` | `documented "1.0.2"` (HISTORY: all versions) | EVP_PKEY_new | reference drop |
| `BIO_free` | `bio.qv` | `documented "1.0.2"` | BIO_new | reference drop |

Not contracted, each fail-closed and named in the follow-ups:
`PEM_read_bio_PrivateKey` (its password callback needs a callback contract),
`BIO_new_mem_buf` (its BIO keeps reading the caller's buffer: a retention) and
the census's remaining callees.

## 5. The checker and its verdict mapping

`quirin-openssl-check` (plugin `openssl.contract_check`, target `openssl` at
crate revision `0.1.0`) answers one request per binding. For each bound
function it checks, from what is compiled into it only: (1) the request's
module digest is a module text it was built with; (2) the pinned headers
declare the function and the declaration's parameter and result kinds equal
the contract's; (3) the declaration states exactly one range, the one the
documentation supports, and it admits the pinned 3.5.5; (4) the cited page was
fetched and the custody clauses match the documented shape (a counted release
is exactly one reference drop; a constructor a nullable returned reference; a
plain function no custody). All pass: a `Safe` claim with no evidence and one
diagnostic per function naming the prototype, header digest, range and page
digest. Any failure: `Unknown` with the failure as a diagnostic.

| What happens | Published |
|---|---|
| checker `Safe`, no certificate (no trusted checker exists: QP008) | `Pending` (`plugin-claim-uncertified`) |
| checker `Unknown` (a check failed) | `Unknown` (its diagnostics kept) |
| package missing or not verifying, spawn failure, crash, timeout, quota or output breach, protocol error | `Unknown` (`provider-failure`) |
| target revision or library mismatch (host gate) | `Unknown` |
| headers' version out of the range, unreadable or ambiguous (core, rule 53) | contract withheld, calls `Unknown` |
| headers' version differs from the lock's (core) | contract withheld, calls `Pending` (`contract-pin-mismatch`) |

No `Unknown` becomes `Safe`: a function applying a bound contract publishes
the join of its own status and the binding's verdict, at least `Pending`.

## 6. Preservation trace

From the checker's answer to the published row, for the OpenSSL bindings (and
any user checker): the response's verdict and diagnostics reach the host
through `VerifierRegistry::verify_plugin_answer` (new), which returns the
firewall's classification and, beside it, the plugin's own answer
(`PluginAnswer`: proposed verdict, diagnostics). **The drop found:** before this
card the classification kept only the host's code and message for
`UncertifiedSafe`, `UnvalidatedUnsafe` and `CertifiedSafe`, so a checker's own
reasons (per function, here) were lost. Now each `plugin_checks` binding row
records `proposed` and `plugin_diagnostics` beside `claim`, `code`, `verdict`
and `reasons`; each function row's `plugin_links` entry names its binding row by
task digest (`binding`), so from a composed row one recovers, per bound callee,
the plugin, its proposed verdict, its own reasons and the classified verdict
with its reasons. The record decides nothing. `user_contract_plugins.rs` pins
it for `Safe`, `Unsafe` and failure answers. Not traced here (left on
`W48-PLUGIN-VERDICT-PRESERVATION`): checkpoint (`.qo`) composition, which
carries the section's digest only, and the Lean record-preservation theorem.

## 7. SDK changes (found by the first dependency)

`user_crate::cargo_manifest` read every `quirin-provider-*` key of
`[dependencies]` as a contract crate, so the first crate depending on the SDK
listed `quirin-provider-plugin` itself as a contract dependency; and it read a
dotted key (`quirin-provider-x.workspace = true`) as `quirin-provider-x.workspace`.
Both fixed (`NON_CONTRACT_PACKAGES`, the key before the first `.`), and
`[dependencies.NAME]` tables are now read; `tests/sdk.rs`
`dependency_keys_name_contract_crates_only` covers the five spellings.

## 8. Core changes and Lean

Generic, library-independent: the clause loader
(`qcore-contract/src/library_version.rs`), the schema
(`qcore-schemas/src/library_version.rs`, four new `PinMismatchReason`s with
`is_human_resolvable`), the probe and decision (`qcore-audit/src/library_version.rs`,
hooked at the end of `apply_pins`), `pin` recording `library_versions`, the
gateway hold's description by reason (`legacy_pin_hold::hold_description_for`)
and the reason phrase (`stale-evidence`). Every report and lock without a
range is byte-identical. `library_version_ranges.rs` runs the user decision's
fixtures through `quirin-audit pin` and `contracts`: in range and pinned ->
`Safe`; unpinned or a changed header version -> `Pending`
(`library-version-changed`); below `since`, past `removed` -> `Unknown`
(`library-version-out-of-range`); unreadable -> `Unknown`; two include
configurations disagreeing -> `Unknown` (`library-version-ambiguous`).

Lean: `proof/QCore/LibraryVersionRange.lean`, audited by
`proof/AuditLibraryVersionRange.lean` (17 theorems, axioms within `propext`,
`Classical.choice`, `Quot.sound`): the in-force predicate
(`decide_inForce_iff` and one theorem per failure), a declaration out of force
discharging nothing (`not_in_force_never_safe`,
`held_call_keeps_chain_non_safe`, the split), and the OpenSSL premises (a
counted release frees only at zero, so it is not final with another reference
outstanding and a final-free reading is refuted; a NULL-accepting release of
NULL is a no-op and discharges nothing).

## 9. Packaging

`providers/package.py checkers` builds each user crate's checker (release,
locked, offline), installs it as the SDK's package layout where its bindings
name it, and checks the user load path against it (`quirin-audit plugin views`
without writing, `pin --verify-contracts`). CI runs it before the provider
workspace tests, reusing the packaging target directory. `contracts.py` gains
the third package class (`USER_LOAD_PATH_PACKAGES`): excluded from the shipped
load path, checked by `check_user_crates` (every declaration bound to a
checker, every declaration of a crate with a version source stating a range, a
lock, the user load path listing exactly these crates, modules and bindings).
The shipped load path, its views and every shipped lock are unchanged.

## 10. The user path, run with the real binaries

[`evidence/user-path.sh`](evidence/user-path.sh), run once with the SDK's
`quirin-plugin`, this card's `quirin-audit` and `quirin-openssl-check`
([logs](evidence/run-1/), [summary](evidence/run-1/run.json)), every step exit
0: `quirin-plugin check` (7 modules, 12 checkers, 0 overrides), `package`,
`verify`, `load-path --extends contract-load-path.json` (writes
`providers/user-crates-load-path.json`), `quirin-audit plugin views --write`
(resource view: all 7 modules), `pin` (12 flexible entries; `= openssl: version
3.5.5 pinned (/usr/include/openssl/opensslv.h)`), `contracts` on
[`evidence/fixture/tls_user.c`](evidence/fixture/tls_user.c) through the real
headers (`checker plugins: 12 binding(s): 0 Safe / 12 Pending / 0 Unknown / 0
provisional Unsafe`; every declaration in force at 3.5.5), and `pin
--verify-contracts` (`lock lists every declaration (0 pinned, 12 flexible)`).
The fixture's rows:

| Function (shaped like) | Status | Why |
|---|---|---|
| `clear_error` (`rd_kafka_transport_ssl_clear_error`) | `Pending` | `ERR_clear_error` binding Pending |
| `init_library` (Monkey `tls_init_library`) | `Pending` | `OPENSSL_init_ssl` twice (`SSL_load_error_strings` expands to it) |
| `ctx_cycle` (acquire, test, drop) | `Pending` | a returned reference dropped once; three bindings Pending |
| `ctx_term` (`rd_kafka_ssl_ctx_term`) | `Unknown` | drops through `h->engine`, `h->ctx`: a reference this function does not hold (rule 13b) |
| `create_server_context` (Monkey) | `Unknown` | `return SSL_CTX_new(...)`: a nullable returned reference handed straight to the return is refused (`nullable resource ... used by a non-null contract`) |

The last row corrects the prediction's third move ([PREDICTION](PREDICTION.md),
"Correction"): the opt-in run moves at most two rows. The same selection the
opt-in `current` run uses ([`evidence/opt_in_providers.py`](evidence/opt_in_providers.py):
the profile manifest's 69 modules plus the OpenSSL crate) resolves on the user
load path without a collision (contracts route on the fixture, exit 0). Under
the prepared build's compile database
(`QCORE_LIBRARY_VERSION_COMPILE_DATABASE`), the probe read 49 distinct include
configurations, all resolving `/usr/include/openssl/opensslv.h` at 3.5.5: in
force, not ambiguous (18 s on the fixture).

## 11. Gate

`python3 scripts/gate_scope.py --base 0f6e17118 --out private/gate-1` on
`9adf8fa14` (full scope: the diff reaches `qcore-schemas`, `qcore-contract`
and `qcore-c-gateway`; provider gate yes; record [`gate/gate-1.json`](gate/gate-1.json),
Clang [`gate/clang-version.txt`](gate/clang-version.txt)):

| Step | Exit | Counts |
|---|---:|---|
| submodules, `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | 0, 0, 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 639 units, 680 summaries; 9515 passed, 0 failed, 17 ignored, 136 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 435 audits (`AuditLibraryVersionRange.lean` among them) |
| Python suite | 1 | 761 tests, 1 error: `test_gate_slot_queue.Stale.test_a_restarted_waiter_goes_to_the_back_and_the_counter_survives` (a gate-slot queue timing test on a loaded host; this card touches no gate-slot code) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | five packages |
| provider tests (`cargo test --manifest-path providers/Cargo.toml --workspace`) | 101 | 36 passed, 1 failed: `conformance/tests/packages.rs` `artifact_dependencies_are_only_schemas_and_serialization` (the dependency pin did not admit a user crate) |
| provider fmt, provider clippy | 0, 0 | |

The provider failure was a deliberate pin to update: commit `24240a300` admits
the user crate's dependencies (the schema surface and the SDK, nothing else
first-party) and counts it apart from the data-only crates. Reruns on `24240a300`: the
failed Python test alone exit 0 (`private/gate-1-reruns-2/python-queue-test.log`);
the conformance `packages` binary exit 0, 6 passed, 0 failed, and the
providers' Python tests exit 0, 26 tests (`private/gate-1-reruns-3/`); the
SDK's and the OpenSSL crate's tests exit 0, 16 passed, 0 failed
(`private/logs/ptest.log`); provider clippy exit 0. The whole provider
workspace was rerun once in the short lane and stopped at its 30-minute
deadline after 27 passed and 0 failed (`private/gate-1-reruns-2/`): the
rust-suite slot it belongs in was held behind other cards' audits for over two
hours, so the full provider-test rerun is left to the landing gate.

## 12. Follow-ups

Registered in [`integration-amendments.json`](integration-amendments.json):
`LIBRARY-VERSION-PROBE-ROUTE-DATABASE` (the current route does not hand its
compile database to the probe), `REFERENCE-DROP-NULL-ACCEPTING` (OpenSSL's
documented NULL-accepting frees against rule 13b's refusal of a possibly-NULL
drop), `FLUENT-BIT-TLS-UNITS-UNANALYSED`; `W48-OPENSSL-CONTRACT-PLUGIN`
narrowed (remaining callees, the measured opt-in arbiter) and
`W48-PLUGIN-VERDICT-PRESERVATION` amended (the drop found and fixed; `.qo`
composition, Lean and negative controls open). Concurrent work:
CURRENT-CONTEXT-NEW-TRIAGE reads Monkey's `context_new` in `tls_openssl.c`,
which calls `SSL_new` on the server context (`SSL_CTX_new` is in `tls_create_server_context`), both contracted here (returned
references); this card changes no gateway leak reading, but its contracts
change which calls that triage sees as uncontracted on an opt-in run.

## Integrator measurement (2026-11-16)

Declared measurement, base `rev:0f6e17118` (the card's base; r8–r12 landed after the newest published baseline `497bdeb90` without a publish, so the card base isolates this card), tip `rev:24240a300`, run from the main checkout (`private/measure/openssl-20261116T080259Z`): exit 0, all three routes complete, no unmatched rows, declared result **match**.

- corpus: 0 rows moved; 0 rows changed content only.
- system: 0 rows moved; 0 rows changed content only.
- slice: 0 rows moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`96260b405ddf199dab7cd79fa0ab20d86e9b39c91e682c00696b9f9e5a5304cf`).

The crate loads only through the generated user load path, which no pinned route uses, so nothing moves; the shipped load path, views and locks are unchanged. Its effect is on an opt-in `current --load-path providers/user-crates-load-path.json` run. Landed through integrate/r14 with CURRENT-CONTEXT-NEW-TRIAGE (phase 1), LINK-STANDING-IMPORT-LIMITERS and OPENSSL-PROVIDER-PLUGIN; all three merged without conflict.
