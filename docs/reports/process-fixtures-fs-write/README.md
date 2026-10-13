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

Every report, sidecar and null-property stream is byte-identical
([artifacts/identity.txt](artifacts/identity.txt),
[artifacts/totals.txt](artifacts/totals.txt)). `run.json`, `audit.log` and
the invocation records are identical once the run name, the binary digest and
the times are removed.

| Route | BEFORE = AFTER | Null property (unconditional / conditional / Unknown / Unsafe) |
|---|---|---|
| corpus | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | 1,113 / 1,435 / 2,355 / 18 |
| system | 93 / 4 / 351, `32741ce2…` | 84 / 125 / 245 |
| slice | 52 / 1 / 347, 4 Pending, `773ab8b3…` | 71 / 103 / 232 |

The system and slice digests differ from the brief's with equal totals,
because those reports embed the measuring worktree's paths, as earlier cards
recorded. Wall time: corpus 462 s / 463 s, system 70 s / 69 s, slice 36 s /
36 s (`--jobs 4`). No verdict moved.

## 6. Gate

Every code step ran at `04e308303` (implementation `b30c9d4e7` plus the
records), using [`scripts/gate.sh`](scripts/gate.sh) `g1 code`. Exits are in
[gate/](gate/), logs in `private/pfw/gate-g1/`, and the summary in
[gate.json](gate.json).

**The standing gate did not pass on its first run.** Two steps failed. Neither
failure is in a file this card touched, and neither step was rerun to obtain a
pass.

| Step | Exit | Count |
|---|---:|---|
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | **101** | `needless_borrows_for_generic_args` at `crates/qcore-verification/src/source_imports.rs:186`, which is unchanged since the base; see below |
| `python3 scripts/run_test_binaries.py` | **101** | 488 units, 501 summaries; **7,955 passed, 1 failed**, 14 ignored (`private/pfw/tests-g1/`); the failure is `std_socket_inet` (below) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **317** audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | **365** OK (and again after the matrix regeneration, 09b) |
| `python3 scripts/build_docs_index.py --check` | 0 | (and after regeneration, 10b / 10c: 0 / 0); `build_matrix.py . --gate gate.json` 0 |
| provider gate: `providers/package.py build` / `verify qcore.libevent` | 0 / 0 | |
| provider gate: `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | **129 passed**, 0 failed, 1 ignored |
| provider gate: `python3 -m unittest discover -s providers/tests` | 0 | 25 OK |
| provider gate: `cargo fmt` / `cargo clippy` in `providers/` (`-D warnings`) | 0 / 0 | |

**Clippy: the host's toolchain changed mid-card.** The host's rustup
`stable` was reinstalled as **1.99.0** (2026-09-28) at 2026-10-14 02:48 +0900.
That was during this card's repeat runs (§4.2). The failed `w3_evidence_v2`
run printed rustup's channel sync from its nested `cargo build`, so a rustup
proxy took part, but what triggered the update is not established.
- **Every measured binary was built by 1.98.0.** That covers the base and
  tip stress binaries, both release binaries, and the arbiters
  (`rustc version 1.98.0 (88d9e12ae 2026-08-18)` in each binary's `.comment`
  section). 1.98.0 is the toolchain main was gated with.
- **The gate ran under 1.99.0.** Its new `needless_borrows_for_generic_args`
  fires at three production sites, all unchanged since the base:
  - `crates/qcore-verification/src/source_imports.rs:186`;
  - `crates/qcore-audit/src/unit_selection.rs:126`;
  - `crates/quirin-source-analysis/src/condvar_join_census.rs:743`.
- **This card's files are lint-clean.** A full `cargo clippy --locked
  --all-targets` without `-D` (`private/pfw/clippy-warnings.log`) lints every
  crate and reports exactly those three warnings and nothing in a file this
  card touched.
- **1.98.0 is not installed** and nothing was installed, so the gate could
  not be rerun on main's toolchain. The card changes no production code, so
  the three sites are registered as
  `STABLE-TOOLCHAIN-1-99-CLIPPY-NEEDLESS-BORROWS` rather than fixed here.

**`std_socket_inet`.** The failing test was
`the_declared_ipv4_example_runs_sanitized_twice_and_writes_the_reference_observation`:
the emitted program exited 82 with empty stderr on its first run. That
example binds the fixed port 49301. The socket card recorded this collision
with a concurrent run of the suite on the same host as
`SOCKET-INET-FIXED-PORT-COLLISION` ("it fails loudly"). The parallel card's
gate shares this host. Rerun alone afterwards, the binary gave 36 passed,
0 failed (`private/pfw/iso-std_socket_inet.log`, recorded here, not
substituted for the gate's exit). The test compiles and runs C through Clang
and writes no fixture through the changed code.

The workspace count is main's: 7,955 + 1 = 7,956, the HOST-UNREAD gate's
7,956 passed. No test was added or removed.

Rule-id check: `rg -o --no-filename '^[0-9]{1,2}[a-z]?\. ' docs/agents/*.md
| sort | uniq -d` prints `1.` to `7.` at the tip. It printed exactly these at
the base too: they are numbered list items in the topic pages, not rule ids.
This card adds no numbered line.

## 7. Prediction against outcome

| Predicted | Outcome |
|---|---|
| census: a table of 15 files (the six and nine more; its summary line mis-states this as 15 beyond the six), one production site | 16 files and one production site: `w3_evidence_v2.rs` was missed and found by the second census |
| one shared module at `tests/support/`, verifier-host re-export, no Cargo change | held |
| `cycle20_verifier_plugins` base > 0 (at least 3 of 20), tip 0 | base > 0 held (2 of 20 pilot, 1 of 50), but under the forecast rate. `subcommand_plugins`, a census-added binary, is the strong case (15 of 50). Tip 0 held |
| `standalone_host` base 0 (argued), tip 0 | held (0 of 70, 0 of 50) |
| release binary byte-identical | held only with the base's commit string pinned: the build script embeds HEAD |
| arbiters unchanged | held: byte-identical on all three routes and the null property |
| ledger 764 → 764; rule 16 amended in place | 764 → 765: the toolchain follow-up was not foreseen; rule 16 amended in place |
| standing gate passes on the first run | **did not**: clippy under the host's new stable (1.99.0) and the fixed-port `std_socket_inet` row (§6) |

## 8. Follow-ups

- **Retired:** `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE`. Every
  executable fixture in the six files, and in every further site the census
  found, is written through the out-of-process writer. Where a binary cannot
  fork concurrently today, the argument is recorded in §2 as well.
- **Amended:** `P02-FLAKE`.
  `real_packages_preserve_unknown_and_reject_mutations` stays listed: its
  rewrites now use the writer, but its recorded `ExecutableFileBusy` was not
  reproduced here.
- **Registered:**
  - `PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS` (§2, production);
  - `STABLE-TOOLCHAIN-1-99-CLIPPY-NEEDLESS-BORROWS` (§6).

Rule 16 ([providers](../../agents/providers.md#constraint-16)) is amended in
place. No new rule id is needed.
