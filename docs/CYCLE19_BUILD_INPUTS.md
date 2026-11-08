# Persistent Cycle 19 build inputs

The optional Monkey audit database must describe the forced Libevent build.
Copying only `compile_commands.json` out of `/tmp` is insufficient: its source
files, working directories and generated include paths must also survive.

From the QCore-C repository root, prepare the pinned source copy and configure
its headers and commands in the ignored, persistent `build/cycle19` directory:

```sh
cargo run -p qcore-audit -- prepare monkey --fluent-bit ../fluent-bit
cargo run --quiet -p qcore-audit -- current \
  --checkout ../fluent-bit --profile-directory contract/profiles/fluent-bit \
  --monkey-database build/cycle19/monkey-build/compile_commands.json
```

**The linked configuration** (card CURRENT-MONKEY-LINKED-CONFIGURATION,
[report](reports/current-monkey-linked-configuration/README.md)):
`fluent-bit` links `lib/monkey` as the Fluent Bit build compiles it (epoll
backend, OpenSSL TLS), not this forced-Libevent copy. `current
--monkey-linked` analyses Monkey's units (`mk_core/` without `mk_core/deps/`,
`mk_server/`, `tls/`) from the run's own `--compile-database` under the label
`monkey`. A `--monkey-database` given with it is only accounted for. A
standalone unit with a linked counterpart (the same file) is replaced by it.
One without a counterpart stays in the denominator as a not-linked `Unknown`
unit under its `monkey-libevent` key. The setup's `prepare --run` passes
both.

`quirin-audit prepare monkey` (formerly a Python script; same commands and
files, [parity evidence](reports/w44-quirin-audit-prepare/README.md)) checks
the checkout's `HEAD`, a clean Monkey source
subtree, and retains a byte-identified copy. CMake explicitly selects Libevent
and disables the other event loops. `qcore-monkey-inputs.json` records source
tracked-source hashes, separate generated-header hashes, CMake arguments,
database identity and paths; `configure.log` records
configuration output. Source code in the upstream checkout is not edited.

Both audit routes analyse translation units concurrently: `--jobs N`
(or `QCORE_AUDIT_JOBS=N`) selects the thread count,
defaulting to the host's available parallelism capped at 16. Outputs are
byte-identical at every job count and `--jobs 1` is the sequential loop; see
[the validation gate](agents/validation.md#corpus-arbiter-and-its-job-count).

The setup is repeatable with unchanged inputs. It refuses an unrecognized
output directory or a changed source copy; use `--output` with a fresh persistent
directory when intentionally changing build inputs. `--cmake` can select an
installed CMake executable. The default output is relative to the repository,
not the shell's temporary directory. Do not commit generated platform-specific
commands, headers or copied third-party source; commit this setup recipe instead.

The normal Fluent Bit audit also needs the upstream checkout's own
`build/compile_commands.json`. The Monkey setup does not create that separate
build configuration. Existing pinned source/build and contract checks continue
to apply. Historical reports retain their original temporary paths as evidence
of those runs; use this recipe for new runs.

This configures the build but does not claim native binaries or sanitizer runs.
If a native observation harness needs built libraries, build the configured tree
with `cmake --build build/cycle19/monkey-build --parallel 2` first and retain its
separate build/observation evidence.

For process-backed Boolean-field contracts, retain the provider package alongside
the build inputs. On first setup, build it with
`python3 -B providers/package.py build --output build/cycle19/provider-packages`.
An existing package directory is reused after verification; the builder refuses
to overwrite it. The checked-in configuration resolves relative paths from its
own directory:

```sh
mkdir -p build/cycle19/reports
python3 -B providers/package.py verify build/cycle19/provider-packages/qcore.chunkio
QCORE_SOURCE_CONTRACTS="$PWD/contract/profiles/fluent-bit/source-contracts.json" \
  cargo run --quiet -p qcore-audit -- current \
  --checkout ../fluent-bit --profile-directory contract/profiles/fluent-bit \
  --monkey-database build/cycle19/monkey-build/compile_commands.json \
  --monkey-source-report build/cycle19/reports/source-effect-source.json \
  --report build/cycle19/reports/source-effect-diagnostics.json
```

`QCORE_CHUNKIO_PACKAGE` still selects a package path using the default bindings.
Do not set both selectors: ambiguous selection rejects before source auditing.
Missing or incompatible explicit packages never fall back to direct evaluation.
The default audit without either selector remains the direct comparison route.

## Persistent CMetrics Clang build

The canonical CMetrics configuration now uses CMake's unmodified compile database:

```sh
cargo run -p qcore-audit -- prepare clang-build
cargo run --quiet -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json
```

It creates an independent pinned source clone and a native Clang GNU17 CMake
build under `build/cycle19/fluent-bit-clang-{source,production}`. On fresh setup,
build the process packages into `build/cycle19/provider-packages-unit` first
using `providers/package.py build`; existing package directories must be verified,
not overwritten. Custom `--clang` must match `QCORE_CLANG` during auditing.
The original GCC build is not treated as equivalent. Exact commands, two native
builds and their one include-path difference are recorded in the
[build report](reports/w2-clang-cmake-build/README.md).

## Second configured source set (ChunkIO)

The `chunkio-core` source set reuses the same persistent Clang CMake build. No
extra preparation is needed once the CMetrics setup above has run:

```sh
cargo run --quiet -p qcore-audit -- contract/profiles/chunkio-core/audit.json
```

It selects `lib/chunkio/src` out of CMake's unmodified compile database under
the same Fluent Bit pin and reports 13 units: 0 Safe / 2 provisional Unsafe /
11 Unknown. No provider is bound, so no source-contract evidence is produced
and no raw unit is promoted. Fail-closed mutations of this configuration are
executed by `docs/reports/remaining-g03/failclosed.py`; the recorded run and
its two generalization findings are in
[the G03 report](reports/remaining-g03/README.md).
