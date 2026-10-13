# PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE: one out-of-process writer for every executable test fixture

Card of wave 48, base `b565f79ab`, prediction frozen at `35d43976d`
([PREDICTION.md](PREDICTION.md)), implementation `b30c9d4e7`. Test
infrastructure only: no production source, `Cargo.toml`, `Cargo.lock`,
contract or Lean change.

## 1. The defect

[EXECUTION-QUOTA-LOAD-INDEPENDENCE](../execution-quota-load-independence/README.md)
measured `ETXTBSY` from a fixture written with `fs::write` in a multi-threaded
test binary. While the write descriptor is open, another test thread's spawn
can `fork`, and the forked child holds a duplicate until its own `exec`.
An `execve` of the file in that window fails with `Text file busy`, whether
the test process or a process it started runs it. That card repaired the
verifier host's own tests with
`crates/qcore-verifier-host/tests/support/process_fixture.rs`
`write_executable`: `/bin/sh -c 'cat > "$1"; chmod'`, so the only write
descriptor belongs to a single-threaded `cat` that has exited before the
call returns. The ledger entry named six files outside that repair.

## 2. Census

The census searched every `set_permissions`, `PermissionsExt`, `from_mode`,
`set_mode`, `OpenOptionsExt` and `.mode(0o…)` site, every `fs::copy`, and
every in-place rewrite of a file that was already executable. It ran before
the code ([PREDICTION.md](PREDICTION.md) §2) and again on the tree after it.
"Concurrent fork" means that another test of the same binary spawns a process
while the write can be open.

| File (base lines) | Executable written | Concurrent fork in the binary | Change |
|---|---|---|---|
| `crates/qcore-verifier-host/tests/standalone_host.rs` (168) | `provider.sh`, five bodies | **no**: three tests, and the only spawning one writes and spawns on its own thread | writer |
| `compiler/tests/cycle20_verifier_plugins.rs` (331, 357, 433, 611, 758) | `install_verifiable`'s in-place envelope rewrite; `hang.sh`; `provider`; `provider.sh`; `write_script` | **yes**: 20 tests, four spawn providers | writer |
| `compiler/examples/reevaluation_followup.rs` (92, 116, 138) | `provider.sh`, three bodies | **no**: one thread forks; the host's reaper threads only `waitpid` | writer |
| `crates/qcore-audit/tests/user_contract_plugins.rs` (173, 468) | `check.sh`; its in-place rewrite (asserted never to run) | **yes**: every test spawns `quirin-audit` | writer |
| `providers/conformance/tests/configured_audit.rs` (112, 333) | `fs::copy` of the built provider; its in-place rewrite | **yes**: `git`, Clang and providers | `copy_executable`, writer (`0o755`) |
| `crates/qcorec/tests/plugin_packages.rs` (17, 306) | `provider`, executable only in `configured_source_scope_dispatches_only_the_enabled_process` | **yes**: every test spawns `quirin` | `install(…, Some(0o755))` |
| `crates/qcore-verifier-host/tests/package_identity.rs` (469, 490) | `decoy.sh`; the verified file rewritten (both asserted never spawned) | yes | writer |
| `crates/qcore-audit/tests/configured_cli.rs` (141) | `counting-clang` wrapper | **yes**: 17 tests spawn `quirin-audit` | writer (`0o755`) |
| `crates/qcore-audit/tests/prepare_cli.rs` (60) | stand-in `clang`, `clang++`, `not-clang`, `cmake` | **yes** | writer (`0o755`) |
| `crates/qcore-audit/tests/subcommand_plugins.rs` (40) | `quirin-audit-NAME` plugins | **yes**: 14 tests | writer (`0o755`) |
| `crates/qcore-c-gateway/tests/c_numeric_abi_llp64.rs` (62) | `clang-msvc` wrapper | **no**: one test | writer (`0o755`) |
| `crates/qcore-c-gateway/tests/prepared_failure.rs` (29) | `clang-wrapper` | **no**: one test | writer (`0o755`) |
| `crates/qcore-verification/tests/source_effect_helper_target.rs` (57) | `clang-target-wrapper` | **no**: one test | writer |
| `providers/conformance/tests/configured_source_contract.rs` (59) | `fs::copy` of `sample_boolean_provider` | **yes** | `copy_executable` (`0o755`) |
| `providers/conformance/tests/packages.rs` (561, 568, 1118) | the shipped package's `provider` mutated and restored in place; substituted (asserted refused) | **yes** | writer, mode kept |
| `compiler/tests/w3_evidence_v2.rs` (344) | `fs::copy` of the built v2 provider | **no**: one spawning test | `copy_executable` (`0o755`) |

