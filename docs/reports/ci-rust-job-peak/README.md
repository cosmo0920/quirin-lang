# CI-RUST-JOB-PEAK: the Linux CI job measured as one sequence

Card CI-RUST-JOB-PEAK, 2026-10-31. Base: origin/main `db2795100`. The user
asked for CI's test runs to fit about 15 GB of attached storage (2026-10-29).
[TEST-STORAGE-BUDGET](../test-storage-budget/README.md) measured the parts of
the Linux `rust` job separately and estimated the whole job at about
10.5–11 GB. This card measures the whole job in one sequence, before and after
its change, and looks at the two related follow-ups. It changes no analysis
code, contract, verdict or Lean module.

## 1. Outcome

Peak disk is the largest sum, sampled every 5 s, of the fresh checkout (its
`target/` and `providers/target/` included) and the job's temporary
directories (`TMPDIR`, `RUNNER_TEMP`, an empty `CARGO_HOME`). Both runs:
fresh shallow clone, `CARGO_BUILD_JOBS=6`, the `rust-suite` slot held for the
whole sequence.

| Run | Peak | of which checkout | Root `target/` at end | `providers/target/` at end | Wall | Root suite | Provider suite |
|---|---:|---:|---:|---:|---:|---|---|
| before (`db2795100`) | **11.04 GB** | 2.60 GB | 4.51 GB | 3.75 GB | 4726 s | 8,927 / 0 / 15 | 133 / 0 / 1 |
| after (card tip) | **10.69 GB** | 2.60 GB | 4.17 GB | 3.75 GB | 4312 s | 8,927 / 0 / 15 | 133 / 0 / 1 |

Results are passed / failed / ignored over cargo's `test result:` lines. Every
step exited 0 in both runs. The peak comes during the provider workspace's
tests, the job's last step, when both workspaces' targets exist together.

- **The whole job peaks at 10.7 GB after the card (11.0 GB before), under the
  15 GB budget**, so task 3 (deleting a finished target between steps) is not
  needed and was not done. The checkout is 2.6 GB of it (0.94 GB of that the
  shallow `.git`), the two targets 7.9 GB, temporary files 0.17 GB.
- **TEST-STORAGE-BUDGET's estimate was close for the total, wrong for one
  part.** The package step's plain `cargo run` probes did not rebuild 1.3 GB
  in the sequence; they rebuilt 0.17 GB (§3).
