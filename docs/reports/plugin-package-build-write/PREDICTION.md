# PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS: prediction

Frozen before any implementation code. Base `b565f79ab` (local `main`). Only
this file exists at the freeze.

## 1. The mechanism

`package::build` (`providers/quirin-provider-plugin/src/package.rs:97`)
writes the packaged checker with `fs::write`, then sets mode `0o755`. While
that write descriptor is open, a fork by any other thread of the calling
process copies it into the child, which keeps it until its own `exec`
(`O_CLOEXEC` closes it only then). An `execve` of the package while such a
duplicate is open fails with `ETXTBSY`. Neither `O_CLOEXEC` nor a temporary
file plus `rename` closes the window: the inode keeps its writer count while
any duplicate lives. Linux has no close-on-fork flag. So the fix cannot
depend on how the calling process's descriptor is opened: **the calling
process must never hold a write descriptor on the executable's inode.**

## 2. Census (production code, workspace and `providers/`, frozen)

Searched: every `set_permissions` / `PermissionsExt` / `from_mode` /
`set_mode` / `OpenOptionsExt` / `.mode(0o…)` site, every `fs::copy` /
`hard_link` / `copy_file`, every `fs::write`/`File::create` in the plugin
crate and `providers/*/src`, and the Python packaging scripts' `chmod` /
`copyfile` / `copy2`. Tests and examples are the sibling card's
(PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE) and are out of scope here.

| # | Site (base) | Executable written | Executed by | Threads that can fork concurrently |
|---|---|---|---|---|
| 1 | `providers/quirin-provider-plugin/src/package.rs:97` `package::build` | the packaged checker, `fs::write` then `0o755` | the host (re-verifies, then spawns), `--verify-package`, any caller | **any thread of a library caller** (public API). Today: the `quirin-plugin package` CLI (one thread) and `tests/sdk.rs` (four tests; the only spawning one spawns after `build` returns) |
| 2 | `crates/qcore-audit/src/prepare.rs:1234` `copy_with_times` | `fs::copy` keeps the mode, so Monkey's executable files (`configure`, `debian.sh`, `qa/run_tests.sh`, libevent's `autogen.sh`, …) are copied executable | nobody in the preparing flow: CMake's configure runs `perl`/compilers, never a copied script | **none today**: `quirin-audit prepare` copies on its only thread, sequentially, before its first `cmake` spawn |
| 3 | `providers/package.py:412-413` `build` | `shutil.copyfile` of each provider, then `chmod(0o755)` | `verify()` runs it (`--verify-package`, `--contract-manifest`) right after | **none today**: the script imports no threading module; every `subprocess.run` is sequential on its one thread |

Not executable writes (data, never executed): `package.rs:108` (the installed
manifest), `main.rs:77` (the user load path), `template.rs:48` (crate
sources, `0o644`), every report/log/JSON writer in the workspace. Scripts that
compile observation binaries let Clang write them, out of process.

Rows 2 and 3 are outside this crate and not trivially the same fix, so they
become named follow-ups with their own DISCHARGE (predicted ids
`PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS`,
`PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS`).

## 3. Mechanism chosen (predicted)

Candidates, weighed:

| Mechanism | Verdict |
|---|---|
| `O_CLOEXEC`, temp file + `rename`, `O_TMPFILE` + `linkat` | **no**: the inode the host execs was open for writing in this process |
| `fork` + `open`/`write`/`_exit` in the child (raw FFI or `libc`) | **no**: `libc` is not an SDK dependency (the conformance pin admits only `qcore-schemas`, `serde`, `serde_json`, `quirin-provider-shared`), hand-declared FFI types differ per platform (`mode_t`), it would be the first `unsafe` in `providers/`, and a non-exec'ing child duplicates *every* parent descriptor for the whole write |
| re-exec of the plugin's own binary | **no**: `package::build` is a library function; a caller's `current_exe` is not `quirin-plugin` |
| re-exec of the packaged checker as its own writer | **no**: runs the user's untrusted binary during packaging and changes the checker runtime (plugin-author-visible) |
| **an exec'd single-threaded writer, `tee`, fed the bytes on a pipe** | **yes** |

