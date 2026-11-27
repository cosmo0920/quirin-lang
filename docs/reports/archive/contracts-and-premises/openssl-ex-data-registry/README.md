# OPENSSL-EX-DATA-REGISTRY: the ex_data premise on every constructor and free

Base `062974679` (main, r33). Named follow-up `OPENSSL-EX-DATA-REGISTRY`
(recorded by OPENSSL-UNSTATED-CALLBACK-SLOTS); user decision 2026-11-19.
[PREDICTION](PREDICTION.md) was committed before any implementation
(`2bf03f5d5`). The declarations, checker, lock, Lean and specification are
`f3db96675`.

The follow-up allowed two discharges: a registry reading, or the premise on
every such declaration. This card lands the second. The first is registered
narrower as OPENSSL-EX-DATA-CLASS-REGISTRY (§5).

## 1. What ran silently

`CRYPTO_get_ex_new_index` (3.5, fetched 2026-11-19 at the page the crate's
`doc-snapshots.json` records) says:

* "When a structure is initially allocated (such as RSA_new()) then the
  new_func() is called for every defined index".
* "When a structure is free'd (such as SSL_CTX_free()) then the free_func()
  is called for every defined index".
* `dup_func()` runs when an `SSL`, `SSL_SESSION`, `EC_KEY` or BIO chain is
  copied.

The page lists the classes: BIO, DH, DSA, EC_KEY, ENGINE, EVP_PKEY, RSA, SSL,
SSL_CTX, SSL_SESSION, UI, UI_METHOD, X509, X509_STORE and X509_STORE_CTX.

Before this card, eight declarations named these callbacks. One did so
through `openssl_ex_data_callbacks`. Seven did so through the statement of
`openssl_unstated_callback_slots` or `openssl_bio_free_callbacks`. Every
other constructor and free of such an object ran them silently.

## 2. The audit

Every declaration of the crate was read against the 3.5 pages. Where a page
is silent, the 3.5 sources were read (`ssl/ssl_lib.c`,
`ssl/record/rec_layer_s3.c`, `ssl/ssl_cert.c`). A declaration qualifies when
it allocates, copies or finally frees an object of an ex_data class. That
includes a replaced or temporary object it frees inside the call.

| Declaration | Classes | Why |
|---|---|---|
| `SSL_CTX_new` | SSL_CTX, X509_STORE | allocates the context and its store |
| `SSL_new` | SSL, BIO | allocates the connection and its record layers' memory BIOs |
| `X509_new` | X509 | allocates a certificate |
| `BIO_new_file` | BIO | allocates a BIO |
| `ENGINE_by_id` | ENGINE | allocates a dynamic engine, freed when it fails to load |
| `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` | X509, EVP_PKEY | allocate the result, freed on a failed read |
| `d2i_PKCS12_bio` | X509 | a signed PKCS#7 safe's certificates are decoded |
| `SSL_free` | SSL, SSL_SESSION, X509, BIO | last drop |
| `SSL_CTX_free` | SSL_CTX, X509_STORE, X509, EVP_PKEY, SSL_SESSION | last drop |
| `BIO_free`, `X509_free`, `EVP_PKEY_free`, `ENGINE_free` | their class | last reference |
| `X509_STORE_free` | X509_STORE, X509 | last reference, and the certificates it holds |
| `PKCS12_free` | X509 | the certificates a signed safe holds |
| `EVP_MD_CTX_free` | EVP_PKEY | a signing context's `EVP_PKEY_CTX` may hold the key's last reference |
| `SSL_write`, `SSL_read`, `SSL_connect`, `SSL_do_handshake` | SSL_SESSION, X509, EVP_PKEY | the handshake creates and frees sessions, peer certificates and keys |
| `SSL_shutdown` | SSL_SESSION | a NewSessionTicket read before the close_notify |
| `SSL_set_accept_state` | BIO | `RECORD_LAYER_reset` frees the old layers and creates `BIO_new(BIO_s_mem())` |
| `SSL_CTX_use_certificate` | X509, EVP_PKEY | frees the replaced certificate, and a key that does not match |
| `SSL_CTX_use_PrivateKey` | EVP_PKEY, X509 | frees the replaced key, and a certificate that does not match |
| `SSL_CTX_use_PrivateKey_file` | EVP_PKEY, X509 | a temporary key, and the replacement |
| `SSL_CTX_use_certificate_chain_file` | X509 | already named it (r30) |
| `SSL_CTX_load_verify_locations`, `SSL_CTX_set_default_verify_paths` | X509 | certificates read from a file, freed when the store already holds them |
| `OPENSSL_init_ssl` | ENGINE | the configuration it loads by default may create engines |

