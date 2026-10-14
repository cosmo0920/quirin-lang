# PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: `package::build` holds no write descriptor on the executable

Card of wave 48 (round 25, third card), base `b565f79ab`. Prediction frozen
at `29b7ddcab` ([PREDICTION.md](PREDICTION.md)); the test alone at
`1e445efca`; the fix at `e84a41d49`. Production change:
`providers/quirin-provider-plugin/src/package.rs` only. No new crate or
dependency, no `unsafe`, no Lean (an I/O mechanism; no verifier-host or
package-verification semantics change).

## 1. The defect

`package::build` wrote the packaged checker with `fs::write`, then set mode
`0o755`. While that write descriptor is open, a fork by any other thread of
the caller copies it into the child, which keeps it until its own `exec`. An
`execve` of the package in that window fails with `ETXTBSY`
([EXECUTION-QUOTA-LOAD-INDEPENDENCE](../execution-quota-load-independence/README.md)
measured the mechanism on test fixtures). `O_CLOEXEC` does not close the
window, a temporary file plus `rename` keeps the inode that was open for
writing, and Linux has no close-on-fork flag. So the caller must never hold
a write descriptor on that inode.

## 2. Census

Every executable write in production code (workspace and `providers/`),
searched before the code ([PREDICTION.md](PREDICTION.md) §2) and again on the
fixed tree, with the same result. Test and example fixtures are
PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE's.

| Site (base) | Executable written | Executed by | Threads that can fork concurrently | Disposition |
|---|---|---|---|---|
| `providers/quirin-provider-plugin/src/package.rs:97` `package::build` | the packaged checker | the host, `--verify-package`, any caller | **any thread of a library caller**. Today: the `quirin-plugin package` CLI (one thread) and `tests/sdk.rs` (spawns after `build` returns) | **fixed** |
| `crates/qcore-audit/src/prepare.rs:1234` `copy_with_times` | `fs::copy` keeps the mode: Monkey's `configure`, `debian.sh`, `qa/run_tests.sh`, libevent's `autogen.sh`, … are copied executable | nobody in the flow: CMake's configure runs `perl` and the compilers, no copied script | none today: `quirin-audit prepare` copies on its only thread before its first `cmake` spawn | follow-up |
| `providers/package.py:412-413` `build` | `shutil.copyfile` of each provider, then `chmod(0o755)` | `verify()` runs it right after (`--verify-package`, `--contract-manifest`) | none today: no threading import; every `subprocess.run` is sequential on one thread | follow-up |

Not executable writes: the installed manifest (`package.rs`), the user load
path (`main.rs:77`), the template's crate sources (`template.rs:48`,
`0o644`). Scripts that compile observation binaries let Clang write them, out
of process.

## 3. The mechanism

| Candidate | Decision |
|---|---|
| `O_CLOEXEC`, temporary file + `rename`, `O_TMPFILE` + `linkat` | rejected: the executed inode was open for writing in the caller |
| `fork` + `open`/`write`/`_exit` in the child | rejected: `libc` is not an SDK dependency (the conformance pin admits `qcore-schemas`, `serde`, `serde_json`, `quirin-provider-shared` only); hand-written FFI types differ per platform (`mode_t`); it would be the first `unsafe` in `providers/`; and a child that never `exec`s holds a copy of **every** parent descriptor for the whole write |
| re-exec of the plugin's own binary | rejected: `build` is a library function, and a caller's `current_exe` is not `quirin-plugin` |
| the packaged checker as its own writer | rejected: runs the user's untrusted binary while packaging, and changes the checker runtime authors build against |
| **`tee -- FILE`, an exec'd single-threaded writer, fed the bytes on a pipe** | **chosen** |

On Unix, `build` spawns `tee -- FILE` (found on `PATH`) with the bytes it has
already read and digested on standard input, stdout to `/dev/null` and stderr
captured; a scoped thread feeds the pipe while the caller collects the
status, so neither pipe can stall the other. `tee` is POSIX (coreutils,
BusyBox, the macOS base system: `/usr/bin/tee`; NixOS has no `/bin/tee`,
hence `PATH`), single-threaded, forks nothing, and has exited before `build`
continues, so no process holds a write descriptor on the package when it
returns. Then:

1. a writer that cannot start or exits unsuccessfully is an `Err` naming the
   writer, the file, the status and its stderr;
2. the file must be a regular file (not a symbolic link) holding exactly the
   digested bytes, read back; otherwise `Err`;
3. the mode is set to `0o755` by path (`chmod`, no descriptor);
4. the installed manifest is still written last, so every `Err` leaves no
   `verifier-plugin.json` and `verify` refuses the directory (the host
   re-verifies every package it runs regardless).

A `tee` that misbehaves cannot make a wrong package verify: step 2 checks the
bytes, step 3 sets the mode, and the host's own re-verification is
unchanged. Feeding the bytes already digested (not the source path) keeps
today's behaviour for every `--executable`, a FIFO included. A sibling's
forked child that copies the pipe's write end delays `tee`'s end of file
until that child `exec`s; it cannot change the bytes.

Off Unix the write is unchanged (`fs::write`): `ETXTBSY` is a Unix `execve`
rule, and std opens Windows handles non-inheritable.

