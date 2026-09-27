# W5: configured resource-audit driver

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Date: 2026-09-07. This is the first bounded W5 orchestration extraction, not
provider-process or whole-workspace consolidation completion.

## Extracted into L3

`crates/qcore-audit` depends only on `qcore-c-gateway`, `qcore-contract` and
`qcore-schemas` within the workspace. It owns compilation-database ingestion,
option projection, per-unit gateway execution, verdict counters and diagnostic
report serialization. `qcore-compiler` uses it only as an examples/test dev
dependency; a guard pins that boundary.

The generic `qcore-audit CONFIG.json` binary consumes a profile, declared
contracts, compile database and selected library names. It verifies pins,
audited-file digests, construct needles and each selected source's actual
pinned bytes, then emits the report and an assumption sidecar. Unknown stays
in the denominator. Unavailable required evidence, unsupported compiler flags,
missing contracts/libraries and conflicting selected build commands reject.

The old Cycle19 example is a small command wrapper. Its specialized
ChunkIO/Monkey/native observation stages remain explicitly in
`compiler/examples/support/cycle19_compatibility.rs`, calling the shared L3
projection engine. The primary five-library selection is data in
`contract/profiles/fluent-bit/audit.json`, with paths drawn from W2 profiles;
its default contracts also come from the profile. The forced-Monkey source-copy
route and its separate evidence gates are retained. No provider module becomes
a dependency of the generic driver.

The inherited pinned option adapter remains explicitly named
`compiler_options_compatibility`; it is not used by the generic CLI. The CLI
rejects flags whose preprocessing effects the current gateway cannot preserve.
The previous ad hoc compilation-database field scanner is replaced with real
per-record JSON deserialization; complete pinned output parity precedes removal.

## Second-codebase demonstration

`create_demo.py` creates a separate, pinned synthetic C repository with balanced
allocation/release, a leaked allocation and an opaque call. It writes only C,
contracts, profile, compile-db and configuration files; no Rust source or
provider is added or changed. Run with a fresh directory:

```sh
python3 docs/reports/w5-configured-audit/create_demo.py /tmp/new-audit-demo
cargo run -p qcore-audit -- /tmp/new-audit-demo/audit.json
```

The executed demonstration reports **1 Safe / 1 provisional Unsafe / 1 Unknown**
across three units, with zero gateway failures. Inputs, report, assumptions and
stdout are archived under `demo/`. These are bounded resource-projection
outcomes under the declared contracts; no production artifact is promoted.
This is a synthetic held-out codebase, not a claim to have remeasured CFL.

## Full pinned parity

The compatibility command was run against all 185 pinned Fluent Bit/Monkey
translation units, using the forced-Libevent database:

```sh
cargo run -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/audit-extracted-source.json /tmp/audit-extracted-diagnostics.json
```

It exits 0. Entire resource diagnostics and Monkey source JSON are
**byte-identical** to the H6/gateway baseline: 3 Safe, 98 provisional Unsafe,
80 Unknown and 4 data-only units, with zero verdict or diagnostic movement.
`parity.json` binds exact hashes and counts; compressed outputs and stdout are
archived here. New assumption-sidecar metadata identifies the data selection.
Raw production units remain unpromoted under the Cycle19 evidence gates.

## Checks and open scope

All standing gates pass: **513 workspace tests**, fmt, clippy, both reevaluation
examples (exit 0), pinned submodule initialization and Lean (53 jobs). Logs
record the exact checks and test counts. The generic CLI tests execute the
second-codebase path and fail-closed mutations; report tests moved alongside
their implementation. Lean semantics are unchanged: this moves orchestration
and validates declared inputs, not the resource transition rules.

Remaining W5 work includes individual provider crates/processes, registry-based
production composition and semantic-kernel consolidation. The specialized
compatibility adapter stays until replacement provider routes reproduce its
complete evidence output. H4 typed Monkey import routing and the remaining
W2/W5 profile/model boundaries are not closed by this extraction.
