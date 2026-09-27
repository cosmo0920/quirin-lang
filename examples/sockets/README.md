# `source.socket.v1` examples — checked TCP and UDP in pure QCore

Four published programs, all written entirely in QCore source and lowered to
self-contained C11 by the existing backend. None links a library, none declares
an extern function, and none reads a manifest: the whole transport mechanism is
the embedded three-arm shim
`crates/qcore-core/src/qcore_socket_compat.h`.

Cards `QUIRIN-SOCKET-V1` and `SOCKET-INET-ADDRESSES`; the evidence packages are
`docs/reports/quirin-socket-v1/README.md` and
`docs/reports/socket-inet-addresses/README.md`, the checked model is
`crates/qcore-core/src/std_socket.rs`, and the tests that hold these files to
the `examples/threads` standard are in `compiler/tests/std_socket.rs` and
`compiler/tests/std_socket_inet.rs`.

| File | What it shows |
|---|---|
| [`tcp_echo.qr`](tcp_echo.qr) | create / bind / listen / connect / accept, a stream `send` and both `recv` forms, three closes |
| [`udp_echo.qr`](udp_echo.qr) | create / bind, `sendto` / `recvfrom`, and the whole-datagram rule |
| [`inet_v4_echo.qr`](inet_v4_echo.qr) | a **declared** IPv4 address and port — inbound `bind S 127.0.0.1:port;` and outbound `connect C 127.0.0.1:port;` |
| [`inet_v6_echo.qr`](inet_v6_echo.qr) | the same over declared IPv6 `[::1]`, with `IPV6_V6ONLY` set explicitly and a host without IPv6 answered by name |

## The one thing to understand first

**Both ends of every connection are sockets of the same program.** There is no
remote host and no name resolution anywhere in `source.socket.v1`. An address a
program *declares* does not change that: `connect client 127.0.0.1:port;` is
admitted only because a `bind` of this same program declares exactly that
address, and the checker resolves it to that socket at check time. An address no
bind of this program answers to is refused at `Q085` by name, and a hostname is
refused at `Q080` by name. This is not a limitation grafted on for testing; it is
what makes the route's two determinism rules decidable:

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

`tcp_echo` and `udp_echo` write no port down at all: every `bind` is to port 0
and the kernel's choice is read back with `getsockname` only to hand this same
program's `connect` or `sendto` an address. `inet_v4_echo` and `inet_v6_echo`
declare a port, and that port is a **check-time constant** — so it is still not a
run-time quantity, and it still never appears in a line or an observation record.
No descriptor number, no segment boundary and no timing appears in any of the
four.

What a declared fixed port does change is collision: two programs on one host
cannot both hold `127.0.0.1:49301`. That is answered loudly rather than quietly —
a collision *inside* one program is refused at check time (`Q082`), and a
collision with anything outside it fails the run with the distinct bind status
rather than producing a different observation. The port-0 forms remain the
collision-free spelling and remain the default.

## Building and running

```sh
quirin check examples/sockets/tcp_echo.qr
quirin emit examples/sockets/tcp_echo.qr -o /tmp/tcp_echo.c
cc -std=c11 -Wall -Wextra -Werror -pedantic -g -O0 \
   -fsanitize=address,undefined -fno-omit-frame-pointer /tmp/tcp_echo.c -o /tmp/tcp_echo
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 /tmp/tcp_echo
```

On Windows the same emitted file builds with `cl /W4 /WX` and needs no extra
library on the command line: the Windows arm of the shim names `ws2_32.lib` from
inside the emitted source.

## Transcripts

All four programs write the same two lines — the byte the echo returned,
rendered twice. These blocks are read out of this file by
`compiler/tests/std_socket.rs::the_readme_transcript_is_the_real_one` and
`compiler/tests/std_socket_inet.rs::the_readme_transcript_is_the_real_one`, and
compared with what the pipeline really produces, so they cannot drift.
`compiler/tests/examples_suite.rs` reads the same two blocks under the same
markers and holds the table above to the directory — every program indexed,
every row a file that exists — without building either program a second time.

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

<!-- transcript: inet_v4_echo -->
```text
101
0x00000065
```

<!-- transcript: inet_v6_echo -->
```text
101
0x00000065
```

## What each example is careful about

### `tcp_echo.qr`

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

### `udp_echo.qr`

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

### `inet_v4_echo.qr` and `inet_v6_echo.qr`

* A **declared address** is a check-time constant: a literal IPv4 dotted quad or
  a bracketed IPv6 literal, and a port that is an integer literal or a name this
  entry bound to one. It is parsed **once**, by the checker, and the emitted C
  carries the address' octets — there is no `inet_pton` call in an emitted
  program, so no second parser can read a spelling differently from the first.
* The two address families are **disjoint**. A socket is created in exactly one
  of them, connecting an IPv4 socket to an IPv6 endpoint is refused at `Q082`,
  and every IPv6 socket the emitted program creates sets `IPV6_V6ONLY` to 1
  **explicitly** — Linux defaults it off and Windows defaults it on, so an
  inherited default would mean the same program binds different things on
  different hosts.
* A declared bind **succeeds when no live socket holds the address and fails when
  one does**, on every platform — one decided property, spelled differently per
  arm because the platforms need opposite options to reach it. POSIX sets
  `SO_REUSEADDR` (without it a second run inside the first run's `TIME_WAIT`
  cannot rebind, which is not a live holder); Winsock deliberately does **not**,
  because there `SO_REUSEADDR` permits a second *live* socket on the same address.
  Neither arm sets `SO_REUSEPORT`.
* A host without IPv6 is answered rather than assumed: the emitted program asks
  the running kernel for the socket, and an explicit "address family not
  supported" result exits with its own distinct status (92) having written
  nothing. It does **not** fall back to IPv4 — a fallback would bind a different
  address from the declared one.
* What is still refused: a hostname (`Q080`, `SOCKET-DNS`), the embedded-IPv4
  IPv6 form (`SOCKET-INET-V6-EMBEDDED-V4`), a zone identifier
  (`SOCKET-INET-V6-ZONE`), an IPv4 component with a leading zero, an IPv6 address
  written without its brackets, a port outside `0..=65535` (`Q084`), and an
  address that names a kernel-chosen port at a `connect` (`Q085` — an ephemeral
  port is not a name).

## What these examples do not show, because it does not exist in v1

* No string or text payload — a payload is a bracketed list of check-time byte
  constants (`SOCKET-STRING-PAYLOAD`).
* No `std.buffer` payload — a module that both buffers and sockets stays with the
  buffer route and keeps that route's refusal (`SOCKET-BUFFER-PAYLOAD`).
* No registration with `source.event.v1` **in these two programs**. That
  composition is no longer a gap: card `SOCKET-EVENT-COMPOSE` landed it, a
  module that both dispatches and sockets now reaches *this* route, which models
  both, and the published example is
  [`../events/socket_server.qr`](../events/socket_server.qr) — indexed on
  the events page because that is where a reader comes for "how do I wait on
  readiness".
* No non-blocking mode and no shutdown.
* No concurrency: `source.socket.v1` is single-threaded and a module that sockets
  and spawns keeps its `Q600`.
