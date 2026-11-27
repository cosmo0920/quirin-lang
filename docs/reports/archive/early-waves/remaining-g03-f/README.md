# G03-F: checkout-root source sets (F1) and Libevent's unmodified CMake database (F2)

Date: 2026-09-09. Workstream W2, card `G03-F` (follow-ons `G03-F1` and
`G03-F2` from `docs/reports/remaining-g03/README.md`). Base commit `5589839`.

This closes the two generalization gaps G03 recorded and deliberately left
unrepaired. It certifies no library. Every verdict below is a bounded resource
projection under declared contracts; `Unknown` stays in the denominator, a
provisional `Unsafe` is never presented as an upstream defect, and **no raw
unit is promoted**.

Headline: the repository's own pinned `third_party/libevent` submodule is now
an installed, configured source set. Its **unmodified CMake compilation
database** is the audit input — nothing is rewritten, dropped or hand-authored
— and the run completes over 21 translation units with **zero compiler-option
refusals and zero gateway failures**.

---

## 1. F1 — a source set whose C sources sit at the checkout root

### 1.1 The gap

`LibraryProfile::source_roots[].root` was validated by
`is_relative_without_traversal`, which admits only `Component::Normal`
segments. `Path::new(".").components()` yields `Component::CurDir`, so `"."`
was refused and Libevent — whose `.c` files live directly at its repository
root — was unrepresentable. G03 recorded this as a schema decision with a
`PROFILE_MANIFEST_VERSION` consequence.

### 1.2 Schema decision: version 2, both versions supported

`docs/design/W2_PROFILE_MANIFEST_SCHEMA.md` §2 states that under
total-or-reject plus `deny_unknown_fields`, *effectively every schema change is
breaking and takes a new `schema_version`*, and that a v1 file stays loadable
only if the v2 change record defines an **exact, total v1 → v2
interpretation**.

Decision, applying that policy literally:

| Question | Decision |
|---|---|
| Bump or compatible? | **Bump.** No field is added, removed or renamed and no closed vocabulary is extended, but the *set of admissible manifests* changes. §2 makes that a version event. |
| `PROFILE_MANIFEST_VERSION` | `1` → **`2`** |
| `SUPPORTED_PROFILE_MANIFEST_VERSIONS` | `[1]` → **`[1, 2]`** |
| Is the v1 → v2 interpretation exact and total? | **Yes, and asymmetrically.** Version 1 refuses every root spelling version 2 reads differently, so any v1 manifest that loads at all means exactly the same thing under v2. The converse does not hold, deliberately: a manifest naming the checkout root **must** declare version 2, and the same bytes under `"schema_version": 1` still reject. |
| Are the shipped profiles migrated? | **No.** `fluent-bit`, `libevent`, `cmetrics-counter` and `chunkio-core` stay byte-identical v1 files. None names a root version 1 refuses, so migrating them would change bytes without changing meaning. `compiler/tests/profile_loader.rs` now pins their declared version at exactly `1`, so a silent migration is a test failure. |

The new spelling is a **literal**, not a normalisation rule:
`qcore_schemas::profile::CHECKOUT_ROOT_SOURCE_ROOT = "."` is compared by string
equality. `"./src"`, `".."`, `"src/.."`, `"/"`, `"/abs"`, `""` and
`"../checkout"` keep their version-1 rejection at both versions. `"src/."` is
*not* in that list and is not a new admission either: `Path::components`
normalises a trailing `.` away, so it was already an ordinary relative root
under version 1.

### 1.3 Implementation

- `crates/qcore-schemas/src/profile.rs`: the two version constants and
  `CHECKOUT_ROOT_SOURCE_ROOT`, re-exported from `qcore_schemas`.
- `crates/qcore-contract/src/profile_loader.rs`:
  - `is_admissible_source_root(root, schema_version)` — the version-aware
    admission, threaded from `manifest.schema_version` inside
    `validate_manifest`;
  - `source_root_path(checkout, root)` — one shared resolver, so every consumer
    agrees that the checkout-root spelling means the checkout **itself**, not
    `checkout/.` (whose textual form would compare unequal to a canonicalised
    entry path).
