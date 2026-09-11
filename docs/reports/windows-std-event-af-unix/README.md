# EVENT-V1-WINDOWS-AFUNIX — capability-selected Windows endpoint pairs

Date: 2026-09-11.

## Result

The emitted `source.event.v1` Windows shim now prefers a pathname-backed
`AF_UNIX` stream pair when both the build SDK and running Windows system expose
it. It retains the existing private AF_INET loopback pair for an older SDK or a
runtime whose initial `socket(AF_UNIX, SOCK_STREAM, 0)` probe explicitly reports
the family, type or protocol unsupported. Other probe failures refuse.

This is a native-backend change only. The QCore source grammar, checked event
program, descriptor-custody model, certificate, assumptions, diagnostics,
model identities and trace bytes did not change. No contract, provider,
gateway, corpus or Lean surface moved.

## Construction and refusal boundary

The Windows arm follows the socket-pair construction in the repository's
pinned post-2.2.2-alpha Libevent source (`third_party/libevent/evutil.c`):

1. `<afunix.h>` is detected at preprocessing time; its presence is not treated
   as runtime evidence.
2. After Winsock startup, `socket(AF_UNIX, SOCK_STREAM, 0)` probes the running
   provider.
3. Two temporary files reserve fresh listener and connector pathnames. Each
   owned placeholder is deleted, converted from UTF-16 to the UTF-8
   `sockaddr_un.sun_path` spelling, then bound to its endpoint before
   listen/connect/accept.
4. The accepted peer must report `AF_UNIX` and the connector's bound pathname
   before either endpoint is published. This prevents a competing process from
   winning the listener race and being paired with QCore's connector. Every
   partial socket is closed on failure.
5. Published AF_UNIX pairs retain their pathname record until both checked
   endpoint closes complete; the second close deletes the reparse point. An
   `atexit` handler makes a best-effort close and deletion of any remaining
   record on an earlier ordinary-return failure. Abnormal process termination
   and an OS refusal of that final deletion remain outside the checked cleanup
   claim.
6. Only an unsupported-family/type/protocol result at step 2 selects the old
   loopback implementation. Resource/provider probe failures and any later
   AF_UNIX failure return the existing pair-failure status; they are not hidden
   by changing transports.

Windows still has no Winsock `socketpair()` call. Both routes return `SOCKET`s,
wait with `WSAPoll`, transfer the single wake byte with `send`/`recv`, and close
with `closesocket`.

## Evidence and provenance

- Upstream canary motivating this increment: Fluent Bit PR 11829, commit
  `358a21b98934881ac4b5ac05c850e893d179668b`. It imported Libevent
  2.2.2-alpha wholesale and was closed unmerged, so it is prototype evidence,
  not a landed Fluent Bit behavior claim.
- Local implementation oracle: pinned Libevent
  `release-2.2.2-alpha-11-gd82464a2`, `evutil.c` runtime probe and Windows
  AF_UNIX socket-pair helpers.
- Microsoft documents pathname sockets as null-terminated UTF-8 Win32 paths,
  stream support, filesystem-backed bind, and the absence of Winsock
  `socketpair()`:
  <https://devblogs.microsoft.com/commandline/af_unix-comes-to-windows/>.
- Libevent 2.2.2-alpha also repairs an unrelated AF_UNIX accept-path overflow
  in its bufferevent address storage. QCore copied only the small pair-selection
  pattern and binds no Libevent runtime:
  <https://github.com/libevent/libevent/security/advisories/GHSA-cvq5-vrvr-j338>.

## Targeted verification

`compiler/tests/std_event.rs` pins the compile-time header gate, runtime probe,
AF_UNIX helper, fail-closed selection and retained loopback helper. On Windows,
its existing native comparison performs three successful trace runs plus one
intentional early-failure cleanup run:

- `QCORE_EVENT_TEST_REQUIRE_AF_UNIX=1` refuses fallback and therefore proves
  the AF_UNIX construction, WSAPoll readiness, wake/drain and normal pathname
  cleanup path;
- `QCORE_EVENT_TEST_FORCE_LOOPBACK=1` proves the compatibility route is still
  live on an AF_UNIX-capable build;
- `QCORE_EVENT_TEST_DISABLE_AF_UNIX=1` preprocesses out `<afunix.h>` and the
  whole AF_UNIX helper, proving the old-SDK compile route rather than merely
  bypassing a compiled helper;
- `QCORE_EVENT_TEST_ABORT_AFTER_AF_UNIX_PAIR=1` returns through the pair-failure
  status after publication and verifies that the ordinary-exit cleanup leaves
  the temp-directory pathname set unchanged.

All three successful runs must reproduce the same checked reference trace. The target passed
26/26 tests on the Windows host, and a post-run temp-directory inspection found
no remaining `qel*.tmp` or `qec*.tmp` reparse point. The adjacent bitwise
native-run repair passed its exact formerly failing test through the same
established Windows harness.

The full workspace gate was not used while iterating; validation was confined
to the changed native targets, formatting and diff checks.