The six named files all had executable writes. Ten more were found, for **16
files**. The prediction listed fifteen of them. It missed `w3_evidence_v2.rs`
(an `fs::copy` of a built provider), which the second census found. Every
row is routed through the writer, including the five whose binary cannot fork
concurrently today. The brief prefers the fix, and the writer keeps those
binaries safe if a spawning test is added.

**Out of scope (data, never executed):** every manifest, response frame, load
path, contract, profile, compile database, C source and log write in these
files. Also out of scope:
- `user_contract_plugins.rs` `copy_tree`: the example crate it copies has no
  executable, and `fs::write` creates `0o644`;
- the non-executable `provider` stand-ins of `plugin_packages.rs`
  ("verification must read it without launching");
- the `.binding` copies in `packages.rs` (147, 242), which are only read;
- the `.h` copies in `ct_corpus.rs` and the other `fs::copy` sites (JSON,
  contracts, headers).

**Production.** `providers/quirin-provider-plugin/src/package.rs`
`package::build` writes the packaged executable with `fs::write`. Its callers
cannot meet the race today. The `quirin-plugin package` CLI is
single-threaded. The one test caller (`providers/quirin-provider-plugin/tests/sdk.rs`)
is the only spawning test of its binary, and it spawns after `build` returns.
The card changes no production path, so this is registered as
`PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS`.
`crates/qcore-audit/src/prepare.rs` `copy_with_times` copies Monkey source
files that the process does not execute.

## 3. Fix

- **The writer moves to the repository root.** It is now
  [`tests/support/process_fixture.rs`](../../../tests/support/process_fixture.rs),
  which follows the `tests/support/tracked_files.rs` convention: it is
  included with `#[path]`, from `providers/` too, and the directory is not a
  test target. It holds:
  - `READ_REQUEST`, unchanged;
  - `write_executable`, `0o700`, unchanged;
  - `write_executable_mode`, for sites that installed `0o755` or must keep a
    package's mode;
  - `copy_executable`, which reads the source and writes through the writer,
    in place of `fs::copy`.
- **The writer reports failures better.** It checks the writer's exit
  status before the input pipe's result. A writer that could not open the
  file now reports its status, not a bare broken pipe.
- **The old path stays valid.** The verifier-host support file is now a
  re-export of the root module. Its five `#[path = "support/process_fixture.rs"]`
  includes, and any include the parallel card adds, keep resolving
  unchanged.
- **Nothing else changes.** There is no new crate or dependency. Production
  code is untouched.

## 4. Proof

### 4.1 Base against tip under the window amplifier

The method is the earlier card's.
[`scripts/amplify.c`](scripts/amplify.c) is its `LD_PRELOAD` close-delay
shim, generalised to a list of path suffixes (or `~substring`). It is
preloaded into the measured binary only and inherited by its children, so at
the tip the writer's `cat` is slowed the same way.
[`scripts/loadbisect.py`](scripts/loadbisect.py) is copied unchanged. Its
forked-child stall stops each child of the test process that has forked but
not yet `exec`ed. [`scripts/campaign.sh`](scripts/campaign.sh) runs each saved
binary with its default test threads from its crate root. Base binaries were
built at `35d43976d` and tip binaries at `b30c9d4e7`, each copied privately
([artifacts/binaries.txt](artifacts/binaries.txt)). Per-run JSON and logs are
in `private/pfw/stress/`, and the summary is in
[artifacts/stress.md](artifacts/stress.md).

| Binary | Amplifier | Base: deviating runs (with ETXTBSY) | Tip |
|---|---|---|---|
| `subcommand_plugins` | close 20 ms, child stall 20 ms | **15 / 50** (9 print it), pilot **7 / 20** (7) | **0 / 50** |
| `cycle20_verifier_plugins` | close 100 ms, child stall 50 ms | **1 / 50** (1), pilot **2 / 20** (2) | **0 / 50** |
| `cycle20_verifier_plugins` | close 20 ms, child stall 20 ms | 0 / 20 | not run |
| `standalone_host` | close 100 ms, child stall 50 ms | 0 / 50 | **0 / 50** |
| `standalone_host` | close 20 ms, child stall 20 ms | 0 / 20 | not run |