- Three consumers moved onto that resolver, with no other behaviour change:
  `crates/qcore-audit/src/lib.rs` (`select_profile_units`),
  `crates/qcore-audit/src/current.rs`, and
  `compiler/examples/support/cycle19_compatibility.rs`.

Nothing else widened. `audited_files[].path`, `pins[].clean_paths`,
`UnitSelection::Exact { file }` and `provider_manifests[]` keep their unchanged
no-traversal rule, so the loader still never reads outside the declared set.

### 1.4 Refusal matrix (F1)

`crates/qcore-audit/tests/root_source_set.rs`, over a hermetic pinned
repository with root-level C sources:

| Case | Schema 1 | Schema 2 |
|---|---|---|
| `"."` (checkout root) | rejected (`source root … must be relative`) | **accepted** |
| `".."` | rejected | rejected |
| `"./src"` | rejected | rejected |
| `"src/.."` | rejected | rejected |
| `"/"` | rejected | rejected |
| `"/abs"` | rejected | rejected |
| `""` | rejected | rejected |
| `"../checkout"` | rejected | rejected |
| `"lib/example/src"` (ordinary) | accepted | accepted |
| declared version `3` | whole profile rejected (`UnsupportedSchemaVersion { found: 3, supported: [1, 2] }`) | — |

Every rejection writes no report.

---

## 2. F2 — Libevent's unmodified CMake compilation database

### 2.1 The gap

`compile_db::compiler_options` accepted four frontend flags and rejected
everything else. Libevent's own `CMakeLists.txt` appends 17 further options
unconditionally under a GNU-like compiler, plus a Release `-O3`, so every one
of its 55 database entries rejected. G03 recorded that extending the set is a
feature: each flag needs a preservation argument through preprocessing, syntax
and the typed AST.

### 2.2 Per-flag preservation arguments (measured)

Method: for each option, run the configured Clang 21.1.8 over the real
`third_party/libevent/log.c` with the real include set, twice — with and
without the option — and compare (a) the `-E -P` preprocessed text byte for
byte and (b) the `-dM -E` predefined-macro set. Raw output:
`flag-preservation-measurement.json`; script:
`measure_flag_preservation.py`.

**Group A — diagnostic-only warning selection.** Measured: preprocessed text
byte-identical, macro set unchanged, and Clang exits 0. The option selects or
deselects a warning group and nothing this gateway reads. None is `-Werror`, so
no admitted unit can be turned into a refusal by its warning set. Transporting
it reproduces the recorded command exactly.

| Option | `CFrontendFlag` variant | `-E -P` identical | macros added | macros removed |
|---|---|---|---|---|
| `-Wextra` | `ExtraWarnings` (pre-existing) | yes | — | — |
| `-Wstrict-aliasing` | `StrictAliasingWarnings` | yes | — | — |
| `-Wstrict-prototypes` | `StrictPrototypes` | yes | — | — |
| `-Wundef` | `UndefinedIdentifierWarnings` | yes | — | — |
| `-Wmissing-prototypes` | `MissingPrototypes` | yes | — | — |
| `-Winit-self` | `InitSelf` | yes | — | — |
| `-Wmissing-field-initializers` | `MissingFieldInitializers` | yes | — | — |
| `-Wdeclaration-after-statement` | `DeclarationAfterStatement` | yes | — | — |
| `-Waddress` | `AddressComparisons` | yes | — | — |
| `-Woverride-init` | `OverrideInit` | yes | — | — |
| `-Wwrite-strings` | `WriteStrings` | yes | — | — |
| `-Wvla` | `VariableLengthArrays` | yes | — | — |
| `-Wno-unused-parameter` | `NoUnusedParameter` | yes | — | — |
| `-Wno-unused-function` | `NoUnusedFunction` | yes | — | — |
| `-Wno-pragmas` | `NoPragmas` | yes | — | — |
| `-Wno-void-pointer-to-enum-cast` | `NoVoidPointerToEnumCast` | yes | — | — |

**Group B — code generation, no preprocessing effect.**

| Option | Variant | `-E -P` identical | macros |
|---|---|---|---|
| `-fno-strict-aliasing` | `NoStrictAliasing` | yes | unchanged |

