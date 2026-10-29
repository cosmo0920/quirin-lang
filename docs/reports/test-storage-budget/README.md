# TEST-STORAGE-BUDGET: the test suites in about 4 GB of disk instead of 86

Card TEST-STORAGE-BUDGET, 2026-10-29. Base: local `main` `62c8e461c`. The
user asked for the test suites to build and run on a CI runner with about
15 GB of attached storage, and asked specifically about dynamic linking for
the test executables. The card changes how the tests are built. It does not
change what is tested or any analysis code.

## 1. Outcome

Peak disk is the largest sum of `target/` and the run's temporary directory
(`TMPDIR`), sampled every 5 s (§4). All runs started from an empty
`target/` on the shared 16-CPU host, with `CARGO_BUILD_JOBS=6`.

| Workspace and configuration | Command | Peak | Of which temp | Wall | Results |
|---|---|---:|---:|---:|---|
| root, **before** (base `62c8e461c`) | `cargo test --workspace --no-fail-fast` | **87.40 GB** | 0.32 GB | 338 s build + 1979 s run | 8,927 / 0 / 15 (§6) |
| root, **after, Linux** (dynamic, the gate's default) | `python3 scripts/run_test_binaries.py` | **3.76 GB** | 0.12 GB | 1251 s | 8,927 / 0 / 15, equal |
| root, after, static in batches of 32 (Windows configuration, on Linux) | `run_test_binaries.py --link static --batch-size 32` | **6.19 GB** | 0.35 GB | 1628 s | 8,927 / 0 / 15, equal |
| providers, **before** | `cargo test --manifest-path providers/Cargo.toml --workspace` | **14.97 GB** | 0.31 GB | 74 s build + 2264 s run | 133 / 0 / 1 |
| providers, **after** | the same with `--locked --offline`, `CARGO_INCREMENTAL=0` as in CI | **5.15 GB** | 0.12 GB | 2360 s | 133 / 0 / 1, equal |

Results are passed / failed / ignored; "equal" means the same test names in
the same executables with the same ignored and failed sets (§6). The root
test executables: 583 of them, 72.3 GB before (mean 124 MB, largest 220 MB),
2.2 GB after (mean 3.8 MB), plus 19 workspace dylibs of 0.46 GB in all. In the
batched run the peak came at the end, when the first 32 examples were compiled
(examples are static there); each test batch freed 3.5–4.7 GB when it was
deleted.

Build-only figures behind the decisions in §3 (`cargo test … --no-run`, peak
of `target/`):

| Build | Peak | Wall |
|---|---:|---:|
| root, base (line tables, incremental) | 86.37 GB | 338 s |
| root, static, line tables, no incremental: not measured separately; the base's `deps/` alone held 69 GB of it | — | — |
| root, static, `debug = 0`, no incremental | 19.38 GB | 127 s |
| root, dynamic, line tables, no incremental (**chosen**) | 3.63 GB | 131 s |
| root, dynamic, `debug = 0`, no incremental | 1.76 GB | 112 s |
| providers, base (full debug info, incremental) | 14.43 GB | 74 s |
| providers, line tables, no incremental (**chosen**) | 3.53 GB | 60 s |
| macOS CI job's commands, static (Linux proxy) | 5.78 GB | 163 s |
| Windows `msvc-core` leg's commands with Clippy, static (Linux proxy) | 4.88 GB | 278 s |

- **Linux (the gate and the CI `rust` job):** 87.4 → 3.8 GB, 23× less, with
  the identical test set and outcomes; the build itself took 99 s instead of
  338 s.
- **Windows (the full-suite leg):** static in batches; measured on Linux at
  6.2 GB. Not run on Windows here (§7).
- **Provider workspace:** 15.0 → 5.2 GB, by line tables instead of full
  debug information and no incremental state.
- **CI `rust` job as a whole** keeps both workspaces' `target/` until it ends:
  the root suite 3.8 GB, Clippy's check artifacts 0.15 GB (measured alone), the
  package build's release target 0.1 GB and its plain `cargo run` source-mode
  probe in the root `target/` about 1.3 GB (it does not pass the configuration,
  so it rebuilds three crates statically), the provider suite 5.2 GB: about
  10.5–11 GB. That sum is estimated from parts measured separately, not
  measured as one sequence (§7).
- **Run time is unchanged**: the same two heavy executables alone, dynamic,
  static, dynamic again: `provider_selection` 134.5 / 131.5 / 132.6 s,
  `contract_views` 188.3 / 182.1 / 177.7 s (`evidence/timing.log`).

## 2. What changed

**`scripts/test_link_config.py`** names the test-build configuration, once:
`build.incremental=false` everywhere, and on Linux `build.rustc-wrapper` =
`scripts/rustc_test_dylib.py`. It returns `--config` arguments; as a script it
prints them one per line for CI (`mapfile -t test_config < <(python3
scripts/test_link_config.py)`).

**`scripts/rustc_test_dylib.py`** is the wrapper. For a crate of the root
workspace (not a registry crate, not `providers/`):

- a library gets `--crate-type dylib` beside its rlib, in the same rustc run,
  so `deps/lib<crate>-<hash>.so` holds its code and line tables once;
- every linked artifact (library, test harness, binary, example) gets
  `-C prefer-dynamic`, so it links the workspace dylibs and `libstd` instead
  of copying them;
- each `--extern name=…/lib<crate>-<hash>.{rmeta,rlib}` is rewritten to the
  rlib plus the sibling `.so` (rustc looks only at the exact paths cargo
  gives for a direct dependency, and cargo names the `.rmeta` of a dependency
  it thinks needs no linking);
- absolute run paths to `deps/` and the toolchain's `target-libdir` are
  linked in, so a workspace binary started with a cleared environment
  (`quirin-audit`'s subcommand plugins, the provider host's `env_clear`) still
  loads its libraries. `env -i target/debug/quirin-audit` runs.

Build scripts, proc-macros, check/Clippy compiles, rustc probes and
non-workspace crates pass through unchanged. Cargo does not know about the
`.so` outputs, so with pipelining it starts a dependent library when rustc
announces the dependency's `.rmeta`, before the `.so` exists; the first
prototype failed exactly so (`crate qcore_schemas required to be available in
rlib format`). `build.pipelining=false` is no longer honoured by this cargo,
so the wrapper withholds that announcement (the `"emit":"metadata"` artifact
line rustc prints on stderr). Cargo then marks the metadata produced when the
rustc run ends, its own fallback for a job that never announced it. The wrapper
keeps cargo's jobserver descriptors open for rustc (`close_fds=False`); the
first full prototype printed rustc's "failed to connect to jobserver" warning
for every crate until it did.

Each crate exists exactly once in a linked image: every workspace library is a
dylib, and a third-party crate is statically included in the first dylib that
needs it (`serde`, `serde_json` and their dependencies in `qcore-schemas`,
`libc` in `qcore-verifier-host`, `toml`, `postcard` and `serde_norway` in
`qcore-audit`). rustc refuses a link that would include one crate twice
("cannot satisfy dependencies so … only shows up once"), so a conflict cannot
pass silently.

**`scripts/run_test_binaries.py`** passes the configuration to every cargo
command it runs (build, capture, doc-tests) and records it in `summary.json`
(`link`). `--link static` builds as plain `cargo test` does. New
`--batch-size N`: the test targets of the selection (from `cargo metadata`:
`--lib`, `--bin`, `--test`, and examples or benches with `test = true`) are cut
into batches of N target names; each batch is built (`cargo test <selection>
--no-run <filters>`), captured and checked against its build as before, run
(parallel units, then its `EXCLUSIVE` units alone), and its executables are
deleted before the next batch is built. `--workspace` with target filters keeps
the workspace's feature unification, so a batch builds the same units as the
whole build. The examples `cargo test` only compiles are compiled with `cargo
build --example …` in batches and deleted (every hard-link name: cargo reports
an uplifted example by its unhashed name). Over the whole run the executables
built must be exactly the selection's test targets, each once, the
stale-`EXCLUSIVE` check covers the selection, and the doc-tests run last; the
first failing build, capture or plan stops the run with exit 101.

**`scripts/gate_scope.py`** runs the reevaluation examples with the
configuration (otherwise `cargo run` would rebuild every crate without the
wrapper) and treats a change to either new script as a full-suite change.

**`providers/Cargo.toml`** gets `[profile.dev] debug = "line-tables-only"`, the
root workspace's setting since W44: the provider tests link the analysis crates
with full DWARF (a 605 MB `qcore_c_gateway` rlib, test executables of
480–673 MB). Release, and so every package, is unchanged (§5).

**CI** (`.github/workflows/ci.yml`): the Linux and Windows jobs set
`CARGO_INCREMENTAL=0`, equal to `build.incremental=false`, so configured and
plain steps share units. The Linux job's test, census, quota and reevaluation
steps pass the configuration. The Windows `msvc-clang-analysis` leg runs the
full suite as `python scripts/run_test_binaries.py --jobs 1 --batch-size 32`
instead of `cargo test --workspace` (`--jobs 1`: one executable at a time, as
`cargo test` ran them there).

**Policy**: [validation, "Test build storage"](../../agents/validation.md#test-build-storage)
is the one authority; the gate block's reevaluation commands carry the
configuration.

## 3. Decisions

**Where the bytes were.** The base build's `target/` peaked at 86.4 GB:
`deps/` 69 GB (583 test executables, 72.3 GB counting the uplifted names;
mean 124 MB, median 154 MB, largest 220 MB), incremental state 6.3 GB,
examples 5.6 GB. One executable, `qcore-compiler`'s `ct_corpus`, is 215.8 MB
static (123.4 MB DWARF line tables, 38.8 MB `.text`, 53.6 MB other sections);
dynamically linked it is 9.4 MB (4.3 / 2.0 / 3.1). The 19 workspace dylibs
together are 0.46 GB.

**Dynamic linking, as the user asked, through a wrapper rather than the
manifests.** The cargo-native form is `crate-type = ["rlib", "dylib"]` in every
library manifest plus `-C prefer-dynamic` in the test builds' `RUSTFLAGS`. A
first prototype did exactly that and worked on Linux, but the crate type is
unconditional: every build on every platform, release included, would also
link a dylib per library (statically containing all its dependencies without
`prefer-dynamic`); on Windows such DLLs of the large crates risk the 65,535
export limit (`LNK1189`) in every CI leg; cargo names path-dependency dylibs
without the hash, so the census-feature build and the normal build would
overwrite one `libqcore_c_gateway.so`; and `RUSTFLAGS` reaches the provider
packages and the cargo runs tests start. The wrapper confines all of it to
commands that pass the test configuration, keeps the hash in every name, and
leaves manifests, lockfile and every other build alone. Its cost is the lost
pipelining; the dynamic build still took 131 s against the base's 338 s,
because linking 3.8 MB executables is cheap.

**Every linked workspace artifact, not only test harnesses.** Static examples
alone were 5.6 GB in the base, and the tests start workspace binaries
(`CARGO_BIN_EXE_*`), some with a cleared environment; the absolute run paths
make those work, and no test starts a workspace binary under Landlock (the
confined providers are shell scripts).

**Debug information stays.** Line tables are 54–85% of a static executable,
but with dynamic linking the whole root build is 3.6 GB; `debug = 0` would
save 1.9 GB and cost file:line in every panic. For the static Windows suite,
`debug = 0` alone still left 19.4 GB, so batching is what meets the budget
there, and with batches line tables fit too (§1). The provider workspace had
full DWARF in dev builds, unlike the root; line tables bring its build from
14.4 to 3.5 GB (incremental state included in the first figure, 3.2 GB of it).

**No incremental state.** A gate or CI build starts from an empty or foreign
`target/`, and the base's 6.3 GB of incremental state is never read again.
Locally, a configured re-run after a source edit recompiles the changed crates
from scratch; plain `cargo test` keeps incremental builds for development.

**Run time.** In the full runs some executables took about twice as long in
the dynamic run as in the static batched one (for example `contract_views`
472 s against 244 s), but the two runs overlapped different neighbours under
different host load, and run alone in the same slot hold the same executables
take the same time either way (§1). The suite's 1144 s run phase, against
W44's 4½ minutes, comes from tests that have since grown
(`production_symbolic_adoption` took 740 s dynamic and 649 s static here and
has no entry in `test_binary_weights.json`), not from the linking.

**Batches only where dylibs are not used.** Windows links statically and runs
the suite in batches of 32 target names. The runner, not a PowerShell loop,
does it, so the batch keeps the capture check, the `EXCLUSIVE` phase and the
fail-closed totals, and the Linux run of the same mode is the measured proxy.
The macOS job and the Windows `msvc-core` leg build only selected targets and
stay as they were apart from `CARGO_INCREMENTAL=0` on Windows (proxies 5.8 and
4.9 GB).

## 4. Method

`measure_peak.py` (this directory) runs a command and samples `du -s
--block-size=1` (allocated bytes) of each watched directory every 5 s, keeping
the largest sum; a peak shorter than 5 s can be missed. Every run had its own
fresh `CARGO_TARGET_DIR` under the worktree's `private/` and its own `TMPDIR`
(`/tmp/tsb-*`, short, so Unix-socket paths stay within limits). The base was
measured in a clone of `62c8e461c` (`private/tsb-base`, libevent submodule
checked out): its build (`cargo test --workspace --no-run`, no slot) and then,
on the same `target/`, the run (`cargo test --workspace --no-fail-fast`), so
the pair is one fresh `cargo test`. Every test run held the `rust-suite` slot
(`private/tsb-measure/after.sh` under `gate_slot.py rust-suite`). Peaks and
samples are in `evidence/`.

## 5. Release builds and identities

No root manifest, `Cargo.lock`, `providers/Cargo.lock` or cargo configuration
file changes, and the configuration exists only as `--config` arguments passed
by the test tooling, so the release `quirin-audit`, the measure tool's `rev:`
side builds (which start cargo with a cleared environment, release, no
configuration) and plain `cargo build` are the builds they were. The slice
identity check was therefore skipped: no release build input changed. The one
manifest change, `providers/Cargo.toml`'s `[profile.dev]`, does not reach
release: `providers/package.py build` on this tree with and without it gave
byte-identical provider executables (all five SHA-256 equal). No pinned digest,
lock or build identity moves.

Observed in passing, not caused by this card: the same package build in the
base clone (`private/tsb-base`, same Rust sources, a different checkout path
and HEAD) gave different executables for all five packages (§7).

## 6. Test parity

`compare_tests.py` (this directory) compares the executed tests of a plain
`cargo test` log with runner summaries or other logs: the multiset of (test
executable without its hash, test name), and the multisets of ignored and
failed tests. `evidence/parity-root.txt` and `evidence/parity-providers.txt`:

- root: the base's plain `cargo test --workspace --no-fail-fast` ran 8,942
  tests, 15 ignored, none failed; the dynamic run and the static batched run
  ran the same 8,942 in the same executables with the same 15 ignored:
  **EQUAL** both. The runner totals are 8,927 passed, 0 failed, 15 ignored,
  617 summaries over 584 units, the brief's figure for the base under the
  gate.
- providers: 134 tests, 1 ignored, before and after: **EQUAL**; 133 / 0 / 1.

Summing the plain log's `test result:` lines gives 8,926 passed, one short:
the gateway rule-switch tests re-execute their own binary and the child's
output is interleaved with the parent's, and in the base log one child's
summary was split mid-line (`test result: . 1 passed` … `ok`). The names
behind it are present and equal; the runner reads each unit's summaries from
its own log and is not affected. The same interleaving is why the comparison
reads every `test <name> ...` fragment of a line rather than whole lines.

## 7. Limits and follow-ups

- **Windows is unexercised here.** The batched runner has run green on Linux
  in static mode with `--jobs 8`, not on Windows and not with `--jobs 1`; the
  first CI run of `msvc-clang-analysis` is its first execution there. Paths,
  `.exe`/`.pdb` names and the capture runner are platform-neutral Python; a
  file still in use at deletion is reported and left (a warning, never a
  verdict). If a dynamic Windows build is wanted later, it needs a DLL export
  count measurement first (follow-up `TEST-DYLIB-WINDOWS-EXPORTS`).
- **The CI `rust` job's total is an estimate** from separately measured parts
  (§1); no run measured the job's whole sequence. `providers/package.py
  build`'s source-mode probe runs a plain `cargo run` in the root workspace and
  so rebuilds three crates statically beside the dynamic build (about 1.3 GB);
  passing the test configuration there is a provider-tooling change left out
  of this card.
- **Plain and configured commands in one `target/` rebuild each other**: the
  wrapper and the incremental setting are in cargo's fingerprint. A developer
  alternating `cargo test` and the gate in one worktree pays a rebuild each
  switch; the file names are the same, so nothing stale is linked.
- **`test_binary_weights.json` is stale** (281 entries for 584 units; the
  longest unit now has none). It only orders the parallel phase; not
  refreshed here, since a refresh belongs with a timing card.
- **Observed, not caused here:** `providers/package.py build` in the base
  clone and in this worktree, same Rust sources, gave different provider
  executables for all five packages, while this tree with and without its
  `providers/Cargo.toml` change gave identical ones; the packages' bytes may
  depend on the checkout path or revision. Not investigated
  (`PACKAGE-BYTES-DEPEND-ON-CHECKOUT`).
- The task-context protocol selected every topic (scripts and CI paths are
  unknown to it) and 1.16 MB of rule text. I read the operations and
  validation pages and constraint 16 (the load-sensitive rows, all green in
  every run here) and searched the rest for the touched paths and tools; I did
  not read the 623 KB gateway page or the semantic topic pages in full. The
  card changes no analysis code, contract or rule.