Notes on the table:
- **ETXTBSY is the base signature.** In `subcommand_plugins` it appears as
  `plugin-spawn: Text file busy (os error 26)`, and in
  `cycle20_verifier_plugins` as `QP006` with `Text file busy (os error 26)`
  in `vp01_vp02_process_proposals_reach_the_firewall_and_cannot_mint_safety`.
- **Six base deviations carry no printed reason.** Those
  `subcommand_plugins` runs failed only an exit-code assertion
  (`left: Some(1) right: Some(0)`): the test prints no `quirin-audit`
  output, and the report that would name the cause is overwritten by the
  next run. They are counted as deviations, not as ETXTBSY.
- **`standalone_host` cannot be made to fail on the base.** Its only
  spawning test writes and spawns on one thread, and no other test of the
  binary forks, as the census argued.

### 4.2 Every touched binary, repeated at high parallelism

[`scripts/repeat.sh`](scripts/repeat.sh) built the tip's test binaries and
ran each touched binary with `--test-threads 32` from its crate root:
- 10 runs of each root-workspace binary;
- 3 runs of each provider conformance binary, which builds the packages;
- 10 runs of the follow-up example.

The verifier-host binaries that include the moved writer were run too. The
summary is in [artifacts/repeat.md](artifacts/repeat.md), and the logs are in
`private/pfw/repeat/`.

Every run passed except one. That was `w3_evidence_v2` run 1, in which the
test's nested `cargo build` of its standalone provider (in `/tmp`, under the
default toolchain) failed inside rustup: `component download failed for
rust-src: could not rename downloaded file`, while it synced the stable
channel. It failed before the fixture copy, the other 9 runs passed, and it
is a host-toolchain event, not a fixture write.

## 5. Measurement

**Release binary.** No production source changed. The tip's release
`quirin-audit` (`50c3ffe4…`) differs from the base's (`ff080d2e…`) only by
the commit string `crates/qcore-audit/build.rs` embeds. The tip was rebuilt
with `QCORE_AUDIT_BUILD_COMMIT` set to the base's HEAD, and that build is
byte-identical to the base binary (`ff080d2e…`).

**Arbiters.** These were run anyway, each binary on the corpus, system and
slice routes with the null property selected:
- `scripts/arbiters.sh`, with BEFORE the base binary and AFTER the tip
  binary;
- `scripts/identity.py`, which compares the results file for file.

ARBITERS-PENDING

## 6. Gate

GATE-PENDING

## 7. Prediction against outcome

| Predicted | Outcome |
|---|---|
| census: a table of 15 files (the six and nine more; its summary line mis-states this as 15 beyond the six), one production site | 16 files and one production site: `w3_evidence_v2.rs` was missed and found by the second census |
| one shared module at `tests/support/`, verifier-host re-export, no Cargo change | held |
| `cycle20_verifier_plugins` base > 0 (at least 3 of 20), tip 0 | base > 0 held (2 of 20 pilot, 1 of 50), but under the forecast rate. `subcommand_plugins`, a census-added binary, is the strong case (15 of 50). Tip 0 held |
| `standalone_host` base 0 (argued), tip 0 | held (0 of 70, 0 of 50) |
| release binary byte-identical | held only with the base's commit string pinned: the build script embeds HEAD |
| arbiters unchanged | ARBITERS-OUTCOME |
| ledger 764 → 764; rule 16 amended in place | held |

## 8. Follow-ups

- **Retired:** `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE`. Every
  executable fixture in the six files, and in every further site the census
  found, is written through the out-of-process writer. Where a binary cannot
  fork concurrently today, the argument is recorded in §2 as well.
- **Amended:** `P02-FLAKE`.
  `real_packages_preserve_unknown_and_reject_mutations` stays listed: its
  rewrites now use the writer, but its recorded `ExecutableFileBusy` was not
  reproduced here.
- **Registered:** `PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS` (§2,
  production).

Rule 16 ([providers](../../agents/providers.md#constraint-16)) is amended in
place. No new rule id is needed.
