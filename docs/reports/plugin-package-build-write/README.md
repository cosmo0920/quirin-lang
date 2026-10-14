# PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: `package::build` holds no write descriptor on the executable

Card of wave 48 (round 25, third card), base `b565f79ab`. It was rebased
twice:
- onto `5ee141946`, when PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE
  landed. The rebase was clean and left every file of this card unchanged.
  Gate g1 (§9) ran on this tree.
- onto `98eddb50e`, when HOST-REQUEST-READ-BY-DETACHED-DESCENDANT landed. That
  card changed only `crates/qcore-verifier-host`, its Lean module and
  records. This rebase was records-only, and the targeted check (§10) ran
  on it.

The card's commits now are:
- prediction frozen at `72744cd48` ([PREDICTION.md](PREDICTION.md));
- the test alone at `d547c06f2`;
- the fix at `4e388ff8a`.

Before the first rebase they were `29b7ddcab`, `1e445efca` and
`e84a41d49`, and after it `18ffb5c21`, `7ff2afcf9` and `854e42ec9`. The
measurement binaries were built from the originals; the card's own files
are the same in every one of these trees. Production change:
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
| `crates/qcore-audit/src/prepare.rs:1234` `copy_with_times` | `fs::copy` keeps the mode: Monkey's `configure`, `debian.sh`, `qa/run_tests.sh`, libevent's `autogen.sh`, … are copied executable | nobody in the flow: CMake's configure runs `perl` and the compilers, no copied script | none today: `quirin-audit prepare` copies on its only thread before its first `cmake` spawn | follow-up `PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS` |
| `providers/package.py:412-413` `build` | `shutil.copyfile` of each provider, then `chmod(0o755)` | `verify()` runs it right after (`--verify-package`, `--contract-manifest`) | none today: no threading import; every `subprocess.run` is sequential on one thread | follow-up `PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS` |

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
and named in its `Err`.

**USER DECISION (2026-10-14): "Accept tee on PATH".** The mechanism lands as
built. The new Unix requirement is that `package::build` needs `tee` on
`PATH`. It is documented on `build` and named in its `Err`. PREDICTION §3
had put this to the user; that question is now answered.

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

## 7. Records

- **Rule.** The SDK packaging paragraph ("User contract crates") of
  [providers](../../agents/providers.md) is amended in place with the
  mechanism and the USER DECISION. No new rule id.
- **Ledger** 764 → 765 ([`scripts/update_ledger.py`](scripts/update_ledger.py)):
  - `PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS` is retired. Its
    DISCHARGE holds by its first clause: `package::build` writes the
    executable without holding a write descriptor in the calling process.
  - `PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS` is registered, with
    its own DISCHARGE.
  - `PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS` is registered, with its
    own DISCHARGE.
- **Other records.** Increment paragraph in
  [wave-48-part7](../../development/increments/wave-48-part7.md); STATUS
  (follow-up count and the fixture-writer paragraph); docs index.
- **No Lean.**

## 8. Byte identity on the rebased tip

[`scripts/identity_rebased.sh`](scripts/identity_rebased.sh) ran at
`3b8a22c90` under the heavy lock, and every exit was 0
([artifacts/identity-rebased.txt](artifacts/identity-rebased.txt)):
- The rebased tip's `quirin-plugin package` of the saved checker is
  byte-identical, in bytes and mode, to the base CLI's package (`a0c85ff3…`
  `0o755`, `af35cd6b…`). The base CLI verifies it.
- `providers/package.py build` at the rebased tip gives the same
  `packages.json` and the same 104 installed files as at `b565f79ab`.

As the integrator directed, the arbiters were not re-run; the
by-construction argument of §6 stands.

## 9. Gate

