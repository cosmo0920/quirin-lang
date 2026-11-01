# PACKAGE-BYTES-DEPEND-ON-CHECKOUT

Card of 2026-11-01, base `c82895503`. It discharges the follow-up
`PACKAGE-BYTES-DEPEND-ON-CHECKOUT`. Now `providers/package.py build` gives the
same bytes from any checkout path, `CARGO_HOME`, target directory or toolchain
location, and `package.py reproduce` checks this by building twice. The user
accepted on 2026-11-01 that all five package digests change once.

## 1. Cause (from CI-RUST-JOB-PEAK) and mechanism

[CI-RUST-JOB-PEAK §6](../ci-rust-job-peak/README.md) found two causes:

1. Cargo's unit hash (`-C metadata`) covers a path package's path relative to
   the workspace root. When the package lies outside that root, the hash covers
   the absolute path instead. The provider workspace's root is `providers/`,
   and it takes `qcore-schemas` from `../crates/qcore-schemas`. So the
   disambiguators of `qcore_schemas`, `quirin_provider_shared` and every
   provider binary depended on the checkout path, and with them the symbols and
   the code layout.
2. 14 `~/.cargo/registry` and `~/.rustup/.../rust-src` paths per executable
   were never remapped.

Chosen mechanism: **the release build compiles a staged workspace whose root
contains every path package, and every absolute path left is remapped to a
constant.** `build` writes `<target>/qcore-package-source/`:

- `Cargo.toml`: `providers/Cargo.toml` rebased one level up. Members become
  `providers/<member>` and workspace path dependencies are rebased the same
  way. Every non-member path package is listed in `exclude`, so it stays a
  dependency and does not become an automatic member (an automatic member's
  dev-dependencies would change the lockfile). A manifest key the rebaser does
  not know, a glob member, or a path that leaves the repository is refused.
  The generated text must parse back to the rebased data.
- `Cargo.lock`: a byte copy. Path packages carry no path in the lockfile, so
  `--locked` holds, and the build refuses a lockfile that changed.
- Every path package from `cargo metadata` (the 13 members and 16 crates under
  `crates/`), copied to its repository-relative path. Unchanged files keep
  their mtime, so a reused target rebuilds only what changed. Size: 38 MB.

Remaps, with the last match winning in rustc: `rust-src` to
`/rustc/<commit-hash>` (the path a toolchain without `rust-src` embeds),
`CARGO_HOME` to `/cargo`, the target to `/target`, and the stage to `/qcore`.
Source paths therefore still read `/qcore/providers/...` and
`/qcore/crates/...`. After the build, `build` refuses an executable that
contains any of these paths: the repository, the stage, the target,
`CARGO_HOME`, the sysroot or the home directory, each in its given and resolved
form. The release profile, flags, `-j 2`, `SOURCE_DATE_EPOCH` and the archive
format are unchanged.

Rejected alternatives:

- **A symlink `providers/qcore-schemas` (or moving the crate under
  `providers/`).** `conformance` reaches the analysis crates, which name
  `crates/qcore-schemas` themselves. Two path sources for one
  `qcore-schemas 24.0.0` would collide in one lockfile. Moving the crate would
  also renumber references across the root workspace.
- **A constant absolute build path** (for example `/tmp/...`). Concurrent
  cards would collide on it, and it needs locking.
- **Stripping symbols.** This leaves the layout differences in place
  (CI-RUST-JOB-PEAK).
- **Stable cargo options.** Cargo has none that hashes a path package relative
  to another root, and `-Ztrim-paths` is unstable.

## 2. The check

`python3 providers/package.py reproduce --output DIR [--target-dir T]
[--work-dir W]` works in four steps:

1. It runs `build` into `DIR`.
2. It copies the checkout's tracked and untracked, not ignored, files to
   `W/second-checkout`. `docs/` (1.4 GB, read by no build) and the submodule
   are left out. Whatever else is missing fails the second build; it cannot
   pass it.
3. It populates `W/second-cargo-home` with only the locked registry
   `.crate` files and index entries, by hard link or copy, plus any
   `config.toml`. A git or non-crates.io source is refused.
4. It runs that copy's own `package.py build` with
   `CARGO_HOME=W/second-cargo-home` and a fresh target, and compares every
   output file byte for byte, `packages.json` and the archives included. The
   second build skips only the checks that write nothing: `contracts.py check`
   and per-package `verify`. Both build root-workspace probes from scratch in
   a new checkout, and the first build ran both on bytes the second must equal.

After a match, `reproduce` removes `W` and the first stage. After a difference
it keeps them and names the differing files. CI's package step now runs
`reproduce` in place of `build`, then the same five `verify` calls.

`scripts/tests/test_package_reproducible.py` (9 tests) covers the parts
without cargo:

- the rebasing of the real manifest and its refusals;
- the TOML round trip;
- mtime-preserving sync and stale-file removal;
- `target` skipping;
- remap order;
- the embedded-path scan;
- the index layout;
- the locked-only `CARGO_HOME` copy and its git-source refusal.

## 3. Evidence

All runs used rustc 1.99.0 (`b940084d7`), with the outputs and logs under
`private/pkg/`.

- **Tip reproduces.** `reproduce` on the code commit wrote
  `reproduced 112 files byte for byte from a second checkout path and CARGO_HOME`
  (`evidence/reproduce-tip.txt`). The second checkout was
  `private/pkg/tip-out.reproduce/second-checkout`, a different path and depth
  from the worktree. Its `CARGO_HOME` was `.../second-cargo-home` instead of
  `~/.cargo`. The final run, on the commit the gate measured, is in §5.
