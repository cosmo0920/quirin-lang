# SOCKET-V1-WINDOWS-RUN — native Winsock execution under Visual Studio

**Card:** `SOCKET-V1-WINDOWS-RUN` (2026-09-12). **Parent:**
`QUIRIN-SOCKET-V1`. **Platform:** Windows x64, Visual Studio 18 Community,
Microsoft C/C++ Optimizing Compiler 19.51.36248 (MSVC tools 14.51.36231).

## Result

The Windows arm of `qcore_socket_compat.h` is now runtime evidence rather than
compile-path-only evidence. The emitted C11 for both published examples was
compiled by Microsoft's actual `cl.exe`, executed against Winsock loopback, and
compared byte for byte with the checked reference observation:

| Program | Native path exercised | Expected stdout | Result |
|---|---|---|---|
| `examples/sockets/tcp_echo.qcore` | `WSAStartup`, TCP create/bind/listen/connect/accept, send/receive, `closesocket` | `101\n0x00000065\n` | exact match; stderr empty |
| `examples/sockets/udp_echo.qcore` | `WSAStartup`, UDP create/bind, sendto/recvfrom with sender check, `closesocket` | `101\n0x00000065\n` | exact match; stderr empty |

A second execution of the TCP program produced identical bytes. No descriptor,
ephemeral port or timing value reached the observation.

## Proof boundary

The Windows-only test path uses `tests/support/windows_c.rs::run_msvc`. That
entry point asks the established `cc`-crate Visual Studio discovery for the
compiler and refuses unless it is both MSVC-family and named `cl`. This excludes
clang-cl from evidence attributed to `cl.exe`.

The compile command supplies no Winsock library argument. The test separately
pins that the emitted source contains
`#pragma comment(lib, "ws2_32.lib")`; successful linking therefore exercises the
dependency declared by the generated Windows arm itself. Successful execution
then crosses the `SOCKET`-rather-than-CRT-fd boundary and the actual Winsock
operations rather than merely preprocessing their declarations.

This card claims native execution and byte agreement, **not sanitizer
coverage**. The bounded waits and distinct emitted failure statuses remain the
parent card's treatment of `SOCKET-V1-LOOPBACK-BUFFERED-SEND`; this run does not
turn that premise into a proof. `SOCKET-V1-APPLE-RUN`,
`SOCKET-V1-EINTR-RESUME`, `SOCKET-V1-CLOSE-ERRNO`, and the other parent-card
follow-ons remain open.

## Tests and continuous gate

Command run on the machine above:

```text
cargo test -p qcore-compiler --test std_socket -- --nocapture
```

Result: **49 passed, 0 failed**, including these Windows-only rows:

- `native_windows::the_tcp_example_runs_under_cl_and_writes_the_reference_observation`
- `native_windows::the_udp_example_runs_under_cl_and_writes_the_reference_observation`
- `native_windows::two_windows_runs_of_one_program_write_identical_bytes`

The Windows `msvc-core` CI lane now includes `--test std_socket` while setting
`QCORE_CLANG` to an unavailable path. The run therefore depends on Visual Studio
and the Windows SDK, not on the existing-C Clang analysis route.

The full workspace gate subsequently exposed a pre-existing Windows-only test
reader failure in `string_slice_io`: its included README had CRLF bytes while
the transcript fence search was LF-spelled. The test now normalizes only its
README input before parsing the fence. This changes neither emitted C nor string
semantics and keeps the same transcript comparison.
