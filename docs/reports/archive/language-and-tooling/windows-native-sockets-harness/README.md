# WINDOWS-NATIVE-SOCKETS-HARNESS — the wave-24 native runs, made runnable on Windows

Date: 2026-09-12.

## Result

The three wave-24 native execution suites — `source.socket.v1`'s TCP and UDP
echo runs, `source.http.v1`'s exchange and raw-request runs, and the
`source.string.v1` rendering runs — now **compile and execute on Windows**
through the established shared native-C harness, the same
`tests/support/windows_c.rs` that runs `std_event`'s 26 tests there. Before
this card the socket and HTTP native modules were `cfg(any(linux, macos))`: on
Windows they did not exist, so there was nothing for a Windows batch to run.

**This card discharges nothing.** It is test plumbing authored on Linux. It
makes `SOCKET-V1-WINDOWS-RUN`, `HTTP-V1-WINDOWS-RUN` and
`STRING-SLICE-IO-WINDOWS-RUN` *dischargeable* by the user's own Windows
workspace run; the discharge conditions are restated in full below. No
semantic surface moved: no checker, emitter, contract, corpus, provider,
gateway, diagnostic code or reference observation was touched, and no test's
assertions, references or fixtures changed.

## What was wired, and where

| File | Change |
|---|---|
| `compiler/tests/std_socket.rs` | `mod native` is now `cfg(any(linux, macos, windows))`; `run` gained a `cfg(target_os = "windows")` arm through `super::windows_c::run`. The file declares the harness with the same `#[cfg(target_os = "windows")] #[path = "../../tests/support/windows_c.rs"] mod windows_c;` line `std_event.rs` and `std_string.rs` use. |
| `compiler/tests/std_http.rs` | The same two changes, same shape. |
| `compiler/tests/string_slice_io.rs` | Windows arm already present (it was authored with one); one CRLF repair, below. |
| `compiler/tests/std_string.rs` | Windows arm already present and correct; **unchanged**. |
| `tests/support/windows_c.rs` | **Unchanged.** No harness extension proved necessary — see below. |

The Unix arms are byte-untouched in behaviour: the same `-std=c11 -Wall -Wextra
-Werror -pedantic -g -O0 -fsanitize=address,undefined` build, the same
`ASAN_OPTIONS`/`UBSAN_OPTIONS`, the same per-run temporary directory named from
the pid, a monotone counter and a nanosecond clock, and the same removal
afterwards. The counter and `use std::process::Command` moved *inside* the
non-Windows block (a function-local `static` is one instance per program, as in
the harness itself), which is what keeps the Windows build free of unused
items; nothing else in those bodies moved.

## What the Windows arm claims, and what it does not

The Unix arms make two claims at once: a **sanitizer** result (ASan/UBSan, plus
LSan on Linux) and a **byte comparison** against the route's reference
observation. The Windows arm makes only the second. MSVC ships no ASan/UBSan
instrumentation in this harness, so:

- **No sanitizer, leak or undefined-behaviour claim is made for the Windows
  arm.** The existing platform record is unchanged: leak detection stays a
  Linux observation, Apple ASan/UBSan stays an Apple one.
- What the Windows arm does establish, when the user runs it, is that the
  emitted program **builds under MSVC at `/W4 /WX`** and that its stdout is the
  checked reference observation, byte for byte, with stderr empty — plus, for
  the socket and HTTP suites, that two runs of one program write identical
  bytes (no port, descriptor or timing reaches the observation).
- A non-zero exit remains a finding in either direction: the emitted socket and
  HTTP programs report a model/wire disagreement as status 88, a primitive
  failure or an expired bounded wait as their own statuses.

The two example tests in each of the socket and HTTP suites keep the word
`sanitized` in their names (`the_tcp_example_runs_sanitized_…` and siblings).
That name is now accurate for the Unix arm only. It was **deliberately not
changed**: the archived reports `docs/reports/quirin-socket-v1/README.md` and
`docs/reports/http-quirin-v1/README.md` cite those exact test paths, and a
frozen archive is not edited to accommodate a later lane (standing constraint
21). The inaccuracy is therefore recorded here and in both files' module
documentation rather than repaired by a rename, as residual
**`WINDOWS-RUN-TEST-NAME-SANITIZED`**.

## Why no harness extension was needed

Two mechanisms had to be confirmed rather than assumed.

**Winsock linkage.** The socket card put the library name inside the emitted
text: the `_WIN32` arm of `crates/qcore-core/src/qcore_socket_compat.h` carries
`#pragma comment(lib, "ws2_32.lib")` under `_MSC_VER`, exactly as the event
shim does. The harness drives MSVC through `cc::Build`, so the pragma is
honoured by `link.exe` with no flag on the command line — which is the same
path `std_event`'s passing Windows runs already take. No library argument is
added by either new arm, and none is needed.

