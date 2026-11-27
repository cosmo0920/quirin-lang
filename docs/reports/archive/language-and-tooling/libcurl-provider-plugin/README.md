# LIBCURL-PROVIDER-PLUGIN: libcurl contracts on the user-contract SDK, with version-ranged declarations

Base `32336c33d` (origin/main). User decision 2026-11-17 (relayed in the card
brief): libcurl's contracts move onto the user-contract provider SDK exactly
as OPENSSL-PROVIDER-PLUGIN did for OpenSSL. The template is that card's
[report](../../contracts-and-premises/openssl-provider-plugin/README.md) and crate; this report states
only what differs. The [prediction](PREDICTION.md) was committed first
(`6c8f615ea`). No analyzer crate, rule, specification section or Lean module
changes: rule 53 and `QCore.LibraryVersionRange` already cover the clause and
its hold.

## 1. The crate

`providers/quirin-provider-libcurl` (namespace `libcurl`), laid out as the
OpenSSL crate: `contracts/libcurl/{escape,global,slist,url}.qv`,
`contracts.lock`, `library-versions.json` (`curl/curlver.h`,
`LIBCURL_VERSION_MAJOR`/`MINOR`/`PATCH`), `quirin-plugin.json` (one binding
per function, all on `checkers/libcurl-check`), the checker
`quirin-libcurl-check` (`src/bin/libcurl_check.rs`, logic in `src/check.rs`),
the pinned prototypes `pinned/libcurl-8.18.0.json`
(`scripts/pin_prototypes.py`) and the documentation record
`doc-snapshots.json` (`scripts/fetch_docs.py`).

The contract set is the six W48-LIBCURL-CONTRACT-PLUGIN established against
the pinned 8.18.0 source, clause for clause, each with a range:

| Function | Module | Range | AVAILABILITY (page sha256 as fetched 2026-11-17) | Custody shape checked |
|---|---|---|---|---|
| `curl_global_init` | `global.qv` | `since "7.8"` | "Added in curl 7.8" (`cb7e688d…`) | none (`flags` borrowed) |
| `curl_slist_append` | `slist.qv` | `since "7.1"` | "Added in curl 7.1" (`ca17ea51…`) | reallocation: `owned(list, nullable)`, `return_reallocated(list, nullable)`, `data` borrowed |
| `curl_slist_free_all` | `slist.qv` | `since "7.1"` | "Added in curl 7.1" (`f47cb619…`) | `released(list, nullable)` |
| `curl_easy_escape` | `escape.qv` | `since "7.15.4"` | "Added in curl 7.15.4" (`38e61ac8…`) | `return_owned(nullable)`, all three borrowed |
| `curl_url` | `url.qv` | `since "7.62.0"` | "Added in curl 7.62.0" (`4feac3eb…`) | `return_owned(nullable)` |
| `curl_url_cleanup` | `url.qv` | `since "7.62.0"` | "Added in curl 7.62.0" (`93ef5111…`) | `released(handle, nullable)` |

Every page names its version, so no `documented` bound is needed. libcurl
publishes one manual (`https://curl.se/libcurl/c/<function>.html`), so a page
has no series; the record keeps each page's SHA-256, its AVAILABILITY,
SYNOPSIS and RETURN VALUE text, and its sentences on NULL and on allocation,
copying and release. The record also shows what the contracts rest on: for
`curl_slist_append` "Pass in NULL in the list argument to create a new list",
"curl_slist_append copies the string" and "A null pointer is returned if
anything went wrong, otherwise the new list pointer is returned"; for both
releases "Passing in a NULL pointer … return immediately with no action".

