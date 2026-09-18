# Strings

Three programs, on the `source.string.v1` route with its output composition and
its `std.buffer` bridge (cards `STRING-SLICE`, `STRING-SLICE-IO` and
`STRING-CONCAT`), the third with a `for` inside the composition (card
`LOOPS-ON-COMPOSED-ROUTES`, wave 39, whose sanitized native run is
`compiler/tests/loops_on_composed_routes.rs`'s).

| Program | What it shows |
|---|---|
| [status_line.qcore](status_line.qcore) | An HTTP/1.1 response head rendered to standard output: literal views, the empty line, a bound view and a slice of one |
| [computed_length.qcore](computed_length.qcore) | The same head with a **computed** `Content-Length`: content the program writes into storage it owns, borrowed as UTF-8, its length taken, and the header line composed from that number |
| [looped_fill.qcore](looped_fill.qcore) | A `for` filling a buffer before it is borrowed, and a second `for` inside a live loan's window writing only the bytes the loan does not hold — the shape `STRING-CONCAT-LOOP-WINDOW` refused until the borrow window was decided over iterations |

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

## Computed text

`computed_length.qcore` uses the two operations card `STRING-CONCAT` added:

* **`borrow_utf8(b)`** views an owned `std.buffer`'s bytes as an immutable
  `string`. The UTF-8 validity decision is made **at check time**, out of the
  buffer's own write history — v1's buffers hold only constant bytes written at
  constant indices, so their contents are a fact the compiler already has. While
  the view is live the buffer may not be written or released; the window closes
  after the last statement that reads the view or anything derived from it.
* **`concat(PIECE, …)`** composes two or more check-time-known pieces — a quoted
  literal, a bound view, one inline slice, or `dec(NAME)`/`hex(NAME)` for the
  text of a bound integer — into a **new static payload**. The compiler computes
  the bytes, so there is no allocation, no run-time copy, and the result's length
  is a check-time constant exactly as a literal's own length is. That last
  property is the whole point: it is what makes a `Content-Length` the program
  derives as decidable as one it wrote.

`dec(n)` produces the same bytes `print dec n;` writes, because both go through
one rendering function. Neither operation derives `alloc` or `mut`; the `alloc`
and `mut` in this program's effect row are the **buffer's**.

## Running them

```sh
cargo test -p qcore-compiler --test string_slice_io   # status_line.qcore
cargo test -p qcore-compiler --test string_concat     # computed_length.qcore
```

Each test compiles its program through the real pipeline, builds the emitted C
under `-Wall -Wextra -Werror -pedantic`, runs it under ASan/UBSan/LSan, and
compares both streams with the reference execution and with the transcript
below, which it reads out of this file.

`compiler/tests/examples_suite.rs` adds the page-level half, without building
either program a second time: it reads each transcript below out of its marked
block and compares it with the reference execution, and it holds the table above
to the directory — every program indexed, every row a file that exists.

## Transcript — `status_line.qcore`

<!-- transcript: status_line stdout -->
```text
HTTP/1.1 200 OK
Content-Type: text/plain; charset=utf-8
Content-Length: 13

hello, world
world
```

Nothing is written to the logger's stream: the program writes no `log` line.

## Transcript — `computed_length.qcore`

Each head line ends with a carriage return the program wrote and a line feed the
sink's framing supplied, so the head below is CRLF-framed. The `11` is derived
from the content and appears nowhere in the source.

<!-- transcript: computed_length stdout -->
```text
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 11

{"ok":true}
```

## Transcript — `looped_fill.qcore`

The first loop runs before any view exists and writes every byte of the owner;
`all` then views `aaaa`. The second loop runs inside `head`'s window — `head`
views `[0, 2)` and is read after the loop — and writes bytes 2 and 3 at every
iteration, which the loan does not hold; `head` still reads `aa`, and a fresh
whole-extent view reads what the loop wrote. A store to byte 1 in that body
would be refused at `Q078`.

<!-- transcript: looped_fill stdout -->
```text
aaaa
aa
aabb
```

## What these are not

Neither of the first two is an HTTP implementation, and neither claims to be. There is no socket,
no request parsing and no header model — writing a message to a wire is
`source.http.v1`'s, and connecting a computed length to that route is the named
follow-on `HTTP-V1-COMPUTED-LENGTH`. In `status_line.qcore` the
`Content-Length` line and the body are two independent literals that no rule
relates; in `computed_length.qcore` they *are* related, and the remaining gap is
the last line's framing: the content's eleven bytes reach standard output
followed by the line feed every rendered line carries, which the message's
content does not contain.

Equality, ordering, search and mutation of text do not exist on this route;
`docs/design/STRING_SLICE_V1.md` §3.7 states what they must be when they arrive.