**Unreferenced internal-linkage functions.** The socket shim is a *subset
library*: a TCP program does not call `qcore_socket_sendto` or
`qcore_socket_recvfrom`, and a UDP program calls neither `qcore_socket_listen`
nor `qcore_socket_accept`. `QCORE_SOCKET_UNUSED` expands to
`__attribute__((unused))` on GCC/Clang and to **nothing** on MSVC, so the
question was whether MSVC `/W4 /WX` would reject the emitted file for an
unreferenced static function (C4505) and make the whole arm unbuildable.

It does not, and the evidence is a measurement that already exists rather than
an argument: the recorded Windows run of `std_event`
(`docs/reports/windows-std-event-af-unix/README.md`, 26/26 on the Windows host)
includes a `-DQCORE_EVENT_TEST_FORCE_LOOPBACK=1` build in which
`qcore_event_pair_afunix` and `qcore_event_windows_afunix_capability` are
compiled but reachable from nothing, and a
`-DQCORE_EVENT_TEST_REQUIRE_AF_UNIX=1` build in which
`qcore_event_pair_loopback` is. Both are plain `static` functions with the same
empty MSVC `UNUSED` macro, and both builds passed through this harness at
`/W4 /WX`. A suppression flag would therefore have been a gate weakened for no
measured reason, and none was added.

What remains genuinely unmeasured from Linux is the rest of the MSVC diagnostic
surface on these two emitted programs — a conversion or shadowing warning at
`/W4` would surface as a build failure on the user's run. That is a **finding
for that run to report**, not a reason to lower `/WX`.

## The CRLF repair

One, in `compiler/tests/string_slice_io.rs`
(`the_published_example_runs_and_matches_its_readme_transcript`). The published
transcript was located by splitting the README on the literal `` ```text\n ``
and then on `'\n'`. In a CRLF checkout — the recurring Windows batch failure
class, recorded for the destructor-contract fixture in
`docs/reports/windows-clang-fixtures/README.md` — the fence spelling does not
match at all and every line carries a trailing `\r`. It now splits on
`` ```text `` and reads the block with `lines().skip(1)`: `skip(1)` drops the
remainder of the fence's own line, which is precisely what the spelled-out
newline used to drop, and `lines()` is newline-agnostic. On an LF checkout the
two are the same list, so the assertion is unchanged where it was already
passing.

No other CRLF hazard was found in the four files. `readme_block` in
`std_socket.rs` and `std_http.rs` already reads its fenced block with `lines()`;
`std_http.rs`'s `split("\r\n")` is over *wire bytes from the certificate*, not
file text, and is correct as it stands. The harness already folds the CRT's
CRLF stream endings back to LF before comparison, and neither route writes a
line containing a CR — the socket observations are `101` and `0x00000065`, the
HTTP ones a status code and a length — so that folding cannot corrupt a
compared line.

## Cross-target verification, and its limits

The `x86_64-pc-windows-msvc` target was installed in this environment, so the
Windows arms were type-checked and linted from Linux rather than only reviewed:

| Command | Outcome |
|---|---|
| `cargo check --target x86_64-pc-windows-msvc -p qcore-compiler --tests` (before the change) | exit 0 — baseline |
| `cargo check --target x86_64-pc-windows-msvc -p qcore-compiler --tests` (after) | exit 0 |
| `cargo clippy --target x86_64-pc-windows-msvc -p qcore-compiler --all-targets -- -D warnings` | exit 0 |

`cargo check` needs no linker, so the absence of MSVC here is not an obstacle.
The check was proved **live** rather than cached: a deliberate type error was
inserted inside the new `cfg(target_os = "windows")` block of
`std_socket.rs::native::run`, the cross-check reported it at the exact line
(E0308), and the file was restored. That rules out the failure mode where a
`cfg`-gated arm is never compiled by anything and rots.

Its limit is exact and worth stating: this verifies the **Rust** of the Windows
arms — that it compiles, borrows and lints for that target. It verifies nothing
about MSVC's treatment of the **C** those tests hand it, and nothing about the
runtime behaviour of Winsock. Both belong to the user's Windows run.

## Residuals and their new discharge conditions

| Residual | Discharge condition, as of this card |
|---|---|
| `SOCKET-V1-WINDOWS-RUN` | The user's Windows workspace run at or after this card executes `compiler/tests/std_socket.rs::native::the_tcp_example_runs_sanitized_and_writes_the_reference_observation`, `…::the_udp_example_runs_sanitized_and_writes_the_reference_observation` and `…::two_runs_of_one_program_write_identical_bytes` and they pass. That discharges the **byte-comparison** half only; the sanitizer half stays a Unix observation. |
| `HTTP-V1-WINDOWS-RUN` | The user's Windows workspace run at or after this card executes `compiler/tests/std_http.rs::native::the_exchange_example_runs_sanitized_and_writes_the_reference_observation`, `…::the_raw_example_runs_sanitized_and_writes_the_reference_observation` and `…::two_runs_of_one_program_write_identical_bytes` and they pass. Byte-comparison half only, as above. |
| `STRING-SLICE-IO-WINDOWS-RUN` | The user's Windows workspace run at or after this card executes `compiler/tests/string_slice_io.rs::the_emitted_program_matches_the_reference_per_stream` and `…::the_published_example_runs_and_matches_its_readme_transcript` and they pass. The second of those is the one the CRLF repair above unblocks in a CRLF checkout. |
| `STRING-SLICE-WINDOWS-RUN` (the base route, not a named wave-24 residual) | `compiler/tests/std_string.rs::the_emitted_program_matches_the_reference_per_stream` and `…::the_silent_program_runs_natively_and_writes_nothing` were already wired; this card confirmed them and changed nothing there. |
| `SOCKET-V1-APPLE-RUN`, `HTTP-V1-APPLE-RUN`, `EVENT-V1-APPLE-RUN` | Untouched. The Apple arms already existed and this card neither ran nor claimed them. |
| `WINDOWS-RUN-TEST-NAME-SANITIZED` (new) | Two socket and two HTTP tests keep `runs_sanitized` in their names while their Windows arm makes no sanitizer claim. A rename would require editing two frozen reports that cite those paths; the accurate statement lives in each file's module documentation and in this report. |

