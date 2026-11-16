# W48-OPENSSL-CONTRACT-PLUGIN: OpenSSL's contracts checked by their own plugin

Card W48-OPENSSL-CONTRACT-PLUGIN (registered follow-up; user decision of
2026-09-28, amended by LIBRDKAFKA-HEADER-INLINES on 2026-11-05). Base
`d7a497822`. New crate `crates/quirin-audit-plugin-openssl` (binary
`quirin-audit-openssl`, `quirin-audit openssl prepare|check`), standing
constraint **52** (new, gateway topic), Lean `QCore.OpensslContractPlugin`
with `proof/AuditOpensslContractPlugin.lean`. The [prediction](PREDICTION.md)
was committed at `8d61d72b5`, before any implementation.

## Headline

* **Pin.** OpenSSL **3.5.5**, the upstream release tag `openssl-3.5.5`
  (commit `67b5686b4419b4cb8caa502711c41815f5279751`), tarball SHA-256
  `b28c91532a8b65a1f983b4c28b7488174e4a01008e29ce8e69bd789f28bc2a89` (§1).
* **Contracts.** The plugin carries 68 bindings, every OpenSSL function of
  the librdkafka census (99 pairs) and of the round-2 ranking (a subset), in
  twelve contract classes; four classes have a checker rule (18 bindings).
* **Check, on the pinned release** ([table](artifacts/bindings.md)):
  **16 bindings proposed `Safe` and published `Pending`**
  (`plugin-claim-uncertified`, QP016), `ERR_clear_error` first; **52
  `Unknown`**: 50 in a class without a rule, and `SSL_get_certificate` and
  `SSL_get_client_CA_list`, whose bodies call. No binding is published
  `Safe`; the plugin never proposes `Unsafe`.
* **Measured routes: no move**, as predicted: no route reads the plugin's
  verdicts (§6). Bringing them to the Fluent Bit callers is
  `W48-OPENSSL-CALLER-COMPOSITION`.
* **Preservation gap found** (§4): the host row of an uncertified `Safe`
  claim drops the plugin's own reason; recorded with an ignored test for
  W48-PLUGIN-VERDICT-PRESERVATION, not fixed here.

## 1. The pin and how it was determined

Fluent Bit vendors no OpenSSL: `CMakeLists.txt` calls `find_package(OpenSSL)`
(line 888 at `046accf6`). On this card's own `quirin-audit prepare` of each
live route's revision — system `046accf6`, corpus `66910c10`, slice
`90978ac6` — CMake found the same library, recorded in each build's cache as
`FIND_PACKAGE_MESSAGE_DETAILS_OpenSSL=[/usr/lib/x86_64-linux-gnu/libcrypto.so][/usr/include][ ][v3.5.5()]`.
`/usr/include/openssl` belongs to `libssl-dev 3.5.5-1ubuntu3.7` (with
`libssl3t64` of the same version), whose upstream is the 3.5.5 release. The
tarball was fetched from the GitHub release; its SHA-256 equals both the
`.sha256` published beside it and openssl.org's, and `git ls-remote` names
the tag's commit. [pin_evidence.py](scripts/pin_evidence.py) writes
[pin-evidence.json](artifacts/pin-evidence.json):

