# Pinned replay CLI relocation

Base: `2e44e963cfed0320a9d1c93ea09c9e70eeb1ff99`.

## Prediction (recorded before implementation)

This is a nonsemantic relocation of the Cycle 19 pinned Fluent Bit replay from the compiler example to `quirin-audit`. Expected changes to every pinned verdict, contract count, semantic profile, assumption, evidence version, and corpus coverage are **zero**. The replacement command should reproduce the old command's pinned output, modulo the command name or intentionally documented presentation metadata. Any variance requires investigation before deleting the legacy route.

## Ownership and command

`quirin-audit pinned [--jobs N] [FLUENT_BIT_ROOT [MONKEY_DATABASE [MONKEY_SOURCE_REPORT [DIAGNOSTICS [BINDINGS [MANIFEST]]]]]]` is the explicit built-in historical replay. Its ordered audit fold and sidecar assembly moved from the compiler example to `qcore-audit::pinned_route`; the new `pinned_cli` parses before starting source work. `pinned` is reserved in the external-subcommand built-in table, and the plugin shadowing regression covers it. The configured audit remains `quirin-audit current`, with its current-revision test now in the owning audit crate. The live runner is `scripts/run_pinned_audit.py`; the frozen W43 runner remains historical evidence.

`quirin-source-analysis` owns the formerly compiler-resident ChunkIO and Libevent source adapters, their source-profile locator, typed match and chunk contract boundaries, unit tests and needed fixtures. `qcore-compiler` uses facade reexports for source compatibility; `qcore-audit` uses the shared crate and lower gateway, contract and verification crates directly. The gateway's generic typed C importer, compiler's separate `pinned_lifecycle` adapter, verification models and the compiler's first-party provider construction stay in their existing owners. Opaque `ChunkSourceSummary`, `ChunkContractCertificate` and `SourceMatch` constructors remain private. This extraction changes no inference, witness admission, resource contract or pin. There is no new Lean semantic obligation; the standing Lean audit is still run as a gate.

The old profile locator produced a non-canonical repository path through `compiler/../contract/profiles`. Contract sidecars include that spelling in raw source keys (evidence constraint 22), so the shared locator deliberately reconstructs the same path from its new crate root. The first new replay attempt produced different raw keys; it was interrupted (exit 130) and is **not** parity evidence. The first old single-job attempt was also interrupted (exit 130) under shared-host load. Both attempts remain in `private/audit-pinned-replay-cli/{new,baseline}/replay/` with their logs and `INTERRUPTED.txt`; fresh four-job paths are used for parity.

The compilation environment still comes from the pinned compile database, with the same `HistoricalStdin` transport; no selected unit, source path, assumption, observed evidence or certificate check changed. The package and profile pins remain the same. Source summary and result formatting follow the original code apart from import ownership and the command name.

## Predecessor evidence and interrupted probes

The latest completed predecessor is the frozen W46 combined corpus archive in
`docs/reports/w46-contract-access-expansion/artifacts/combined/corpus/`.
Its `run.json.gz` records exit 0, repository source commit `e6e31f49`, the
same literal Fluent Bit source, Monkey compile database and ChunkIO package
paths, `/usr/bin/clang`, unset `QCORE_SOURCE_CONTRACTS`, and 16 workers.
`git diff --name-only e6e31f49..2e44e963` contains only `docs/` paths:
the baseline source, provider and contract code was unchanged. The input
Fluent Bit checkout is still clean at `66910c10`; the compile database and
package directory predate the archived run. A fresh old four-worker probe in
this isolated worktree reproduced all 54 contract source texts and every
shared selection/profile field after substituting only the checkout prefix in
its raw source keys. That probe received SIGTERM (exit 143) after about 17
minutes, before ordered verdict publication; it cannot serve as a completed
baseline. Its command, prelude, partial sidecar and exit are preserved at
`private/audit-pinned-replay-cli/baseline/replay-jobs4/`. The direct wrapper
had no timeout. A bounded gdb attempt to inspect that process was denied by
host ptrace policy; its refusal is preserved alongside the probe. The earlier
single-worker old and new probes were deliberately interrupted (exit 130), as
recorded above. No interrupted attempt is counted as parity evidence.