The judgement is conservative. A call that may touch such an object on some
path qualifies.

**Not qualifying:**

* Creating or freeing shapes, each listed in the audit as touching no class:
  * `EVP_MD_CTX_new` (an `EVP_MD_CTX`);
  * `OPENSSL_sk_new_null` and `OPENSSL_sk_free` (`sk_free` frees the stack,
    never its elements);
  * `SSL_get1_peer_certificate` (`X509_up_ref`, `ssl_lib.c`).
* Plain calls:
  * the getters (`SSL_get_certificate`, `SSL_get_client_CA_list`, which
    returns a held list, `SSL_get_error`, the `X509_STORE_CTX_*` and
    `X509_*` getters, `SSL_CTX_get_cert_store`, `X509_STORE_get0_objects`);
  * the option and flag setters;
  * every callback slot setter;
  * `SSL_CTX_add_*custom_ext`;
  * the `ERR_*` functions;
  * `EVP_EncodeBlock`, `EVP_sha*`, `TLS_server_method`;
  * `OPENSSL_sk_value` and `OPENSSL_sk_push`.

## 3. The declarations and the checker

* **The declarations.** 29 declarations gain `requires
  resource_premise(openssl_ex_data_callbacks, "…")`, with the statement r30
  gave it, and a comment citing the reason. The eight that already
  mentioned ex_data keep their other premises unchanged, so the parallel
  OPENSSL-FREE-SESSION-REMOVE-INVOCATION card's edits stay separable.
* **The audit table.** `documented.rs` gains:
  * `EX_DATA_CLASSES`, the page's list;
  * `ExData::{Runs { classes, how }, NoClass { why }}`;
  * `EX_DATA_AUDIT`, with 34 rows: 30 `Runs` and 4 `NoClass`;
  * `EX_DATA_ONLY`.

  The premise lists that already existed (`HANDSHAKE`, `SHUTDOWN`,
  `SSL_FREED`, `BIO_FREED`, `UNSTATED`, `PEM_READ`, `PKCS12_READ`) and the
  two `SSL_CTX_use_*` lists gain `Premise::ExDataCallbacks`.
* **`check.rs`.** `ex_data_audited`, called by `check_function`, refuses a
  declaration in each of these cases:
  * it names the premise and the audit does not list it as `Runs`;
  * the audit lists it as `Runs` and it does not name the premise;
  * its documented shape creates or frees an object (`creates_or_frees`:
    the two constructors, `Release`, `CountedRelease`,
    `NullAcceptingRelease`) and the audit does not list it at all;
  * the audit names a class that is not in `EX_DATA_CLASSES`, or no class;
  * the audit lists it twice.

  So a later constructor or free cannot be added without an audit row.
* **Tests** (in the crate's existing unit target, so the read census needs
  no change):
  * `every_ex_data_class_object_names_the_premise`. Every `Runs` row's
    declaration names the premise, and dropping it refuses with a
    diagnostic naming the premise. Every creating or freeing entry is
    audited. The modules name the clause exactly 30 times.
  * `a_drifted_ex_data_reading_is_refused`. It covers each refusal above on
    synthetic entries, and the crate's own `X509_new`,
    `SSL_set_accept_state`, `SSL_new` and `EVP_MD_CTX_free` with the
    premise dropped.
  * `a_drifted_unstated_slot_reading_is_refused` now applies its
    chain-file drift inside that declaration, because the module holds
    other ex_data premises.
* **Lean.** `QCore.OpensslExDataPremise`, audited by
  `proof/AuditOpensslExDataPremise.lean` (16 theorems). It models
  `ex_data_audited`:
  * the audit is total over constructors and frees;
  * the premise stands exactly where a run is read, over ex_data classes;
  * a dropped premise or a second listing refuses;
  * the premise moves no verdict, through
    `OpensslUnstatedSlots.the_verdict_ignores_the_premise`;
  * the `vector_*` cases are taken from the crate's audit.

  Axioms are within `propext`, `Classical.choice` and `Quot.sound`
  (`private/logs/lean-2.log`).
