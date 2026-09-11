# `source.http.v1` examples — a checked, executed HTTP/1.1 subset in pure Quirin

Two published programs. Both are written entirely in Quirin source, lowered to
self-contained C11 by the existing backend, and **run** under ASan/UBSan/LSan
with their output byte-compared against the compiler's own reference execution.
Neither links a library, declares an extern function or reads a manifest: the
whole transport is the embedded shim `crates/qcore-core/src/qcore_socket_compat.h`
and the whole HTTP implementation is a byte sequence the compiler constructed.

Card `HTTP-QUIRIN-V1`; the evidence package is
`docs/reports/http-quirin-v1/README.md`, the checked model is
`crates/qcore-core/src/std_http.rs`, and the tests that hold these files to the
`examples/threads` standard are in `compiler/tests/std_http.rs`.

| File | Route | What it shows |
|---|---|---|
| [`exchange.qcore`](exchange.qcore) | `source.http.v1` | a **declared** request and response: the compiler builds and checks the wire bytes |
| [`raw_request.qcore`](raw_request.qcore) | `source.socket.v1` | the same kind of bytes **written by hand** as a string payload — and what the compiler then does *not* know |

## What is being claimed, and what is not

This is **an executed HTTP/1.1 subset with the boundary named.** It is not
HTTP/1.1 conformance, and no part of this directory should be read as claiming
it. §"What is out" below lists everything that is missing, each with the named
follow-on that owns it.

What *is* claimed: for the bounded subset these programs use, a family of
RFC 9112 facts is decided by the compiler, before anything runs, and a program
that breaks one is refused with its own diagnostic.

## The thing to understand first

Every component of a declared message is a quoted literal, so its **bytes — and
therefore its length — are known at check time.** That is what turns a protocol
rule into a compiler rule:

| The compiler decides, statically | Code | Written where |
|---|---|---|
| the version literal is exactly `HTTP/1.1` | `Q091` | `request`/`response` |
| the method is a `token`, and a lower-case `get` is a different method RFC 9110 §9.1 says no server implements | `Q091` | `request` |
| the request target is origin-form: `/path`, no space, no fragment | `Q091` | `request` |
| the status code is three digits in `100..=599` | `Q091` | `response` |
| a field name is a `token`, a field value is field-content with no surrounding whitespace | `Q092` | `header` |
| **no component may carry a `CR` or an `LF`** | `Q092` | every component |
| a request carries exactly one `Host` field (RFC 9112 §3.2) | `Q092` | `header` |
| a `Connection` option is one v1's transport actually honours | `Q092` | `header` |
| **a declared `Content-Length` equals the content's byte count** | `Q093` | `header` + `content` |
| content with no declared length, a duplicate length, and any `Transfer-Encoding` | `Q093` | `header` + `content` |

The `CR`/`LF` rule is the load-bearing one. No source program writes a `CRLF`:
the compiler writes them — one after the start line, one after each field, one
to end the head — and because no component may carry one, **every `CRLF` in a
message head is one the compiler put there.** Header injection is therefore not
expressible on this route, and the framing is a property rather than an
intention.

## The wire bytes `exchange.qcore` builds

Read out of the certificate by
`compiler/tests/std_http.rs::the_readme_wire_transcript_is_the_real_one`, so
this cannot drift from the program. Each line below ends with a `CRLF` the
compiler wrote; the blank line before the content is the empty line that ends
the head.

<!-- transcript: wire-rq -->
```text
GET /index.html HTTP/1.1
Host: 127.0.0.1
User-Agent: quirin/0.4
Accept: text/plain
Connection: close
```

<!-- transcript: wire-rs -->
```text
HTTP/1.1 200 OK
Content-Type: text/plain; charset=utf-8
Content-Length: 6
Connection: close

hello\n
```

## Both ends are in one program

`connect client listener;` names a **listening socket of this same program**,
not an address. That is the transport route's determinism rule, inherited whole:

1. the model carries the exact bytes each `send` commits, so a `recv` the
   program's own history does not satisfy is **refused at check time** (`Q095`)
   rather than blocking at run time;
2. because the model holds those bytes, a `recv NAME MESSAGE;` is a **verified**
   receive — the checker proves the arriving bytes are that message's, and the
   emitted program compares them again at run time and exits 88 if they differ.

An external peer is the named follow-on `SOCKET-EXTERNAL-PEER`; it is a change
to the determinism argument, not to the syntax.

## Building and running

```sh
quirin check examples/http/exchange.qcore
quirin emit examples/http/exchange.qcore -o /tmp/exchange.c
cc -std=c11 -Wall -Wextra -Werror -pedantic -g -O0 \
   -fsanitize=address,undefined -fno-omit-frame-pointer /tmp/exchange.c -o /tmp/exchange
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 /tmp/exchange
```

