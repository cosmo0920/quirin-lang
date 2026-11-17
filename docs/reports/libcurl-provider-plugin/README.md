# LIBCURL-PROVIDER-PLUGIN: libcurl contracts on the user-contract SDK, with version-ranged declarations

Base `32336c33d` (origin/main). User decision 2026-11-17 (relayed in the card
brief): libcurl's contracts move onto the user-contract provider SDK exactly
as OPENSSL-PROVIDER-PLUGIN did for OpenSSL. The template is that card's
[report](../openssl-provider-plugin/README.md) and crate; this report states
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

RESULTS