`-fno-strict-aliasing` relaxes the aliasing rule the optimizer may assume. It
is transported so the recorded command is reproduced, and — exactly like the
pre-existing `AssemblerNoExecStack` — **no object or aliasing property is
certified by admitting it**.

**Group C — optimization level, preprocessing-visible.** This is the finding of
the card. `-O` above `-O0` is *not* a code-generation-only request:

| Option | Variant | `-E -P` identical | macros added | macros removed |
|---|---|---|---|---|
| `-O0` | `Optimization(None)` | yes (baseline) | — | — |
| `-O1` | `Optimization(Level1)` | **no** | `__OPTIMIZE__`, `__USE_EXTERN_INLINES`, `_EXTERN_INLINE`, `_BITS_STDIO_H`, `htonl`/`htons`/`ntohl`/`ntohs`, `fread_unlocked`, `fwrite_unlocked` | `__NO_INLINE__` |
| `-O2` | `Optimization(Level2)` | **no** | same as `-O1` | `__NO_INLINE__` |
| `-O3` | `Optimization(Level3)` | **no** | same as `-O1` | `__NO_INLINE__` |
| `-Os` | `Optimization(Size)` | yes (this file) | `__OPTIMIZE__`, `__OPTIMIZE_SIZE__`, `htonl`/`htons`/`ntohl`/`ntohs` | `__NO_INLINE__` |
| `-Oz` | `Optimization(MinimumSize)` | yes (this file) | same as `-Os` | `__NO_INLINE__` |
| `-Og` | `Optimization(Debug)` | **no** | same as `-O1` | `__NO_INLINE__` |

Clang's preprocessor defines `__OPTIMIZE__` (and `__OPTIMIZE_SIZE__` for the
size levels) and drops `__NO_INLINE__`, and glibc headers branch on those
macros. **Dropping the build's level therefore analyses a different translation
unit.** The archived G03 attempt dropped `-O3`; that was unsound as an identity
claim, and this card measures the consequence directly (§2.5).

The level set is closed and each spelling is named exactly. Bare `-O`,
`-Ofast` and every other spelling stay refused rather than being folded onto a
neighbouring level.

### 2.3 Implementation

- `crates/qcore-c-gateway/src/legacy_c.rs`: `CFrontendFlag` gains the 17
  variants above plus `Optimization(COptimizationLevel)`; the new
  `COptimizationLevel` is a closed 7-member enum whose `spelling()` reproduces
  the build's own text. `CFrontendFlag::append_to` — the single site that feeds
  preprocessing, syntax and the typed AST via `append_frontend_flags` — appends
  each verbatim.
- `crates/qcore-audit/src/compile_db.rs`: the admission table moves into a
  documented `frontend_flag(word)` allowlist keyed on the exact spelling. The
  fail-closed default is untouched: an option absent from the table still falls
  through to
  ``compiler option `{word}` cannot be preserved by the bounded gateway``.

Adding a row means supplying a preservation argument, not widening a default.
There is no catch-all variant and no "ignore unknown flag" path.

### 2.4 The multi-target database: an explicit declared narrowing

Measured on the real database: 55 entries over 28 files; **27 of 28 files carry
two configurations**, one per static-library target
(`event_core_static` and `event_static`). The two commands differ **only in
`-o`**.

`same_configuration` compares directory plus the complete command text, so this
is genuine ambiguity, and `select_profile_units` correctly refuses it —
`conflicting compilation commands for …`. Silently choosing one is exactly the
defect recorded in `docs/reports/remaining-g01/README.md`.

The repair is an explicit, digested, reviewable **configuration** input rather
than a relaxed comparison:

```json
"compile_target": { "object_prefix": "CMakeFiles/event_core_static.dir/" }
```

`compile_db::select_object_prefix` keeps only records whose `-o` argument
begins with that fragment, before anything else reads the database.
Deliberately:

- it **narrows, never merges** — `same_configuration`,
  `select_unique_configuration` and `select_profile_units` are unedited, and
  two retained records for one file that differ in any other argument still
  reject (test: `residual_ambiguity_inside_the_selected_target_still_refuses`);
- an empty result, a prefix starting with `-`, a prefix containing `$` and a
  whitespace prefix each reject with no report;
