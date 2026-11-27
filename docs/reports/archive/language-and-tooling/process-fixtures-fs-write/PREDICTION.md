# PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE: prediction

Frozen before any implementation code. Base `b565f79ab` (local `main`). Only
this file exists at the freeze.

## 1. The mechanism (from EXECUTION-QUOTA-LOAD-INDEPENDENCE)

A test thread that writes an executable with `fs::write` (or `fs::copy`)
holds a write descriptor on that inode while it writes. If another thread of
the same process forks in that window, the forked child keeps a duplicate
until its own `exec` (`O_CLOEXEC` closes it only then). An `execve` of the
file while any such duplicate is open fails with `ETXTBSY`. The repair in
`qcore-verifier-host` was `tests/support/process_fixture.rs`
`write_executable`: the bytes go through `/bin/sh -c 'cat > "$1"; chmod'`, so
the only write descriptor belongs to a single-threaded `cat` that forks
nothing and has exited before the call returns.

The race needs two things in one process: a write descriptor on a file that
is later executed, and a fork by another thread while it is open. A file that
is never executed (a manifest, a response frame, C source, a non-executable
copy of an image) cannot meet `ETXTBSY` and is out of scope.

## 2. Census (whole workspace, frozen)

Searched: every `set_permissions` / `PermissionsExt` / `from_mode` /
`OpenOptionsExt` / `.mode(0o…)` site, every `fs::copy`, and every in-place
rewrite of a file that is already executable. "Concurrent fork" means another
test of the same binary (libtest threads) spawns a process while the write can
be open.

| # | File:line (base) | Executable written | Executed by | Concurrent fork in the binary |
|---|---|---|---|---|
| 1 | `crates/qcore-verifier-host/tests/standalone_host.rs:168` | `provider.sh`, five bodies (`write_script`) | the host, in the test thread | **no**: 3 tests, only this one spawns, and it writes before it spawns on the same thread |
| 2 | `compiler/tests/cycle20_verifier_plugins.rs:331` | `install_verifiable` rewrites the script in place with its envelope | the host | **yes**: 20 tests, four spawn process providers |
| 2 | `…:357` | `hang.sh` | the host | yes |
| 2 | `…:433` | `provider` | discovery verifies it (reads only); not spawned | yes |
| 2 | `…:611` | `provider.sh` | the host | yes |
| 2 | `…:758` | `provider.sh`, `write_script` (six bodies) | the host, on a helper thread | yes |
| 3 | `compiler/examples/reevaluation_followup.rs:92,116,138` | `provider.sh`, three bodies | the host | **no**: a single-threaded `main`; every fork is issued by the main thread after the write returned; the host's reaper threads only `waitpid` |
| 4 | `crates/qcore-audit/tests/user_contract_plugins.rs:173` | `check.sh` | the `quirin-audit` subprocess, and the library route in-process | **yes**: 7 tests, each spawns `quirin-audit` |
| 4 | `…:468` | `check.sh` rewritten in place (asserted never to run) | nobody, by design | yes |
| 5 | `providers/conformance/tests/configured_audit.rs:112` | `fs::copy` of the built provider into `package/provider` | `run_config`, in-process | **yes**: 7 tests; `git`, Clang and the provider are spawned |
| 5 | `…:333` | `package/provider` rewritten in place | `run_config` | yes |
| 6 | `crates/qcorec/tests/plugin_packages.rs:17` | `install` writes `provider`; executable (`0o755`, `:306`) only in `configured_source_scope_dispatches_only_the_enabled_process` | the `quirin` subprocess | **yes**: 7 tests, each spawns `quirin` |
| 7 | `crates/qcore-verifier-host/tests/package_identity.rs:469` | `decoy.sh` (asserted never spawned) | nobody, by design | yes: 10 spawning tests |
| 7 | `…:490` | the verified `provider.sh` rewritten (asserted refused before spawning) | nobody, by design | yes |
| 8 | `crates/qcore-audit/tests/configured_cli.rs:141` | `counting-clang` wrapper | `quirin-audit` subprocess via `QCORE_CLANG` | **yes**: 17 tests spawn `quirin-audit` |
| 9 | `crates/qcore-audit/tests/prepare_cli.rs:60` | stand-in `clang`, `clang++`, `not-clang`, `cmake` | `quirin-audit prepare` subprocess | **yes**: 5 tests spawn `quirin-audit` |
| 10 | `crates/qcore-audit/tests/subcommand_plugins.rs:40` | `quirin-audit-NAME` plugin scripts | `quirin-audit` subprocess | **yes**: 14 tests |
| 11 | `crates/qcore-c-gateway/tests/c_numeric_abi_llp64.rs:62` | `clang-msvc` wrapper | Clang calls in the test thread | **no**: one test |
| 12 | `crates/qcore-c-gateway/tests/prepared_failure.rs:29` | `clang-wrapper` | Clang calls in the test thread | **no**: one test |
| 13 | `crates/qcore-verification/tests/source_effect_helper_target.rs:57` | `clang-target-wrapper` | a child copy of the test binary | **no**: one test (the child runs with one thread) |
| 14 | `providers/conformance/tests/configured_source_contract.rs:59` | `fs::copy` of `sample_boolean_provider` | the configured route, in-process | **yes**: 6 tests |
| 15 | `providers/conformance/tests/packages.rs:561,568` | the shipped package's `provider`, mutated then restored in place | `raw()` and the host | **yes**: 6 tests |
| 15 | `…:1118` | `provider` substituted (asserted refused before spawning) | nobody, by design | yes |