The writer: `Command::new("tee").arg("--").arg(file)`, stdin piped, stdout
null, stderr captured. The bytes are written on a scoped thread while the
caller waits, so a long stderr cannot deadlock. `tee` is POSIX, in
coreutils, BusyBox and macOS's base system, single-threaded, and forks
nothing, so its descriptor is gone when it has exited, which is before
`build` continues. It is resolved through `PATH` (it is `/usr/bin/tee` on
macOS and not under `/bin` on NixOS); a missing or wrong `tee` cannot make a
wrong package verify, because:

1. a writer that cannot start, or exits unsuccessfully, is an `Err` naming
   the writer, the file and its stderr;
2. the file is read back and must be a regular file (not a symbolic link)
   whose bytes equal the bytes digested; otherwise `Err`;
3. the mode is then set to `0o755` by path (`chmod`, no descriptor);
4. the installed manifest is written last, so any `Err` leaves no manifest
   and `verify` refuses the directory; the host re-verifies every package
   it runs regardless.

Pipe, not path: feeding the bytes already digested keeps today's behaviour
for every `--executable` (a FIFO or process substitution is read once, as
today). A pipe duplicated into a sibling's forked child only delays `tee`'s
end of file until that child's `exec`; it cannot change the bytes.

Off Unix the write is unchanged (`fs::write`): `ETXTBSY` is a Unix
`execve` rule and std opens Windows handles non-inheritable.

The signature, output layout, digest, installed manifest and mode are
unchanged. The one new requirement, `tee` on `PATH` on Unix, is documented
on `build` and named in its `Err`. **If the integrator judges that requirement
an author-visible behaviour change, this is the decision to put to the
user.**

## 4. Tests (predicted)

* `tests/sdk.rs`: a multi-threaded caller (two sibling threads spawning
  `/bin/true` back to back) builds eight packages and immediately executes
  each (`--verify-package` in the package directory, whose output must equal
  the installed manifest); each package verifies, its digest is the SHA-256
  of the source checker, its installed manifest is byte-for-byte the
  template with that digest (today's layout), and its mode is `0o755`.
* Unit tests in `package.rs`: a writer that cannot start, one that exits
  unsuccessfully, one that exits successfully without writing and one that
  writes other bytes are each an `Err` naming the cause; a build through a
  failing writer leaves no installed manifest and `verify` refuses it.

The sdk test is committed **before** the fix, so the base stress binary
carries the same test.

## 5. Stress (predicted)

Harness: EXECUTION-QUOTA-LOAD-INDEPENDENCE's `amplify.c` close-delay shim (as
generalised by PROCESS-FIXTURES-FS-WRITE) preloaded into the `sdk` test
binary, delaying the close of a write descriptor on
`/quirin-plugin-example-checker` by 20 ms, plus `loadbisect.py`'s forked-child
stall (20 ms) on the test binary's own children.

| Binary | Runs | Predicted deviating runs |
|---|---|---|
| base (`sdk` test at the test-only commit) | 30 | **> 0**, signature `Text file busy (os error 26)` |
| tip | 30 | **0** |

## 6. Measurement (predicted)

* No root-workspace crate links `quirin-provider-plugin`, so the release
  `quirin-audit` is byte-identical to base once its embedded commit string is
  pinned; corpus 935 / 9 / 3,890, system 93 / 4 / 351, slice 52 / 1 / 347 /
  4 Pending and the null property (1,113 / 1,435 / 2,355 / 18; 84 / 125 /
  245; 71 / 103 / 232) do not move.
* `providers/package.py build` does not use the SDK: its five packages'
  executable, manifest and archive digests are identical base to tip.
* `quirin-plugin package` of the same example checker gives the same
  `package_digest` and byte-identical `verifier-plugin.json` and executable at
  base and tip.

## 7. Lean

None: an I/O mechanism inside the SDK. No verifier-host or
package-verification semantics change.