- a record whose `-o` cannot be read does not match, so it is dropped rather
  than kept on an assumption about its target;
- the selection is recorded in the report sidecar as
  `compile_target_selection`, so no report presents a target-selected result
  without naming the selection it read.

`AuditConfig::schema_version` stays `1`. This follows the established
precedent for that file — `source_revision_policy` and `source_contracts` were
both added as optional `#[serde(default)]` fields without a bump — and the
profile manifest's stricter policy does not govern the audit configuration.
Every installed `audit.json` remains byte-identical.

### 2.5 What dropping `-O3` would have done (measured)

`optimization_effect.py` runs the same audit twice over the same sources: once
with CMake's unmodified command, once with `-O3` removed.

| Measure | With `-O3` | Without `-O3` |
|---|---|---|
| Units | 21 | 21 |
| Verdicts | 0 Safe / 4 provisional Unsafe / 17 Unknown | identical |
| Aggregate diagnostics | leak=5, unknown_alias=344, unsupported=4940 | identical |
| Recorded preprocessed-text digests | baseline | **20 of 21 differ** |

Stated honestly: on this corpus the published verdicts and diagnostic totals
did **not** move. What moved is the analysed program — 20 of 21 units
preprocess to different text — so the dropped-flag database recorded a
compile-task identity for a translation unit the build never compiles. That is
an identity defect, not a demonstrated verdict defect.

---

## 3. The `libevent-core` source set

| Property | Value |
|---|---|
| Source set | `libevent-core` (installed at `contract/profiles/libevent-core/`) |
| Pin | `third_party/libevent` `d82464a277d0f42703702c4dfd9af6af38595a83` (release-2.2.2-alpha-11) |
| Manifest | `profile.json`, **schema version 2**, one source root `{"root": ".", "selection": "top_level"}` |
| Audited files | 21 digest-pinned root-level `.c` files |
| Compile database | `build/cycle19/libevent-clang-build/compile_commands.json` — CMake's **unmodified** output, 55 entries |
| Declared target | `CMakeFiles/event_core_static.dir/` |
| Resource contracts | `contracts.json` → `libevent_memory.qcore` (5 declared `event_mm_*` contracts), inside the profile directory — **not** under `contract/qcore/`, so the pinned corpus count stays 60 |
| Provider binding | none (`provider: null`) |

Reproduce:

```sh
git -c protocol.file.allow=always submodule update --init --recursive
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit
python3 -B scripts/prepare_libevent_core_audit.py
cargo run --quiet -p qcore-audit -- contract/profiles/libevent-core/audit.json
```

`prepare_libevent_core_audit.py` verifies the submodule pin and clean tree,
CMake-configures and natively builds `event_core_static` with Clang, checks
that the native build changed no tracked source, re-derives the selected unit
set from the database and requires it to equal the declared audited-file table
digest for digest, and writes `libevent-build-provenance.json`
(archived here). `--write-profile` regenerates the digest table from the
verified checkout, per design Q2.

### 3.1 Audit result

```
21 units: 0 Safe / 4 provisional Unsafe / 17 Unknown / 0 data-only / 0 gateway failures
```

Functions across the set: **32 Safe / 4 Unsafe / 597 Unknown**, plus 51
functions listed under `not_projected` rather than dropped. Full transcript:
`libevent-audit-stdout.log`; sidecar: `libevent-core-audit.assumptions.json`.

