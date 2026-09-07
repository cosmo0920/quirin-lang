# P01 — Independent provider packages and layering audit

Work card P01 (W5/W0, `verify_or_extend`), 2026-09-07. Baseline checkpoint
`6c2782f`. This is an audit of the *existing* standalone provider packaging as
the reference provider implementation, plus three small guard tests that close
gaps the audit found. No provider, host, protocol, CLI or packaging behavior is
changed; no gate is weakened and no verdict, contract-corpus pin or CT mapping
moves.

Scope of the claims below: dependency direction, the build → verify →
enable → disable → rollback lifecycle of an installed package, tamper
rejection, and byte-level build reproducibility on one host. Nothing here is a
statement about provider *semantics*: all five shipped packages remain
protocol-v1 `Unknown` providers, and the ChunkIO/CMetrics source-field results
still require trusted replay in `qcore-verification` before any obligation is
discharged.

**Process separation is not an OS sandbox.** The manifest's `limits`,
`capabilities`, `filesystem_inputs` and `network_access` fields are
*declarations* consumed by the host, not enforcement mechanisms. The
process-host containment is its existing Unix process-group, deadline and
output-size behavior. This report does not claim otherwise, and finding F2
below turns on exactly that distinction.

## 1. Dependency matrix

### 1.1 Core workspace → providers

The core workspace (`Cargo.toml`) declares `exclude = ["providers"]` and lists
no member under `providers/`. `cargo metadata --offline` over the *resolved*
core graph returns these packages:

```
cc, find-msvc-tools, itoa, libc, memchr, proc-macro2, qcore-audit,
qcore-c-gateway, qcore-compiler, qcore-contract, qcore-core, qcore-frontend,
qcore-schemas, qcore-verification, qcore-verifier-host, qcorec, quote, serde,
serde_core, serde_derive, serde_json, shlex, syn, unicode-ident, zmij
```

No `qcore-provider-*` package appears anywhere in the resolved graph, at any
dependency kind. Per-member declared dependencies:

| Core member | Workspace dependencies |
|---|---|
| `crates/qcore-schemas` (2.0.0) | *(none; serde/serde_json only)* |
| `crates/qcore-core` | *(none; `cc` dev-dependency only)* |
| `crates/qcore-frontend` | `qcore-core` |
| `crates/qcore-contract` | `qcore-core`, `qcore-frontend`, `qcore-schemas` |
| `crates/qcore-c-gateway` | `qcore-contract`, `qcore-schemas` |
| `crates/qcore-verifier-host` | `qcore-frontend`, `qcore-schemas` |
| `crates/qcore-verification` | gateway, core, frontend, contract, schemas, host |
| `crates/qcore-audit` | gateway, contract, schemas, verification, host |
| `compiler` | all extracted layers (+ `qcore-audit` as a dev-dependency) |
| `crates/qcorec` | `qcore-compiler` |

**Empirical independence check.** With the entire `providers/` directory moved
out of the worktree, the core workspace still builds and passes its complete
suite: 64 suites, **821 tests, 0 failures** — identical to the run with
`providers/` present. The directory was restored afterwards and
`git status` is clean apart from the three test files listed in §5.

### 1.2 Provider crates → core

`cargo metadata --manifest-path providers/Cargo.toml --no-deps --offline`:

| Provider crate | Version | Normal dependencies | Dev dependencies |
|---|---|---|---|
| `qcore-provider-chunkio` | 0.4.0 | `qcore-schemas` (path), `serde`, `serde_json` | — |
| `qcore-provider-cmetrics` | 0.2.0 | `qcore-schemas` (path), `serde`, `serde_json` | — |
| `qcore-provider-ctraces` | 0.1.0 | `qcore-schemas` (path), `serde`, `serde_json` | — |
| `qcore-provider-libevent` | 0.1.0 | `qcore-schemas` (path), `serde`, `serde_json` | — |
| `qcore-provider-librdkafka` | 0.1.0 | `qcore-schemas` (path), `serde`, `serde_json` | — |
| `qcore-provider-conformance` | 0.1.0 | `qcore-schemas` (path), `serde`, `serde_json` | `qcore-audit`, `qcore-c-gateway`, `qcore-contract`, `qcore-core`, `qcore-verification`, `qcore-verifier-host` |

No artifact crate names a core crate other than the admitted published schema
surface `qcore-schemas`, which itself has no workspace-path dependency. The
conformance crate is the *test harness*: it links the real host and gateway
only through dev-dependencies, so its schemas-only `src/bin` process fixtures
(`sample_boolean_provider`, `configured_audit_provider`,
`callback_proposal_writer`) cannot link core internals, and it carries no
`provider-template.json`, so `package.py` never packages it.