* **Specification.** C04.18's OpenSSL paragraph gains "The ex_data premise
  on every constructor and free". No gateway rule changes: a named premise
  is metadata, and no route code changed.

**Lock.** This followed r32's order, with logs `private/logs/regen-*.log`:

1. `quirin-plugin load-path`, from `providers/`, over libcurl then openssl:
   exit 0.
2. `package.py checkers`: exit 1, refusing the stale views as expected in a
   fresh worktree.
3. `plugin views --write`: exit 0.
4. `pin --source-root …fixture --crate quirin-provider-openssl`: exit 0.
5. `contracts.py sync`: exit 0.
6. `package.py checkers`: exit 0.

**Lock diff, reviewed.** `contracts.lock` changes the 29 `contract_sha256`
and `pinned_at` values of exactly the newly premised declarations. The other
78 entries and every top-level key are unchanged. Both load paths
(`providers/contract-load-path.json`, `providers/user-crates-load-path.json`)
and `quirin-plugin.json` are byte-identical to base. This is as predicted.

## 4. Measurement

**Pinned routes.** No move is predicted, and the arbiter is deferred. No
pinned route loads the OpenSSL crate. The prediction's `measure-prediction`
block (none on all three routes) is for the integrator's `quirin-audit
measure --declared`.

**The predecessor fixture's user path.** This is the `contracts` step over
`openssl-provider-remaining-callees/evidence/fixture`:

* the tip binary and tree: `private/logs/userpath-tip.log`, exit 0;
* the base binary in the base side tree: `userpath-base.log`, exit 0.

Every function has the same status on both sides
([comparison](evidence/user-path/fixture-base-tip.txt)). `read_cert` and
`pkcs12_cycle` stay `Pending` and `read_cert_reuse` stays `Unknown`. There
are 113 bindings, all `Pending`. This report publishes no premise names on
either side.

**The opt-in `current` run (this card's arbiter).** It ran `quirin-audit
current --monkey-linked` over Fluent Bit 046accf6 with
OPENSSL-SSL-CTX-CALLBACK-SLOTS' script (`SOURCE_DATE_EPOCH=1767225600`,
`--jobs 12`), through the drivers
[`measure-base.sh`/`measure-tip.sh`](evidence/opt-in/).

* **Side trees.** Each side is a `git archive` side tree with a release
  `quirin-audit` built in it. Base is `062974679` and tip is `f3db96675`.
  Each tree's `package.py checkers` exited 0.
* **Inputs** were prepared by the base binary's `prepare` into
  `private/fb-046accf6` (exit 0, `private/logs/prepare.log`).
* **Timing.** The two runs overlapped in time, and the gate overlapped the
  tip run.

| Run | Files / configurations | Functions Safe / Pending / Unknown / Unsafe | Bindings |
|---|---|---|---|
| base (`private/opt-in/base-1`) | 180 / 181: 9 Safe, 0 Pending, 5 Unsafe, 163 Unknown, 4 data-only | 1098 / 2 / 3699 / 5 | 113, all `Pending` |
| tip (`private/opt-in/tip-1`) | same | 1098 / 2 / 3699 / 5 | 113, all `Pending` |

Both runs exit 0 with no unmatched row
([runs, invocations](evidence/opt-in/),
[comparison](evidence/opt-in/comparison-base-tip.json) by
OPENSSL-PROVIDER-REMAINING-CALLEES' `compare.py`).

**As predicted, no row moved and no row changed content.** No row went to
`Safe`, and none moved away from `Safe` or `Pending`. There is no new
`Unsafe`. No function row names the premise on either side. The premise text
appears only in the units' `reading_site_environments`, the contract texts
the reading environments embed: 102 occurrences at base and 1581 at tip,
over the same 44 environments, whose digests change with the texts. That is
where the newly named premise is published. There is no Safe-to-non-Safe
move to call out: a named premise is metadata (C04.18) and decides no
verdict.

## 5. Why the registry reading was not attempted

A sound reading would be a run-wide closed set over
`CRYPTO_get_ex_new_index`'s `new_func`, `dup_func` and `free_func` actuals.
In 3.x the `SSL_get_ex_new_index` family are macros over it. The set would
be keyed by the `class_index` literal, refusing a non-constant class, and
invoked at the class's constructors and frees. Four things it needs are
missing from the vocabulary:

1. **A registration keyed by a literal integer argument.** C04.19.3's
   registration needs a holder pointer formal, and `class_index` is an
   `int`.
2. **An invocation with no object parameter.** `X509_new` takes none, and
   neither does the `X509_STORE` that `SSL_CTX_new` creates.
3. **An invocation beside a reference drop or release.** C04.19 admits
   none. The parallel card OPENSSL-FREE-SESSION-REMOVE-INVOCATION is
   changing that rule for the frees, so building on it here would race
   that card.
4. **The free callback's `ptr` argument.** It is the value an undeclared
   `CRYPTO_set_ex_data` or `SSL_set_ex_data` kept, so a candidate that
   frees it (the usual free callback) needs a stated retention, or it
   refuses.

**Value at the corpus.** At Fluent Bit 046accf6 the only callers are in
nghttp2's `neverbleed.c` (`RSA_get_ex_new_index` and
`EC_KEY_get_ex_new_index` with free callbacks). That unit is not in the
production compile database (`grep -c neverbleed` over it: 0), so the reading
would decide an empty set and move no measured row.

**Decision.** That is a large card for no measured gain, and it would
overlap a running card. So the reading is registered narrower and not
attempted.

## 6. Gate

Command: `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
062974679 --out private/gate-1`. This is the scoped card gate (Clang 21,
Lean 4.35.0), with the provider gate included: 18 steps, `gate_scope: 18
steps passed`, exit 0.

* Record: [`gate/gate-1.json`](gate/gate-1.json).
* Logs: `private/gate-1/` and `private/logs/gate-1.log`.
* Tree: `f3db96675`. The report, evidence and amendments were uncommitted
  (docs only).
* Overlap: the gate ran concurrently with the tip opt-in run, so its wall
  times are inflated.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` (and the per-crate clippy steps for `qcore-c-gateway`, `qcore-contract`, `qcore-verification` and its lib) | 0 | 1 submodule |