| Unit | Verdict | Projected / source functions | Diagnostics | `reason_code` |
|---|---|---|---|---|
| `buffer.c` | Unknown | 101/103 | unknown_alias=100, unsupported=1097 | `unsupported-syntax` |
| `bufferevent.c` | Unknown | 54/55 | unknown_alias=14, unsupported=473 | `unsupported-syntax` |
| `bufferevent_filter.c` | provisional Unsafe | 19/19 | leak=1, unknown_alias=13, unsupported=144 | null |
| `bufferevent_pair.c` | Unknown | 14/14 | unknown_alias=3, unsupported=96 | `unsupported-syntax` |
| `bufferevent_ratelim.c` | provisional Unsafe | 44/44 | leak=2, unknown_alias=19, unsupported=406 | null |
| `bufferevent_sock.c` | Unknown | 21/21 | unknown_alias=12, unsupported=183 | `unsupported-syntax` |
| `epoll.c` | Unknown | 10/10 | unknown_alias=5, unsupported=93 | `unsupported-syntax` |
| `event.c` | Unknown | 172/174 | unknown_alias=87, unsupported=1230 | `unsupported-syntax` |
| `evmap.c` | Unknown | 15/17 | unknown_alias=5, unsupported=104 | `unsupported-syntax` |
| `evthread.c` | Unknown | 18/29 | unknown_alias=15, unsupported=104 | `unsupported-syntax` |
| `evutil.c` | provisional Unsafe | 81/92 | leak=1, unknown_alias=42, unsupported=467 | null |
| `evutil_rand.c` | Unknown | 8/8 | unsupported=3 | `unsupported-syntax` |
| `evutil_time.c` | Unknown | 10/13 | unknown_alias=2, unsupported=45 | `unsupported-syntax` |
| `listener.c` | provisional Unsafe | 16/32 | leak=1, unknown_alias=4, unsupported=125 | null |
| `log.c` | Unknown | 14/14 | unknown_alias=5, unsupported=41 | `unsupported-syntax` |
| `poll.c` | Unknown | 5/6 | unknown_alias=3, unsupported=71 | `unsupported-syntax` |
| `select.c` | Unknown | 7/8 | unknown_alias=2, unsupported=70 | `unsupported-syntax` |
| `signal.c` | Unknown | 13/13 | unknown_alias=7, unsupported=106 | `unsupported-syntax` |
| `signalfd.c` | Unknown | 5/5 | unknown_alias=4, unsupported=58 | `unsupported-syntax` |
| `strlcpy.c` | Unknown | 0/1 | — | `incomplete-obligation-coverage` |
| `watch.c` | Unknown | 6/6 | unknown_alias=2, unsupported=24 | `unsupported-syntax` |

This is an Unknown-heavy result and that is the expected, valid outcome: it is
a withdrawal of unsupported claims, not a finding about Libevent. The four
provisional `Unsafe` units are **not** presented as upstream defects. **No raw
unit is promoted.** The `libevent` source set that Cycle 18's event-reactor
provider uses is a separate profile and is untouched.

### 3.2 Reason codes

All 21 unit rows classify correctly against `qcore_schemas::reason` (schemas
6.0.0) and **no new reason string appeared**, so the classification mapping
needed no extension:

| `reason_code` | Units | Why |
|---|---|---|
| `unsupported-syntax` | 16 | `resource projection unresolved: N unsupported operation(s), …` |
| `incomplete-obligation-coverage` | 1 | ``C function `event_strlcpy_` was not found`` |
| `null` | 4 | the four provisional `Unsafe` units carry **no reason at all**; `reason_code(None)` is `null` by the fail-closed rule, never a defaulted category |

Zero unclassifiable reasons were produced, so nothing fell through to `null`
by refusal. One pre-existing boundary is re-measured, not repaired:
resource-projection **function** rows carry no `reason` field at all, so all
633 of them have no `reason_code` — this is the same gap CORPUS-REPIN records
under `REASON-CODES-B`, unchanged by this card.

---

## 4. Lean

`proof/QCore/SourceRootAdmission.lean` (new module, audited by
`proof/AuditSourceRootAdmission.lean`) models the F1 admission decision:
`is_admissible_source_root` and `source_root_path`. **18 theorems**, all
depending only on `propext`, `Classical.choice` and `Quot.sound`; no new axiom,
no `native_decide`, no placeholder. Lean audits move **38 → 39**.

What is proved:

- **Widening.** Version 2 admits everything version 1 does; admission is
  monotone in the version; on any root not spelled `"."` the two versions decide
  *identically*; the checkout root is admissible at version 2 and refused at
  version 1; over the corpus, exactly one spelling separates the versions.
- **No escape.** Walking any admissible root from the checkout never steps
  above it and never reaches a filesystem root — so admitting `"."` opens no
  traversal. The checkout-root spelling resolves to the checkout *itself*
  (empty location), while an ordinary root resolves strictly below it.