**Verdict: layering is clean in both directions.** No finding.

The artifact-side rule was already pinned by
`providers/conformance/tests/packages.rs::artifact_dependencies_are_only_schemas_and_serialization`.
The core-side rule and the harness/packaging rules were unpinned; §5 adds them.

## 2. Lifecycle: build → verify → enable → disable → rollback

All commands were run against a freshly built package set in a scratch
directory. Tampering was performed on **copies**; no built package or repository
file was mutated in place.

### 2.1 Build

```sh
python3 -B providers/package.py build --output $S/pkgA --target-dir $S/targetA
```

Produces five package directories, five deterministic `.tar.gz` archives and
`packages.json` recording, per artifact, the executable, manifest and archive
SHA-256, plus the `rustc -vV` identity and the provider lockfile hash. CMetrics
0.2.0 executable digest for this host/toolchain:
`614d19bc5623f1c8ff518f3c897fb36d608c878167b66a8fe2e1d77f6861fa82`.

Re-running `build` with an existing `--output` fails closed:
`ValueError: output already exists: …` (exit 1).

### 2.2 Transcript

| # | Command | Exit | Result |
|---|---|---|---|
| 01 | `package.py verify pkgA/qcore.cmetrics` | 0 | manifest printed; `--verify-package` self-check passed |
| 02 | `qcorec plugins verify pkgA/qcore.cmetrics` | 0 | manifest printed |
| 03 | `qcorec plugins enable pkgA/qcore.cmetrics --config plugins.json` | 0 | `enabled package in …` |
| 04 | `qcorec plugins enable pkgA/qcore.chunkio --config plugins.json` | 0 | second selection recorded |
| 05 | `qcorec plugins list --config plugins.json` | 0 | `qcore.chunkio 0.4.0 …` then `qcore.cmetrics 0.2.0 resource_contracts` |
| 06 | `qcorec plugins list` (repeat) | 0 | byte-identical to #05 |
| 07 | same two packages enabled in reverse order into a second config, then `list` | 0 | byte-identical to #05 |
| 08 | `qcorec plugins inspect qcore.cmetrics --config plugins.json` | 0 | pinned manifest, digest `614d19bc…` |
| 09 | `qcorec check --plugins plugins.json examples/cycle12.qc` | 0 | `checked 1 function(s); entry main` |
| 10 | `qcorec plugins disable qcore.cmetrics --config plugins.json` | 0 | `disabled qcore.cmetrics in …` |
| 11 | `qcorec plugins list --config plugins.json` | 0 | only `qcore.chunkio` remains |
| 12 | `qcorec check --plugins plugins.json …` after disable | 0 | disabled entry never enters the registry |

**Hash retention.** `plugins enable` pins the *complete* validated manifest —
`package_digest` included — under the plugin ID in the selection file, and
`plugin_config::registry` re-verifies the package on every load and compares the
whole manifest against the pinned copy. `packages.json` independently retains
executable/manifest/archive digests from the build record.

