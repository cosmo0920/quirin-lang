# W48-LIBCURL-CONTRACT-PLUGIN: libcurl contracts through a quirin-audit plugin with its own checker

Card W48-LIBCURL-CONTRACT-PLUGIN (user decision 2026-09-28; registered
follow-up of the same name). Base `d7a497822`. The
[prediction](PREDICTION.md) was committed at `142680fe8`, before any
implementation. New crate `crates/quirin-audit-plugin-libcurl` (subcommand
`quirin-audit libcurl`, checker package `quirin-libcurl-checker`); new Lean
module `QCore.LibcurlContractPlugin` audited by
`proof/AuditLibcurlContractPlugin.lean`. No analyzer crate, shipped contract,
load-path entry, specification section or numbered rule changes; the plugin
adds no normative behaviour beyond contract specification C11.1 and C11.2.
Usage: [audit subcommand plugins](../../../../development/audit-subcommand-plugins.md#the-libcurl-plugin).

## Headline

* **Pinned preparation.** libcurl **8.18.0**, the upstream release tarball
  `curl-8.18.0.tar.gz`, SHA-256
  `e9274a5f8ab5271c0e0e6762d2fce194d5f98acc568e4ce816845b2dcc0cf88f` (tag
  `curl-8_18_0`, commit `2eebc58c4b8d68c98c8344381a9f6df4cca838fd`), and
  each of the 12 files the checker reads by its own digest
  (`src/pin.rs`). `prepare` refuses any other tarball; `check`, `install`
  and the checker package refuse any tree whose read files differ. How the
  version was determined: §1.
* **Contracts and the checker.** One binding per ranked callee:
  `curl_global_init` first, then the librdkafka census's twelve real
  functions. The plugin's lexical checker establishes **6** contracts
  against the pinned source (`curl_global_init`, `curl_slist_append`,
  `curl_slist_free_all`, `curl_easy_escape`, `curl_url`,
  `curl_url_cleanup`; 54 obligations, all hold) and answers **6** `Unknown`
  with a named reason and checked witnesses (`curl_easy_setopt`,
  `curl_easy_getinfo`, `curl_easy_perform`, `curl_easy_cleanup`,
  `curl_url_set`, `curl_url_get`): §2,
  [record](artifacts/libcurl-check.json).
* **Through the firewall only.** Subcommand route (C11.2): 6 `Pending`
  (`QP016`, uncertified) and 6 `Unknown` rows, none `Safe`
  ([report](artifacts/check.report.json)). Checker-binding route (C11.1),
  `quirin-audit current` over librdkafka's two libcurl callers at Fluent Bit
  `046accf6` with and without the plugin's user load path: the six bindings
  are `Pending` (`QP008`), **`rd_http_global_init` Unknown → Pending**
  (its only blocker was `curl_global_init`), six more callers link a bound
  contract and stay `Unknown` on other blockers, no row moves to `Safe`, no
  row moves into or out of `Unsafe` ([comparison](artifacts/callers-comparison.json)).
  This is exactly the prediction's second half.
* **Arbiter (corpus, system), this card's own preparations:** §4.
* **The 44 typecheck stubs** (152 pairs) are not bound and not contracted:
  §3 and follow-up `W48-LIBCURL-TYPECHECK-STUBS`.

## 1. The pin and how it was determined

The measured routes were prepared on Ubuntu 26.04 hosts (`Ubuntu clang
21.1.8`, GNU ld 2.46: FLUENT-BIT-LINK-PREPARE). W43-LIBC-CONTRACTS records
the routes' libcurl declarations at `/usr/include/x86_64-linux-gnu/curl/curl.h`
(`libcurl4-openssl-dev`), and FLUENT-BIT-LINK-PREPARE links `fluent-bit`
against the distribution's `libcurl.so.4`. Ubuntu 26.04 (`resolute`,
`resolute-updates`) ships only curl 8.18.0 (`8.18.0-1ubuntu2.x`; this host
has the runtime `8.18.0-1ubuntu2.8`). Cross-checks, all in this card's
private download directory:

| Check | Result |
|---|---|
| `curl.se/download/curl-8.18.0.tar.gz` SHA-256 against Ubuntu's `curl_8.18.0.orig.tar.gz` (`.dsc` of `8.18.0-1ubuntu2.8`) | equal (`e9274a5f…`); the `.asc` signature files are equal too (`449692ee…`) |
| `git ls-remote` of `refs/tags/curl-8_18_0` | tag object `77e260ec…`, commit `2eebc58c…` |
| headers of `libcurl4-openssl-dev_8.18.0-1ubuntu2.8_amd64.deb` against the tarball's `include/curl/` | equal except Ubuntu's `CURLOPT_STREAM_DEPENDS{,_E}` deprecation in `curl.h` |
| the census's 44 `Wcurl_easy_*_err_*` names against `typecheck-gcc.h` | exactly 8.18.0's set (8.17.0 and 8.19.0 define the same set: corroborating, not discriminating) |
| Ubuntu's patch series against the 12 read files | 4 files touched: `curl.h` (the deprecation), `lib/urlapi.c` (appends `Curl_url_same_origin`), `lib/getinfo.c` and `lib/setopt.c` (read only for `Unknown` witnesses); no patched hunk is in a function a `Safe` answer rests on |

What is pinned is the upstream release; each claim names
`upstream-release` as an assumption. The plugin reads no network: the
tarball is an argument.

## 2. The checker and the bindings

The checker ([`checker.rs`](../../../../../crates/quirin-audit-plugin-libcurl/src/checker.rs))
states, per binding, lexical obligations over the pinned files' tokens in six
decidable shapes: a definition's exact header (exactly one definition, no
preprocessor directive in its body), a declaration's exact spelling, a
contiguous token sequence in a body, every occurrence of a parameter inside
named shapes, the complete set of `return` expressions, and a file-level
sequence (allocator macros, file-scope initializers, switch cases). A
binding is `Safe` exactly when the tree matches the pin and every obligation
holds; there is no `Unsafe` answer. The obligations' soundness (that a shape
implies its clause) and five named assumptions (`upstream-release`,
`release-build`, `default-allocators`, `libc-c-standard-abi`, `heap-kind`)
are the checker's premises; `QCore.LibcurlContractPlugin` proves the
decision and its composition over them.

| Rank | Function | Answer | Contract (`libcurl/libcurl.qv`) or reason |
|---:|---|---|---|
| 1 | `curl_global_init` | Safe (5 obligations) | `borrowed(flags)`, `return_none` |
| 2 | `curl_slist_append` | Safe (13) | `owned(list, nullable)`, `borrowed(data)`, `return_reallocated(list, nullable)`: the realloc reading |
| 3 | `curl_slist_free_all` | Safe (7) | `released(list, nullable)`, `return_none` |
| 4 | `curl_easy_setopt` | Unknown | option-indexed retention over a variadic tail (`CURLOPT_HTTPHEADER`, `CURLOPT_ERRORBUFFER` keep the pointer: witnessed in `lib/setopt.c`) |
| 5 | `curl_easy_cleanup` | Unknown | `Curl_close` may run callbacks the handle registered |
| 6 | `curl_easy_escape` | Safe (17) | `borrowed(handle)`, `borrowed(text)`, `borrowed(length)`, `return_owned(nullable)` |
| 7 | `curl_easy_getinfo` | Unknown | option-indexed out-parameter (`CURLINFO_CONTENT_TYPE` hands back memory the handle owns: witnessed in `lib/getinfo.c`) |
| 8 | `curl_easy_perform` | Unknown | runs callbacks registered by an earlier `curl_easy_setopt` |
| 9 | `curl_url` | Safe (5) | `return_owned(nullable)` |
| 10 | `curl_url_cleanup` | Safe (7) | `released(handle, nullable)`, `return_none` |
| 11 | `curl_url_get` | Unknown | an out-slot allocation, which resource-v1 reads only with a kind |
| 12 | `curl_url_set` | Unknown | the string flows to parsing helpers whose non-retention is not established |

