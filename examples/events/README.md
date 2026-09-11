# Events — `source.event.v1`

One event context, a registration on a descriptor, dispatch turns that declare
what they report, and a teardown. Card `STD-EVENT`.

The checked half of this route is **not new**: the custody and lifecycle
questions are answered by the generation-indexed reactor and descriptor-custody
models that have been in `crates/qcore-core` since cycle 19, with matching Lean
in `proof/QCore/DescriptorCustody.lean`. Until this card no source program could
reach any of it. What is new is the surface, and one decision of its own — the
determinism boundary below.

Design record: [`docs/design/STD_EVENT_V1.md`](../../docs/design/STD_EVENT_V1.md).

## The programs

| Program | What it shows |
|---|---|
| [`self_pipe.qcore`](self_pipe.qcore) | The executable subset: the program induces its own readiness and every dispatch turn declares what it reports. |
| [`write_interest.qcore`](write_interest.qcore) | The check-only arm: a write registration is checked in full and emits no C. |

## `self_pipe.qcore`

```qcore
import std.event;

fn main() -> i32 effects { io } {
    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    dispatch ready reader;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;
}
```

```
QCORE_EVENT_OPEN
QCORE_EVENT_ENDPOINTS 0 1
QCORE_EVENT_WATCH 0 0 READ
QCORE_EVENT_WAKE 0
QCORE_EVENT_DISPATCH 0
QCORE_EVENT_DISPATCH IDLE
QCORE_EVENT_UNWATCH 0
QCORE_EVENT_CLOSE 0
QCORE_EVENT_CLOSE 1
QCORE_EVENT_TEARDOWN
QCORE_EVENT_RESULT 0
```

Every number in that trace is a **model** identity the checker assigned, never a
real file descriptor. A real descriptor is not model-determined — the same
program run twice may get different numbers — so a trace carrying one could not
be compared against a reference at all. This is the discipline card
`STD-FMT-BORROW` fixed for a borrowed render, applied to the same problem, and
it is also why the trace above is identical on all three platform arms.

## The determinism boundary

A certificate compares **one** observation, so a dispatch turn has to report one
thing. Three devices make that true, and one thing is refused by name:

* `wake D;` writes one byte into the peer of a pair **this program created**, so
  the readiness a turn waits on is readiness this program caused. There is no
  external peer, no timer and no signal in v1.
* `dispatch ready E;` / `dispatch idle;` declare what the turn reports, and the
  emitted program **verifies the declaration** against what `poll` actually
  reported, exiting non-zero on disagreement. The platform premise is checked at
  run time, never trusted.
* A `write` registration is accepted, checked, and **not executed** — its
  readiness is a function of the kernel's send buffer, which no statement here
  induces.

And the refusal:

> Two or more registrations ready in one turn. `poll(2)` reports a *set*, and
> the order a reactor runs one ready set's callbacks in is the implementation's
> rather than one the program induced — the reactor model says so itself
> (`explore_activation_orders`: "equal-priority events are deliberately
> nondeterministic"). Covering such a turn waits on comparison up to an admitted
> reordering; the boundary here does not depend on that and composes with it
> later.

So this is refused, at `Q061`, with that ground in the message:

```qcore
    watch first a read;
    watch second c read;
    wake a;
    wake c;
    dispatch ready first;      // two registrations are ready at this turn
```

## Premises (printed, not discharged)

An executable module on this route rests on exactly one named platform premise,
and `quirin check` is not where you find it — the certificate carries it:

> `EVENT-V1-LEVEL-TRIGGERED-READINESS` — a `poll(2)`/`WSAPoll` wait on a
> socket-pair endpoint with `POLLIN` requested reports `POLLIN` when, and only
> when, a byte written into its peer has not yet been read, given that this
> process is the sole holder of both endpoints and no other thread touches them.

It is a **premise**, not a proof. What keeps it honest is that the emitted
program checks it: a false premise produces a failed run, never a wrong verdict.

## Building and running

Linux and macOS, through the workspace test that compiles the emitted C under
`-Wall -Wextra -Werror -pedantic` and the sanitizers and compares its stdout with
the reference execution:

```sh
cargo test -p qcore-compiler --test std_event
```

Windows, from a Visual Studio Developer PowerShell:

```powershell
./build-windows.ps1 -Run
```

The emitted program is self-contained on every arm: the readiness primitives are
embedded in the emitted C — POSIX `poll(2)` and `socketpair(2)` on Linux and
macOS, `WSAPoll` and an AF_UNIX-first stream pair on Windows — so there is no
QCore include path, no linked QCore artifact, and on Windows no extra library
flag. An older Windows SDK or runtime selects the retained loopback pair.

### Verification status, arm by arm

| Arm | Selected by | Wait / pair | Descriptor | Verification |
|---|---|---|---|---|
| Linux and other POSIX | `#else` | `poll(2)` / `socketpair(AF_UNIX, SOCK_STREAM, 0)` | `int` | **Runtime-verified** in the standing gate: compiled `-Werror -pedantic`, run under ASan/UBSan/LSan, stdout byte-compared with the reference. |
| Apple | `__APPLE__` | the same POSIX pair, behind `_DARWIN_C_SOURCE` | `int` | Compile path designed in; the same workspace test runs it when the gate runs on macOS. **Not measured by this card** — residual `EVENT-V1-APPLE-RUN`. |
| Windows | `_WIN32` | `WSAPoll` / AF_UNIX pathname listen-connect-accept, with loopback fallback | `SOCKET` | **Runtime-verified** by `compiler/tests/std_event.rs`: MSVC runs an AF_UNIX-required build, a forced runtime fallback, and a no-`afunix.h` build and byte-compares each with the reference. `#pragma comment(lib, "ws2_32.lib")` keeps them self-contained; `build-windows.ps1` remains the manual runner. |

The Windows arm is the one whose *implementation* differs rather than only its
declarations, because Winsock has no `socketpair(2)` and `WSAPoll` takes
`SOCKET`s rather than CRT file descriptors. With `<afunix.h>` available, the
shim probes `socket(AF_UNIX, SOCK_STREAM, 0)` at run time and constructs a
pathname-backed listener/connect/accept pair. Only an unsupported
family/type/protocol probe result selects the private AF_INET loopback fallback;
other probe failures and every later AF_UNIX construction failure refuse the
pair instead of changing transports. Both have the same
two-ended, process-owned, one-byte wake/drain semantics. Nothing about their
provenance reaches an observation, because the trace carries model identities
only.
