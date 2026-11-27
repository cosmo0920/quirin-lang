# REFERENCE-DROP-NULL-ACCEPTING: the NULL-accepting reference drop

Base `32336c33d` (origin/main). Follow-up `REFERENCE-DROP-NULL-ACCEPTING`
(registered by OPENSSL-PROVIDER-PLUGIN), launched by the user on 2026-11-17.
OpenSSL documents that its frees of reference-counted objects do nothing on a
NULL argument, but `resource_reference_dropped` (contract specification
C04.17, gateway rule 13b) had no nullability, so a caller freeing a pointer
that may be NULL stayed `Unknown`. This card adds
`resource_reference_dropped(p, nullable)` end to end and declares the OpenSSL
user crate's six frees with it. [Prediction](PREDICTION.md), committed before
implementation (`0e9a6c7c4`).

## 1. The form

```qcore
ensures normal resource_reference_dropped(p, nullable);
```

* **Non-NULL edge:** exactly rule 13b's drop (decrement, possibly-final last
  drop, over-drop of an acquired family, refusal of a reference the function
  does not hold).
* **NULL edge:** a no-op on the family. It discharges nothing and creates
  nothing (`QCore.LibraryVersionRange`'s `null_release_is_noop` and
  `null_drop_discharges_nothing` are the premise).

Route G reads a drop's actual in three classes
(`legacy_reference_count::drop_actual`, the pure table
`reference_count::decide_drop_actual`):