The completed new four-worker replay is measured against the W46 archive. The worker-count difference is an execution schedule, covered by
W44's parallel-unit byte parity, and does not relax the comparison here: all
five complete artifacts and every verdict and coverage count must agree after
explicit accounting for checkout-root provenance. If they do not, the old
command remains in place.

## Completed parity and retirement

The migrated release binary (`sha256 ae9511e5a062b7d7d569775f493944cabc28b374d72c185e07401f81ac6393f4`)
ran `quirin-audit pinned --jobs 4` against the literal W46 pinned Fluent Bit
source, Monkey compilation database and ChunkIO package. Its direct-wrapper
`private/audit-pinned-replay-cli/new/replay-jobs4/run.json`
reports 2026-09-27 08:20:18–09:02:46 UTC, exit **0**, all five artifacts and
1,218 log lines. The W46 predecessor's frozen `run.json.gz` reports exit **0**
at 16 workers and 1,342.71 seconds. The fresh old four-worker probe received
SIGTERM (exit **143**) before verdict publication, as recorded above; its
absence is not hidden by calling it a successful baseline.

The [strict parity result](parity.json), generated by [`compare.py`](compare.py)
with [`source_effect_provenance.py`](source_effect_provenance.py), exits **0**
and accounts for every artifact:

| Artifact | Measured inventory | Comparison to completed W46 predecessor |
|---|---:|---|
| `resource-diagnostics.json` | 185 units: 6 Safe, 8 provisional Unsafe, 167 Unknown, 4 data-only | byte-identical |
| `resource-diagnostics.chunk-contracts.json` | 96 results, 21 teardown records | byte-identical |
| `monkey-source.json` | 3 callbacks, 15 wrappers | byte-identical |
| `resource-diagnostics.contracts.json` | 54 modules | exactly byte-identical after replacing the old checkout root with this worktree root; 50 raw module keys change by that prefix, 4 provider-qualified keys stay fixed; all text, pins and contract-revision status match |
| `resource-diagnostics.source-effects.json` | 96 results | 93 byte-identical rows; the other 3 match after the verified cwd-bound identity cascade below |

The audit log is identical after masking only its output-directory spelling,
with 1,218 lines on both sides. The [exhaustive row diff](diff.json), generated
by `docs/reports/corpus-repin/diff_artifacts.py`, finds 0 added/removed/changed
units, chunk-contract rows, callbacks, wrappers or contract source texts; it
records the three raw source-effect row differences. The W44
[raw-byte comparator](raw-compare.json) exits **1** as expected for the two
provenance-bearing artifacts while confirming the other three artifacts and
output-path-masked log. Raw differences remain visible; no archived report or
measured artifact was rewritten.

The three rows are `cio_chunk_is_locked`, `cio_chunk_unlock` and
`flb_input_chunk_release_lock`, all still `checked_function_contract` with the
same scope, semantic profile, bounds, assumptions, evidence version, source
and preprocessed digests, contract/profile digests and Clang identity. The
pinned compatibility importer hashes the process current directory in its
configuration digest (`crates/qcore-c-gateway/src/boolean_source.rs`). Reconstructing
the exact pinned compile options gives 37 include paths / 21 definitions for
`cio_chunk.c`, and 52 / 63 for `flb_input_chunk.c`; only the current-directory
component changes between the main checkout and this worktree. The resulting
configuration hashes match both reports exactly. The comparator then
recomputes the serialized source-identity hash, task build digest, obligation
ID and full task/witness digest for **both** versions of each row, checks the
bounds and coverage references, and compares the complete rows after masking
only that checked identity cascade. All three pass. These are distinct
provenance identities, not changed source effects or verdicts.

Only after these measurements passed were the legacy compiler example and
`support/cycle19_compatibility.rs` removed. The built-in command, shared
source-analysis crate and live `scripts/run_pinned_audit.py` now own current
execution. `scripts/tests/test_retired_scripts.py` guards against restoring
the old files. The [combined integration gate](../audit-checkpoint-files/integration/README.md) passed all steps at `55ac39f5`: 6,059 Rust tests, 365 Python tests and 213 Lean audits. Its first-attempt layering and clippy failures and their corrections are retained in that record. Private targeted check logs and intermediate failed probes are
under `private/audit-pinned-replay-cli/`, including the successful retired-file
check (3 tests, exit 0) and the earlier source/CLI build and test records.
