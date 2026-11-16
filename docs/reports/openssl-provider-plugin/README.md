# OPENSSL-PROVIDER-PLUGIN: OpenSSL contracts on the user-contract SDK (in progress)

Base `0f6e17118` (integrate/r12). This card discharges, or narrows,
`W48-OPENSSL-CONTRACT-PLUGIN`. Status: **census committed; stopped for two
user-owned placement choices** (section 3) before any contract is written or
bound. No analyzer, contract, lock or Lean file has changed.

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
  OpenSSL to determine since each of the functions." (Source: the per-series
  manual pages at `https://docs.openssl.org/<series>/man3/<page>/`, their
  HISTORY sections and the migration guide; reachable from this host on
  2026-11-16, HTTP 200 for `3.5/man3/X509_new/`.)

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
3, `SSL_free` 3, then 63 callees with one or two pairs (the full table is in
`census.md`). One `current` row is `Unsafe` (Monkey, co-blocked); every other
blocked row is `Unknown`.

Sources: Fluent Bit's own OpenSSL callers (`src/tls/openssl.c` 73 functions /
133 sites, `flb_crypto.c` 35/69, `flb_hmac.c` 21/27, `flb_hash.c`,
`flb_oauth2.c`, `flb_fips.c`, `flb_utils.c`, `out_azure_kusto`) are analysed by
**no** route; Monkey's `lib/monkey/tls/tls_openssl.c` (37/54) is analysed by
`current` only (under `--monkey-linked`), and `lib/monkey/plugins/tls/tls_openssl.c`
by none.

**Pinned identity (a version, per the 2026-11-16 decision).**
`#include <openssl/opensslv.h>` preprocessed by `/usr/lib/llvm-21/bin/clang`
with every `-I` the two prepared compile databases pass: OpenSSL **3.5.5**,
`OPENSSL_VERSION_NUMBER` = `0x30500050` (MAJOR 3, MINOR 5, PATCH 5, no
pre-release), from `/usr/include/openssl/opensslv.h` (sha256 `7acbe889…`;
chain `macros.h`, `x86_64-linux-gnu/openssl/opensslconf.h`,
`configuration.h`, digests in `census.json`). One header set: the CMake cache
names `OPENSSL_INCLUDE_DIR=/usr/include` (OpenSSL found as v3.5.5, librdkafka
linked against `OpenSSL::SSL`), neither database passes an OpenSSL include
directory, and the checkout has no bundled `opensslv.h`. Package
`libssl-dev 3.5.5-1ubuntu3.7 amd64`, installed 2026-10-02, before the
2026-11-09 prepare.

## 3. Open choices (stop)

Two placements are user-facing choices fail-closed does not settle; the card
stops before writing contracts until they are decided.

1. **Where the supported version range lives**: (A) a generic declaration
   clause in the `.qv` text, with the pinned version identity in the
   generated lock (recommended); (B) the lock alone; (C) the
   `quirin-plugin.json` binding. The checker task carries no identity of the
   audited build's headers, so a run-time mismatch is checkable only by the
   core (A, B), not by the checker (C).
2. **Where the crate is loaded from**: a providers-workspace member loaded
   through a generated user load path (recommended: the shipped
   `contract-load-path.json` carries no checker bindings, so a crate there
   would apply OpenSSL contracts without the plugin firewall), or shipped with
   checker support added to the shipped load path (a core change that moves the
   user's default `current` run).