**Discovery determinism.** `enable` sorts selections by plugin ID before
writing, so the listing is identical across repeated queries (#06) and
independent of the order in which packages were selected (#07). §5 pins this.

**Rollback.** Disable is a local edit of the selection file only; the package
directory is untouched, and a disabled entry may remain configured after its
directory is removed (#11–#12 and #17–#19 below). Re-`enable` of a reviewed
directory is the upgrade/rollback decision; there is no default fallback.

### 2.3 Post-enable drift and missing packages

| # | Situation | Command | Exit | Message |
|---|---|---|---|---|
| 13 | manifest changed after enable | `qcorec check --plugins …` | 1 | ``plugin `qcore.cmetrics` changed since enable; enable the reviewed package again`` |
| 14 | executable changed after enable | `qcorec check --plugins …` | 1 | `executable digest mismatch` |
| 17 | package directory removed after enable | `qcorec check --plugins …` | 1 | `No such file or directory (os error 2)` |
| 18 | `qcorec plugins disable qcore.cmetrics --config …` on the removed package | 0 | rollback succeeds without the package present |
| 19 | `qcorec check --plugins …` after that disable | 0 | compiles with no providers selected |

All failures are fail-closed: no default registry substitution, and the
selection file is never rewritten by a failed operation.

## 3. Tamper rejection

Each case is a fresh copy of `pkgA/qcore.cmetrics`.

| Tamper | `qcorec plugins verify` | `qcorec plugins enable` | `package.py verify` |
|---|---|---|---|
| A. one byte flipped in `provider` | **exit 1** `executable digest mismatch` | **exit 1** `executable digest mismatch` | **exit 1** `executable digest mismatch` |
| B. `package_digest` first hex character changed in the manifest | **exit 1** `executable digest mismatch` | **exit 1** `executable digest mismatch` | **exit 1** `executable digest mismatch` |
| C. manifest `network_access` set to `true` and `limits.timeout_ms` widened | **exit 1** `verifier plugins may not request network access` | **exit 1** same | **exit 1** self-check failed |
| D. manifest `limits.timeout_ms`/`memory_bytes` widened only (schema-valid, digest intact) | **exit 0 — ACCEPTED** | **exit 0 — ACCEPTED** | **exit 1** `installed manifest differs from the executable's package identity, scope, versions or digest` |
| E. manifest `targets[0].library` changed `cmetrics` → `chunkio` (schema-valid, digest intact) | **exit 0 — ACCEPTED** | **exit 0 — ACCEPTED** | **exit 1** same self-check failure |

Cases A–C reject at every stage, which is the required behavior for both
tampering forms the card names (executable byte, manifest byte). Cases D and E
are the audit's substantive result and are recorded as finding **F2**.

Independently, the existing suites already pin the process-level equivalents:
`providers/conformance/tests/packages.rs::real_packages_preserve_unknown_and_reject_mutations`
rejects four manifest mutations and an appended executable byte at dispatch,
and `crates/qcorec/tests/plugin_packages.rs::package_tampering_never_updates_the_selection_file`
pins that a digest mismatch never rewrites the selection file.

## 4. Offline reproducibility

### 4.1 Cache preparation

There is no `.cargo/config.toml` and no vendored source tree; `--locked
--offline` resolves entirely from the shared `$CARGO_HOME` registry cache
(`~/.cargo/registry`). Preparation therefore requires one prior *online*
`cargo fetch --locked` per workspace, after which every later invocation is
offline. The two lockfiles require:

- core `Cargo.lock`: 25 packages, 15 from the registry (`cc`, `find-msvc-tools`,
  `itoa`, `libc`, `memchr`, `proc-macro2`, `quote`, `serde`, `serde_core`,
  `serde_derive`, `serde_json`, `shlex`, `syn`, `unicode-ident`, `zmij`).
- `providers/Cargo.lock`: 26 packages, 12 from the registry — the same set minus
  `cc`, `shlex` and `find-msvc-tools`, which enter the core graph only through
  `qcore-core`'s `cc` **dev**-dependency and so do not propagate to the provider
  workspace.

The provider workspace checks used in this audit all ran `--locked --offline`
from that prepared cache with no network access:
`cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline`,
the matching `clippy` invocation, and `package.py build` (which passes
`--locked --offline` to `cargo build` itself).

### 4.2 Two clean builds

The w5 reports claim two clean reproducible builds. **Reproduced.** Two builds
into separate output directories with separate, initially empty target
directories:

```sh
python3 -B providers/package.py build --output $S/pkgA --target-dir $S/targetA
python3 -B providers/package.py build --output $S/pkgB --target-dir $S/targetB
```

`packages.json` from the two runs is **identical**, including the `rustc`
identity string and `lock_sha256`. Per artifact:

| Plugin ID | executable SHA-256 | manifest | archive |
|---|---|---|---|
| `qcore.chunkio` | `11599e9b…` equal | equal | equal |
| `qcore.cmetrics` | `614d19bc…` equal | equal | equal |
| `qcore.ctraces` | `67575edf…` equal | equal | equal |
| `qcore.libevent` | `5607da32…` equal | equal | equal |
| `qcore.librdkafka` | `1edebf10…` equal | equal | equal |

Reproducibility here is relative to this recorded source tree, lockfile,
toolchain (`rustc 1.98.0 (88d9e12ae 2026-08-18)`, `x86_64-unknown-linux-gnu`)
and target directory. The builder achieves it with release mode, a separate
target directory, `CARGO_INCREMENTAL=0`, `SOURCE_DATE_EPOCH=0`, dropped ambient
`RUSTFLAGS`, `--remap-path-prefix=<repo>=/qcore`, `-j 2`, and deterministic
USTAR/gzip metadata (`mtime=0`, fixed modes, sorted entries). **No
cross-toolchain, cross-platform or cross-host byte identity is claimed or
tested.**

## 5. Gaps closed with tests

Three guards were missing. Each new test pins a *rejection* or an invariant that
already holds; none pins a weakness as desirable, and no existing expectation
was relaxed.

1. `compiler/tests/workspace_layering.rs::core_workspace_does_not_depend_on_provider_implementations`
   — asserts the core workspace excludes `providers`, that no member path lies
   under `providers/`, and that no member manifest names a `qcore-provider-*`
   crate or a `providers/` path in any dependency table.
2. `crates/qcorec/tests/plugin_packages.rs::discovery_order_is_independent_of_selection_order`
   — three packages enabled in two different orders into two selection files
   produce byte-identical `plugins list` output and byte-identical selection
   files, and a repeated query is stable.
3. `providers/conformance/tests/packages.rs` — the existing artifact-dependency
   guard is extended so the conformance harness's core-crate dependencies must
   be **dev**-only (its normal dependencies stay schemas-only, which is what its
   `src/bin` process fixtures link); and a new
   `only_template_carrying_crates_are_packaged` asserts that exactly the five
   `qcore-provider-*` crates carry `provider-template.json` and that the
   conformance crate carries none.

## 6. Findings

| ID | Severity | Finding |
|---|---|---|
| F1 | none | Core↔provider layering is clean in both directions (§1). Recorded as a positive result, now pinned by test 1 above. |
| F2 | low | The CLI package lifecycle does not bind non-digest manifest declarations to the executable's embedded template (§3, cases D/E). |
| F3 | informational | `providers/package.py` reports verification failures as an uncaught Python traceback. |

### F2 in detail

`qcorec plugins verify` and `qcorec plugins enable` accept a package whose
manifest declaration fields were edited, as long as the edit stays schema-valid
and `package_digest` still matches the executable bytes. Observed accepted
edits: widened `limits.timeout_ms`/`limits.memory_bytes`, and
`targets[0].library` changed from `cmetrics` to `chunkio`. Only
`package.py verify` — which additionally *executes* `provider --verify-package`
— or the binary's own `--verify-package` mode detects it, reporting `installed
manifest differs from the executable's package identity, scope, versions or
digest` (exit 2).

This is a **documented boundary, not a regression**: `providers/README.md`
already states that the CLI verifier "is read-only and does not execute the
package" and "establishes a snapshot identity, not a signature, runtime
compatibility, embedded-template binding or source authenticity". The audit's
contribution is to demonstrate the exact edits that pass and to name the
consequence.

Impact bound — this is **not** a `Safe`-admission path:

- `limits` are host-side declarations, not enforcement (see the sandbox note
  above), so widening them changes the deadline/output budget the host applies,
  not any containment guarantee that existed.
- A widened `targets` entry can cause the host to route a request to a provider
  whose binary does not claim that library; the binary still rejects requests
  whose selected identity does not match its embedded manifest, the core still
  **refuses every protocol-v1 `Safe` proposal**, unchecked `Unsafe` proposals
  still downgrade to `Unknown` (`QP010`), and any v2 source-field witness still
  requires independent trusted replay before it can discharge an obligation.

No fix is applied here: making `enable` execute the package would change
documented CLI behavior (the read-only verifier is a deliberate design choice)
and is outside this card's scope. Two candidate repairs for a future card:
require the `--verify-package` self-check at `enable`, or record the manifest
SHA-256 in `packages.json` and check the installed manifest against a retained
build record. Until then, the operational rule in `providers/README.md` stands:
run `package.py verify` on an installed package and retain the archive hash from
a trusted build record.

### F3 in detail

`package.py verify` exits nonzero on every failure — fail-closed holds — but
surfaces `ValueError` / `CalledProcessError` tracebacks rather than a
diagnostic. Ergonomics only; not repaired here to keep this card's diff to test
additions.

## 7. Gate results

Run at the P01 change (three test files touched, no production code):

| Gate | Result |
|---|---|
| `cargo test --workspace` | **821 passed**, 0 failed (64 suites; baseline 819 + 2 new) |
| `cargo test --workspace` with `providers/` absent | **821 passed**, 0 failed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **46 passed**, 0 failed (16 suites; baseline 45 + 1 new) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `python3 -m unittest discover -s scripts/tests` | **43 passed** |
| `cd proof && lake build` | 62 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | **20 Lean audits passed** |

No production code changed, so no audit artifact, verdict, contract-corpus
count, CT mapping or Lean model is affected by this card. The Cycle 19 raw units
remain unpromoted: 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units. CT coverage remains 11/50 mapped.

## 8. Open items this card does not close

- F2's embedded-template binding at `enable` time (candidate repair above).
- Signature verification, source authenticity and immutable-execution identity
  for installed packages: still absent by design; verification remains a
  snapshot check.
- Enforcement of declared `limits`, `capabilities` and `filesystem_inputs`
  (cards P02/P03), and any OS-sandbox claim.
- Cross-toolchain / cross-platform reproducibility.
- Specialized provider semantics: all five shipped packages remain v1
  `Unknown`, and no raw production unit is promoted by anything in this report.
