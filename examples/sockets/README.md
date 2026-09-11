# `source.socket.v1` examples — checked TCP and UDP in pure QCore

Two published programs, both written entirely in QCore source and lowered to
self-contained C11 by the existing backend. Neither links a library, neither
declares an extern function, and neither reads a manifest: the whole transport
mechanism is the embedded three-arm shim
`crates/qcore-core/src/qcore_socket_compat.h`.

Card `QUIRIN-SOCKET-V1`; the evidence package is
`docs/reports/quirin-socket-v1/README.md`, the checked model is
`crates/qcore-core/src/std_socket.rs`, and the tests that hold these files to
the `examples/threads` standard are in `compiler/tests/std_socket.rs`.

| File | What it shows |
|---|---|
| [`tcp_echo.qcore`](tcp_echo.qcore) | create / bind / listen / connect / accept, a stream `send` and both `recv` forms, three closes |
| [`udp_echo.qcore`](udp_echo.qcore) | create / bind, `sendto` / `recvfrom`, and the whole-datagram rule |

## The one thing to understand first

**Both ends of every connection are sockets of the same program.** There is no
remote host, no spelled address and no name resolution anywhere in
`source.socket.v1`. That is not a limitation grafted on for testing; it is what
makes the route's two determinism rules decidable:

1. The model carries the exact bytes each `send` commits, so a `recv` the
   program's own history does not already satisfy is **refused at check time**
   (`Q085`) rather than blocking at run time.
2. Because the model holds those bytes, it can say exactly which ones each
   receive takes. The certificate publishes them and **the emitted program
   compares what it actually read against them**, exiting with status 88 on
   disagreement.

Admitting an external peer is the named follow-on `SOCKET-EXTERNAL-PEER`, and it
is a change to the determinism argument rather than to the syntax.

## Nothing that is not model-determined reaches the output

Every `bind` is to port 0 and the kernel's choice is read back with
`getsockname` only to hand this same program's `connect` or `sendto` an address.
No port, no descriptor number, no segment boundary and no timing appears in a
line, a certificate field or an observation record. Two concurrent runs of these
programs — and parallel runs of the whole test suite — therefore write the same
bytes, which is a correctness requirement rather than a convenience.

## Building and running

```sh
quirin check examples/sockets/tcp_echo.qcore
quirin emit examples/sockets/tcp_echo.qcore -o /tmp/tcp_echo.c
cc -std=c11 -Wall -Wextra -Werror -pedantic -g -O0 \
   -fsanitize=address,undefined -fno-omit-frame-pointer /tmp/tcp_echo.c -o /tmp/tcp_echo
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 /tmp/tcp_echo
```

On Windows the same emitted file builds with `cl /W4 /WX` and needs no extra
library on the command line: the Windows arm of the shim names `ws2_32.lib` from
inside the emitted source.

## Transcripts

Both programs write the same two lines — the byte the echo returned, rendered
twice. These blocks are read out of this file by
`compiler/tests/std_socket.rs::the_readme_transcript_is_the_real_one` and
compared with what the pipeline really produces, so they cannot drift.

<!-- transcript: tcp_echo -->
```text
101
0x00000065
```

<!-- transcript: udp_echo -->
```text
101
0x00000065
```

## What each example is careful about

### `tcp_echo.qcore`

* `connect client listener;` names the **listening socket**, not an address. The
  model knows a connection is queued, so the `accept` that follows cannot block —
  an `accept` with nothing queued is refused at `Q085`.
* `let first = recv client;` is the one-byte value form and binds what it took;
  `recv client 4;` drains the rest. Closing a stream socket with bytes still
  unread is refused at `Q085`, because it would reset the connection and the peer
  would observe that at a moment this program does not decide.
* All three sockets are closed. A leak, a double close or a use after close is
  refused at `Q083` by the *unedited* ownership checker — the same cycle-3
  checker the sequential route runs.

### `udp_echo.qcore`

* A datagram is taken whole. `recvfrom server 5;` succeeds because the queued
  datagram is five bytes; `recvfrom server 1;` on the same queue is refused at
  `Q084` rather than truncating it, and the one-byte value form therefore only
  accepts a one-byte datagram.
* A UDP socket is bound but never connected, so the emitted program reads the
  source address `recvfrom` reports and compares it with the bound address of the
  socket the model says sent the datagram. A stray datagram delivered to the same
  ephemeral port fails the run instead of being read as this one.
* `listen` and `connect` on a datagram socket are refused at `Q082`, and `send`
  and `sendto` never stand in for each other.

## What these examples do not show, because it does not exist in v1

* No string or text payload — a payload is a bracketed list of check-time byte
  constants (`SOCKET-STRING-PAYLOAD`).
* No `std.buffer` payload — a module that both buffers and sockets stays with the
  buffer route and keeps that route's refusal (`SOCKET-BUFFER-PAYLOAD`).
* No composition with `source.event.v1` — registering a socket with the reactor
  is `SOCKET-EVENT-COMPOSE`.
* No non-blocking mode, no socket options, no shutdown, no `SO_REUSEADDR`, no
  IPv6, and no concurrency: `source.socket.v1` is single-threaded and a module
  that sockets and spawns keeps its `Q600`.