- **After:** the steps that rebuilt root-workspace crates beside the test
  build (the quota step, the package step's probes) now reuse it. The quota step compiled nothing and the package step wrote
  nothing to the root target, so that target ends at 4.17 GB instead of 4.51
  and the peak is 0.35 GB lower. The executed tests are equal
  (`compare_tests.py` of TEST-STORAGE-BUDGET over the two runs' logs: 8,942
  root tests with the same 15 ignored, 134 provider tests with the same 1
  ignored, none failed; `evidence/parity-*.txt`).

Per step (`evidence/before/steps.tsv`, `evidence/after/steps.tsv`): Clippy
0.15 GB; the census step 0.65 GB (its own `indirect-census` build of the
gateway, and the gateway's dependencies under `-p` features); the root suite
3.36 GB; the quota step 0.18 GB before, nothing after; the package step
0.17 GB of root target before and none after, beside the same 0.11 GB release
target and 22 MB of packages; the provider suite 3.75 GB of target and up to 0.17 GB of temporary
files. The registry download into the empty `CARGO_HOME` is 26 MB.

## 2. What changed

**`providers/package.py`** gets `root_workspace_executable(kind, name, probe,
manifest)`: it builds one root-workspace example or binary with `cargo build
--workspace --example|--bin NAME` and the test-build configuration
(`scripts/test_link_config.py`, loaded by path), reads the executable from
cargo's JSON messages (exactly one, or the probe fails) and returns it; the
caller runs it. The source-mode probe uses it, and so do the three probes of
**`providers/contracts.py`** (`provider_resource_views`, `contract_inventory`,
`quirin-audit pin --verify-contracts`), which `package.py build` runs through
`contracts.py check`. Each failure message keeps its probe's name. Before, each
was a plain `cargo run -p <crate>`.

**`.github/workflows/ci.yml`**: the quota step selects its test with
`--workspace --test execution_quota` instead of `-p qcore-verifier-host --test
execution_quota`; `execution_quota` is one target, in `qcore-verifier-host`.

Packages are unchanged: `package.py build` on the base tree and on the card's
tip, in this worktree with the same release target path, gave the same 112
files byte for byte, `packages.json` included (`evidence/package-identity.txt`).
The probes only classify sources; the packages are the release build of the
provider workspace in its own target directory, which the card does not touch.

## 3. Why it rebuilt, and why it no longer does

`-p <crate>` resolves features for that crate alone, while `cargo test
--workspace` unifies them over every member, so the same library gets other
features and another unit hash. In the base sequence: the quota step's `-p
qcore-verifier-host` rebuilt `serde_json`, `qcore-schemas`, `qcore-frontend` and
`qcore-verifier-host` (0.18 GB); the package step's probes rebuilt
`qcore-contract` and the two examples `contract_inventory` and
`source_extension_modes` (0.17 GB allocated; `evidence/before/root-target-written-after-step-9.txt`).
The reevaluation examples (`-p qcore-compiler`) and `quirin-audit`'s probe
rebuilt nothing: those crates' own features equal the workspace's.
`--workspace` with a target filter keeps the workspace's feature selection and
builds only that target, so it reuses the test step's units: after a
configured `cargo test --workspace --no-run`, the quota command and all four
probes compiled nothing (a separate build in this worktree: the package build
added 0.09 MB to that target).

The test-build configuration matters for the incremental setting, not for the
wrapper. Cargo leaves `build.rustc-wrapper` out of its fingerprint, so a plain
build with `CARGO_INCREMENTAL=0` reuses configured units, dynamically linked
ones included: the base run's plain `cargo run -p qcore-audit --bin
quirin-audit` ran the dynamic binary the test step had linked, and a plain
`cargo build --workspace --example source_extension_modes` after a configured
build compiled nothing (21 units fresh). Without `CARGO_INCREMENTAL=0` (a
developer's default) the profile differs and everything rebuilds: in this
worktree the three plain `contracts.py` probes added 3.4 GB to a configured
target. Passing the configuration makes the probes share in both settings. The
validation page said the wrapper was in the fingerprint; it now says what was
observed.

Not changed: the census step's `-p qcore-c-gateway --features indirect-census
--lib` compiles the gateway's dependencies under `-p` features too. With
`--workspace` the feature flag would have to name the package and the step would
run every workspace library's tests; the step is 0.65 GB in all, so it stays.
Clippy's check units cannot be shared with a build.

## 4. Method and what was not reproduced

`ci_rust_job.sh` (this directory) reads the `rust` job's steps from the clone's
own `.github/workflows/ci.yml` (`workflow_steps.py`), so each run executed its
revision's job; it runs every `run:` body as GitHub's default shell does
(`bash --noprofile --norc -eo pipefail`) with the job's `env:`, in a fresh
`git clone --depth 1` with `git submodule update --init --recursive --depth 1`
(what `actions/checkout` with `submodules: recursive` fetches), records each
step's exit and the size of every watched directory at its end, and continues
past a failing step (none failed). `measure_peak.py` of TEST-STORAGE-BUDGET
samples `du -s --block-size=1` of the clone and the temporary root every 5 s
around it; a peak shorter than 5 s can be missed.

Not reproduced: the two `uses:` steps. The checkout is a local clone of the
same revision instead of a network fetch; the toolchain is the host's rustup
stable (1.99.0) rather than `dtolnay/rust-toolchain@stable`, and its files, like
the runner's preinstalled software, lie outside the watched directories, as on
the runner they lie outside the job's working space. The host has 16 CPUs,
more than a hosted runner, which changes wall time, not bytes. `HOME` was not
watched; nothing the job runs is known to write there besides `CARGO_HOME`.

## 5. WINDOWS-BATCHED-SUITE-FIRST-RUN

Not observable from here. `git ls-remote origin` lists `main` at `db2795100`
and branch refs but no check results; GitHub's public API answers
`/repos/cosmo0920/quirin-lang/actions/runs` with 404 (the repository is
private), and this host has no `gh` CLI or token. The follow-up stays open,
amended with that.

## 6. PACKAGE-BYTES-DEPEND-ON-CHECKOUT: cause found, not fixed

Reproduced: `package.py build` of the same sources (`db2795100`) in this
worktree and in a second clone (`private/pkgclone`) gave different
executables for all five packages (`evidence/package-checkout-digests.txt`).

Cause: cargo's unit hash (`-C metadata`, which rustc turns into each crate's
disambiguator in symbol names) of a path package outside the workspace root
depends on its absolute path. The provider workspace takes `qcore-schemas` from
`../crates/qcore-schemas`, outside its root `providers/`.

- Release rlib names across the two checkouts
  (`evidence/package-unit-hashes.txt`): all ten registry crates and the five
  provider-workspace libraries that do not depend on `qcore-schemas` have the
  same hash; `qcore-schemas` and `quirin-provider-shared` (which depends on it)
  differ.
- In the executables (`package_diff.py`, `evidence/package-diff-*.txt`), the
  v0 crate disambiguators of `std`, `core`, `alloc`, `serde`, `serde_json` and
  every other registry crate are equal; those of `qcore_schemas`,
  `quirin_provider_shared` and the provider binary itself differ, in all five
  packages. The changed symbol names change `.strtab` and `.symtab`, and the
  code layout in `.text`, `.eh_frame` and `.gcc_except_table`; 24 of 32
  sections are equal.
- Not the checkout path in the bytes: `--remap-path-prefix` leaves no
  repository path in any executable. The only absolute paths left are under
  `~/.cargo/registry` and `~/.rustup` (14 per executable), not remapped, so a
  different `CARGO_HOME` or user would change the bytes as well.

Not fixed: every fix moves every package's bytes, and none is small. A fixed
build path (building from a copy at a constant location), making
`qcore-schemas` a path inside the provider workspace, or stripping symbols
(which would not remove the layout differences) each changes the five package
digests and needs its own decision; cargo has no stable option to hash a path
package relative to another root. The follow-up stays open with the cause and
the second (registry and toolchain path) dependency recorded.

## 7. Limits and follow-ups

- One run per side. Sampling every 5 s; the peak's composition is in
  `peak.json`.
- The job's figure depends on the provider suite's target (3.75 GB) and the
  checkout (2.6 GB) as much as on the root suite; a larger checkout (for
  example `fetch-depth: 0`) would add its history.
- No new follow-up.

## 8. Gate

GATE_SECTION