**Visible to plugin authors:** the signature, package layout, digest,
installed manifest and mode are unchanged, and no item is added to the public
API. The new requirement is `tee` on `PATH` on Unix, documented on `build`
and named in its `Err`. The integrator should decide whether that needs the
user's sign-off (PREDICTION §3).

## 4. Tests

- `tests/sdk.rs`
  `a_multi_threaded_caller_builds_and_executes_packages_while_siblings_spawn`
  (committed before the fix, so the base stress binary has it). Two sibling
  threads spawn `/bin/true` back to back, and both are forking before the
  first build. The test thread builds four packages and **executes each the
  moment `build` returns** (`--verify-package` in the package directory). An
  `ETXTBSY` fails it with the raw OS error. Each package must:
  - report a manifest equal to the installed one;
  - have the source checker's SHA-256 as its digest;
  - have an executable byte-equal to the source;
  - have a `verifier-plugin.json` byte-equal to the template with that digest
    (`to_vec_pretty` plus a newline, today's layout);
  - have mode `0o755`.

  `package::verify` accepts the first package (in a debug build it scans the
  whole 39 MB executable, so it runs once). The prediction said eight
  packages. Four are built because the debug test took 45–100 s at eight
  under the round's load.
- `package.rs` unit tests (Unix):
  - the real writer installs 300,000 bytes exactly, as a regular `0o755`
    file;
  - five failing writers are each an `Err` naming the file and the cause: one
    that cannot start, one that exits 3 (status and stderr quoted), one that
    exits 0 without writing, one that leaves a symbolic link, and one that
    writes other bytes;
  - a build through a writer that half-writes and fails is an `Err`, leaves
    no installed manifest, and `verify` refuses it. A build through the real
    writer gives today's package byte for byte.

## 5. Stress

The harness is EXECUTION-QUOTA-LOAD-INDEPENDENCE's, by way of
PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE:
- [`scripts/amplify.c`](scripts/amplify.c), copied unchanged: an
  `LD_PRELOAD` shim that delays `close` of a write descriptor on a path ending
  `/quirin-plugin-example-checker`. Only the package's executable is opened
  for writing under that name. The shim is inherited, so at the tip `tee`'s
  close is delayed the same way.
- [`scripts/loadbisect.py`](scripts/loadbisect.py), copied with one added
  flag, `--parallel-child-stall`. Its forked-child stall stops each child
  that is still the test image (forked, not yet `exec`ed). The copied stall
  handles children **one at a time**, so it caught about one of the three
  forking threads' children per stall period. The flag gives each stopped
  child its own timer.

[`scripts/stress.sh`](scripts/stress.sh) runs
[`scripts/campaign.sh`](scripts/campaign.sh) on release test binaries built
at `1e445efca` (base) and `e84a41d49` (tip)
([artifacts/binaries.txt](artifacts/binaries.txt)), filtered to the new test.
Per-run JSON and logs are in `private/ppb/stress/`; the summary is
[artifacts/stress.md](artifacts/stress.md).

| Close delay / child stall (parallel) | Base: deviating runs (ETXTBSY) | Tip |
|---|---|---|
| 100 ms / 200 ms | **10 / 50** (10) | **0 / 50** |
| 20 ms / 20 ms | **9 / 50** (9) | **0 / 50** |

Every base deviation is `package N could not be executed right after build:
Text file busy (os error 26)`.

**Why release binaries.** The pilots are in
[artifacts/stress.md](artifacts/stress.md). The debug base binary did not
deviate in 6 runs, with 20 ms / 20 ms and 3 s / 6 s serial stalls. In debug,
the base `build` hashes the 39 MB executable between the close and the
return, and that takes seconds. A child forked during the write has
`exec`ed long before the test runs the package, unless the stall exceeds
the hash time. The serial stall cannot do that and still catch children
forked inside the window. In release the hash of the 2.8 MB checker takes
milliseconds. The serial stall still gave 0 of 15 there; the parallel stall
gave 3 of 10 in the pilot and the figures above.

## 6. Identity

- **SDK package.** The base and tip `quirin-plugin package` of the same
  checker produce byte-identical executables (`a0c85ff3…`, mode `755`) and
  byte-identical `verifier-plugin.json` (`af35cd6b…`). The `package_digest`
  is the input's SHA-256 in both, and the base CLI verifies the tip package
  ([artifacts/sdk-package-identity.txt](artifacts/sdk-package-identity.txt)).
- **First-party packages.** `providers/package.py build` was run at base and
  at tip, each into a fresh output and target directory
  ([`scripts/packages.sh`](scripts/packages.sh)). Both builds exited 0.
  `packages.json` is equal at base and tip. All 104 installed files of the
  five packages are identical in bytes and mode
  ([artifacts/packages.txt](artifacts/packages.txt)). `package.py` does not
  build the SDK, so this is the expected result.
- **Arbiters and the null property.** The card's diff touches no
  root-workspace input: no member source, `Cargo.lock` unchanged, and no
  `include_str!` or `include_bytes!` target. No root crate depends on
  `quirin-provider-plugin`. So `quirin-audit` and every arbiter output
  (corpus 935 / 9 / 3,890, system 93 / 4 / 351, slice 52 / 1 / 347 / 4
  Pending, null property 1,113 / 1,435 / 2,355 / 18; 84 / 125 / 245; 71 /
  103 / 232) are unchanged **by construction**. The arbiters were not re-run
  on the pre-rebase tip.