| Comparison | Result |
|---|---|
| CMake's OpenSSL, three prepared revisions | v3.5.5 at `/usr/include`, all three |
| distribution headers against the prepared release, 142 files | 116 identical; 23 differ only in the generated "from `../include/…`" comment; 3 differ in substance: `configuration.h` (the distribution's `Configure` options), `fips_names.h`, and one `ssl.h` define (`SSL_VALUE_QUIC_MAX_PENDING_CONNS`) |
| the 68 bindings' prototypes in the distribution's headers (Clang) | **68 of 68** equal the contract set's |

So the check reads the upstream release Fluent Bit's distribution build is
made from, not that build: the distribution's patches and configuration are
an assumption every claim names (`release`, `configuration`), and preparing
the distribution's own source is `W48-OPENSSL-DISTRIBUTION-BUILD`. No git
submodule was added; the pin is three digests checked fail-closed by the
plugin (`pin::OPENSSL`): the tarball, the extracted tree
(`d62a5bdb…`, 5,730 files) and the prepared tree (`8364e1c0…`, 5,772 files,
the six files that spell the build directory excluded). Two preparations in
different directories gave the same prepared digest.

## 2. The plugin

`quirin-audit openssl prepare --tarball FILE` refuses another tarball,
extracts it into the host's output, checks the tree digest, runs `perl
Configure linux-x86_64 no-asm` with `CC` the given Clang and `make
build_generated` (OpenSSL generates 31 of its headers), checks with `make -n
OBJECT` that each defining file compiles with the contract set's flag set
(`shell::compile_flags` splits the printed line and refuses anything the
shell would do more than split), and checks the prepared digest. It writes
[openssl-preparation.json](artifacts/openssl-preparation.json) and claims
nothing.

`quirin-audit openssl check` answers one claim per binding, artifact
`openssl@3.5.5::FUNCTION`, property `openssl-contract/CLASS`, profile
`quirin.openssl-contract-check.v1`. The contracts are
`crates/quirin-audit-plugin-openssl/contracts/openssl-3.5.5.json`
(`quirin-openssl-contracts/1`, total-or-refuse). A binding's obligations:

1. `release-pin`: the prepared tree has the pinned digest and version (an
   absent tree, another version or another digest fails every binding);
2. `declared`: a header probe (one translation unit including the bound
   headers, Clang's JSON AST) declares the contract's prototype;
3. for a class with a rule, `defined`: a dump of the defining file, compiled
   with the pinned flags, holds exactly one external definition in that file
   with that prototype, and no other declaration of another type;
4. `class-rule` (`checker.rs`, each a closed whitelist):
   * `no-pointer-interface` — no parameter, a `void` or builtin-integer
     result spelled so: `ERR_clear_error`, `ERR_peek_error`,
     `ERR_peek_last_error`;
   * `static-object-return` — the body is exactly `return &V;` for the
     contract's `V`, the one `static const` object of that name in the
     defining file: `EVP_sha256`, `EVP_sha512`;
   * `pure-reader` — no call, assignment, increment, loop, `goto`, static
     local or read of a global; names are parameters, body locals or
     enumerators: eight getters (`X509_STORE_CTX_get_error`, `…_depth`,
     `…_current_cert`, `X509_OBJECT_get0_X509`, `X509_STORE_get0_objects`,
     `X509_get_issuer_name`, `SSL_CTX_get_cert_store`); `SSL_get_certificate`
     and `SSL_get_client_CA_list` are refused (their bodies call);
   * `member-store` — the body is exactly the contract's stores
     `p->m op= e` and the escaping pointer parameters equal the contract's:
     `X509_STORE_CTX_set_error`, `SSL_CTX_set_options`,
     `SSL_CTX_set_default_passwd_cb` (the callback escapes into `ctx`),
     `SSL_CTX_set_default_passwd_cb_userdata` (the caller's `u` escapes).

`Safe` is proposed only when every obligation is discharged, with the check
record offered by reference; anything else is `Unknown` with the first
undischarged obligation as its reason; `Unsafe` is never proposed. Every
claim's `assumptions` name the release (upstream, not the distribution
build), the configuration, Clang's AST as the C semantics, the link (no
interposition), and per class library-owned state or the caller's argument
validity. [openssl-check.json](artifacts/openssl-check.json) keeps every
obligation's detail and the evidence (file, line, file SHA-256, object,
flags) per binding.

The class rules are syntactic and intra-procedural on purpose: a
refcounted releaser (`X509_free` drops one reference through
`ASN1_item_free`), a constructor or an I/O call cannot be checked without
inter-procedural reasoning this checker does not have, so those classes are
`Unknown` with their contract text recorded, never guessed.

## 3. Coverage of the ranked callees

[bindings.md](artifacts/bindings.md) lists all 68 in the contract set's
order (`ERR_clear_error` first, then by census pairs). Published: **16
`Pending`** (`ERR_clear_error`, `ERR_peek_error`, `ERR_peek_last_error`,
`EVP_sha256`, `EVP_sha512`, the seven readers and four stores above) and
**52 `Unknown`**. The residuals by class, highest pairs first:

| Class (no rule) | Bindings | Pairs |
|---|---:|---:|
| refcounted-release | `X509_free` 6, `BIO_free` 5, `EVP_PKEY_free` 4, `ENGINE_free` 3, `SSL_CTX_free` 2, `SSL_free`, `X509_STORE_free`, `OSSL_PROVIDER_unload` | 23 |
| constructor | `BIO_new_mem_buf` 4, `BIO_new_file` 2, `PEM_read_bio_PrivateKey` 2, `d2i_PKCS12_bio` 2, `X509_new`, `SSL_CTX_new`, `EVP_MD_CTX_new`, `OPENSSL_sk_new_null`, `ENGINE_by_id`, `OSSL_PROVIDER_load`, `SSL_get1_peer_certificate`, `PEM_read_bio_X509` | 18 |
| configuration | `SSL_CTX_ctrl` 2, `SSL_CTX_set_default_verify_paths` 2, five more | 11 |
| release | `OPENSSL_sk_pop_free` 2, `PKCS12_free` 2, `EVP_MD_CTX_free`, `OPENSSL_sk_free`, `CRYPTO_free` | 7 |
| protocol-io | `SSL_write` 2, `SSL_read`, `SSL_connect`, `SSL_do_handshake`, `SSL_shutdown`, `SSL_get_error` | 7 |
| ownership-transfer | `SSL_CTX_use_PrivateKey`, `SSL_CTX_use_certificate`, `SSL_CTX_use_cert_and_key`, `ENGINE_load_ssl_client_cert` | 4 |
| container | `OPENSSL_sk_push` 2, `OPENSSL_sk_value` 2 | 4 |
| buffer-writer | `ERR_error_string_n`, `EVP_EncodeBlock`, `X509_NAME_oneline`, `ERR_peek_last_error_all` | 4 |
| pure-reader refused (calls) | `SSL_get_certificate`, `SSL_get_client_CA_list` | 2 |

`W48-OPENSSL-UNCHECKED-CLASSES` tracks them.

## 4. Composition: firewall, join and what is kept

The plugin's claims enter the host only through `quirin-audit`'s subcommand
route (rule 10h, C11.2): `classify_claim` and the reasoned join
(`ReasonedVerdict`, C11.1). Measured through the shipped host
(`qcore_audit::subcommand_plugins::main`, tests below):

| Plugin answer | Published row |
|---|---|
| `Safe`, check record by reference | `Pending` (`plugin-claim-uncertified`, QP016) |
| `Unknown` with its reason | `Unknown` (`incomplete-obligation-coverage`), the plugin's reason in the row's detail |
| pin mismatch / absent tree | 68 rows `Unknown`, diagnostic `openssl-pin-mismatch` |
| tarball not the pinned one, usage error, 1 ms timeout | run failed, report `Unknown`, no claim joined |
| a host `Unsafe` row for the artifact | stays `Unsafe` |

**Gap.** For an uncertified `Safe` (or `Unsafe`) claim the row's reason
multiset holds only the firewall's reason (`proposed Safe without a
certificate…`); the plugin's own reason (here, the list of discharged
obligations) reaches the report only in its `claims` record, so a projection
of the row cannot recover it. An `Unknown` claim's reason is kept. The test
`the_row_of_an_uncertified_safe_claim_keeps_the_plugins_reason`
(`tests/obligations.rs`) asserts the preserving behaviour and fails on the
base host; it is `#[ignore]`d naming **W48-PLUGIN-VERDICT-PRESERVATION** as
its discharger. The host composition path was not edited.

## 5. Lean

`proof/QCore/OpensslContractPlugin.lean`, audited by
`proof/AuditOpensslContractPlugin.lean` (17 theorems, axioms `propext` and
`Quot.sound` only). It imports `QCore.SubcommandPlugins`, whose `row` and
`firewall` it reuses, and proves: the proposal is `Safe` exactly when every
obligation is discharged (`a_binding_is_proposed_safe_iff_every_obligation_is_discharged`),
never `Unsafe`, `Unknown` on a pin mismatch, an unruled class or any
undischarged obligation; the published row is never `Safe`
(`a_binding_row_is_never_safe`), `Pending` exactly when the run completed and
every obligation is discharged, `Unknown` otherwise, and never below the
host's; every node the pure-reader whitelist admits has no effect (no call,
write, divergence or outside read: `an_admitted_node_has_no_effect`); the
member-store escapes are exactly the stored pointer parameters, so an
agreeing contract omits none (`a_stored_pointer_is_a_stated_escape`); a
no-pointer interface reaches no caller object. Clang's AST read into the
modelled nodes, and Clang's AST as C semantics, are premises pinned by the
tests, not proved.