- **Refusals.** Every traversal/absolute/empty spelling is refused at both
  supported versions, over an inhabited (7-member) corpus; and the refusal is
  recorded as deliberately conservative rather than minimal (`"./src"` would
  not escape and is refused anyway, because admission tests the literal).
- **Version gating.** An unsupported version admits nothing; a higher version
  number buys nothing; both supported versions load an ordinary root; a
  version-1 profile decides identically at both versions.

**Lean judged applicable for F1 and not applicable for F2.** F2's content is
the per-flag preservation argument, which is a claim about what a specific
Clang build does to a specific token stream — measurement-backed prose per the
existing convention (`w2-preprocessor-transport`,
`macos-cycle19-compiler-options`), not a checkable relation. Modelling it in
Lean would restate the transcription without adding a decidable property.

What is **not** proved, and stays a Rust boundary: that Rust's
`Path::components` produces the component lists the module transcribes (pinned
behaviourally by `crates/qcore-audit/tests/root_source_set.rs`), and that the
other declared-path validators keep their own rules (they are unchanged).

---

## 5. Unchanged routes, verified by measurement

Both canonical audits were run twice from the same prepared build tree — once
with this branch's `crates/` and `compiler/`, once with base commit `5589839`'s
sources restored into the same worktree — and are **byte-identical**:

| Artifact | SHA-256 (base `5589839` and this branch) |
|---|---|
| `cmetrics-audit.json` | `fea7c0455056679f857060f5d2bf017c2f474e8defaf9234b2abbff259767462` |
| `cmetrics-audit.assumptions.json` | `5ca7062ed1fa9712a64e41247e3fbe16b8a8d000759db650ecc14585f31e472c` |
| `chunkio-core-audit.json` | `a686fb6f9acb9c2f688191abd685b2e9f232ed73e448406ff72f1dadb16f332c` |
| `chunkio-core-audit.assumptions.json` | `74947a48bccde97f5953411cfefd865b224ad15079dd50584e467082a3213f9b` |

Both keep their recorded results: CMetrics `1 checked function contract /
6 Unknown` with `1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown`;
`chunkio-core` `13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown`.

The pinned Cycle 19 route is untouched **by construction**: the whole diff to
`compiler/examples/support/cycle19_compatibility.rs` is one line replacing
`root.join(&selection.root)` with `source_root_path(&root, &selection.root)`,
which is the identity for every root the `fluent-bit` profile declares (that
profile is version 1, so `"."` cannot appear in it), and
`compiler_options_compatibility` — the historical standard-input projection the
pinned route uses — is not touched at all. The four archived 185-unit artifacts
are therefore unchanged, and were not re-measured.

---

## 6. Deliberate pin updates

Four pinned expectations conflicted with this change. Each was updated
deliberately, keeping its load-bearing negative half:

1. `compiler/tests/profile_loader.rs`,
   `the_shipped_profiles_load_and_are_internally_consistent` — the shipped
   profiles were asserted to declare `PROFILE_MANIFEST_VERSION`. They stay at
   version 1 (byte-identical), so the assertion is now: declared version is
   exactly `1`, and that version is in the supported set. The supported-set
   pin moves `[1]` → `[1, 2]`, plus a new pin that
   `PROFILE_MANIFEST_VERSION == 2`.
2. `compiler/tests/profile_loader.rs`,
   `f3_unknown_schema_version_rejects_the_whole_profile` — the probe moves
   from `2` (now supported) to `3`. The refusal, and the verbatim reporting
   of the supported set, are unchanged.
3. `crates/qcore-audit/tests/second_source_set.rs`,
   `a_compiler_option_the_gateway_cannot_preserve_rejects_the_database` — the
   unsupported-option probe moves from `-O2` (now transported) to
   `-ffast-math`, which defines `__FAST_MATH__`, changes floating-point
   semantics and has no preservation argument here. The probe history
   (`-Wextra` → `-O2` → `-ffast-math`) is recorded in the test.