**Not added.** The other libcurl callees of the audited units are
`curl_easy_init`, `curl_easy_setopt` (26 sites), `curl_easy_getinfo`,
`curl_easy_perform`, `curl_url_get`, `curl_url_set` and (through
`RD_IF_FREE`) `curl_easy_cleanup`. All but `curl_easy_init` are
W48-LIBCURL-UNCONTRACTED-CALLEES, out of scope. `curl_easy_init` ("Added in
curl 7.1", NULL on failure) would be a `return_owned(nullable)`, but
resource-v1's heap custody does not name the releaser, so a `free` of the
handle would read as its release while the only documented release,
`curl_easy_cleanup`, stays uncontracted. That is not an honest statement in
the existing vocabulary, so `curl_easy_init` stays uncontracted. Its one caller
(`rd_http_req_init`) is also blocked by `curl_easy_setopt`.

## 2. The pinned identity

The routes' libcurl headers are the distribution's, Ubuntu 26.04
`libcurl4-openssl-dev 8.18.0-1ubuntu2.8` (W48 §1; on the user's host,
`/usr/include/x86_64-linux-gnu/curl`). This host has only the runtime
packages, so the development package was downloaded with `apt-get download`
(`.deb` sha256 `eb12af81…f55fe`) and extracted under `private/`. Its
`curlver.h` reports 8.18.0 (`LIBCURL_VERSION_NUM 0x081200`, sha256
`ace17dbb…`). `pin_prototypes.py` preprocessed `<curl/curl.h>` with
`-isystem` on that directory. It recorded every libcurl header's digest, the
six declarations as the preprocessor leaves them, and `CURLcode`'s integer
type, which it establishes by compiling `_Static_assert`s on the type's size
and on `(CURLcode)-1 > 0`: `unsigned int`, hence `u32` (rule 12u).

`quirin-audit pin --crate quirin-provider-libcurl`, with the probe database
cut to the audited units (§4), recorded `curl` 8.18.0 in the lock. The
lock's `header` is provenance only (the identity is the version): it names
the private extraction path on this host, and a re-pin on a host with the
package installed would record `/usr/include/x86_64-linux-gnu/curl/curlver.h`
with the same digest.

## 3. The checker

`libcurl.contract_check` (target `libcurl`, revision `0.1.0`) answers one
request per binding, from what is compiled into it only, as the OpenSSL
checker does, with three differences. (1) Types: `long` is `i64` (LP64), and
an enumeration is the integer type the pinned record establishes for it.
(2) The range must be `since` the version the fetched page's AVAILABILITY
names. A range copied into the checker's own table is cross-checked against
the snapshot, so editing either side alone fails. (3) Custody is
checked as an exact set: every `ensures` other than a borrow and
`return_none` must be the shape's clauses, every parameter outside the shape
is borrowed, the custody parameter is not, `return_none` appears exactly when
the result carries no custody, and the only `requires` are the contract
version, `flexible` and the range. The page must also carry the sentences the
shape rests on. Every check passes: a `Safe` claim without evidence, which
the firewall publishes `Pending` (`plugin-claim-uncertified`). Any failure is
`Unknown`. The verdict mapping is the OpenSSL report's §5 table, unchanged.

## 4. Registration and the user path

`providers/contracts.py` `USER_LOAD_PATH_PACKAGES` gains the crate. `package.py
checkers` and the CI step build and install it unchanged (the CI comment now
names it). `providers/user-crates-load-path.json` is regenerated. The
conformance test's pin of user crates moves from 1 to 2 (a deliberate pin
update: the same dependency rule for the second crate). The shipped
`contract-load-path.json`, its views and every shipped lock are unchanged.