## 6. Measurement

Prediction ([PREDICTION.md](PREDICTION.md)): no move on any route. The host
cannot run `rev:` sides (`unshare: write failed /proc/self/uid_map:
Operation not permitted`), so, as LIST-ENTRY-RELEASE-IDENTITY did, both sides
are binaries built in `git archive` exports (`private/base-src` at
`d7a497822`, `private/tip-src` at `aa8db89bd`, the libevent submodule cloned
at its pin) and measured on this card's preparations
([inputs](artifacts/measure-inputs.json); figures are on these
preparations, not the published baselines):

MEASURE-RESULT

## 7. Tests

| File | Tests | What |
|---|---:|---|
| `src/` unit tests | 12 | tree digests (order-free, every byte and name, a symbolic link refused, byproducts excluded); the release version; the compile-line splitter and what it refuses; the contract set (68 bindings, 99 pairs, `ERR_clear_error` first, 18 checked; another release, version, a checked class without a rule, a repeat, an unknown field refused); the AST stream and type helpers |
| `tests/classes.rs` | 4 | every class rule over Clang's AST of small C definitions: admitted shapes, and a call, assignment, increment, loop, static local, global read, other store, missing escape, hidden pointer typedef, non-const or foreign object refused |
| `tests/obligations.rs` | 3 + 1 ignored | the obligations over a release-shaped tree pinned by its own digest (each obligation's failure; a one-byte change, an absent tree and another version fail the pin for every binding); the host's `classify_claim`/`join` on the claims (no row `Safe`, `Pending` for a discharged binding, a host `Unsafe` kept); the ignored preservation test of §4 |
| `tests/host.rs` | 6 + 1 ignored | through `qcore_audit::subcommand_plugins::main` (§4's table); ignored without `QUIRIN_OPENSSL_TARBALL`: prepare and check of the real release (16 `Pending`, 52 `Unknown`), run with the tarball: 7 passed ([log](artifacts/test-host-with-tarball.log)) |
| `crates/quirin-audit-plugin/tests/dependencies.rs`, `compiler/tests/workspace_layering.rs` | (amended) | the plugin's normal closure is the SDK and `qcore-schemas`; `qcore-audit` is a dev-dependency only |

`scripts/run_test_binaries.py` runs `quirin-audit-plugin-openssl::test::host`
in its `EXCLUSIVE` phase (process plugins under declared quotas), and
`scripts/gate_scope_walkers.json` declares the layering test's new literal.

## 8. Gate

GATE-RESULT

## 9. Follow-ups

Registered by [integration-amendments.json](integration-amendments.json):

* `W48-OPENSSL-CALLER-COMPOSITION` — no analysis route reads the plugin's
  binding verdicts, so `rd_kafka_transport_ssl_clear_error` (blocked by
  `ERR_clear_error` alone) and the census callers do not move.
* `W48-OPENSSL-UNCHECKED-CLASSES` — the 52 `Unknown` bindings of §3.
* `W48-OPENSSL-DISTRIBUTION-BUILD` — the check reads the upstream release,
  not the distribution's patched build Fluent Bit links.
* `W48-PLUGIN-SDK-SHARED-HELPERS` — process, digest and lookup helpers are
  repeated across the libevent, measure and OpenSSL plugins (and libcurl's in
  parallel); not factored into the SDK here, by the brief.

Amended: `W48-PLUGIN-VERDICT-PRESERVATION` (the §4 gap and its test) and
`W45-SUBCOMMAND-CERTIFICATE-ROUTE` (a shipped plugin now makes 16 uncertified
`Safe` claims with check-record references). Retired:
`W48-OPENSSL-CONTRACT-PLUGIN`.

Files likely to conflict with parallel cards: root `Cargo.toml` and
`Cargo.lock`, `proof/QCore.lean` and `proof/AuditAll.lean` (rerun
`gen_lean_roots.py`), `docs/agents/gateway.md` and `docs/agents/README.md`
(rerun `gen_rule_docs.py`), `docs/reports/INDEX.md` (rerun
`build_docs_index.py`), `scripts/run_test_binaries.py` (`EXCLUSIVE`),
`scripts/gate_scope_walkers.json`, `crates/quirin-audit-plugin/tests/dependencies.rs`,
`compiler/tests/workspace_layering.rs`, `docs/development/architecture.md`
and `docs/development/audit-subcommand-plugins.md`.
