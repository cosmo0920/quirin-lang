# Records — `source.record.v1`

The first source-level aggregate type: a named record with a fixed list of
distinctly named `i32` fields, a construction, and a field read, admitted on
`source.buffer.v1` — the only route v1 admits it on. Card `AGGREGATES-V1`
landed the construct; the evidence package is
[`docs/reports/aggregates-v1/README.md`](../../docs/reports/aggregates-v1/README.md),
and this family is its named follow-on `EXAMPLES-AGGREGATES`.

| Program | What it shows |
|---|---|
| [`point.qcore`](point.qcore) | CT09's "distinct ordinary struct fields" scenario — three fields, three different values, three reads — with no buffer anywhere: a record is a plain value, so this program's effect row is `io` alone |
| [`pixel_writes.qcore`](pixel_writes.qcore) | A record beside a real resource: fields drive a buffer's stored values and one store's *index*, the buffer owes its `drop`, and the records owe nothing |

## The form

```ebnf
decl    = "type", NAME, "{", field, { ",", field }, [ "," ], "}" ;
field   = NAME, ":", "i32" ;
stmt    = "let", NAME, "=", NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
        | "let", NAME, "=", NAME, ".", NAME, ";" ;
assign  = NAME, ":", OPERAND ;
OPERAND = [ "-" ], INTEGER | NAME ;             (* `source.buffer.v1`'s own *)
```

The declaration head is the already-reserved `type`, told from the alias
`type NAME = …;` by the `{` — no word is reserved for any of this, and a field
name is never collected as a reference, so fields named `drop`, `close` or
`print` would all be ordinary. The one exception is six words this family's
programs deliberately avoid: a type or field named for one of the
whole-token-stream route selectors (`spawn`, `join`, `qcore_buffer_new`,
`qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload`) selects the
concurrency route from its own spelling and refuses fail-closed — `Q015`
declared, `Q000` constructed (`AGGREGATE-SELECTOR-WORDS`).

## The three rules a record adds, and nothing else

1. **Formation decides distinctness.** A type declares 1 to 32 fields and no
   two may share a name. The bound is a refusal, never a truncation: every
   field of every constructed value is published in the certificate and
   emitted as a distinct C member, so a record the model cannot enumerate is
   not modelled at all.
2. **Construction is total.** Every declared field is supplied exactly once
   and nothing else is — which is what makes the field read a function rather
   than a partial rule with an error arm, and what the refused program below
   pins.
3. **A record carries no custody.** Every field is an `i32`, so a construction
   and a field read contribute no ownership operation, open no obligation and
   derive no effect. `drop c;` on a record refuses at `Q052` with the route's
   own "is not a buffer this entry acquired".

Everything else is inherited. A field value is `source.buffer.v1`'s own
operand — an integer literal or a name bound to a check-time constant — and a
field read binds an ordinary scalar of that route's environment, so it stands
anywhere such a scalar does: a rendering operand, a stored byte, an index.
`pixel_writes.qcore`'s fourth store takes both its index and its value out of
one record, with nothing added to make that so.

## What the C looks like

One struct per declared type, one member per field in declared order, one
designated initializer per construction, one member read per field read:

<!-- emitted: point-struct -->
```c
struct qcore_record_Point {
    int32_t f_x;
    int32_t f_y;
    int32_t f_weight;
};
```

The `qcore_record_` and `f_` prefixes are load-bearing — a QCore type or field
name is a QCore word and nothing stops it being a C keyword — and `r_` is a
third binding prefix beside `b_` and `s_`, so a record, a buffer and a scalar
of one source name are three distinct C objects. Constructions are emitted in
*declared* order however the source wrote them: `point.qcore`'s second value is
written `{ weight: 9, y: 2, x: 1 }` and initializes `.f_x, .f_y, .f_weight`.

## Transcripts

`point.qcore` writes both streams, and no order is claimed between them: C
models none between two separately buffered streams, so each is compared on
its own. Standard output carries the first value's three fields —

<!-- transcript: point stdout -->
```text
65
66
40
```

— and standard error carries the second value's two, read in the reverse of
declared order:

<!-- transcript: point stderr -->
```text
[INFO] 9
[INFO] 1
```

Three *different* numbers out of three declared fields is the point: a model
that aliased two fields, or a lowering that gave two fields one C member,
would print one value twice.

`pixel_writes.qcore` writes one stream — the four bytes read back out of the
buffer, `QCor` in ASCII, the same payload the loops family copies:

<!-- transcript: pixel_writes stdout -->
```text
81
67
111
114
```

## A refused program

A construction that does not supply every declared field is refused rather
than defaulted:

<!-- refusal: Q051 -->
```qcore
import std.fmt;
import std.io;

type Point { x: i32, y: i32 }

fn main() -> unit effects { io } {
    let p = Point { x: 1 };
    print dec 7;
    return;
}
```

and refused like this:

<!-- diagnostic: Q051 -->
```text
refusal.qcore:7:5: Q051: this `Point` construction supplies no `y`; `source.record.v1` constructs a record only from a complete field list, so every field of a constructed value has a value a reader can predict
    let p = Point { x: 1 };
    ^
```

Both blocks are read out of this file by the test: the program above is
compiled, and the diagnostic below is compared with what the compiler really
said, span caret included. Totality is what makes the field read total in
turn — a constructed value has a value for every field it can be asked for.

Four more refusals the rules above name, each checked by the same test:

| Written | Refused at |
|---|---|
| `drop c;` on a record | `Q052` — "is not a buffer this entry acquired" |
| `let z = p.z;` — a field `Point` does not declare | `Q052` |
| `let p = Square { … };` — a type nothing declares | `Q013`, name resolution, before this route's reader runs |
| `pixel_writes.qcore` without its `drop pixels;` | `Q054`, the unedited ownership checker — the record beside it launders nothing |

## Running it

```sh
cargo test -p qcore-compiler --test examples_suite
```

The test compiles both programs through the real pipeline, asserts they take
the `source.buffer.v1` route, builds the emitted C under
`-std=c11 -Wall -Wextra -Werror -pedantic`, runs it under ASan/UBSan (and LSan
on Linux), and compares **both** streams with the reference execution and with
the transcripts above, which it reads out of this file.

The two programs are each other's leak control. `point.qcore` allocates
nothing at all, so its sanitized run is a *negative* control: LSan reporting a
leak would mean the aggregate had acquired something the model says it cannot.
`pixel_writes.qcore` allocates one real buffer and releases it, so its run is
a *positive* control: a lost release would be a finding, and the records
beside the buffer change nothing about that.

## What this is not

* **No field write.** `p.x = 5;` is not a statement of this revision
  (`AGGREGATE-FIELD-WRITE`); a v1 record is construct-then-read.
* **No owned or view fields, no nesting, no non-`i32` field.** Each is a named
  follow-on with the machinery it is missing recorded
  (`AGGREGATE-OWNED-FIELD`, `AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`,
  `AGGREGATE-FIELD-TYPES`).
* **One route.** v1 admits the construct on `source.buffer.v1` only; every
  other route keeps the refusal it already had, measured and pinned rather
  than widened (`AGGREGATE-ROUTE-WIDENING`).
* **No loop bound out of a field.** A field read stands at an index or a
  stored byte, but a `for` bound read out of a field is refused — the value is
  one the checker computes and the lowering does not
  (`AGGREGATE-BOUND-FROM-FIELD`).