| Rust suite (scoped) | 0 | 274 units, 281 summaries; 3456 passed, 0 failed, 9 ignored, 10 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 453 audits (new `AuditOpensslExDataPremise`) |
| Python suite | 0 | 779 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests | 0 | 49 summaries; 160 passed, 0 failed, 1 ignored |
| provider fmt, provider clippy | 0, 0 | |

No known flake fired. No test target was added, so the read census was not
re-recorded.

Outside the gate, during the work:

* the crate's checker tests: 17 passed, and `user_crate` 2 passed
  (`private/logs/ptest-1.log`);
* `lake build` of the module and its audit: exit 0
  (`private/logs/lean-2.log`).

## 7. Follow-ups

[`integration-amendments.json`](integration-amendments.json):

* retires `OPENSSL-EX-DATA-REGISTRY` by its second discharge;
* registers **OPENSSL-EX-DATA-CLASS-REGISTRY**, the reading of §5.

`python3 scripts/apply_amendments.py docs/reports/openssl-ex-data-registry/integration-amendments.json --check` exits 0.

**Files likely to conflict.** With OPENSSL-FREE-SESSION-REMOVE-INVOCATION,
which edits the frees in the same crate:

* `providers/quirin-provider-openssl/contracts/openssl/ssl.qv`
  (`SSL_free`, `SSL_CTX_free`), `bio.qv` and `x509.qv`;
* `src/documented.rs` (`SSL_FREED`, `UNSTATED`, `BIO_FREED` and the
  entries) and `src/check.rs`;
* `contracts.lock`, which should be regenerated, not merged by hand;
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18's OpenSSL paragraph;
* the generated `proof/QCore.lean`, which should be regenerated with
  `scripts/gen_lean_roots.py`.

CALLBACK-CANDIDATE-LITERAL-PREMISE and CALLBACK-SUPPLIED-NULL-ACTUAL touch
gateway code that this card does not touch.