- **The check fails when the bytes depend on the checkout.** A negative
  control (`evidence/negative-control.diff`) built from `providers/Cargo.toml`
  again, with the old `--remap-path-prefix=<repo>=/qcore` added, so the
  embedded-path scan passes. `reproduce` then exited 1 and named all 16
  byte-carrying files as differing (`evidence/negative-control.txt`). The file
  was then restored from the commit's bytes and touched, and the restore was
  checked (`MUTATION` absent, `git diff` empty).
- **No host path remains.** The scan passed in all four builds. The paths that
  remain in an executable are `/cargo/registry/src/...`,
  `/rustc/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/...` and
  `/qcore/...`.

### Reviewed digest diff

Base `c82895503` against the tip, both built by `package.py build` on this host
(`evidence/package-digests.txt`, `digest_diff.py`). Of the 112 files, 16
differ: in each package, `provider`, `verifier-plugin.json` and the `.tar.gz`,
plus `packages.json`. Each `verifier-plugin.json` differs in exactly one line,
`package_digest`. The contracts, `contracts.json`, `rustc` and `lock_sha256`
are equal.

| Package | Executable SHA-256, base -> tip |
|---|---|
| qcore.chunkio | `2884cde3…` -> `401df12f…` |
| qcore.cmetrics | `79d24c52…` -> `07db6db6…` |
| qcore.ctraces | `89b611f5…` -> `3a994034…` |
| qcore.libevent | `63e9a623…` -> `ad0eaeea…` |
| qcore.librdkafka | `c1a4621c…` -> `530d117e…` |

The one reason for all five: the executables are now compiled from the staged
workspace with constant remaps. Every path package's unit hash is taken over
its relative path under the stage root (`providers/<member>`,
`crates/qcore-schemas`, where it used to be an absolute path or a path relative
to `providers/`). The registry and `rust-src` locations read `/cargo/...` and
`/rustc/<hash>/...` instead of host paths. So each executable's symbols, layout
and panic-location strings change once, and each package digest, manifest,
archive and `packages.json` follow from that.

**Pins and published artifacts.** Nothing in the tree pins a real package's
executable digest:

- The profile manifests under `contract/profiles/*/providers/` say
  `builtin:<id>`.
- The pinned corpus and `quirin-audit measure` read the frozen mirror
  `provider-packages-wave20`.
- `build/cycle19/provider-packages` is a local, uncommitted build.

So no lock is regenerated and no published audit artifact changes, and no
`measure` identity run is needed. The targeted diff is the table above. The
`contract-inventory.lock` files pin contract texts, which are unchanged.

### CI storage and time

These are measured from this host's runs, not from a new whole-job run. The
CI step adds the following during the package step only:

- the second checkout (58 MB);
- the second `CARGO_HOME` (26 MB);
- the second target (145 MB);
- the second output (21 MB);
- the first stage (38 MB).

All of it is removed before the step ends. The release target the provider
suite runs beside is 113.18 MB. CI-RUST-JOB-PEAK measured 113.26 MB for it, so
it is unchanged. At the package step the job held 6.9 GB
(CI-RUST-JOB-PEAK `evidence/after/steps.tsv`), so the transient 0.29 GB brings
it to about 7.2 GB. The peak (10.69 GB, during the provider suite) is not
raised. The step's wall time grows by one release build of the five providers
at `-j 2` (§5).

## 4. Scope

There is no analyzer, contract, verdict or Lean change; no semantic law moves.
The docs were updated in `providers/README.md` ("Build, verify and reproduce")
and `docs/agents/validation.md` ("Test build storage", the CI-job bullet). The
gate's provider step still runs `package.py build`.

Limits:

- The check varies the checkout path and `CARGO_HOME`. It does not vary the
  toolchain location. The sysroot remap and the scan cover that location, but
  no run moved the toolchain.
- Cross-toolchain and cross-platform identity are still not claimed.
- `reproduce` needs `git` in the checkout.

No new follow-up.

## 5. Final reproduce run and gate

`reproduce` on `6fa825531` with fresh targets
(`evidence/reproduce-final.txt`): exit 0, 112 files byte for byte, 210.7 s of
wall time for both builds and the contracts check (probes already built).
The output is identical (`diff -r`) to the earlier tip run's. Afterwards the
target held only `release/` (113.19 MB) and the work directory was gone.

### Card gate

The gate ran `python3 scripts/gate_scope.py --base c82895503` on `170347106`
and exited 0 (`gate.json`, `gate-console.txt`). It ran in full mode, because
`providers/README.md` is named by a string literal in `qcore-contract` tests.
Results:

- submodules 0 (one);
- fmt 0;
- Clippy 0;
- Rust suite 0: 585 units, 618 summaries, 8,951 passed, 0 failed, 15 ignored;
- both reevaluation examples 0;
- Python 0, 577 tests;
- provider package build 0, all five verified 0;
- provider tests 0: 133 / 0 / 1;
- provider fmt 0 and provider Clippy 0.

Lean: none.

The gate's package build in a third target directory gave output identical to
the tip run's (`diff -r`).

An earlier gate run on `bb98dbc30` failed only the Python suite. Rule 29's
tripwire `test_tracked_files` flagged the two `rglob` walks of the new test's
temporary directories. Commit `170347106` reviews them in `REVIEWED_WALKS`.
