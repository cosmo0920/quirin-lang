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
| [`wake_payload.qcore`](wake_payload.qcore) | A **value** that came off an endpoint: the byte a wake delivered, bound by the turn that drained it and piped into a logger line. Card `COMPOSE-EVENT-LINES`. |
| [`write_interest.qcore`](write_interest.qcore) | The check-only arm: a write registration is checked in full and emits no C. |

**Every line in these programs is spelled as a pipeline.** `value |> log info dec;`
is the form, and the statement form it desugars to emits the same C byte for
byte — so this is a rule about how the examples read, not a restriction on the
route.

## `self_pipe.qcore`

```qcore
import std.event;
import std.logger;

fn main() -> i32 effects { io } {
    log_level trace;
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

This example explicitly selects `log_level trace`, so the diagnostic logger
renders the retained event observation on standard output:

```text
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

Standard error is empty. Without `log_level`, the same structured event
sequence is retained by the checker and the null diagnostic sink writes
nothing. The selected level is an explicit source decision; it is not an
unconditional compiler-synthesized program transcript.
Every identity in that trace is a **model**
identity the checker assigned, never a
real file descriptor. A real descriptor is not model-determined — the same
program run twice may get different numbers — so a trace carrying one could not
be compared against a reference at all. This is the discipline card
`STD-FMT-BORROW` fixed for a borrowed render, applied to the same problem, and
it is also why the trace above is identical on all three platform arms.

## `wake_payload.qcore` — the byte a wake delivered

```qcore
import std.event;
import std.logger;
import std.fmt;

fn main() -> i32 effects { io } {
    log_level trace;
    event open;
    endpoints a b;
    watch reader a read;
    wake a 72;
    wake a 105;
    let first = dispatch ready reader;
    first |> log info dec;
    let second = dispatch ready reader;
    second |> log info hex;
    dispatch idle;
    unwatch reader;
    close a;
    close b;
    event close;
    return 0;
}
```

`wake D BYTE;` delivers one declared byte; `wake D;` delivers the route's own.
`let NAME = dispatch ready E;` is the **same turn** `dispatch ready E;` is —
same projection, same determinism decision, same trace record — naming the byte
that turn's callback drained. It is the one value form this route has, and the
value is one the program itself put on the wire.

Two streams, and no order is claimed between them. Standard output carries the
explicitly selected system trace:

```text
QCORE_EVENT_OPEN
QCORE_EVENT_ENDPOINTS 0 1
QCORE_EVENT_WATCH 0 0 READ
QCORE_EVENT_WAKE 0
QCORE_EVENT_WAKE 0
QCORE_EVENT_DISPATCH 0
QCORE_EVENT_DISPATCH 0
QCORE_EVENT_DISPATCH IDLE
QCORE_EVENT_UNWATCH 0
QCORE_EVENT_CLOSE 0
QCORE_EVENT_CLOSE 1
QCORE_EVENT_TEARDOWN
QCORE_EVENT_RESULT 0
```

and standard error carries the two source-authored lines:

```text
[INFO] 72
[INFO] 0x00000069
```

The payload is **not** in the trace. A `wake` projects to no reactor operation
at all, so its byte is not a reactor fact; it is a value, and it reaches an
observation only where a source line renders it. That is why every transcript
on this page published before this program is byte-identical after it. Without
`log_level trace;` the same program writes the two `[INFO]` lines and nothing
else: the retained trace reaches the null sink.

The byte is **verified, not trusted**, exactly as the ready set is. The emitted
program compares what it actually read against the byte the checker predicted
and exits non-zero on disagreement, so the second named premise —

> `EVENT-V1-STREAM-BYTE-ORDER` — a connected stream endpoint pair delivers the
> bytes written into one end to a reader of the other end in the order they were
> written, none lost and none invented, given that this process is the sole
> holder of both endpoints and no other thread touches them.

— produces a failed run if it is false, never a line no model computed. A
program that drains without naming a byte carries this premise no more than a
program with no wait carries the readiness one.

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

An executable module on this route rests on at most **two** named platform
premises, and `quirin check` is not where you find either — the certificate
carries them. A program that performs a wait carries the first; a program that
*names* a drained byte carries `EVENT-V1-STREAM-BYTE-ORDER` above as well, and
one that does neither carries none:

> `EVENT-V1-LEVEL-TRIGGERED-READINESS` — a `poll(2)`/`WSAPoll` wait on a
> socket-pair endpoint with `POLLIN` requested reports `POLLIN` when, and only
> when, a byte written into its peer has not yet been read, given that this
> process is the sole holder of both endpoints and no other thread touches them.

It is a **premise**, not a proof. What keeps it honest is that the emitted
program checks it: a false premise produces a failed run, never a wrong verdict.

## Building and running

Linux and macOS, through the workspace test that compiles the emitted C under
`-Wall -Wextra -Werror -pedantic` and the sanitizers and checks empty stdout plus
the explicitly selected diagnostic log:

```sh
cargo test -p qcore-compiler --test std_event
cargo test -p qcore-compiler --test compose_event_lines
```

The first target runs `self_pipe.qcore`; the second runs `wake_payload.qcore`
and compares **both** streams — the selected system trace on stdout and the two
source-authored lines on stderr — against the reference observation.

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
| Linux and other POSIX | `#else` | `poll(2)` / `socketpair(AF_UNIX, SOCK_STREAM, 0)` | `int` | **Runtime-verified** in the standing gate: compiled `-Werror -pedantic`, run under ASan/UBSan/LSan, with the selected system trace pinned on stdout and stderr quiet. |
| Apple | `__APPLE__` | the same POSIX pair, behind `_DARWIN_C_SOURCE` | `int` | Compile path designed in; the same workspace test runs it when the gate runs on macOS. **Not measured by this card** — residual `EVENT-V1-APPLE-RUN`. |
| Windows | `_WIN32` | `WSAPoll` / AF_UNIX pathname listen-connect-accept, with loopback fallback | `SOCKET` | **Runtime-verified** by `compiler/tests/std_event.rs`: MSVC runs an AF_UNIX-required build, a forced runtime fallback, and a no-`afunix.h` build, pinning the selected system trace on stdout and keeping stderr quiet. `#pragma comment(lib, "ws2_32.lib")` keeps them self-contained; `build-windows.ps1` remains the manual runner. |

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