Nothing here is promoted, and no count moved: the pinned 185-unit corpus, the
leak figure of 272, the CT mapping and the contract corpus are all untouched by
a change confined to three test files' execution plumbing.

## Ephemeral-port and temporary-directory discipline

Preserved on both arms, and it is load-bearing because the suite runs its
binaries in parallel. Every socket in every fixture binds to port 0 and the
kernel's choice is read back only by this same program's `connect`/`sendto`, so
two concurrent runs cannot collide — that is a property of the emitted program
and is therefore identical on Windows. The Windows arm builds in
`std::env::temp_dir()/qcore-windows-threads-<pid>-<counter>` and removes the
directory after the run, the harness's existing discipline; the Unix arms keep
their own pid/counter/nanosecond directory. Neither arm writes into the shared
session scratchpad or under `target/`.

## Lean

**No Lean change, and the close is legitimate without one.** This card moved no
semantic surface: no definition, theorem, model, diagnostic, emitted byte or
published verdict changed, and the only non-`cfg` edit outside test plumbing is
a newline-agnostic README parse. The Lean audit gate was run unchanged as part
of the standing gate below and is reported by text, not by inference.

## Standing gate (Linux)

| Step | Result |
|---|---|
| `cargo test --workspace` | **3,854 passed, 8 ignored** across 185 result lines, one flaky row (below) |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **103 of 103 Lean audits passed**, exit 0 (`$HOME/.elan/bin` on PATH) |
| `cargo check`/`clippy` for `x86_64-pc-windows-msvc` | exit 0 (above) |

Two run-order notes, both stated rather than smoothed over:

1. The first workspace run aborted at `libevent_submodule` because this
   worktree's `third_party/libevent` had never been initialised. That is the
   gate's own documented prerequisite, not a finding:
   `git submodule update --init --recursive` checked out the pinned
   `d82464a277d0f42703702c4dfd9af6af38595a83`, and the numbers above are the
   complete run after it.
2. One failure in the complete run,
   `qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`,
   failed with `Io(Broken pipe)` where it expects `Terminated`. That is
   verbatim the third **P02-FLAKE** row of standing constraint 16 — the
   load-sensitive confined-fixture write test — and it was verified in
   isolation as that constraint requires:
   `cargo test -p qcore-verifier-host --test confined_process_execution --
   --test-threads=1` passes **12 of 12**. Nothing was weakened to make it pass.

The four target suites all pass on Linux with their Unix arms unchanged:
`std_socket` 49, `std_http` 67, `std_string` 35, `string_slice_io` 27, with
`native::the_tcp_example_runs_sanitized_and_writes_the_reference_observation`,
`native::the_udp_example_…`, `native::the_exchange_example_…`,
`native::the_raw_example_…` and both `native::two_runs_of_one_program_write_identical_bytes`
among them.

## Follow-ons

1. `WINDOWS-RUN-TEST-NAME-SANITIZED` — decide whether the four `runs_sanitized`
   test names are renamed once a card is willing to publish provenance beside
   the two frozen reports that cite them.
2. The MSVC `/W4` diagnostic surface of the emitted socket and HTTP programs is
   unmeasured; the first Windows run is its measurement, and any `/W4`
   diagnostic it reports is a finding about the emitted C, not a reason to
   lower `/WX` in the harness.
3. The Apple arms of all three suites remain unrun.

## Integration note (2026-09-12, integrator)

While this card was in flight, the user's Windows host landed commit
`360d9464` — its own validated `native_windows` module for
`compiler/tests/std_socket.rs` plus a `tests/support/windows_c.rs`
extension, with real MSVC runs closing `SOCKET-V1-WINDOWS-RUN`
(`docs/reports/socket-v1-windows-run/README.md`). At integration the
socket half of this card was therefore **withdrawn in favour of the
validated wiring**: `std_socket.rs` carries the user's version, and this
card's contribution stands for `std_http.rs` (the Windows arm and its
discharge condition for `HTTP-V1-WINDOWS-RUN`) and
`string_slice_io.rs` (the CRLF transcript repair;
`STRING-SLICE-IO-WINDOWS-RUN`'s condition unchanged). §"What was wired"
above describes the card's own branch, not the merged tree, for the
socket file.