4. `compiler/tests/composed_five_providers.rs` — installing
   `contract/profiles/libevent-core/` adds a row to the per-profile provider
   map. It binds no provider, so the row is empty; the load-bearing fact ("no
   single profile declares the whole five-provider composition") is preserved
   and additionally pinned as a structural assertion over every profile.

No fail-closed gate was weakened, no diagnostic was downgraded and no
diagnostic code was added or changed.

---

## 7. Refusal matrix (F2)

`crates/qcore-audit/tests/root_source_set.rs`, all writing no report:

| Case | Result |
|---|---|
| all 17 added options plus `-O3` present | accepted; every one appears in the recorded `preprocessor_options` and in the complete `arguments` vector of all 3 compile tasks |
| `-ffast-math` | rejected, naming the option |
| `-Ofast` | rejected, naming the option |
| `-O` (bare) | rejected, naming the option |
| `-fvisibility=hidden` | rejected, naming the option |
| `-flto` | rejected, naming the option |
| multi-target database, no declared target | rejected: `conflicting compilation commands for …` |
| declared target, second record differing by `-DVARIANT=1` | rejected: `conflicting compilation commands for …` |
| `object_prefix` selecting nothing | rejected |
| `object_prefix` = `-o` / `$TARGET/` / whitespace | rejected |

---

## 8. Gate

Run in this worktree at the branch tip.

| Section | Result |
|---|---|
| `cargo test --workspace` | **1662 passed, 0 failed, 1 ignored** (1651 at the wave-6 checkpoint; +10 new `root_source_set` tests, +1 restructured) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **39 audits** passed (38 before) |
| provider workspace `cargo test` | **58 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **108 tests, OK** |
| `providers/package.py build` | all five packages built and verified |
| `libevent-core` audit | 21 units: 0 Safe / 4 provisional Unsafe / 17 Unknown / 0 data-only / 0 gateway failures |
| `cmetrics-counter` audit | 1 checked / 6 Unknown; 1 unit provisional Unsafe; artifacts byte-identical to base |
| `chunkio-core` audit | 13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown; artifacts byte-identical to base |

One pre-existing finding, untouched by this card and reported rather than
fixed: `cargo fmt --all -- --check` in the **providers** workspace reports one
diff in `providers/conformance/tests/packages.rs:239` (a line-wrap in a file
last changed by P01-F2, commit `1b3d64d`). `git diff` shows no change from this
card under `providers/`, so this predates the branch; the providers Clippy run
is clean.

---

## 9. Open boundaries

- **Not every Clang option is transported.** `frontend_flag` is an exact-match
  allowlist of 28 spellings (21 named options plus the 7 `-O` levels), beside
  the two pre-existing parameterised forms `-Wundef-prefix=<prefix>` and
  `-arch <architecture>`. `-Ofast`, bare `-O`, `-flto`,
  `-fvisibility=hidden`, `-Werror`, sanitizer flags, `-march`/`-mtune`,
  `--sysroot` and everything else stay refused, by design. Each future
  addition needs its own measurement.
- **`-Os`/`-Oz` were measured on one file.** Their preprocessed text was
  identical there, but their macro set is not; they are transported for the
  same reason as the other levels, and their `-E -P` equality is not claimed
  generally.
- **Declared-target selection is a narrowing, not a resolution of G01.** The
  pinned Cycle 19 compatibility adapter still chooses silently among
  configurations for `src/flb_input_chunk.c` and 27 Monkey/libevent units;
  that route is untouched, and G01's census stands.
- **No compiler equivalence.** Reproducing the option vector is not a claim
  that GCC and Clang agree, nor a claim about code generation, object layout or
  link behaviour. The `event_core_static` native build is provenance, not an
  object-code equivalence claim.
- **The four provisional `Unsafe` Libevent units are unpromoted and
  unattributed.** Their `leak` diagnostics are bounded-model residuals under
  declared `event_mm_*` contracts, which are themselves declared trust.
- **F1's component transcription is a premise.** The Lean model assumes Rust's
  `Path::components` yields the transcribed lists; that is pinned by test, not
  extracted.
- **Function-level `reason_code` remains absent** on the resource-projection
  route (`REASON-CODES-B`), unchanged here.
- **Windows/macOS** were not exercised for the new options; the added flags are
  GNU-style spellings a Clang C driver accepts, and the hermetic fixture uses
  the gateway-selected Clang, but no non-Linux run is claimed.