Data, out of scope (never executed): every `verifier-plugin.json`,
`response.bin`, load path, contract, profile, compile database, C source and
log write in these files; `user_contract_plugins.rs:61` `copy_tree` (the
copied example crate has no executable, and `fs::write` creates `0o644`);
`plugin_packages.rs:17` in every other test and `:217` (deliberately not
executable: "verification must read it without launching");
`packages.rs:147,242` (a non-executable `.binding` copy that is only read).

Production: `providers/quirin-provider-plugin/src/package.rs:97`
(`package::build`) writes the packaged executable with `fs::write`. Its
callers are the single-threaded `quirin-plugin package` CLI and one test
(`providers/quirin-provider-plugin/tests/sdk.rs`, whose binary has one
spawning test, the one that builds). It is not reachable concurrently today.
The card changes no production path, so this is recorded as a follow-up, not
fixed.

**Predicted size: 15 files outside the six named plus the production site;
the ledger's six all have at least one executable write.**

## 3. Fix (predicted)

* One shared module, `tests/support/process_fixture.rs` at the repository
  root (the convention of `tests/support/tracked_files.rs`, included with
  `#[path]`, including from `providers/`), holding `READ_REQUEST`,
  `write_executable` (`0o700`, unchanged) and a mode-taking variant for the
  sites that install `0o755`. The verifier-host copy becomes a thin re-export
  so that its five includes (and any include a parallel card adds) keep
  resolving. No new crate, dependency, `Cargo.toml` or `Cargo.lock` change.
* Every executable row above routes through it, the argued rows too
  ("prefer the fix"). Data rows stay `fs::write`.
* No production code, analyzer, gateway, host or contract change; no Lean.

## 4. Proof (predicted)

* Window amplifier: the earlier card's `LD_PRELOAD` close-delay shim,
  generalised to a list of file-name suffixes, plus its forked-child stall,
  on `cycle20_verifier_plugins`: **base > 0 deviating runs** (predicted at
  least 3 of 20, `QP0xx` or `Spawn("Text file busy")` shapes in the four
  spawning rows); **tip 0** over the same number of runs.
* `standalone_host` under the same amplifier: **base 0** (no concurrent
  fork, the argued row), tip 0. That is the recorded explanation for a base
  that cannot be made to fail.
* Each touched test binary run repeatedly at `--test-threads 32`: 0
  failures at the tip.

## 5. Measurement and gates (predicted)

* `quirin-audit` release binary byte-identical at base and tip (no
  production source changes), so the corpus (935 / 9 / 3,890, `bdf5f1c8…`),
  system (93 / 4 / 351), slice (52 / 1 / 347 / 4 Pending) and null property
  (corpus 1,113 / 1,435 / 2,355 / 18 Unsafe; system 84 / 125 / 245; slice 71
  / 103 / 232) are unchanged.
* Test counts unchanged (no test added or removed). Standing gate and
  provider gate pass on the first run.
* Ledger: `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE` retired; one
  follow-up registered for the production `package::build` write; 764 → 764.
* Rule 16 amended in place (its sentence naming the fixture writer); no new
  rule id.