[`evidence/user-path.sh`](evidence/user-path.sh), run with the real binaries
(`private/user-path-4/`), every step exit 0: `quirin-plugin check` (4
modules, 6 checkers, 0 overrides), `package` and `verify`
(`libcurl.contract_check`), `load-path` over both user crates,
`quirin-audit plugin views --write`, `pin --crate quirin-provider-libcurl`
(`0 pinned, 6 flexible`; `= curl: version 8.18.0 pinned`) and
`pin --verify-contracts` (both user crates' locks list every declaration).
`package.py checkers` (both packages installed, views and locks verified) and
`contracts.py check` exit 0.

**Finding (the generator's order).** `quirin-plugin load-path` writes the
crates sorted but the checker bindings in argument order, and `contracts.py
check` expects them in sorted crate order. The first regeneration
(`openssl`, `libcurl` as arguments) was refused as stale. Naming the crates
sorted fixes it, and `providers/README.md` now says so. Registered:
`USER-LOAD-PATH-CHECKER-ORDER`.

## 5. Tests

- `providers/quirin-provider-libcurl` unit tests (4): every contract passes
  its own checks; every declaration is documented and bound, and exactly the
  six; drifted contracts are refused (a non-nullable release, a release read
  as a borrow, a non-null owned result, a range below or other than the
  documented `since`, `documented` in place of `since`, a range excluding
  8.18.0, another library's range, no range, a wrong signature, a signed
  `CURLcode`, a plain return read as owned, the append read as a fresh
  allocation, the copied string read as retained).
- `tests/user_crate.rs` (3): the crate reads as a user crate and lists in a
  user load path; the packaged checker answers each binding `Safe` (a claim,
  no evidence) and a range below `since`, a range excluding the pinned version
  or an undeclared function `Unknown`; the crate is on the user load path only
  (absent from `contract-load-path.json`, every declaration bound once to the
  checker package, lock and prototypes at 8.18.0).
- `crates/qcore-audit/tests/libcurl_user_crate_versions.rs` (2), end to end
  with `quirin-audit pin` and `contracts` over the crate's committed modules
  and `library-versions.json`, under a `curlver.h` the test writes. At 8.18.0
  pinned, every applied declaration is in force. At 8.19.0, both callers are
  `Pending` (`library-version-changed`). At 7.61.0, `url_cycle` is `Unknown`
  (`library-version-out-of-range`, below 7.62.0) and `init_once` `Pending`
  (changed, still in range). At 7.7.0, `init_once` is `Unknown` (below 7.8).
  With no header, both are `Unknown` (`library-version-unreadable`). Restored,
  the declarations are in force again. The scratch load path binds no checker
  so that the range decision is isolated, and the report says why an in-force
  row there is not a published claim. A second test pins the crate's absence
  from the shipped load path.

## 6. The opt-in `current` run

**Inputs (this card's own; the host had none).** Fluent Bit was cloned from
upstream into `private/fb/`. `quirin-audit prepare --revision 046accf6` (the
base binary, `target/release/quirin-audit` sha256 `bb48af09…`, built from
this worktree; no analyzer source differs from the base at any commit of
this card) prepared the system route. CMake saw the extracted
`libcurl4-openssl-dev` headers through `CMAKE_INCLUDE_PATH` and
`CMAKE_LIBRARY_PATH`, as W48 did (`CURL_INCLUDE_DIR` is the extracted
directory; `find_package(CURL)` reports `v8.18.0`), so librdkafka is
configured with curl. The production database is 1,414 commands (sha256
`a1f9e82b…`), and 815 of them pass the libcurl include directory
([`probe_database.py`](evidence/probe_database.py)).

**Run** ([`callers.sh`](evidence/callers.sh), logs `private/callers-1/`, every
step exit 0; [comparison](evidence/callers-comparison.json) by
[`compare_callers.py`](evidence/compare_callers.py), exit 0). The selection is
W48's two librdkafka units, the provider selection is the profile's 69
default modules, `--jobs 2`, `SOURCE_DATE_EPOCH=1767225600`. `without` runs
on the shipped load path. Each `with-*` side runs on
`providers/user-crates-load-path.json` with `quirin-provider-libcurl` added,
under one probe configuration:

| Side | Probe configuration | libcurl declarations | Functions (Safe / Pending / Unknown / Unsafe) | Units |
|---|---|---|---|---|
| without | — | — | 6 / 0 / 39 / 4 | Unsafe, Unsafe |
| with-selected | the two selected units' commands (`selected.json`, sha256 `5e128caa…`) | all 6 in force (8.18.0 = lock) | 6 / **1** / 38 / 4 | Unsafe, Unsafe |
| with-default | Clang's default search path | all 6 held, `library-version-unreadable` | 6 / 1 / 38 / 4 | Unsafe, Unsafe |
| with-full | the whole prepared database | all 6 held, `library-version-unreadable` | 6 / 1 / 38 / 4 | Unsafe, Unsafe |

Every side checks 18 bindings (OpenSSL's 12 and libcurl's 6), all `Pending`
(`plugin-claim-uncertified`). No row is unmatched, none reaches `Safe`, none
moves into or out of `Unsafe`.

**with-selected, every row attributed.** Seven rows link a bound libcurl
contract (each link `Pending`), exactly W48's seven:

| Row | Status | Diagnostics removed (none added) | Still blocked by |
|---|---|---|---|
| `rd_http_global_init` | **Unknown → Pending** | uncontracted `curl_global_init` | nothing: the binding's `Pending` |
| `rd_http_get` | Unknown | uncontracted `curl_slist_append`, `curl_slist_free_all` | the typecheck stubs, `curl_easy_*` (W48-LIBCURL-TYPECHECK-STUBS, W48-LIBCURL-UNCONTRACTED-CALLEES) |
| `rd_http_get_params_append` | Unknown | uncontracted `curl_url`, `curl_url_cleanup` | `curl_url_set`/`_get` |
| `rd_kafka_oidc_jwt_bearer_build_request_body` | Unknown | uncontracted `curl_easy_escape`; unresolved identity `scope_escaped` | other refusals |
| `rd_kafka_oidc_token_jwt_bearer_refresh_cb` | Unknown | uncontracted `curl_slist_append`, `curl_slist_free_all`; unresolved identity `headers` | other refusals |
| `rd_kafka_oidc_token_client_credentials_refresh_cb` | Unknown | uncontracted `curl_slist_free_all` | other refusals |
| `rd_kafka_oidc_token_metadata_azure_imds_refresh_cb` | Unknown | none | other refusals |

This is the prediction's first row, exactly.

**with-default and with-full: an unpredicted move, and a fail-closed defect
found.** Both probes are unreadable, as predicted: this host has no
`curl/curlver.h` on the default path, and 599 of the full database's commands
pass no libcurl include directory. Every declaration's record reads
`in_force: false`, `reason: library-version-unreadable`. Rule 53 says such a
call is held `Unknown` (`stale-evidence`). Yet `rd_http_global_init` is
published **`Pending`**. Its uncontracted-call diagnostic is replaced by "call
to `curl_global_init` whose pinned contract is not in force: the audited
source does not match its lock (contract-pin-mismatch)", which is the
human-resolvable hold's text, not the library-version hold's ("… not in force
at the audited library version (library-version-unreadable)", which
`legacy_pin_hold::hold_description_for` chooses when the classifier's
environment carries the withheld entry's reason). On the `contracts` route
the same hold is `Unknown`, as `libcurl_user_crate_versions.rs` asserts. So
on the `current` route an unreadable (and, by the same path, out-of-range or
ambiguous) version publishes a held call `Pending`, a value below its
standing. No row reaches `Safe`, but an `Unknown` is published as a `Pending`
that no review can resolve. The cause is not localised. The run-level
evaluation records the right reason, but the environment the classifier
consults when it describes the held call evidently lacks it. The card changes
no gateway code, so the defect is registered as
`LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING` (§9), with this run as its
reproduction. Until it is fixed, an opt-in `current` run reads correctly only
where the probe reads the headers (with-selected), and a `Pending` row there
must be read against the sidecar's `library_versions` record.

**The probe's configurations (a finding for LIBRARY-VERSION-PROBE-ROUTE-DATABASE).**
With the whole database the probe is unreadable, because only the targets
linking libcurl get its include directory. A route database handed to the
probe as it is today (that follow-up's discharge) would therefore hold every
libcurl call on any host whose libcurl headers are off the default path. The
probe has to read the configurations of the units that call the library (or
the units audited), not every command. Registered with that follow-up's
evidence (§9). On the user's host the package installs the headers on the
default path (`/usr/include/x86_64-linux-gnu`), so every configuration
resolves them.

## 7. The measured routes

Prediction: corpus, system and slice verdict-identical (`none`). This card
changes no analyzer crate. Its only `crates/` change is a test
(`crates/qcore-audit/tests/libcurl_user_crate_versions.rs` and its fixture),
and the crate is reachable only through the user load path, which no pinned
or configured route reads (W45-PLUGIN-ROUTE-COVERAGE). The arbiter comparison
(`quirin-audit measure --declared PREDICTION.md`) is left to the integrator's
landing measurement, as for OPENSSL-PROVIDER-PLUGIN. This host has no prepared
corpus inputs, and its `rev:` sides need user namespaces it refuses (W48 §4).

## 8. Gate

`python3 scripts/gate_scope.py --base 32336c33d --out private/gate-1` through
`private/toolchain/env.sh`, on `b60e5e30c` (scoped: rust-suite packages
qcore-audit, qcore-compiler, qcorec, the three audit plugins and
quirin-source-analysis plus 43 test targets; Lean none; provider gate yes;
record `private/gate-1/gate.json`):

| Step | Exit | Counts |
|---|---:|---|
| submodules, fmt, Clippy (7 invocations) | 0 | |
| Rust suite | 0 | 270 units, 277 summaries; 3,413 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Python suite | 1 | 779 tests, 2 failures (below) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 147 passed, 0 failed, 1 ignored |
| provider fmt, provider Clippy | 0, 0 | |

The Python failures: (1) `test_gate_scope_census` named the new test's two
directory literals (`providers/quirin-provider-libcurl`, `providers`) as
undeclared walkers. The test now spells each file it reads by its whole path
(commit after `b60e5e30c`), and `gate_scope.py --plan` reports no undeclared
literal. (2) `test_gate_slot_queue.Status.test_each_waiter_shows_its_queue_position`
saw a holder where it expected a waiter, a load-sensitive slot test on a host
at load 60 to 70 that this card's code does not touch. Reruns after the fix:
`python3 -m unittest discover -s scripts/tests -p "test_gate_s*.py"` exit 0,
255 tests (`private/logs/py-rerun.log`); the edited test
`cargo test -p qcore-audit --test libcurl_user_crate_versions` exit 0, 2
passed (`private/logs/libcurl-versions-test-2.log`); `cargo fmt` and
`cargo clippy -p qcore-audit --tests` exit 0. Also run outside the gate:
`python3 providers/package.py checkers` exit 0, `providers/contracts.py check`
exit 0, and the providers' Python tests exit 0 (26 tests).

## 9. Follow-ups

Registered or amended in [`integration-amendments.json`](integration-amendments.json):

- `LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING` (new): §6, the fail-closed
  defect: on `current` a call held for an unreadable, out-of-range or
  ambiguous library version is published `Pending` with the
  contract-pin-mismatch text, where rule 53 requires `Unknown`.
- `LIBRARY-VERSION-PROBE-UNIT-CONFIGURATIONS` (new): the probe reads every
  configuration, and a library only some targets include is unreadable under
  a whole route database.
- `USER-LOAD-PATH-CHECKER-ORDER` (new): §4.
- `LIBCURL-CONTRACTS-TWO-HOMES` (new): the same six declarations now live in
  this crate and in the `quirin-audit libcurl` plugin's `install` output (a
  crate also named `quirin-provider-libcurl`, namespace `libcurl`, without
  ranges). The plugin stays, per the brief. Which one owns libcurl's contracts,
  and whether `install` should emit this crate, is open.
- `W48-LIBCURL-UNCONTRACTED-CALLEES` (amended): unchanged scope; the six
  established contracts now ride this crate too, and `curl_easy_init` is not
  contracted (§1).
- `W45-PLUGIN-ROUTE-COVERAGE` (amended): this crate's bindings, too, reach
  only `current` and `contracts`.

## 10. Files likely to conflict

`providers/contracts.py` (`USER_LOAD_PATH_PACKAGES`), `providers/Cargo.toml`
and `Cargo.lock`, `providers/user-crates-load-path.json` (regenerate with both
crates named sorted, then `plugin views --write`), `providers/README.md`,
`providers/conformance/tests/packages.rs` (the user-crate pin) and the CI
comment. OPENSSL-PROVIDER-REMAINING-CALLEES and REFERENCE-DROP-NULL-ACCEPTING
touch the same roots.