On Windows the same emitted file builds with `cl /W4 /WX` and needs no extra
library on the command line: the Windows arm of the transport shim names
`ws2_32.lib` from inside the emitted source. That arm is **compile-path-correct
only** here — no Windows or Apple run, sanitizer result or byte comparison is
claimed by this card (`HTTP-V1-WINDOWS-RUN`, `HTTP-V1-APPLE-RUN`).

## Transcripts

These blocks are read out of this file by
`compiler/tests/std_http.rs::the_readme_transcript_is_the_real_one` and compared
with what the pipeline really produces, so the page cannot drift from the
programs.

<!-- transcript: exchange -->
```text
200
6
```

<!-- transcript: raw_request -->
```text
204
```

v1 renders **scalars**. A rendered text line inside this route's transport body
would need `source.string.v1`'s own text emission there, which is the named
follow-on `HTTP-V1-TEXT-TRANSCRIPT`.

## The emitted C, and why there is only one `main`

`exchange.qcore` is the first program in this repository that genuinely
**composes two routes' emitted fragments**. The HTTP route contributes a banner
and one `static const unsigned char` array per transferred message; the socket
route contributes the compatibility header, the includes and the standard-IO
prelude; `c_program::merge` concatenates them, deduplicates the prelude both ask
for, and orders them by declared section; and `c_program::assemble` writes the
**one** entry point, whose body is the transport's statements. That is card
`C-EMIT-MAIN-UNIFY`'s architecture used as designed, and
`the_composed_program_has_exactly_one_entry_point` pins the result.

One message is one object in the emitted C. `send client rq;` and
`recv server rq;` both name `qcore_http_rq`, so "the server verifies exactly
what the client sent" is visible in the generated file rather than something a
reader has to check by comparing two arrays.

## What `raw_request.qcore` is for

It writes the same kind of bytes by hand, as a string payload
(`send C "GET / HTTP/1.1\r\n…";`) — the socket card's follow-on
`SOCKET-STRING-PAYLOAD`, landed by this card. `\r` is one of the escapes
`source.string.v1` defines, so a CRLF is spellable.

The point is the contrast. In that file the compiler checks that the receive is
satisfiable and that the declared bytes are the committed ones, and **nothing
else**: it does not know the payload is HTTP, so it does not require a `Host`
field, does not relate a `Content-Length` to anything, and would not notice a
`CRLF` in the wrong place or a missing final empty line. A payload a program
*writes* is bytes; a message a program *declares* is checked.

## What is out, and who owns it

Each of these is a named follow-on with a recorded blocker, not an oversight.

| Missing | Follow-on | Why |
|---|---|---|
| a `Content-Length` of a **computed** value | `STRING-SLICE-CONCAT` | v1 has no computed text at all: every payload is a literal or a slice of one. A length that is a number the program worked out needs a building operation, which needs storage the program owns |
| reading a message out of **unknown** input | `HTTP-V1-PARSE` | received bytes are verified against a declared expectation. Parsing needs a varying-value tier — a loop over an index the checker cannot fix — which no route in this language has |
| chunked transfer coding | `HTTP-V1-CHUNKED` | v1 frames every message by `Content-Length`; a `Transfer-Encoding` field refuses at `Q093` |
| persistent connections and pipelining | `HTTP-V1-PERSISTENT` | the emitted exchange closes after one message pair, so a `Connection` option other than `close` would be a claim the transport does not honour |
| `100 (Continue)` | `HTTP-V1-EXPECT-CONTINUE` | it is a two-response exchange whose second response depends on the first, which v1's declared-transfer model does not express |
| an external peer | `SOCKET-EXTERNAL-PEER` | every peer is a socket of the same program; that is what makes the receive rule decidable |
| TLS | `HTTP-V1-TLS` | no cryptography, no certificate handling, and no library to borrow either from |
| a message body that is a `std.buffer` extent | `HTTP-V1-BUFFER-CONTENT` | a module that buffers stays with the buffer route and keeps its refusal |
| `obs-text` (bytes `%x80-FF`) in a field value | `HTTP-V1-OBS-TEXT` | RFC 9110 §5.5 makes it opaque; a route whose evidence is a byte comparison declines to carry bytes whose meaning it cannot state |
| a message larger than 1024 bytes | `HTTP-V1-MESSAGE-BOUND` | the transport bounds one transfer so that a send provably never blocks |
| slicing, `len` or rendered text beside HTTP | `HTTP-V1-STRING-COMPOSE` | this route models a bound view as a payload and nothing else of `source.string.v1` |
| composition with `source.event.v1` | `HTTP-V1-EVENT-COMPOSE` | a module that dispatches stays with the event route |