| Actual | Drop not marked nullable | NULL-accepting drop |
|---|---|---|
| definitely NULL (no target, not unknown, possibly null: the gateway's own definition) | refused, `reference-drop-unheld` (as `Other` before the card) | no-op |
| exactly one live family or NULL (an identity binding of one site, not unknown, not an unresolved effect; e.g. a nullable returned reference with no NULL test, a branch join with NULL) | refused, `reference-drop-unheld` (as before) | that family's drop |
| anything else (a held family, an unbound formal, a member read, an invalidated name, …) | rule 13b's table | rule 13b's table, unchanged |

A drop not marked nullable therefore decides exactly as at base: the two NULL
classes were `Binding::Other` before and are decided as `Other` now. Every
shipped reference declaration (librdkafka's) is non-nullable, which is why no
pinned route can move.

Rule 13c (a declaration whose body the unit defines): a NULL-accepting drop
stands only where its body does nothing on NULL structurally: it opens with
`if (!p) return;` or `if (p == NULL) return;` (`NULL`, `0` or `((void *) 0)`),
or it touches the object only by forwarding it whole to a declared
NULL-accepting drop at the same position, never through a member access.
Otherwise it is withdrawn there (`reference-withdrawn`).

**Grammar and loader** (G05, total-or-reject). `nullable` is the only word a
drop takes after its parameter; `nonnull` (what a drop without the word
means), any other word, a second word, and any word after a keep's parameter
reject the module. **Schema:** `ReferenceClause::nullable`, omitted from every
serialized form when false; adding a public field is a major change, so
`qcore-schemas` is 28.0.0 (its own commit, `ea72db13f`, so a merge can
renumber it; every consumer's requirement and the three lock files follow).
`ReferenceOperand::nullable` (the route-G instruction) is likewise omitted when
false, and the clause-standing label gains `, nullable` only when set.

Specification: contract spec C04.17 ("The NULL-accepting drop" and its
placement sentence), grammar G05 (`reference_predicate`), gateway rule 13b
(amended in place: the form is a clause of it).

## 2. The OpenSSL frees

Each free now declares the NULL-accepting drop, and its range is the oldest
documentation series checked whose official page states the NULL behaviour
(the 1.0.2 pages do not state it for any of the six; the 1.1.1 `ENGINE_add`
page does not state it for `ENGINE_free`). `doc-snapshots.json` keeps every
page's SHA-256 as fetched; the one new page, `ENGINE_add` at 3.0, was fetched on
2026-11-17 with `fetch_docs.py --append` (new: it adds entries with their own
`fetched_on` and refuses a page/series already recorded). Refetching
`SSL_free` at 3.5, 1.1.1 and 1.0.2 and `X509_new`, `BIO_new`, `EVP_PKEY_new`,
`SSL_CTX_free` at 1.1.1 on 2026-11-17 gave the recorded digests byte for byte
(not committed; `private/docs/`).

| Function | Range before → after | Page and series | Sentence the snapshot records |
|---|---|---|---|
| `X509_free` | `documented "1.0.2"` → `"1.1.1"` | `X509_new` 1.1.1 (`e461da4c…`) | "If a is NULL nothing is done." |
| `EVP_PKEY_free` | `"1.0.2"` → `"1.1.1"` | `EVP_PKEY_new` 1.1.1 (`f893b83c…`) | "If key is NULL, nothing is done." |
| `SSL_free` | `"1.0.2"` → `"1.1.1"` | `SSL_free` 1.1.1 (`c2331b6a…`) | "If ssl is NULL nothing is done." |
| `SSL_CTX_free` | `"1.0.2"` → `"1.1.1"` | `SSL_CTX_free` 1.1.1 (`e06a3592…`) | "If ctx is NULL nothing is done." |
| `BIO_free` | `"1.0.2"` → `"1.1.1"` | `BIO_new` 1.1.1 (`032ee76b…`) | "If a is NULL nothing is done." |
| `ENGINE_free` | `"1.1.1"` → `"3.0"` | `ENGINE_add` 3.0 (`005219d0…`, new) | "If the argument to ENGINE_free() is NULL, nothing is done." |

Every range still admits the pinned OpenSSL 3.5.5. The checker
(`quirin-openssl-check`) gains the documented shape `NullAcceptingRelease`:
the contract must state exactly one custody clause,
`resource_reference_dropped(p, nullable)`, and the cited snapshot at the
range's series must record the exact NULL sentence; a plain drop for one of
these functions, the form at a series whose page does not state it, and a
final free are refused (`a_drifted_contract_is_refused`), and
`every_null_accepting_drop_cites_the_oldest_series_stating_it` checks every
fetched series of each page: none older than the bound states the sentence,
the bound's does.

The crate's `contracts.lock` was re-pinned (`quirin-audit pin`, the tip
binary): the six declarations' digests move, the other six and
`library_versions` are unchanged; `providers/contracts.py sync` wrote nothing
else; `pin --verify-contracts` exit 0 (`0 pinned, 12 flexible`). The user path
of OPENSSL-PROVIDER-PLUGIN (`evidence/user-path.sh`, this card's binaries,
`private/user-path-1/`): every step exit 0; the fixture's `contracts` run
reads `checker plugins: 12 binding(s): 0 Safe / 12 Pending / 0 Unknown / 0
provisional Unsafe`, so the checker accepts the new shape.

## 3. Lean

`proof/QCore/NullableReferenceDrop.lean`, audited by
`proof/AuditNullableReferenceDrop.lean` (12 theorems, axioms within
`propext`; no placeholder, no project axiom), over `QCore.ReferenceCount`'s
tables: `a_plain_actual_decides_as_today` (the non-NULL edge),
`a_non_nullable_drop_refuses_a_possibly_null_actual` and
`a_nullable_drop_of_an_unheld_reference_is_refused` (the two negative
controls), `only_the_null_edge_is_a_no_op`,
`the_one_or_null_edge_is_the_family_drop`,
`a_final_nullable_drop_is_the_last_reference`; on a replay state with the
definitely-NULL names, `the_null_edge_changes_nothing`,
`the_null_edge_discharges_and_creates_nothing` (families, acquisitions,
names, the next family, the refusal and violation flags and the outcome),
`the_non_null_edge_is_the_drop`, `nullable_drop_traces`; and
`nullable_drop_vectors_hold`, the table `NULLABLE_DROP_VECTORS` that
`tests/reference_drop_null_accepting.rs` binds to the Lean text row by row.

## 4. Tests

* `crates/qcore-c-gateway/tests/reference_drop_null_accepting.rs` (route G,
  fixture `refcount_nullable.qv`): the declaration loads withheld with
  `nullable`; the non-NULL edge (balanced after a NULL test, a keep dropped,
  a leak with two references held, an over-drop `DoubleRelease`, a use after
  the last drop refused); the NULL edge (a nullable returned reference dropped
  with no NULL test is `Safe`, a definitely-NULL pointer, a branch join with
  NULL, a NULL drop beside an acquired reference leaves it owed: a leak
  candidate); the negative controls (`rc_drop` of a possibly-NULL and of a
  definitely-NULL actual still refuse; `rc_drop_null` of a caller's formal and
  of a member read still refuse); rule 13c withdrawing an unguarded body; the
  Lean vectors.
* `reference_count.rs` unit tests: the vectors, and that every row of
  `REFERENCE_VECTORS` decides identically through `decide_drop_actual` with
  either nullability (`a_non_nullable_drop_is_unchanged`).
* `legacy_reference_count.rs`: `a_null_accepting_drop_body_does_nothing_on_null`.
* `crates/qcore-contract/tests/reference_clause.rs`:
  `a_null_accepting_drop_loads_and_every_other_word_rejects`.
* The OpenSSL checker's tests above.

## 5. Setup

Toolchain: `private/toolchain/env.sh` (system Clang 21 at
`/usr/lib/llvm-21/bin/clang`, Lean 4.35.0). The host has no shared prepared
inputs, so the routes were prepared in this worktree with the base binary's
`quirin-audit prepare` from the local Fluent Bit clone, as
LERI-HELD-AND-LINK-MEMBER-PREMISES's were: system and the opt-in run at
`046accf6` (`private/prep/system`), the slice at upstream `90978ac6`
(`private/prep/slice`), the corpus at `66910c10` with its production database
copied into `fluent-bit-clang-source/build/` (`private/corpus-inputs`), and
`providers/package.py build` at the base for `provider-packages-wave20`
([inputs](artifacts/inputs.json)). Absolute figures therefore differ from the
published baselines; every comparison is base against tip on the same inputs.

## 6. The opt-in `current` run

[`scripts/optin.sh`](scripts/optin.sh): `quirin-audit current --load-path
providers/user-crates-load-path.json`, the contract selection of
OPENSSL-PROVIDER-PLUGIN's `evidence/opt_in_providers.py` (the profile
manifest's modules plus `quirin-provider-openssl`), over a unit selection of
the seven files holding the OpenSSL callers its census names on `current`
([selection](artifacts/optin-selection.json)). Base: the base tree's crate,
its checker packaged from the base, the base binary (`d385436f…`); tip: this
branch's crate, lock and checker, the tip binary (`10da9cf3…`, built from the
measured export below). Monkey's `tls_openssl.c` is selected as a group of
the Fluent Bit database (the configuration linked into `fluent-bit`);
`--monkey-linked` would add every Monkey unit and refuses a group selecting
one. Both runs exit 0 (`private/optin/{base,tip}/run.log`).

| | base | tip |
|---|---|---|
| units (7 files, 7 configurations) | 0 Safe / 0 Pending / 7 Unknown | 0 / 0 / 7 Unknown |
| function rows | 422: 131 Safe / 2 Pending / 289 Unknown | 422: 131 / 2 / 289 |
| checker bindings | 12 Pending | 12 Pending |

Row by row ([`scripts/compare_optin.py`](scripts/compare_optin.py),
[compare.json](artifacts/compare.json)): **no row moves and no row's
diagnostics change** — as predicted. What differs is outside the function
rows: the six declarations' text and the checker's binding diagnostics,
which name the new ranges (`documented "1.1.1"`, `ENGINE_free` `documented
"3.0"`), every binding still `Pending`. Three of the census's files
(`rdhttp.c`, `rdkafka_sasl_oauthbearer.c`, `rdkafka_sasl_oauthbearer_oidc.c`)
have no record in this preparation's database (the census used the
integrator's linked database, `89fa31d21`): each stays one `Unknown` unit in
the denominator on both sides.

Every move attributed: there is none, and why. On both sides the six frees
carry the same 59 diagnostics in 13 rows ([`scripts/count_rows.py`](scripts/count_rows.py)), every one
`reference-drop-unheld`. 3
drop a member read (`rk->rk_conf.ssl.ctx`, `rktrans->rktrans_ssl`, …:
`REFCOUNT-MEMBER-REFERENCE-DROP`), 56 a plain name whose object came from an
OpenSSL producer this crate does not declare (`PEM_read_bio_X509`,
`PEM_read_bio_PrivateKey`, `SSL_get1_peer_certificate`, `X509_new`,
`BIO_new_mem_buf`, `ENGINE_by_id`, `d2i_PKCS12_bio`, …) or a member, so the
name denotes no family the function holds: the NULL-accepting form changes
only a NULL or one-family-or-NULL actual, and none of these is one.

## 7. Native measurement and the arbiter

`quirin-audit measure --declared PREDICTION.md` (the measure plugin built
from the measured code, [`scripts/arbiter.sh`](scripts/arbiter.sh): `--no-slots --jobs 8`), over
[inputs.json](artifacts/inputs.json). Both sides are checkout directories,
each built in place by the tool: exports of `32336c33d` (base) and
`a44ed44e7` (tip: the code commit before it was amended into `bcf1c91ab`,
which adds only the OpenSSL crate's re-pinned lock and the specification
text, neither read by a pinned route; the amendments file names `bcf1c91ab`
as the measured tip because `a44ed44e7` is no longer on the branch) under
`private/{base,tip}-src`. One measurement-driver adaptation in both exports,
disclosed as LERI-HELD-AND-LINK-MEMBER-PREMISES did: `scripts/run_pinned_audit.py`
hard-codes `QCORE_CLANG=/usr/bin/clang`, which this host does not have, so both
point it at `/usr/lib/llvm-21/bin/clang`.

The first measurement (`measure/arbiter-1`) completed system and slice and
failed the corpus route on both sides: `could not verify Cycle 18 Libevent
source: Pin(GitUnavailable)`, the exports' `third_party/libevent` being a copy
of the worktree's gitlink rather than a repository. Retained, discarded as
corpus evidence; the corpus was remeasured alone (`measure/arbiter-2`,
`--routes corpus`) after replacing it in both exports with a clone at the
pinned `d82464a2` (which rebuilt both binaries: base `c81956ec…`, tip
`e4854dfe…`; arbiter-1's were `d385436f…`, `10da9cf3…`).

| Route | rows | base = tip | moved | content changed | declared |
|---|---:|---|---:|---:|---|
| corpus ([record](artifacts/native-comparison-corpus.json.gz), sha256 `08d9d953…` uncompressed) | 4,704 | 1,082 Safe / 6 Unsafe / 3,616 Unknown; leak 7, double release 1 | 0 | 0 (4,704 environment-only) | **match** |
| system ([record](artifacts/native-comparison-system-slice.json.gz), sha256 `c59c280a…`) | 448 | 107 / 4 / 337 | 0 | 4 | **match** |
| slice (same record) | 404 | 63 Safe / 1 Unsafe / 336 Unknown / 4 Pending | 0 | 8 | **match** |

The system and slice content changes are environment-only: each row's
`review_holds` names its holding module by the checkout path, which differs
between `private/base-src` and `private/tip-src` (`flb_output_exit` checked
byte for byte after substituting the path). No unit row and no unit verdict
moves; diagnostic counts are identical on every route. The system and slice
base figures equal LERI-HELD-AND-LINK-MEMBER-PREMISES's preparations
(107 / 4 / 337 at base there was before its own move; these are this base's).

## 8. Gate

`python3 scripts/gate_scope.py --base 32336c33d --out private/gate-1` under
`private/toolchain/env.sh`, on `00ecd18cb` (full scope: the diff reaches
`qcore-schemas`; Lean all; provider gate yes; record
[`gate/gate-1.json`](gate/gate-1.json), Clang [`gate/clang-version.txt`](gate/clang-version.txt);
logs `private/gate-1/`). It ran while the corpus arbiter and other cards'
audits loaded the host.

| Step | Exit | Counts |
|---|---:|---|
| submodules, `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` | 0, 0, 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 651 units, 696 summaries; 9,623 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 443 audits (`AuditNullableReferenceDrop.lean` among them) |
| Python suite | 1 | 779 tests, 1 failure |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | five packages |
| provider tests, fmt, clippy | 0, 0, 0 | 46 summaries; 141 passed, 0 failed, 1 ignored |

The Python failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`:
the committed acceptance matrix records `qcore_schemas_rust_api` 27.0.0 and
the tree is 28.0.0 (this card's bump). The matrix is the integrator's record,
regenerated at landing by `build_matrix.py . --gate <gate.json>`
(`scripts/land.py`), as CURRENT-CONTEXT-NEW-TRIAGE D3's 27.0.0 bump was.

The commits after the gate (this section, the measurement records and the
arbiter's figures in §7) adds report files only and reaches no crate.

## 9. Follow-ups

Registered in [`integration-amendments.json`](integration-amendments.json):
`REFERENCE-DROP-NULL-ACCEPTING` retired (discharged here);
`REFCOUNT-MEMBER-REFERENCE-DROP` amended (the OpenSSL member drops it now
blocks); `REFERENCE-DROP-NULL-ACCEPTING-CANDIDATES` registered (none of
librdkafka's declared drops accepts NULL; the OpenSSL reference-counted frees
not yet declared should take the form where their pages state it). Files
likely to conflict with concurrent cards: the qcore-schemas version in every
`Cargo.toml` and the three lock files (renumber at merge), the OpenSSL crate's
`contracts.lock` (regenerate by `quirin-audit pin`), `doc-snapshots.json`,
`src/check.rs`, and the generated `docs/agents/gateway.md`/`README.md`
(rerun `gen_rule_docs.py`).

On the final tip the Python suite was rerun alone (the only step the
post-gate paths select: `gate_scope.py --paths ... --plan`, 0 crates): 779
tests, the same single matrix-count failure (`private/python-final.log`).
