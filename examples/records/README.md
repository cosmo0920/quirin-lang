# Records — `source.record.v1`

The first source-level aggregate type: a named record with a fixed list of
distinctly named fields, a construction, and a field read, admitted on
`source.buffer.v1` — the only route v1 admits it on. Card `AGGREGATES-V1`
landed the construct with `i32` fields only; the evidence package is
[`docs/reports/aggregates-v1/README.md`](../../docs/reports/aggregates-v1/README.md),
and this family is its named follow-on `EXAMPLES-AGGREGATES`. Card
`AGGREGATE-OWNED-FIELD` (wave 39) added the field that **owns** a buffer —
[`docs/reports/aggregate-owned-field/README.md`](../../docs/reports/aggregate-owned-field/README.md)
— and the third program below is its example.

| Program | What it shows |
|---|---|
| [`point.qcore`](point.qr) | CT09's "distinct ordinary struct fields" scenario — three fields, three different values, three reads — with no buffer anywhere: a record of scalars is a plain value, so this program's effect row is `io` alone |
| [`pixel_writes.qcore`](pixel_writes.qr) | A record beside a real resource: fields drive a buffer's stored values and one store's *index*, the buffer owes its `drop`, and the records owe nothing |
| [`owned_field.qcore`](owned_field.qr) | A record that **owns** two buffers: construction moves them in, the fields stand at every custody position a buffer name does, a field read moves one out, and `drop f;` releases the field still held — the four rules of the ownership IR, exercised in one program |

## The form

```ebnf
decl    = "type", NAME, "{", field, { ",", field }, [ "," ], "}" ;
field   = NAME, ":", ( "i32" | "bytes" ) ;
stmt    = "let", NAME, "=", NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
        | "let", NAME, "=", NAME, ".", NAME, ";"                 (* read, or move out *)
        | "let", NAME, "=", NAME, ".", NAME, "[", OPERAND, "]", ";"
        | NAME, ".", NAME, "[", OPERAND, "]", "=", OPERAND, ";"
        | "let", NAME, "=", "extent", "(", NAME, ".", NAME, ")", ";"
        | "drop", NAME, ".", NAME, ";"
        | "drop", NAME, ";" ;                                     (* a record that owns *)
assign  = NAME, ":", ( OPERAND | NAME ) ;       (* a buffer name at a `bytes` field *)
OPERAND = [ "-" ], INTEGER | NAME ;             (* `source.buffer.v1`'s own *)
```

`bytes` is the field type of an owned buffer, and it is the acquisition's own
already-reserved word (`let b = bytes(4);`), so the owned field reserves
nothing either. The task route's `Buffer` names a different resource with a
different model and was measured and not taken.

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
3. **A record of scalars carries no custody.** With every field an `i32`, a
   construction and a field read contribute no ownership operation, open no
   obligation and derive no effect. `drop c;` on such a record refuses at
   `Q052` with the route's own "is not a buffer this entry acquired".

Everything else is inherited. A field value is `source.buffer.v1`'s own
operand — an integer literal or a name bound to a check-time constant — and a
field read binds an ordinary scalar of that route's environment, so it stands
anywhere such a scalar does: a rendering operand, a stored byte, an index.
`pixel_writes.qcore`'s fourth store takes both its index and its value out of
one record, with nothing added to make that so.

## A record that owns

A field declared `bytes` holds an owned buffer, and the record is then a
**custody shape**, decided by the cycle-3 ownership IR addressing the field by
its path. Four rules, each the IR's rather than the route's:

1. **Construction moves in.** `Frame { tag: 7, head: h, body: b }` moves `h`
   and `b` into the record; from that statement on a use of `h` or `b` is
   refused at `Q054` as the checker's own "use after move".
2. **A field read of an owned field is a partial move.** `let taken = f.body;`
   hands the body to `taken`, which owes it, and leaves `f.body` a hole: a
   later `f.body[0]` is "use of `f.body` after move", at `Q054`.
3. **`drop f;` releases every field still held, in declaration order, and
   nothing twice.** A hole is tolerated. A second `drop f;` is a double drop
   of `f`; `drop f.head;` alone releases one field and leaves a hole.
4. **A leak sees into the record.** Remove `drop f;` and the checker refuses
   with "live resources escape normal exit: f.head" — the path, not the
   record.

A record with an owned field is never copyable, is still refused at every
scalar position (`Q052`, "read the field you mean first"), and its fields stand
at every position a buffer name does: `extent(f.body)`, `f.body[1] = 67;`,
`let b0 = f.body[0];`, `drop f.body;`. Two shapes are refused by name rather
than modelled: a `lend(…)` in an entry that composes such a record
(`OWNED-FIELD-LEND-WINDOW`), and an owned-field statement inside a `for` body
(`OWNED-FIELD-LOOP-BODY`), both at `Q051`.

The struct holds the pointer, and the initializer *is* the move:

<!-- emitted: frame-struct -->
```c
struct qcore_record_Frame {
    int32_t f_tag;
    unsigned char *f_head;
    unsigned char *f_body;
};
```

`owned_field.qcore` writes one stream — the scalar field, the extent read
through a field, one byte of each owned field, one byte written through a
field and read back, and one byte written through the moved-out binding:

<!-- transcript: owned_field stdout -->
```text
7
4
72
81
67
111
```

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

The test compiles all three programs through the real pipeline, asserts they
take the `source.buffer.v1` route, builds the emitted C under
`-std=c11 -Wall -Wextra -Werror -pedantic`, runs it under ASan/UBSan (and LSan
on Linux), and compares **both** streams with the reference execution and with
the transcripts above, which it reads out of this file.

The programs are each other's leak control. `point.qcore` allocates
nothing at all, so its sanitized run is a *negative* control: LSan reporting a
leak would mean the aggregate had acquired something the model says it cannot.
`pixel_writes.qcore` allocates one real buffer and releases it, so its run is
a *positive* control: a lost release would be a finding, and the records
beside the buffer change nothing about that. `owned_field.qcore` is the
positive control for the record that owns: one buffer is released through the
whole-record `drop`, the other through the binding it was moved out to, and
LSan would report either release going missing.

The card's own test file, `compiler/tests/aggregate_owned_field.rs`, carries
the refusal table with codes and the native run beside this suite's.

## What this is not

* **No field write.** `p.x = 5;` is not a statement of this revision
  (`AGGREGATE-FIELD-WRITE`); a v1 record is construct-then-read, and an owned
  field is moved in once and moved out at most once.
* **No view field, no nesting, no field type but `i32` and `bytes`.** Each is a
  named follow-on with the machinery it is missing recorded
  (`AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`, `AGGREGATE-FIELD-TYPES`).
* **No lend beside a record that owns, and no such record in a `for` body.**
  Both are refused by name (`OWNED-FIELD-LEND-WINDOW`,
  `OWNED-FIELD-LOOP-BODY`) rather than modelled.
* **One route.** v1 admits the construct on `source.buffer.v1` only; every
  other route keeps the refusal it already had, measured and pinned rather
  than widened (`AGGREGATE-ROUTE-WIDENING`).
* **No loop bound out of a field.** A field read stands at an index or a
  stored byte, but a `for` bound read out of a field is refused — the value is
  one the checker computes and the lowering does not
  (`AGGREGATE-BOUND-FROM-FIELD`).
