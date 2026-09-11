# Strings

One program, on the `source.string.v1` route with its output composition (cards
`STRING-SLICE` and `STRING-SLICE-IO`).

| Program | What it shows |
|---|---|
| [status_line.qcore](status_line.qcore) | An HTTP/1.1 response head rendered to standard output: literal views, the empty line, a bound view and a slice of one |

## What a rendered string is

A Quirin `string` is an immutable `{ ptr, len }` view of UTF-8 bytes. Rendering
one writes **exactly those bytes**, framed by the sink's own line discipline:

* no re-encoding and no escaping — what is in the view is what reaches the
  stream;
* no terminator — a payload may contain a `0` as an ordinary byte inside its
  length, and the emitted C writes it with a length-bounded `fwrite` rather than
  any `printf` conversion, every one of which would stop there;
* no newline beyond the one every line already has, and the `[LEVEL] ` prefix of
  a `log` line is the same prefix a rendered number gets.

There is **no rendering selector** for a string. `dec` and `hex` are `std.fmt`'s
two total functions from an `i32` to digits; a `string` is already the bytes, so
there is nothing for either to do to one and writing `print dec s;` for a string
`s` is refused. That is also why this program imports `std.string` and `std.io`
and **not** `std.fmt`: it names no rendering.

## Running it

```sh
cargo test -p qcore-compiler --test string_slice_io
```

The test compiles this program through the real pipeline, builds its emitted C
under `-Wall -Wextra -Werror -pedantic`, runs it under ASan/UBSan/LSan, and
compares both streams with the reference execution and with the transcript
below, which it reads out of this file.

## Transcript

```text
HTTP/1.1 200 OK
Content-Type: text/plain; charset=utf-8
Content-Length: 13

hello, world
world
```

Nothing is written to the logger's stream: the program writes no `log` line.

## What this is not

It is not an HTTP implementation, and it makes no claim about one. There is no
socket, no request parsing, no header model and no length agreement between the
`Content-Length` line and the body — the two are independent literals, and a
reader should read this as *text reaching a stream* rather than as a protocol.
Equality, ordering, search, concatenation and mutation do not exist on this
route either; `docs/design/STRING_SLICE_V1.md` §3.7 states what they must be
when they arrive.