The contracts are a **user** crate's module (C11.1), flexible (listed as
explicit assumptions of a run that applies them: "6 flexible trusted"), and
every one is bound to the checker package by `install`; a test pins that the
load path binds each contracted function exactly once. Heap custody does not
name the releaser, as for every heap contract (`cfl_sds_create`/`destroy`
likewise); the `heap-kind` assumption says so.

## 3. The typecheck stubs (the separate question)

`typecheck-gcc.h` defines each `Wcurl_*_err_*` with `CURLWARNING`
(lines 271–274) as a `static void` function, `__attribute__((__warning__))`,
whose whole body is `__asm__("");`, and the `curl_easy_setopt` macro calls
them only inside `if(__builtin_constant_p(option))` (lines 44–46) under
type tests the compiler folds; the check record carries both as checked
evidence and the 52 stub names the header defines (a superset of the
census's 44). They are not runtime calls of libcurl, so a libcurl contract
is the wrong owner: whether the gateway may read a call in a branch Clang
proves dead, or a body that is only an empty `asm`, or whether the routes
should be configured with `CURL_DISABLE_TYPECHECK`, is a gateway or
configuration decision. The plugin binds none, asserts nothing, and a test
pins that none is bound; on the callers run `rd_http_get` stays `Unknown`
with only stubs and other blockers left. Registered:
`W48-LIBCURL-TYPECHECK-STUBS`.

## 4. Measurement

**Prediction** (committed first): no move on the corpus or system route,
because neither the pinned nor the configured route runs a subcommand plugin
or reads a user load path (`W45-PLUGIN-ROUTE-COVERAGE`). A subcommand
plugin's claims reach only its own report's artifact rows, never an audit
report's unit or function rows or a `.qo` checkpoint; carrying them there is
`SUBCOMMAND-CLAIMS-INTO-AUDIT-ROWS`, registered by
W48-PLUGIN-VERDICT-PRESERVATION (not yet on this card's base) and named here
as the dependency of the subcommand route. Only the checker-binding route
(`user_plugins.rs`) reaches function rows, which is why `install` exists; on the `current`
route with the plugin's load path, `rd_http_global_init` Unknown → Pending
and nothing to `Safe`.

**Preparations (this card's own).** The host had no prepared inputs and no
libcurl development headers. Fluent Bit was cloned from upstream into
`private/`; `quirin-audit prepare` (the base binary) prepared the corpus at
`66910c10` (plus `prepare monkey` and `providers/package.py build`) and the
system route at `046accf6`, with CMake shown the Ubuntu
`libcurl4-openssl-dev 8.18.0-1ubuntu2.8` headers extracted privately
(`CMAKE_INCLUDE_PATH`, `CMAKE_LIBRARY_PATH`), so librdkafka is configured
`WITH_CURL` and `rdhttp.c` is in both databases, as on the user's host. This
host has no `libsasl2` or `libzstd` development files, so the configuration
differs from the published baselines there; every figure below compares
base and tip on the same inputs ([inputs](artifacts/inputs.json)).

**Callers run** ([script](scripts/callers.sh),
[comparison](artifacts/callers-comparison.json), tip host binary): the two
librdkafka units that call libcurl, `quirin-audit current` with the
profile's 69 default modules as a provider selection, without and with the
plugin's crate. Function tallies 6 Safe / 39 Unknown / 4 Unsafe →
6 Safe / **1 Pending** / 38 Unknown / 4 Unsafe; units Unsafe/Unsafe on both
sides; the sidecar's `plugin_checks`: 6 bindings, 6 Pending
([section](artifacts/plugin-checks-with.json)). Rows linking a bound
contract: `rd_http_global_init` Unknown → Pending; `rd_http_get`,
`rd_http_get_params_append`, `rd_kafka_oidc_jwt_bearer_build_request_body`
and three OIDC refresh callbacks stay Unknown on their other blockers
(the stubs, `curl_url_set`/`_get`, nested calls).

**Arbiter** (`quirin-audit measure --declared`, [script](scripts/arbiter.sh)):
directory sides, the base export `d7a497822` and the
tip export `238c66cf2` (the plugin crate as measured; later commits change
only the crate's tests, docs and records, no analyzer crate), each built in
place (`cargo build --release --locked -p qcore-audit`), host
`tip-src/target/release/quirin-audit` with its `quirin-audit-measure`, gate
slots on, `--jobs 3`. This host refuses user namespaces, so no `rev:` side
can run (as MEMBER-PATH-DIRECT-REFUSAL-TEXT found).

| Run | Route | Result | Totals (Safe / Unknown / Unsafe), base = tip | Rows |
|---|---|---|---|---|
| [arbiter-2](artifacts/native-comparison-arbiter-2.json) | system | **match** | 103 / 341 / 4 | 448: 0 moved, 444 environment-only, 4 content-changed |
| arbiter-2 | corpus | did not complete | | both sides exit 1 (below) |
| [arbiter-3](artifacts/native-comparison-arbiter-3.json) | corpus | **match** | 1,087 / 3,656 / 10 (units 13 / 160 / 8) | 4,753: 0 moved, 0 content-changed, 4,753 environment-only |

Every row is attributed: no status moved on either route, as predicted. The
four system content changes are `review_holds` of `flb_output_exit` and
`flb_storage_create` (both configurations): the held declaration's `module`
path spells the side's own checkout (`base-src/…` against `tip-src/…`),
because a directory side is not bound at one fixed path; the text is
otherwise identical. Environment-only rows differ in the digests that spell
the build commit and checkout.

Two harness repairs, identical on both sides, neither touching an analyzer
or an input: (1) the exports' `scripts/run_pinned_audit.py` hard-codes
`QCORE_CLANG=/usr/bin/clang`, absent on this host (arbiter-1, discarded:
both corpus sides exit 1 before any unit); both were edited to
`/usr/lib/llvm-21/bin/clang`, the Clang every other route uses. (2) `git
archive` omits `third_party/libevent`, so the pinned route's libevent pin
check read the enclosing repository's revision and refused (arbiter-2's
corpus); `third_party/libevent` was cloned at its pinned `d82464a2` into
both exports, as a `rev:` side does. Side binaries: arbiter-2 base
`8af3e8fa…`, tip `2ed9a656…`; arbiter-3 base `ad3810a5…`, tip `8726cb25…`
(rebuilt in place after the submodule clone). Outputs in the
worktree-private `measure/arbiter-{1,2,3}/`.

## 5. Composition and the preservation gaps

Every answer enters only through the firewall: `classify_claim` (C11.2) and
`verify_plugin_claim` (C11.1), then the join. Tests pin: an established
binding is `Pending` on both routes and never `Safe`; an `Unknown` keeps the
plugin's reason; a host `Unsafe` row stays `Unsafe`; a package whose files
are not the pin answers `Unknown` for every binding; a package whose
executable changed is a `failure` (`Unknown`).

Two places drop the plugin's own reason for an **established** binding,
recorded for `W48-PLUGIN-VERDICT-PRESERVATION` (its scope; not fixed here):

1. Subcommand route: a `Safe` claim's row carries only the firewall's
   `QP016` text; the plugin's reason survives in the report's raw `claims`
   list, not in the row ([report](artifacts/check.report.json)).
2. Checker-binding route: `verify_plugin_claim` replaces the checker's
   diagnostics with the `QP008` message, so neither the `plugin_checks` row
   nor a caller's `plugin_links` carries which obligations established the
   contract. Demonstrated by the ignored test
   `the_checkers_reason_for_an_established_binding_reaches_the_published_row`,
   which fails today (run below).

The verdict value is never lost on either route; only the reason text of an
uncertified `Safe` is.

## 6. Tests and Lean

`crates/quirin-audit-plugin-libcurl/tests/plugin.rs` (15 run by default, 3
ignored needing the release) and 4 unit tests of the reader: ranking against
the census; no stub bound; every obligation reads a pinned file; the module
declares exactly the contracted functions with their clauses; the load path
binds each once; every obligation shape holds and fails as written (C++
linkage blocks, directives in bodies, duplicate definitions); an unverified
tree establishes nothing; the pin refuses a tree file by file; the
subcommand refuses a wrong tarball, a missing flag, an unknown step, and a
tree that is not the pin (`check`, `install`); host classification and join;
the checker package fail-closed (pin mismatch, tampered executable).
Release-gated (`QUIRIN_LIBCURL_SOURCE`, [runner](scripts/release_tests.sh),
`private/w48-libcurl/ignored-tests.log`, exit 101 by design): the release
establishes exactly the six and every witness holds, 2 passed; the checker
package publishes each established binding `Pending` (`QP008`), passed; the
preservation-gap test fails as recorded in §5.

`QCore.LibcurlContractPlugin` (18 theorems, imports `QCore.VerdictSemigroup`
read-only): the checker's decision (`established_iff`, pin mismatch,
uncontracted, a failed obligation); what is published on both routes (they
agree; never `Safe`, never `Unsafe`; established → `Pending`; unestablished,
failed run and pin mismatch → `Unknown`); composition (never below the host,
`Unsafe` stays, a bound caller is never `Safe`, an unbound caller keeps the
host value, a `Safe` host over established bindings is `Pending`, every
reason kept). Axioms: `propext` and `Quot.sound` only.

## 7. Gate

Scoped card gate ([script](scripts/gate.sh), [gate.json](gate.json)) at
`89546493f` through the host toolchain (Ubuntu clang 21.1.8, Lean 4.35.0),
log directory `private/w48-libcurl/gate-1/`. The diff reaches the root
`Cargo.toml` and `Cargo.lock`, so the whole Rust suite and every Lean audit
ran.

| Step | Exit | Count |
|---|---:|---|
| submodules | 0 | 1 |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 641 units, 685 summaries: **9,537 passed / 0 failed / 20 ignored** |
| provider lock (`cargo metadata --locked` on `providers/`) | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | **436** audits (`AuditLibcurlContractPlugin.lean` new) |
| Python suite | **1** | 779 tests, 1 error |

The Python error is a teardown race in
`test_gate_slot_queue.Stale.test_a_live_owner_without_a_lock_is_not_reclaimed`
(`OSError: Directory not empty` removing the test's scratch `gate`
directory while the busy host ran other cards' slot holders); this card
changes no gate-slot code. Verified without weakening anything: the module
alone passed 5 of 5 (`private/w48-libcurl/gate-1/slot-queue-{1..5}.log`) and
the whole Python suite rerun passed, 779 tests, exit 0
(`python-rerun.log`). The provider gate is not selected (`providers/` is
unchanged). Release-gated tests: §6.

## 8. Files likely to conflict

Root `Cargo.toml` (members) and `Cargo.lock`; `proof/QCore.lean`
(regenerate with `scripts/gen_lean_roots.py`);
`compiler/tests/workspace_layering.rs` and
`crates/quirin-audit-plugin/tests/dependencies.rs` (one entry each);
`scripts/gate_scope_walkers.json` (one entry);
`docs/reports/gate-scope-precise-selection/read-census.json` (re-recorded
here at `6a3a3ec7c`, 640 units, because the four new targets put the census
below the 90 % the census test needs; re-record again after a merge);
`docs/development/architecture.md` and
`docs/development/audit-subcommand-plugins.md` (one section each). The
sibling W48-OPENSSL-CONTRACT-PLUGIN touches the same places.

## 9. Follow-ups

Registered by [integration-amendments.json](integration-amendments.json):
`W48-LIBCURL-TYPECHECK-STUBS`, `W48-LIBCURL-UNCONTRACTED-CALLEES`,
`W48-LIBCURL-DISTRIBUTION-PATCHES`, `PLUGIN-CHECKER-TRANSPORT-SHARED`.
Amended: `W48-PLUGIN-VERDICT-PRESERVATION` (the two gaps of §5),
`W45-PLUGIN-ROUTE-COVERAGE` (the arbiter routes cannot see these bindings),
`W45-SUBCOMMAND-CERTIFICATE-ROUTE` (the first shipped plugin that claims),
`W47-EXTERNAL-RESIDUAL-RANKING-R2` (`curl_global_init`). Retired:
`W48-LIBCURL-CONTRACT-PLUGIN`, each unmet part named as a residual above.