[`gate.json`](gate.json) and [`gate/`](gate/) hold the record. The gate
ran once (g1) at `3b8a22c90` (base `b565f79ab`, rebased onto `5ee141946`).
The run used [`scripts/gate-whole.sh`](scripts/gate-whole.sh), which held
one exclusive `flock` on the shared heavy lock for the whole gate (03:21:30Z
to 03:38:55Z), then [`scripts/write_gate_json.py`](scripts/write_gate_json.py)
and [`scripts/finish-gate.sh`](scripts/finish-gate.sh). Logs are in
`private/ppb/gate-g1` of this worktree. Nothing was rerun.

| Step | Exit | Count |
|---|---|---|
| `cargo check --workspace --all-targets` | 0 | |
| submodule update (`--init --recursive`) | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `scripts/run_test_binaries.py` | 0 | 488 units, 501 summaries; **7,956 passed, 0 failed**, 14 ignored (`private/ppb/tests-g1`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `scripts/check_lean_audits.py` | 0 | 317 audits (no Lean change) |
| `python3 -m unittest discover -s scripts/tests` (08, and 09b after the matrix) | 0 / 0 | 365 OK |
| `build_docs_index.py --check` (09, 10c) | 0 / 0 | |
| `build_matrix.py . --gate gate.json` (09a) | 0 | |
| `providers/package.py build` / `verify qcore.libevent` | 0 / 0 | |
| `cargo test` in `providers/` | 0 | **133 passed, 0 failed** (129 before, plus the new `sdk` test and three `package.rs` unit tests; the `sdk` binary finished in 18.18 s) |
| `python3 -m unittest discover -s providers/tests` | 0 | 25 OK |
| `cargo fmt` / `clippy -D warnings` in `providers/` | 0 / 0 | |

Clang: Ubuntu clang version 21.1.8 (6ubuntu1). Toolchain: the host's
rustup stable 1.99.0.

The rule-id duplicate check `rg -o --no-filename '^[0-9]{1,2}[a-z]?\. '
docs/agents/*.md | sort | uniq -d` prints `1.`–`7.` on this tree and on main
alike. The card brief list in `operations.md` (items 1–7) collides with the
plain-numbered rules in `diagnostics.md` and `gateway.md`. This card adds
no numbered line to `docs/agents/`. The bold-form check is empty.

## 10. Rebase onto `98eddb50e` and the targeted check

HOST-REQUEST-READ-BY-DETACHED-DESCENDANT landed after g1. Its commits touch
only `crates/qcore-verifier-host`, Lean `QCore.RequestConsumption` and
records, which are disjoint from this card's code. Under the landing-race
policy the integrator asked for a records-only rebase and a targeted check,
not a second full gate.

**Rebase.**
- STATUS and part 7 conflicted; both cards' paragraphs were kept, with this
  card's increment newest first.
- The acceptance matrix was taken from main and rebuilt with
  [`scripts/update_ledger.py`](scripts/update_ledger.py): 765 entries, equal
  to `dimensions.named_followups`.
- [`scripts/rebase_audit.py`](scripts/rebase_audit.py) is
  `audit_card_lines.sh` in Python, given
  `git diff --output=… 5ee141946 2d7f7fbdc` over the record files. It
  confirms that all 37 record lines the card added are still present and
  that the ledger is as intended. Exit 0.
- No conflict markers remain, and the bold rule-id duplicate check is
  empty.

**Targeted check.** [`scripts/targeted.sh`](scripts/targeted.sh) ran at
`f55d6615d` under `flock` on the heavy lock, from 03:54:13Z to 04:02:53Z.
Logs are in `private/ppb/targeted-t1`.

| Step | Exit | Count |
|---|---|---|
| `cargo check --workspace --all-targets` | 0 | |
| `cargo test` in `providers/` (`--workspace --locked --offline`) | 0 | 133 passed, 0 failed |
| `cargo test -p qcore-verifier-host --locked` | 0 | 149 passed, 0 failed |
| `build_matrix.py . --gate gate.json` | 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 OK |
| `build_docs_index.py`, then `--check` | 0 / 0 | |
